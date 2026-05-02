use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::session::SessionHandle;
use crate::AppState;

pub(super) static START_TIME: std::sync::LazyLock<Instant> = std::sync::LazyLock::new(Instant::now);
pub(super) static SSE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) const PROFILE_ROUNDS_NEEDED: u32 = 3;
pub(super) const SNAPSHOT_MESSAGE_LIMIT: usize = 50;

pub(super) fn next_sse_id() -> String {
    SSE_COUNTER.fetch_add(1, Ordering::Relaxed).to_string()
}

pub(super) async fn load_or_404(state: &AppState, id: Uuid) -> Result<SessionHandle, ApiError> {
    state.store.get(id).await.ok_or(ApiError::NotFound)
}

pub(super) fn default_profile_json() -> serde_json::Value {
    json!({
        "experience_level": {"domain_knowledge": "beginner"},
        "learning_style": {"preferred_format": ["text"]},
        "available_time": {"hours_per_week": 5}
    })
}

pub(super) fn from_agent_value<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T, ApiError> {
    serde_json::from_value(value)
        .map_err(|e| ApiError::Internal(format!("Failed to parse agent output: {e}")))
}
