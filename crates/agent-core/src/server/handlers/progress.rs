use axum::{extract::State, Json};
use serde_json::json;
use uuid::Uuid;

use super::helpers::load_or_404;
use crate::error::ApiError;
use crate::AppState;

pub async fn get_all_progress(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _handle = load_or_404(&state, id).await?;

    let progress_list = state
        .storage
        .get_all_progress(id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get progress: {e}")))?;

    Ok(Json(json!({
        "session_id": id.to_string(),
        "progress": progress_list,
    })))
}
