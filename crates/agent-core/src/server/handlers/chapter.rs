use axum::response::sse::{Event, KeepAlive, Sse};
use axum::{extract::State, Json};
use futures::StreamExt;
use serde_json::json;
use uuid::Uuid;

use blup_agent::step::*;

use super::helpers::{default_profile_json, load_or_404, next_sse_id};
use super::types::SseEvent;
use crate::error::ApiError;
use crate::state::domain as d;
use crate::state::types::Transition;
use crate::AppState;

// ── start_chapter (sync) ──

pub async fn start_chapter(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;

    let (chapter_title, profile_json) = {
        let s = handle.read().await;
        let current = s.state();
        if current != crate::state::types::SessionState::ChapterLearning {
            return Err(ApiError::InvalidTransition(format!(
                "Cannot start chapter in state {current}"
            )));
        }

        if let Some(cached) = s.chapter_contents.get(&ch_id).cloned() {
            return Ok(Json(json!({
                "id": Uuid::new_v4().to_string(),
                "role": "assistant",
                "content": cached,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })));
        }

        let title = s
            .curriculum
            .as_ref()
            .and_then(|c| c.chapters.iter().find(|ch| ch.id == ch_id))
            .map(|ch| ch.title.as_str())
            .unwrap_or(&ch_id)
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
        (title, profile)
    };

    let content = state
        .agent
        .teach_chapter(&ChapterContext {
            chapter_id: ch_id.clone(),
            chapter_title,
            profile: profile_json,
            curriculum_context: json!({}),
        })
        .await
        .map_err(ApiError::from)?;

    {
        let mut s = handle.write().await;
        s.current_chapter_id = Some(ch_id.clone());
        s.chapter_contents.insert(ch_id.clone(), content.clone());
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);
    }

    Ok(Json(json!({
        "id": Uuid::new_v4().to_string(),
        "role": "assistant",
        "content": content,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

// ── start_chapter_stream (SSE) ──

pub async fn start_chapter_stream(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>, ApiError> {
    let handle = load_or_404(&state, id).await?;

    let (cached, chapter_title, profile_json) = {
        let s = handle.read().await;
        let current = s.state();
        if current != crate::state::types::SessionState::ChapterLearning {
            return Err(ApiError::InvalidTransition(format!(
                "Cannot start chapter in state {current}"
            )));
        }

        let cached = s.chapter_contents.get(&ch_id).cloned();

        let title = s
            .curriculum
            .as_ref()
            .and_then(|c| c.chapters.iter().find(|ch| ch.id == ch_id))
            .map(|ch| ch.title.as_str())
            .unwrap_or(&ch_id)
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

        (cached, title, profile)
    };

    let ping_interval = std::time::Duration::from_secs(state.config.sse_ping_interval_secs);
    let stream_state = state.clone();
    let stream_handle = handle.clone();
    let stream_ch_id = ch_id.clone();

    let stream = async_stream::stream! {
        yield Ok(Event::default()
            .event("status")
            .id(next_sse_id())
            .data(serde_json::to_string(&SseEvent::Status {
                state: "CHAPTER_LEARNING".to_string(),
                message: format!("Loading chapter: {chapter_title}"),
            }).expect("SSE serialize")));

        if let Some(ref cached_content) = cached {
            yield Ok(Event::default()
                .event("done")
                .id(next_sse_id())
                .data(serde_json::to_string(&SseEvent::Done {
                    result: json!({
                        "chapter_id": stream_ch_id,
                        "content": cached_content,
                    }),
                }).expect("SSE serialize")));
            return;
        }

        let agent_stream = stream_state.agent.teach_chapter_stream(ChapterContext {
            chapter_id: stream_ch_id.clone(),
            chapter_title,
            profile: profile_json,
            curriculum_context: json!({}),
        });

        let mut agent_stream = std::pin::pin!(agent_stream);
        let mut full_content = String::new();
        let mut chunk_count = 0u32;

        while let Some(event_result) = agent_stream.next().await {
            match event_result {
                Ok(AgentStreamEvent::Status { state: st, message }) => {
                    yield Ok(Event::default()
                        .event("status")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Status { state: st, message })
                            .expect("SSE serialize")));
                }
                Ok(AgentStreamEvent::Chunk { content, index }) => {
                    full_content.push_str(&content);
                    chunk_count += 1;
                    yield Ok(Event::default()
                        .event("chunk")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Chunk { content, index })
                            .expect("SSE serialize")));

                    if chunk_count.is_multiple_of(5) && !full_content.is_empty() {
                        let mut s = stream_handle.write().await;
                        s.chapter_contents.insert(stream_ch_id.clone(), full_content.clone());
                        s.updated_at = chrono::Utc::now();
                    }
                }
                Ok(AgentStreamEvent::Error { code, message }) => {
                    yield Ok(Event::default()
                        .event("error")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Error { code, message })
                            .expect("SSE serialize")));
                    return;
                }
                Ok(AgentStreamEvent::Done { .. }) => {
                    {
                        let mut s = stream_handle.write().await;
                        s.current_chapter_id = Some(stream_ch_id.clone());
                        s.chapter_contents.insert(stream_ch_id.clone(), full_content.clone());
                        s.updated_at = chrono::Utc::now();
                        stream_state.store.persist(s.id);
                    }
                    yield Ok(Event::default()
                        .event("done")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Done {
                            result: json!({
                                "chapter_id": stream_ch_id,
                                "content": full_content,
                            }),
                        }).expect("SSE serialize")));
                    return;
                }
                Err(e) => {
                    yield Ok(Event::default()
                        .event("error")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Error {
                            code: "AGENT_ERROR".to_string(),
                            message: e.to_string(),
                        }).expect("SSE serialize")));
                    return;
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(ping_interval)))
}

// ── complete_chapter ──

pub async fn complete_chapter(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;
    let mut s = handle.write().await;

    s.state_machine.transition(Transition::ChapterComplete)?;

    let progress = d::ChapterProgress {
        chapter_id: ch_id.clone(),
        status: "completed".to_string(),
        completion: 100.0,
        time_spent_minutes: None,
        exercises_completed: None,
        exercises_total: None,
        last_accessed: Some(chrono::Utc::now().to_rfc3339()),
        notes: Vec::new(),
        difficulty_rating: None,
    };

    let progress_value = serde_json::to_value(&progress)
        .map_err(|e| ApiError::Internal(format!("Chapter progress serialize: {e}")))?;

    state
        .agent
        .validator()
        .validate(&progress_value, "chapter_progress")
        .map_err(|e| ApiError::Internal(format!("Chapter progress validation: {e}")))?;

    s.current_chapter_id = None;
    s.updated_at = chrono::Utc::now();
    state.store.persist(s.id);

    if let Err(e) = state
        .storage
        .upsert_progress(id, &ch_id, progress_value.clone())
        .await
    {
        tracing::warn!(session_id = %id, chapter_id = %ch_id, error = %e, "Failed to persist progress to storage");
    }

    Ok(Json(progress_value))
}
