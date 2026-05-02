use axum::{extract::State, Json};
use serde_json::json;
use uuid::Uuid;

use super::helpers::{load_or_404, SNAPSHOT_MESSAGE_LIMIT};
use super::types::CreateSessionResponse;
use crate::error::ApiError;
use crate::state::session::SessionListEntry;
use crate::AppState;

pub async fn create_session(
    State(state): State<AppState>,
) -> Result<Json<CreateSessionResponse>, ApiError> {
    match state.store.create().await {
        Some(handle) => {
            let s = handle.read().await;
            let session_id = s.id;

            if let Err(e) = state.storage.create_session_with_id(session_id).await {
                tracing::warn!(session_id = %session_id, error = %e, "Failed to persist session to storage");
            }

            Ok(Json(CreateSessionResponse {
                session_id: session_id.to_string(),
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
    if state.store.get(id).await.is_none() {
        return Err(ApiError::NotFound);
    }
    state.store.delete(id).await;

    if let Err(e) = state.storage.delete_session(id).await {
        tracing::warn!(session_id = %id, error = %e, "Failed to delete session from storage");
    }

    Ok(Json(json!({ "deleted": true })))
}

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
