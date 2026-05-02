use axum::response::sse::{Event, KeepAlive, Sse};
use axum::{extract::State, Json};
use futures::StreamExt;
use serde_json::json;
use uuid::Uuid;

use blup_agent::step::*;

use super::helpers::{from_agent_value, load_or_404, next_sse_id};
use super::types::SseEvent;
use crate::error::ApiError;
use crate::state::domain as d;
use crate::state::types::Transition;
use crate::AppState;

// ── submit_goal (sync) ──

pub async fn submit_goal(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(goal): Json<d::LearningGoal>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if goal.description.len() < 10 {
        return Err(ApiError::Validation(
            "description must be at least 10 characters".to_string(),
        ));
    }

    let handle = load_or_404(&state, id).await?;

    let (goal_desc, goal_domain, goal_context, write_version) =
        advance_goal_state(&state, &handle, &goal, id).await?;

    let feasibility_value = state
        .agent
        .check_feasibility(&FeasibilityContext {
            learning_goal: goal_desc,
            domain: goal_domain,
            context: if goal_context.is_empty() {
                None
            } else {
                Some(goal_context)
            },
        })
        .await
        .map_err(ApiError::from)?;

    let feasibility: d::FeasibilityResult = from_agent_value(feasibility_value)?;

    let committed = state
        .store
        .try_mutate(id, write_version, |s| {
            let feasible = feasibility.feasible;
            s.feasibility_result = Some(feasibility.clone());
            if feasible {
                if let Err(e) = s.state_machine.transition(Transition::GoalFeasible) {
                    tracing::error!(error = %e, "Transition GoalFeasible failed unexpectedly");
                }
            } else if let Err(e) = s.state_machine.transition(Transition::GoalInfeasible) {
                tracing::error!(error = %e, "Transition GoalInfeasible failed unexpectedly");
            }
            s.state().to_string()
        })
        .await;

    match committed {
        Some(state_name) => {
            persist_feasibility(&state, id, &feasibility, &state_name).await;
            Ok(Json(json!({
                "feasibility": feasibility,
                "state": state_name
            })))
        }
        None => Err(ApiError::InvalidTransition(
            "Session was modified by another request; please retry".to_string(),
        )),
    }
}

// ── submit_goal_stream (SSE) ──

pub async fn submit_goal_stream(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(goal): Json<d::LearningGoal>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>, ApiError> {
    if goal.description.len() < 10 {
        return Err(ApiError::Validation(
            "description must be at least 10 characters".to_string(),
        ));
    }

    let handle = load_or_404(&state, id).await?;

    let (goal_desc, goal_domain, goal_context, _version) =
        advance_goal_state(&state, &handle, &goal, id).await?;

    let ping_interval = std::time::Duration::from_secs(state.config.sse_ping_interval_secs);
    let stream_state = state.clone();
    let stream_handle = handle.clone();

    let agent_stream = state.agent.check_feasibility_stream(FeasibilityContext {
        learning_goal: goal_desc,
        domain: goal_domain,
        context: if goal_context.is_empty() {
            None
        } else {
            Some(goal_context)
        },
    });

    let stream = async_stream::stream! {
        let mut agent_stream = std::pin::pin!(agent_stream);
        while let Some(event_result) = agent_stream.next().await {
            match event_result {
                Ok(AgentStreamEvent::Status { state: st, message }) => {
                    yield Ok(Event::default()
                        .event("status")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Status { state: st, message })
                            .expect("SSE serialize")));
                }
                Ok(AgentStreamEvent::Chunk { content, index }) => {
                    yield Ok(Event::default()
                        .event("chunk")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Chunk { content, index })
                            .expect("SSE serialize")));
                }
                Ok(AgentStreamEvent::Error { code, message }) => {
                    yield Ok(Event::default()
                        .event("error")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Error { code, message })
                            .expect("SSE serialize")));
                    return;
                }
                Ok(AgentStreamEvent::Done { result }) => {
                    if let Some(feasibility_val) = result.get("feasibility") {
                        if let Ok(feasibility) = serde_json::from_value::<d::FeasibilityResult>(feasibility_val.clone()) {
                            let feasible = feasibility.feasible;
                            let session_id_for_storage = stream_handle.read().await.id;
                            let mut s = stream_handle.write().await;
                            s.feasibility_result = Some(feasibility);
                            let state_name = if feasible {
                                if let Err(e) = s.state_machine.transition(Transition::GoalFeasible) {
                                    tracing::error!(error = %e, "Transition GoalFeasible failed unexpectedly");
                                }
                                "PROFILE_COLLECTION"
                            } else {
                                if let Err(e) = s.state_machine.transition(Transition::GoalInfeasible) {
                                    tracing::error!(error = %e, "Transition GoalInfeasible failed unexpectedly");
                                }
                                "GOAL_INPUT"
                            };
                            s.updated_at = chrono::Utc::now();
                            stream_state.store.persist(s.id);

                            if let Some(ref fr) = s.feasibility_result {
                                if let Ok(fr_json) = serde_json::to_value(fr) {
                                    if let Err(e) = stream_state.storage.save_feasibility_result(session_id_for_storage, fr_json).await {
                                        tracing::warn!(session_id = %session_id_for_storage, error = %e, "Failed to persist feasibility result to storage (SSE)");
                                    }
                                }
                            }
                            if let Err(e) = stream_state.storage.update_session_state(session_id_for_storage, state_name).await {
                                tracing::warn!(session_id = %session_id_for_storage, error = %e, "Failed to update session state in storage (SSE)");
                            }
                        }
                    }
                    yield Ok(Event::default()
                        .event("done")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Done { result })
                            .expect("SSE serialize")));
                    return;
                }
                Err(e) => {
                    yield Ok(Event::default()
                        .event("error")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Error {
                            code: "AGENT_ERROR".to_string(),
                            message: e.to_string(),
                        }).expect("SSE serialize")));
                    return;
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(ping_interval)))
}

