use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::domain as d;
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

pub(super) fn build_curriculum_context(
    curriculum: Option<&d::CurriculumPlan>,
    chapter_id: &str,
) -> serde_json::Value {
    let Some(curriculum) = curriculum else {
        return json!({
            "title": "Unknown",
            "chapters": [],
        });
    };

    let current_index = curriculum
        .chapters
        .iter()
        .position(|ch| ch.id == chapter_id);
    let current_chapter = current_index.and_then(|index| curriculum.chapters.get(index));
    let previous_chapter = current_index.and_then(|index| {
        index
            .checked_sub(1)
            .and_then(|i| curriculum.chapters.get(i))
    });
    let next_chapter = current_index.and_then(|index| curriculum.chapters.get(index + 1));

    json!({
        "title": curriculum.title,
        "description": curriculum.description,
        "estimated_duration": curriculum.estimated_duration,
        "prerequisites_summary": curriculum.prerequisites_summary,
        "learning_objectives": curriculum.learning_objectives,
        "chapters": curriculum.chapters,
        "current_chapter": current_chapter,
        "previous_chapter": previous_chapter,
        "next_chapter": next_chapter,
        "chapter_index": current_index.map(|index| index + 1),
        "chapter_count": curriculum.chapters.len(),
    })
}

pub(super) fn build_profile_history(messages: &[d::SessionMessage]) -> serde_json::Value {
    let history = messages
        .iter()
        .filter_map(|message| match message.content_type.as_deref() {
            Some("profile_answer") | Some("profile_question") => Some(json!({
                "role": message.role,
                "content": message.content,
                "content_type": message.content_type,
                "metadata": message.metadata,
                "timestamp": message.timestamp,
            })),
            _ => None,
        })
        .collect::<Vec<_>>();
    json!(history)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_curriculum_context_enriches_neighbor_metadata() {
        let curriculum = d::CurriculumPlan {
            title: "Rust Fundamentals".to_string(),
            description: Some("Learn Rust safely".to_string()),
            chapters: vec![
                d::ChapterData {
                    id: "ch1".to_string(),
                    title: "Getting Started".to_string(),
                    order: 1,
                    objectives: vec!["Install Rust".to_string()],
                    prerequisites: vec![],
                    estimated_minutes: Some(30),
                    key_concepts: vec![],
                    exercises: vec![],
                },
                d::ChapterData {
                    id: "ch2".to_string(),
                    title: "Ownership".to_string(),
                    order: 2,
                    objectives: vec!["Understand moves".to_string()],
                    prerequisites: vec!["ch1".to_string()],
                    estimated_minutes: Some(45),
                    key_concepts: vec![],
                    exercises: vec![],
                },
            ],
            estimated_duration: "2 weeks".to_string(),
            prerequisites_summary: vec!["Basic programming".to_string()],
            learning_objectives: vec!["Write safe Rust".to_string()],
        };

        let context = build_curriculum_context(Some(&curriculum), "ch2");
        assert_eq!(context["chapter_index"], 2);
        assert_eq!(context["chapter_count"], 2);
        assert_eq!(context["current_chapter"]["id"], "ch2");
        assert_eq!(context["previous_chapter"]["id"], "ch1");
        assert!(context["next_chapter"].is_null());
    }

    #[test]
    fn test_build_profile_history_filters_profile_messages() {
        let messages = vec![
            d::SessionMessage {
                id: "1".to_string(),
                role: "user".to_string(),
                content: "No experience".to_string(),
                timestamp: "2025-01-15T10:30:00Z".to_string(),
                chapter_id: None,
                content_type: Some("profile_answer".to_string()),
                metadata: None,
            },
            d::SessionMessage {
                id: "2".to_string(),
                role: "assistant".to_string(),
                content: "How much time do you have each week?".to_string(),
                timestamp: "2025-01-15T10:31:00Z".to_string(),
                chapter_id: None,
                content_type: Some("profile_question".to_string()),
                metadata: None,
            },
            d::SessionMessage {
                id: "3".to_string(),
                role: "assistant".to_string(),
                content: "Chapter answer".to_string(),
                timestamp: "2025-01-15T10:32:00Z".to_string(),
                chapter_id: Some("ch1".to_string()),
                content_type: Some("explanation".to_string()),
                metadata: None,
            },
        ];

        let history = build_profile_history(&messages);
        assert_eq!(history.as_array().unwrap().len(), 2);
        assert_eq!(history[0]["content"], "No experience");
        assert_eq!(history[1]["content_type"], "profile_question");
    }
}
