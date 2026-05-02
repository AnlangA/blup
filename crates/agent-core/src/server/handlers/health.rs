use axum::{extract::State, Json};
use serde_json::json;

use super::helpers::START_TIME;
use crate::AppState;

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
