use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use axum::response::sse::{Event, KeepAlive, Sse};
use axum::{extract::State, Json};
use futures::StreamExt;
use serde_json::json;
use uuid::Uuid;

use blup_agent::step::*;

use super::types::*;
use crate::error::ApiError;
use crate::state::domain as d;
use crate::state::domain::SessionMessage;
use crate::state::session::{SessionHandle, SessionListEntry};
use crate::state::types::Transition;
use crate::AppState;

static START_TIME: std::sync::LazyLock<Instant> = std::sync::LazyLock::new(Instant::now);
static SSE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_sse_id() -> String {
    SSE_COUNTER.fetch_add(1, Ordering::Relaxed).to_string()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn load_or_404(state: &AppState, id: Uuid) -> Result<SessionHandle, ApiError> {
    state.store.get(id).await.ok_or(ApiError::NotFound)
}

const PROFILE_ROUNDS_NEEDED: u32 = 3;

/// Convert agent engine Value output into a typed domain struct.
fn from_agent_value<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T, ApiError> {
    serde_json::from_value(value)
        .map_err(|e| ApiError::Internal(format!("Failed to parse agent output: {e}")))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let uptime_secs = START_TIME.elapsed().as_secs();
    let session_count = state.store.count().await;
    Json(json!({
        "status": "ok",
        "version": "0.1.0",
        "uptime_secs": uptime_secs,
        "session_count": session_count,
        "model": state.config.llm_model,
    }))
}

pub async fn create_session(
    State(state): State<AppState>,
) -> Result<Json<CreateSessionResponse>, ApiError> {
    match state.store.create().await {
        Some(handle) => {
            let s = handle.read().await;
            Ok(Json(CreateSessionResponse {
                session_id: s.id.to_string(),
                state: s.state().to_string(),
            }))
        }
        None => Err(ApiError::ServiceUnavailable),
    }
}

pub async fn list_sessions(State(state): State<AppState>) -> Json<Vec<SessionListEntry>> {
    Json(state.store.list().await)
}

pub async fn delete_session(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.store.delete(id).await;
    Ok(Json(json!({ "deleted": true })))
}

const SNAPSHOT_MESSAGE_LIMIT: usize = 50;

pub async fn get_session_status(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;
    let s = handle.read().await;
    let messages_total = s.messages.len();
    let messages = if messages_total > SNAPSHOT_MESSAGE_LIMIT {
        &s.messages[messages_total - SNAPSHOT_MESSAGE_LIMIT..]
    } else {
        &s.messages[..]
    };
    Ok(Json(json!({
        "session_id": s.id.to_string(),
        "state": s.state().to_string(),
        "goal": s.goal,
        "feasibility_result": s.feasibility_result,
        "profile": s.profile,
        "profile_rounds": s.profile_rounds,
        "curriculum": s.curriculum,
        "current_chapter_id": s.current_chapter_id,
        "chapter_contents": s.chapter_contents,
        "messages": messages,
        "messages_total": messages_total,
    })))
}

// ---- submit_goal ----

pub async fn submit_goal(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(goal): Json<LearningGoal>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if goal.description.len() < 10 {
        return Err(ApiError::Validation(
            "description must be at least 10 characters".to_string(),
        ));
    }

    let handle = load_or_404(&state, id).await?;

    let (goal_desc, goal_domain, goal_context, write_version) = {
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
        (
            goal.description.clone(),
            goal.domain.clone(),
            goal.context.clone().unwrap_or_default(),
            v,
        )
    };

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
                let _ = s.state_machine.transition(Transition::GoalFeasible);
            } else {
                let _ = s.state_machine.transition(Transition::GoalInfeasible);
            };
            s.state().to_string()
        })
        .await;

    match committed {
        Some(state_name) => Ok(Json(json!({
            "feasibility": feasibility,
            "state": state_name
        }))),
        None => Err(ApiError::InvalidTransition(
            "Session was modified by another request; please retry".to_string(),
        )),
    }
}

// ---- submit_goal_stream (SSE) ----

