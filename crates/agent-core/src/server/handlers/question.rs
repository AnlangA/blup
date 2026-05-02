use axum::{extract::State, Json};
use serde_json::json;
use uuid::Uuid;

use blup_agent::step::*;

use super::helpers::{default_profile_json, load_or_404};
use super::types::QuestionRequest;
use crate::error::ApiError;
use crate::state::domain::SessionMessage;
use crate::AppState;

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
            .unwrap_or_else(default_profile_json);

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
