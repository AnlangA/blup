use axum::{extract::State, Json};
use serde_json::json;
use uuid::Uuid;

use blup_agent::step::*;

use super::helpers::{build_profile_history, from_agent_value, load_or_404, PROFILE_ROUNDS_NEEDED};
use super::types::ProfileAnswer;
use crate::error::ApiError;
use crate::state::domain as d;
use crate::state::domain::SessionMessage;
use crate::state::types::Transition;
use crate::AppState;

pub async fn submit_profile_answer(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(answer): Json<ProfileAnswer>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;

    let (goal_desc, goal_domain, current_round, is_complete, profile_history) = {
        let s = handle.read().await;
        if s.state() != crate::state::types::SessionState::ProfileCollection {
            return Err(ApiError::InvalidTransition(format!(
                "Cannot submit profile answer in state {}",
                s.state()
            )));
        }
        let new_round = s.profile_rounds + 1;
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
        let profile_history = build_profile_history(&s.messages);
        (
            desc,
            domain,
            new_round,
            new_round >= PROFILE_ROUNDS_NEEDED,
            profile_history,
        )
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
            profile_history,
        })
        .await
        .map_err(ApiError::from)?;

    match profile_step {
        ProfileStep::Complete { profile } => {
            let typed_profile: d::UserProfile = from_agent_value(profile)?;
            let user_message = SessionMessage {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                content: answer.answer,
                chapter_id: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
                content_type: Some("profile_answer".to_string()),
                metadata: Some(json!({
                    "question_id": answer.question_id,
                    "round": current_round,
                })),
            };
            let mut s = handle.write().await;
            s.state_machine.transition(Transition::ProfileComplete)?;
            s.profile_rounds = current_round;
            s.profile = Some(typed_profile.clone());
            s.messages.push(user_message);
            s.updated_at = chrono::Utc::now();
            state.store.persist(s.id);
            drop(s);

            if let Ok(profile_json) = serde_json::to_value(&typed_profile) {
                if let Err(e) = state.storage.save_user_profile(id, profile_json).await {
                    tracing::warn!(session_id = %id, error = %e, "Failed to persist user profile to storage");
                }
                if let Err(e) = state
                    .storage
                    .update_session_state(id, "CURRICULUM_PLANNING")
                    .await
                {
                    tracing::warn!(session_id = %id, error = %e, "Failed to update session state in storage");
                }
            }

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
        } => {
            let timestamp = chrono::Utc::now().to_rfc3339();
            let user_message = SessionMessage {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                content: answer.answer,
                chapter_id: None,
                timestamp: timestamp.clone(),
                content_type: Some("profile_answer".to_string()),
                metadata: Some(json!({
                    "question_id": answer.question_id,
                    "round": current_round,
                })),
            };
            let assistant_message = SessionMessage {
                id: Uuid::new_v4().to_string(),
                role: "assistant".to_string(),
                content: next_question_hint.clone(),
                chapter_id: None,
                timestamp,
                content_type: Some("profile_question".to_string()),
                metadata: Some(json!({
                    "round": current_round + 1,
                })),
            };
            let mut s = handle.write().await;
            s.state_machine.transition(Transition::ProfileContinue)?;
            s.profile_rounds = current_round;
            s.messages.push(user_message);
            s.messages.push(assistant_message);
            s.updated_at = chrono::Utc::now();
            state.store.persist(s.id);

            Ok(Json(json!({
                "is_complete": false,
                "round": round,
                "total_rounds": total_rounds,
                "next_question": next_question_hint,
                "state": "PROFILE_COLLECTION"
            })))
        }
    }
}