pub async fn submit_goal_stream(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(goal): Json<LearningGoal>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>, ApiError> {
    if goal.description.len() < 10 {
        return Err(ApiError::Validation(
            "description must be at least 10 characters".to_string(),
        ));
    }

    let handle = load_or_404(&state, id).await?;

    let (goal_desc, goal_domain, goal_context) = {
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
        state.store.persist(s.id);
        (
            goal.description.clone(),
            goal.domain.clone(),
            goal.context.clone().unwrap_or_default(),
        )
    };

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
                    // Parse feasibility from the result
                    if let Some(feasibility_val) = result.get("feasibility") {
                        if let Ok(feasibility) = serde_json::from_value::<d::FeasibilityResult>(feasibility_val.clone()) {
                            let mut s = stream_handle.write().await;
                            let feasible = feasibility.feasible;
                            s.feasibility_result = Some(feasibility);
                            let _ = if feasible {
                                s.state_machine.transition(Transition::GoalFeasible)
                            } else {
                                s.state_machine.transition(Transition::GoalInfeasible)
                            };
                            s.updated_at = chrono::Utc::now();
                            stream_state.store.persist(s.id);
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

// ---- submit_profile_answer ----

pub async fn submit_profile_answer(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(answer): Json<ProfileAnswer>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;

    let (goal_desc, goal_domain, current_round, is_complete) = {
        let mut s = handle.write().await;
        let current = s.profile_rounds;
        if current < PROFILE_ROUNDS_NEEDED - 1 {
            s.state_machine.transition(Transition::ProfileContinue)?;
        } else {
            s.state_machine.transition(Transition::ProfileComplete)?;
        }
        s.profile_rounds += 1;
        let new_round = s.profile_rounds;
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);

        let desc = s
            .goal
            .as_ref()
            .map(|g| g.description.as_str())
            .unwrap_or("Unknown goal")
            .to_string();
        let domain = s
            .goal
            .as_ref()
            .map(|g| g.domain.as_str())
            .unwrap_or("general")
            .to_string();
        (desc, domain, new_round, new_round >= PROFILE_ROUNDS_NEEDED)
    };

    let profile_step = state
        .agent
        .collect_profile(&ProfileContext {
            learning_goal: goal_desc,
            domain: goal_domain,
            answer: answer.answer.clone(),
            round: current_round,
            total_rounds: PROFILE_ROUNDS_NEEDED,
            is_final: is_complete,
        })
        .await
        .map_err(ApiError::from)?;

    match profile_step {
        ProfileStep::Complete { profile } => {
            let typed_profile: d::UserProfile = from_agent_value(profile)?;
            let mut s = handle.write().await;
            s.profile = Some(typed_profile.clone());
            s.updated_at = chrono::Utc::now();
            state.store.persist(s.id);
            Ok(Json(json!({
                "is_complete": true,
                "profile": typed_profile,
                "state": "CURRICULUM_PLANNING"
            })))
        }
        ProfileStep::Intermediate {
            round,
            total_rounds,
            next_question_hint,
        } => Ok(Json(json!({
            "is_complete": false,
            "round": round,
            "total_rounds": total_rounds,
            "next_question": next_question_hint,
            "state": "PROFILE_COLLECTION"
        }))),
    }
}

// ---- get_curriculum ----

pub async fn get_curriculum(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;

    let (goal_desc, profile_json) = {
        let s = handle.read().await;
        let current = s.state();
        if current != crate::state::types::SessionState::ChapterLearning
            && current != crate::state::types::SessionState::CurriculumPlanning
        {
            return Err(ApiError::InvalidTransition(format!(
                "Cannot get curriculum in state {current}"
            )));
        }

        if let Some(ref curriculum) = s.curriculum {
            return Ok(Json(
                serde_json::to_value(curriculum).map_err(|e| ApiError::Internal(e.to_string()))?,
            ));
        }

        let desc = s
            .goal
            .as_ref()
            .map(|g| g.description.as_str())
            .unwrap_or("Unknown goal")
            .to_string();
        let profile = s
            .profile
            .clone()
            .map(|p| {
                serde_json::to_value(p).unwrap_or(json!({
                    "experience_level": {"domain_knowledge": "beginner"},
                    "learning_style": {"preferred_format": ["text"]},
                    "available_time": {"hours_per_week": 5}
                }))
            })
            .unwrap_or_else(|| {
                json!({
                    "experience_level": {"domain_knowledge": "beginner"},
                    "learning_style": {"preferred_format": ["text"]},
                    "available_time": {"hours_per_week": 5}
                })
            });
        (desc, profile)
    };

    let curriculum_value = state
        .agent
        .generate_curriculum(&CurriculumContext {
            learning_goal: goal_desc,
            profile: profile_json,
        })
        .await
        .map_err(ApiError::from)?;

    let curriculum: d::CurriculumPlan = from_agent_value(curriculum_value)?;

    {
        let mut s = handle.write().await;
        s.curriculum = Some(curriculum.clone());
        let _ = s.state_machine.transition(Transition::CurriculumReady);
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);
    }

    Ok(Json(
        serde_json::to_value(curriculum).map_err(|e| ApiError::Internal(e.to_string()))?,
    ))
}

// ---- start_chapter ----

pub async fn start_chapter(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;

    let (chapter_title, profile_json) = {
        let s = handle.read().await;
        let current = s.state();
        if current != crate::state::types::SessionState::ChapterLearning {
            return Err(ApiError::InvalidTransition(format!(
                "Cannot start chapter in state {current}"
            )));
        }

        if let Some(cached) = s.chapter_contents.get(&ch_id).cloned() {
            return Ok(Json(json!({
                "id": Uuid::new_v4().to_string(),
                "role": "assistant",
                "content": cached,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })));
        }

        let title = s
            .curriculum
            .as_ref()
            .and_then(|c| c.chapters.iter().find(|ch| ch.id == ch_id))
            .map(|ch| ch.title.as_str())
            .unwrap_or(&ch_id)
            .to_string();

        let profile = s
            .profile
            .clone()
            .map(|p| {
                serde_json::to_value(p).unwrap_or(json!({
                    "experience_level": {"domain_knowledge": "beginner"},
                    "learning_style": {"preferred_format": ["text"]},
                    "available_time": {"hours_per_week": 5}
                }))
            })
            .unwrap_or_else(|| {
                json!({
                    "experience_level": {"domain_knowledge": "beginner"},
                    "learning_style": {"preferred_format": ["text"]},
                    "available_time": {"hours_per_week": 5}
                })
            });
        (title, profile)
    };

    let content = state
        .agent
        .teach_chapter(&ChapterContext {
            chapter_id: ch_id.clone(),
            chapter_title,
            profile: profile_json,
            curriculum_context: json!({}),
        })
        .await
        .map_err(ApiError::from)?;

    {
        let mut s = handle.write().await;
        s.current_chapter_id = Some(ch_id.clone());
        s.chapter_contents.insert(ch_id.clone(), content.clone());
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);
    }

    Ok(Json(json!({
        "id": Uuid::new_v4().to_string(),
        "role": "assistant",
        "content": content,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

// ---- start_chapter_stream (SSE) ----

pub async fn start_chapter_stream(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>, ApiError> {
    let handle = load_or_404(&state, id).await?;

    let (cached, chapter_title, profile_json) = {
        let s = handle.read().await;
        let current = s.state();
        if current != crate::state::types::SessionState::ChapterLearning {
            return Err(ApiError::InvalidTransition(format!(
                "Cannot start chapter in state {current}"
            )));
        }

        let cached = s.chapter_contents.get(&ch_id).cloned();

        let title = s
            .curriculum
            .as_ref()
            .and_then(|c| c.chapters.iter().find(|ch| ch.id == ch_id))
            .map(|ch| ch.title.as_str())
            .unwrap_or(&ch_id)
            .to_string();

        let profile = s
            .profile
            .clone()
            .map(|p| {
                serde_json::to_value(p).unwrap_or(json!({
                    "experience_level": {"domain_knowledge": "beginner"},
                    "learning_style": {"preferred_format": ["text"]},
                    "available_time": {"hours_per_week": 5}
                }))
            })
            .unwrap_or_else(|| {
                json!({
                    "experience_level": {"domain_knowledge": "beginner"},
                    "learning_style": {"preferred_format": ["text"]},
                    "available_time": {"hours_per_week": 5}
                })
            });

        (cached, title, profile)
    };

    let ping_interval = std::time::Duration::from_secs(state.config.sse_ping_interval_secs);
    let stream_state = state.clone();
    let stream_handle = handle.clone();
    let stream_ch_id = ch_id.clone();

    let stream = async_stream::stream! {
        yield Ok(Event::default()
            .event("status")
            .id(next_sse_id())
            .data(serde_json::to_string(&SseEvent::Status {
                state: "CHAPTER_LEARNING".to_string(),
                message: format!("Loading chapter: {chapter_title}"),
            }).expect("SSE serialize")));

        if let Some(ref cached_content) = cached {
            yield Ok(Event::default()
                .event("done")
                .id(next_sse_id())
                .data(serde_json::to_string(&SseEvent::Done {
                    result: json!({
                        "chapter_id": stream_ch_id,
                        "content": cached_content,
                    }),
                }).expect("SSE serialize")));
            return;
        }

        let agent_stream = stream_state.agent.teach_chapter_stream(ChapterContext {
            chapter_id: stream_ch_id.clone(),
            chapter_title,
            profile: profile_json,
            curriculum_context: json!({}),
        });

        let mut agent_stream = std::pin::pin!(agent_stream);
        let mut full_content = String::new();
        let mut chunk_count = 0u32;

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
                    full_content.push_str(&content);
                    chunk_count += 1;
                    yield Ok(Event::default()
                        .event("chunk")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Chunk { content, index })
                            .expect("SSE serialize")));

                    // Checkpoint partial content every 5 chunks
                    if chunk_count.is_multiple_of(5) && !full_content.is_empty() {
                        let mut s = stream_handle.write().await;
                        s.chapter_contents.insert(stream_ch_id.clone(), full_content.clone());
                        s.updated_at = chrono::Utc::now();
                    }
                }
                Ok(AgentStreamEvent::Error { code, message }) => {
                    yield Ok(Event::default()
                        .event("error")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Error { code, message })
                            .expect("SSE serialize")));
                    return;
                }
                Ok(AgentStreamEvent::Done { .. }) => {
                    {
                        let mut s = stream_handle.write().await;
                        s.current_chapter_id = Some(stream_ch_id.clone());
                        s.chapter_contents.insert(stream_ch_id.clone(), full_content.clone());
                        s.updated_at = chrono::Utc::now();
                        stream_state.store.persist(s.id);
                    }
                    yield Ok(Event::default()
                        .event("done")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Done {
                            result: json!({
                                "chapter_id": stream_ch_id,
                                "content": full_content,
                            }),
                        }).expect("SSE serialize")));
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

// ---- ask_question ----

pub async fn ask_question(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
    Json(question): Json<QuestionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if question.question.is_empty() {
        return Err(ApiError::Validation(
            "question must not be empty".to_string(),
        ));
    }

    let (profile_json, chapter_content, conversation_history, curriculum_context) = {
        let handle = load_or_404(&state, id).await?;
        let s = handle.read().await;
        let current = s.state();
        if current != crate::state::types::SessionState::ChapterLearning {
            return Err(ApiError::InvalidTransition(format!(
                "Cannot ask question in state {current}"
            )));
        }
        let profile = s
            .profile
            .clone()
            .map(|p| {
                serde_json::to_value(p).unwrap_or(json!({
                    "experience_level": {"domain_knowledge": "beginner"},
                    "learning_style": {"preferred_format": ["text"]},
                    "available_time": {"hours_per_week": 5}
                }))
            })
            .unwrap_or_else(|| {
                json!({
                    "experience_level": {"domain_knowledge": "beginner"},
                    "learning_style": {"preferred_format": ["text"]},
                    "available_time": {"hours_per_week": 5}
                })
            });

        let ch_content = s.chapter_contents.get(&ch_id).cloned().unwrap_or_default();

        let history: Vec<&SessionMessage> = s
            .messages
            .iter()
            .filter(|m| {
                m.chapter_id
                    .as_deref()
                    .map(|cid| cid == ch_id)
                    .unwrap_or(false)
            })
            .collect();

        let curriculum = s
            .curriculum
            .clone()
            .map(|c| serde_json::to_value(c).unwrap_or(json!({"title": "Unknown", "chapters": []})))
            .unwrap_or(json!({"title": "Unknown", "chapters": []}));

        (
            profile,
            ch_content,
            serde_json::to_string(&history).unwrap_or_default(),
            serde_json::to_string(&curriculum).unwrap_or_default(),
        )
    };

    let content = state
        .agent
        .answer_question(&QaContext {
            question: question.question.clone(),
            chapter_content,
            profile: profile_json,
            conversation_history: serde_json::from_str(&conversation_history).unwrap_or(json!([])),
            curriculum_context: serde_json::from_str(&curriculum_context).unwrap_or(json!({})),
        })
        .await
        .map_err(ApiError::from)?;

    let now = chrono::Utc::now().to_rfc3339();
    let user_msg = SessionMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        content: question.question,
        chapter_id: Some(ch_id.clone()),
        timestamp: now.clone(),
        content_type: None,
        metadata: None,
    };
    let assistant_msg = SessionMessage {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: content.clone(),
        chapter_id: Some(ch_id),
        timestamp: now,
        content_type: None,
        metadata: None,
    };

    {
        let handle = load_or_404(&state, id).await?;
        let mut s = handle.write().await;
        s.messages.push(user_msg);
        s.messages.push(assistant_msg.clone());
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);
    }

    Ok(Json(
        serde_json::to_value(&assistant_msg).map_err(|e| ApiError::Internal(e.to_string()))?,
    ))
}

// ---- get_messages_paginated ----

#[derive(serde::Deserialize)]
pub struct MessagesQuery {
    page: Option<u32>,
    per_page: Option<u32>,
}

pub async fn get_messages_paginated(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    axum::extract::Query(params): axum::extract::Query<MessagesQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;
    let s = handle.read().await;

    let per_page = params.per_page.unwrap_or(50).min(200) as usize;
    let page = params.page.unwrap_or(1).max(1) as usize;
    let total = s.messages.len();
    let total_pages = ((total as f64) / (per_page as f64)).ceil() as usize;

    let start = ((page - 1) * per_page).min(total);
    let end = (start + per_page).min(total);
    let page_messages = &s.messages[start..end];

    Ok(Json(json!({
        "messages": page_messages,
        "page": page,
        "per_page": per_page,
        "total": total,
        "total_pages": total_pages,
    })))
}

// ---- complete_chapter ----

pub async fn complete_chapter(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;
    let mut s = handle.write().await;

    s.state_machine.transition(Transition::ChapterComplete)?;

    let progress = d::ChapterProgress {
        chapter_id: ch_id.clone(),
        status: "completed".to_string(),
        completion: 100.0,
        time_spent_minutes: None,
        exercises_completed: None,
        exercises_total: None,
        last_accessed: Some(chrono::Utc::now().to_rfc3339()),
        notes: Vec::new(),
        difficulty_rating: None,
    };

    let progress_value = serde_json::to_value(&progress)
        .map_err(|e| ApiError::Internal(format!("Chapter progress serialize: {e}")))?;

    state
        .agent
        .validator()
        .validate(&progress_value, "chapter_progress")
        .map_err(|e| ApiError::Internal(format!("Chapter progress validation: {e}")))?;

    s.current_chapter_id = None;
    s.updated_at = chrono::Utc::now();
    state.store.persist(s.id);

    Ok(Json(progress_value))
}
