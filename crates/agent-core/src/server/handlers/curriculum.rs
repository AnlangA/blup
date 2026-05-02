use axum::{extract::State, Json};
use serde_json::json;
use uuid::Uuid;

use blup_agent::step::*;

use super::helpers::{default_profile_json, from_agent_value, load_or_404};
use crate::error::ApiError;
use crate::state::domain as d;
use crate::state::types::Transition;
use crate::AppState;

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
            .unwrap_or_else(default_profile_json);
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
        if let Err(e) = s.state_machine.transition(Transition::CurriculumReady) {
            tracing::error!(error = %e, "Transition CurriculumReady failed unexpectedly");
        }
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);
    }

    if let Ok(curriculum_json) = serde_json::to_value(&curriculum) {
        if let Err(e) = state.storage.save_curriculum(id, curriculum_json).await {
            tracing::warn!(session_id = %id, error = %e, "Failed to persist curriculum to storage");
        }
        if let Err(e) = state
            .storage
            .update_session_state(id, "CHAPTER_LEARNING")
            .await
        {
            tracing::warn!(session_id = %id, error = %e, "Failed to update session state in storage");
        }
    }

    Ok(Json(
        serde_json::to_value(curriculum).map_err(|e| ApiError::Internal(e.to_string()))?,
    ))
}