// ── Shared helpers ──

/// Advance the goal state and persist the goal. Returns (desc, domain, context, version).
async fn advance_goal_state(
    state: &AppState,
    handle: &crate::state::session::SessionHandle,
    goal: &d::LearningGoal,
    id: Uuid,
) -> Result<(String, String, String, u64), ApiError> {
    let mut s = handle.write().await;
    let current = s.state();
    match current {
        crate::state::types::SessionState::Idle => {
            s.state_machine.transition(Transition::SubmitGoal)?;
            s.state_machine.transition(Transition::SubmitGoal)?;
        }
        crate::state::types::SessionState::GoalInput => {
            s.state_machine.transition(Transition::SubmitGoal)?;
        }
        crate::state::types::SessionState::FeasibilityCheck => {
            // Previous attempt failed mid-way; allow retry without transitions.
        }
        _ => {
            return Err(ApiError::InvalidTransition(format!(
                "Cannot submit goal in state {current}"
            )));
        }
    }
    s.goal = Some(d::LearningGoal {
        description: goal.description.clone(),
        domain: goal.domain.clone(),
        context: goal.context.clone(),
        current_level: goal.current_level.clone(),
    });
    s.version += 1;
    s.updated_at = chrono::Utc::now();
    let v = s.version;
    state.store.persist(s.id);

    let goal_json = serde_json::to_value(goal).unwrap_or_default();
    if let Err(e) = state.storage.save_goal(id, goal_json).await {
        tracing::warn!(session_id = %id, error = %e, "Failed to persist goal to storage");
    }
    if let Err(e) = state
        .storage
        .update_session_state(id, "FEASIBILITY_CHECK")
        .await
    {
        tracing::warn!(session_id = %id, error = %e, "Failed to update session state in storage");
    }

    Ok((
        goal.description.clone(),
        goal.domain.clone(),
        goal.context.clone().unwrap_or_default(),
        v,
    ))
}

async fn persist_feasibility(
    state: &AppState,
    id: Uuid,
    feasibility: &d::FeasibilityResult,
    state_name: &str,
) {
    if let Ok(feasibility_json) = serde_json::to_value(feasibility) {
        if let Err(e) = state
            .storage
            .save_feasibility_result(id, feasibility_json)
            .await
        {
            tracing::warn!(session_id = %id, error = %e, "Failed to persist feasibility result to storage");
        }
        if let Err(e) = state.storage.update_session_state(id, state_name).await {
            tracing::warn!(session_id = %id, error = %e, "Failed to update session state in storage");
        }
    }
}
