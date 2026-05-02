use axum::{extract::State, Json};
use serde_json::json;
use uuid::Uuid;

use blup_agent::step::*;

use super::helpers::{from_agent_value, load_or_404, PROFILE_ROUNDS_NEEDED};
use super::types::ProfileAnswer;
use crate::error::ApiError;
use crate::state::domain as d;
use crate::state::types::Transition;
use crate::AppState;

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
        } => Ok(Json(json!({
            "is_complete": false,
            "round": round,
            "total_rounds": total_rounds,
            "next_question": next_question_hint,
            "state": "PROFILE_COLLECTION"
        }))),
    }
}
