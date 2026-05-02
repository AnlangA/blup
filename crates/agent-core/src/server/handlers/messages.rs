use axum::{extract::State, Json};
use serde_json::json;
use uuid::Uuid;

use super::helpers::load_or_404;
use crate::error::ApiError;
use crate::AppState;

#[derive(serde::Deserialize)]
pub struct MessagesQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
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
