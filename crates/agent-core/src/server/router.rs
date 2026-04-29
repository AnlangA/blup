use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use axum::{
    extract::State,
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use uuid::Uuid;

use crate::llm::client::{GatewayMessage, GatewayRequest};
use crate::models::types::*;
use crate::state::session::SessionHandle;
use crate::state::types::Transition;
use crate::AppState;

static START_TIME: std::sync::LazyLock<Instant> = std::sync::LazyLock::new(Instant::now);
static SSE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/session", post(create_session))
        .route("/api/session/:id", get(get_session_status))
        .route("/api/session/:id/goal", post(submit_goal))
        .route("/api/session/:id/goal/stream", post(submit_goal_stream))
        .route(
            "/api/session/:id/profile/answer",
            post(submit_profile_answer),
        )
        .route("/api/session/:id/curriculum", get(get_curriculum))
        .route("/api/session/:id/chapter/:ch_id", get(start_chapter))
        .route(
            "/api/session/:id/chapter/:ch_id/stream",
            get(start_chapter_stream),
        )
        .route("/api/session/:id/chapter/:ch_id/ask", post(ask_question))
        .route(
            "/api/session/:id/chapter/:ch_id/complete",
            post(complete_chapter),
        )
        .layer(CorsLayer::permissive())
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Error responses
// ---------------------------------------------------------------------------

fn not_found() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: ErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: "Session not found".to_string(),
            },
        }),
    )
}

fn invalid_transition(e: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            error: ErrorDetail {
                code: "INVALID_STATE_TRANSITION".to_string(),
                message: e.to_string(),
            },
        }),
    )
}

fn internal_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: ErrorDetail {
                code: "INTERNAL_ERROR".to_string(),
                message: msg.to_string(),
            },
        }),
    )
}

// ---------------------------------------------------------------------------
// Session helpers
// ---------------------------------------------------------------------------

async fn load_or_404(
    state: &AppState,
    id: Uuid,
) -> Result<SessionHandle, (StatusCode, Json<ErrorResponse>)> {
    state.store.get(id).await.ok_or_else(not_found)
}

// ---------------------------------------------------------------------------
// LLM helpers
// ---------------------------------------------------------------------------

fn system_msg(content: &str) -> GatewayMessage {
    GatewayMessage {
        role: "system".to_string(),
        content: content.to_string(),
    }
}

fn user_msg(content: &str) -> GatewayMessage {
    GatewayMessage {
        role: "user".to_string(),
        content: content.to_string(),
    }
}

async fn llm_json(
    state: &AppState,
    system_prompt: &str,
    user_prompt: &str,
    schema_name: &str,
) -> Result<serde_json::Value, String> {
    let request = GatewayRequest {
        model: state.config.llm_model.clone(),
        messages: vec![system_msg(system_prompt), user_msg(user_prompt)],
        temperature: Some(0.3),
        max_tokens: Some(4096),
        stream: false,
    };

    let response = state
        .llm
        .complete(request)
        .await
        .map_err(|e| format!("LLM error: {e}"))?;

    let json_str = extract_json(&response.content);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
        format!(
            "LLM response was not valid JSON: {e}. Raw (truncated): {}",
            &response.content[..response.content.len().min(400)]
        )
    })?;

    state
        .validator
        .validate(&parsed, schema_name)
        .map_err(|e| format!("Schema validation failed: {e}"))?;

    tracing::info!(
        schema = schema_name,
        model = %response.model,
        tokens = response.usage.total_tokens,
        "LLM call completed and validated"
    );

    Ok(parsed)
}

async fn llm_text(
    state: &AppState,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, String> {
    let request = GatewayRequest {
        model: state.config.llm_model.clone(),
        messages: vec![system_msg(system_prompt), user_msg(user_prompt)],
        temperature: Some(0.3),
        max_tokens: Some(4096),
        stream: false,
    };

    let response = state
        .llm
        .complete(request)
        .await
        .map_err(|e| format!("LLM error: {e}"))?;

    tracing::info!(
        model = %response.model,
        tokens = response.usage.total_tokens,
        content_length = response.content.len(),
        "LLM text call completed"
    );

    Ok(response.content)
}

fn extract_json(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(inner) = trimmed
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .or_else(|| {
            trimmed
                .strip_prefix("```")
                .and_then(|s| s.strip_suffix("```"))
        })
    {
        return inner.trim().to_string();
    }
    trimmed.to_string()
}

fn next_sse_id() -> String {
    SSE_COUNTER.fetch_add(1, Ordering::Relaxed).to_string()
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
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

async fn create_session(
    State(state): State<AppState>,
) -> Result<Json<CreateSessionResponse>, StatusCode> {
    match state.store.create().await {
        Some(handle) => {
            let s = handle.read().await;
            Ok(Json(CreateSessionResponse {
                session_id: s.id.to_string(),
                state: s.state().to_string(),
            }))
        }
        None => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn get_session_status(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let handle = load_or_404(&state, id).await?;
    let s = handle.read().await;
    Ok(Json(json!({
        "session_id": s.id.to_string(),
        "state": s.state().to_string(),
        "goal": s.goal,
        "feasibility_result": s.feasibility_result,
        "profile": s.profile,
        "profile_rounds": s.profile_rounds,
        "curriculum": s.curriculum,
        "current_chapter_id": s.current_chapter_id,
        "chapter_contents": serde_json::to_value(&s.chapter_contents).unwrap_or(json!({})),
        "messages": s.messages,
    })))
}

// ---- submit_goal ----

async fn submit_goal(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(goal): Json<LearningGoal>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if goal.description.len() < 10 {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: ErrorDetail {
                    code: "VALIDATION_ERROR".to_string(),
                    message: "description must be at least 10 characters".into(),
                },
            }),
        ));
    }

    let handle = load_or_404(&state, id).await?;

    // Phase 1: acquire lock, transition state, store goal
    let (goal_desc, goal_domain, goal_context) = {
        let mut s = handle.write().await;
        let current = s.state();
        if current == crate::state::types::SessionState::Idle {
            s.state_machine
                .transition(Transition::SubmitGoal)
                .map_err(invalid_transition)?;
        }
        s.state_machine
            .transition(Transition::SubmitGoal)
            .map_err(invalid_transition)?;

        s.goal = Some(serde_json::to_value(&goal).map_err(|e| internal_error(&e.to_string()))?);
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);

        (
            goal.description.clone(),
            goal.domain.clone(),
            goal.context.clone().unwrap_or_default(),
        )
    }; // lock released here before LLM call

    let mut vars = HashMap::new();
    vars.insert("learning_goal".to_string(), goal_desc.clone());
    vars.insert("domain".to_string(), goal_domain.clone());
    vars.insert("context".to_string(), goal_context);

    let system_prompt = state
        .prompts
        .load_and_render("feasibility_check", 1, &vars)
        .map_err(|_| internal_error("Failed to load prompt"))?;

    let user_prompt = format!("Learning goal: {goal_desc}\nDomain: {goal_domain}");

    let feasibility = llm_json(&state, &system_prompt, &user_prompt, "feasibility_result")
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Feasibility check failed");
            internal_error(&e)
        })?;

    // Phase 2: reacquire lock, store result, transition
    let (_feasible, state_name) = {
        let mut s = handle.write().await;
        let feasible = feasibility
            .get("feasible")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        s.feasibility_result = Some(feasibility.clone());
        let _ = if feasible {
            s.state_machine.transition(Transition::GoalFeasible)
        } else {
            s.state_machine.transition(Transition::GoalInfeasible)
        };
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);
        (
            feasible,
            if feasible {
                "PROFILE_COLLECTION"
            } else {
                "GOAL_INPUT"
            },
        )
    };

    Ok(Json(json!({
        "feasibility": feasibility,
        "state": state_name
    })))
}

// ---- submit_goal_stream (SSE) ----

async fn submit_goal_stream(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(goal): Json<LearningGoal>,
) -> Result<
    Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>,
    (StatusCode, Json<ErrorResponse>),
> {
    if goal.description.len() < 10 {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: ErrorDetail {
                    code: "VALIDATION_ERROR".to_string(),
                    message: "description must be at least 10 characters".into(),
                },
            }),
        ));
    }

    let handle = load_or_404(&state, id).await?;

    let (goal_desc, goal_domain, goal_context) = {
        let mut s = handle.write().await;
        let current = s.state();
        if current == crate::state::types::SessionState::Idle {
            s.state_machine
                .transition(Transition::SubmitGoal)
                .map_err(invalid_transition)?;
        }
        s.state_machine
            .transition(Transition::SubmitGoal)
            .map_err(invalid_transition)?;

        s.goal = Some(serde_json::to_value(&goal).map_err(|e| internal_error(&e.to_string()))?);
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);

        (
            goal.description.clone(),
            goal.domain.clone(),
            goal.context.clone().unwrap_or_default(),
        )
    };

    let mut vars = HashMap::new();
    vars.insert("learning_goal".to_string(), goal_desc.clone());
    vars.insert("domain".to_string(), goal_domain.clone());
    vars.insert("context".to_string(), goal_context);

    let system_prompt = state
        .prompts
        .load_and_render("feasibility_check", 1, &vars)
        .map_err(|_| internal_error("Failed to load prompt"))?;

    let user_prompt = format!("Learning goal: {goal_desc}\nDomain: {goal_domain}");

    let ping_interval = std::time::Duration::from_secs(state.config.sse_ping_interval_secs);

    let stream = async_stream::stream! {
        yield Ok(Event::default()
            .id(next_sse_id())
            .data(serde_json::to_string(&SseEvent::Status {
                state: "FEASIBILITY_CHECK".to_string(),
                message: "Checking goal feasibility...".to_string(),
            }).expect("SSE event serialization failed")));

        // Use streaming LLM call for real-time token delivery
        let stream_request = GatewayRequest {
            model: state.config.llm_model.clone(),
            messages: vec![system_msg(&system_prompt), user_msg(&user_prompt)],
            temperature: Some(0.3),
            max_tokens: Some(4096),
            stream: true,
        };

        let chunk_stream = state.llm.stream(stream_request);
        tokio::pin!(chunk_stream);
        let mut full_text = String::new();
        let mut has_error = false;
        let mut error_msg = String::new();

        while let Some(result) = chunk_stream.next().await {
            match result {
                Ok(chunk) => {
                    full_text.push_str(&chunk.content);
                    yield Ok(Event::default()
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Chunk {
                            content: chunk.content.clone(),
                            index: chunk.index,
                        }).expect("SSE event serialization failed")));
                }
                Err(e) => {
                    has_error = true;
                    error_msg = e.to_string();
                    break;
                }
            }
        }

        if has_error {
            tracing::error!(error = %error_msg, "Feasibility check stream failed");
            yield Ok(Event::default()
                .id(next_sse_id())
                .data(serde_json::to_string(&SseEvent::Error {
                    code: "LLM_ERROR".to_string(),
                    message: error_msg,
                }).expect("SSE event serialization failed")));
            return;
        }

        // Parse and validate the accumulated JSON
        let json_str = extract_json(&full_text);
        match serde_json::from_str::<serde_json::Value>(&json_str) {
            Ok(parsed) => {
                if let Err(e) = state.validator.validate(&parsed, "feasibility_result") {
                    yield Ok(Event::default()
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Error {
                            code: "VALIDATION_ERROR".to_string(),
                            message: e.to_string(),
                        }).expect("SSE event serialization failed")));
                    return;
                }

                let feasible = parsed
                    .get("feasible")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                {
                    let mut s = handle.write().await;
                    s.feasibility_result = Some(parsed.clone());
                    let _ = if feasible {
                        s.state_machine.transition(Transition::GoalFeasible)
                    } else {
                        s.state_machine.transition(Transition::GoalInfeasible)
                    };
                    s.updated_at = chrono::Utc::now();
                    state.store.persist(s.id);
                }

                yield Ok(Event::default()
                    .id(next_sse_id())
                    .data(serde_json::to_string(&SseEvent::Done {
                        result: json!({
                            "feasibility": parsed,
                            "state": if feasible { "PROFILE_COLLECTION" } else { "GOAL_INPUT" }
                        }),
                    }).expect("SSE event serialization failed")));
            }
            Err(e) => {
                yield Ok(Event::default()
                    .id(next_sse_id())
                    .data(serde_json::to_string(&SseEvent::Error {
                        code: "PARSE_ERROR".to_string(),
                        message: format!("LLM response was not valid JSON: {e}"),
                    }).expect("SSE event serialization failed")));
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(ping_interval)))
}

// ---- submit_profile_answer ----

const PROFILE_ROUNDS_NEEDED: u32 = 3;

async fn submit_profile_answer(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(answer): Json<ProfileAnswer>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let handle = load_or_404(&state, id).await?;

    // Phase 1: read current state and determine transition
    let (goal_desc, goal_domain, current_round, is_complete) = {
        let mut s = handle.write().await;
        let current = s.profile_rounds;
        if current < PROFILE_ROUNDS_NEEDED - 1 {
            s.state_machine
                .transition(Transition::ProfileContinue)
                .map_err(invalid_transition)?;
        } else {
            s.state_machine
                .transition(Transition::ProfileComplete)
                .map_err(invalid_transition)?;
        }
        s.profile_rounds += 1;
        let new_round = s.profile_rounds;
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);

        let desc = s
            .goal
            .as_ref()
            .and_then(|g| g.get("description"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown goal")
            .to_string();
        let domain = s
            .goal
            .as_ref()
            .and_then(|g| g.get("domain"))
            .and_then(|v| v.as_str())
            .unwrap_or("general")
            .to_string();
        (desc, domain, new_round, new_round >= PROFILE_ROUNDS_NEEDED)
    };

    let mut vars = HashMap::new();
    vars.insert("learning_goal".to_string(), goal_desc.clone());
    vars.insert("domain".to_string(), goal_domain.clone());
    vars.insert("answer".to_string(), answer.answer.clone());
    vars.insert("round".to_string(), current_round.to_string());
    vars.insert("is_final".to_string(), is_complete.to_string());

    let system_prompt = state
        .prompts
        .load_and_render("profile_collection", 1, &vars)
        .map_err(|_| internal_error("Failed to load profile prompt"))?;

    let round_desc = match current_round {
        1 => "experience level",
        2 => "learning preferences",
        _ => "",
    };
    let user_prompt = if is_complete {
        format!(
            "Final round. Build the complete profile. Goal: {goal_desc}\nDomain: {goal_domain}\nLatest answer: {}",
            answer.answer
        )
    } else {
        format!(
            "Profile collection round {current_round}/{}: {round_desc}. User answer: {}",
            PROFILE_ROUNDS_NEEDED, answer.answer
        )
    };

    if is_complete {
        let profile = llm_json(&state, &system_prompt, &user_prompt, "user_profile")
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Profile collection failed");
                internal_error(&e)
            })?;

        {
            let mut s = handle.write().await;
            s.profile = Some(profile.clone());
            s.updated_at = chrono::Utc::now();
            state.store.persist(s.id);
        }

        Ok(Json(json!({
            "is_complete": true,
            "profile": profile,
            "state": "CURRICULUM_PLANNING"
        })))
    } else {
        Ok(Json(json!({
            "is_complete": false,
            "round": current_round,
            "total_rounds": PROFILE_ROUNDS_NEEDED,
            "next_question": match current_round {
                1 => "How would you describe your preferred learning style?",
                2 => "How much time can you dedicate each week?",
                _ => "Tell me more about your background.",
            },
            "state": "PROFILE_COLLECTION"
        })))
    }
}

// ---- get_curriculum ----

async fn get_curriculum(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let handle = load_or_404(&state, id).await?;

    // Check state and cached curriculum
    let (goal_desc, profile_json) = {
        let s = handle.read().await;
        let current = s.state();
        if current != crate::state::types::SessionState::ChapterLearning
            && current != crate::state::types::SessionState::CurriculumPlanning
        {
            return Err(invalid_transition(format!(
                "Cannot get curriculum in state {current}"
            )));
        }

        if let Some(ref curriculum) = s.curriculum {
            return Ok(Json(curriculum.clone()));
        }

        let desc = s
            .goal
            .as_ref()
            .and_then(|g| g.get("description"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown goal")
            .to_string();
        let profile = s.profile.clone().unwrap_or_else(|| {
            json!({
                "experience_level": {"domain_knowledge": "beginner"},
                "learning_style": {"preferred_format": ["text"]},
                "available_time": {"hours_per_week": 5}
            })
        });
        (desc, profile)
    }; // read lock released

    let mut vars = HashMap::new();
    vars.insert("learning_goal".to_string(), goal_desc.clone());
    vars.insert(
        "user_profile".to_string(),
        serde_json::to_string(&profile_json).unwrap_or_default(),
    );

    let system_prompt = state
        .prompts
        .load_and_render("curriculum_planning", 1, &vars)
        .map_err(|_| internal_error("Failed to load curriculum prompt"))?;

    let user_prompt = format!(
        "Goal: {goal_desc}\n\nProfile: {}",
        serde_json::to_string_pretty(&profile_json).unwrap_or_default()
    );

    let curriculum = llm_json(&state, &system_prompt, &user_prompt, "curriculum_plan")
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Curriculum generation failed");
            internal_error(&e)
        })?;

    {
        let mut s = handle.write().await;
        s.curriculum = Some(curriculum.clone());
        let _ = s.state_machine.transition(Transition::CurriculumReady);
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);
    }

    Ok(Json(curriculum))
}

// ---- start_chapter ----

async fn start_chapter(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let handle = load_or_404(&state, id).await?;

    // Check state and cache
    let (chapter_title, profile_json) = {
        let s = handle.read().await;
        let current = s.state();
        if current != crate::state::types::SessionState::ChapterLearning {
            return Err(invalid_transition(format!(
                "Cannot start chapter in state {current}"
            )));
        }

        // Return cached content if available
        if let Some(cached) = s.chapter_contents.get(&ch_id).cloned() {
            tracing::info!(chapter_id = %ch_id, "Returning cached chapter content");
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
            .and_then(|c| c.get("chapters"))
            .and_then(|chapters| chapters.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|ch| ch.get("id").and_then(|v| v.as_str()) == Some(&ch_id))
            })
            .and_then(|c| c.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or(&ch_id)
            .to_string();

        let profile = s.profile.clone().unwrap_or_else(|| {
            json!({
                "experience_level": {"domain_knowledge": "beginner"},
                "learning_style": {"preferred_format": ["text"]},
                "available_time": {"hours_per_week": 5}
            })
        });
        (title, profile)
    }; // read lock released before LLM call

    let mut vars = HashMap::new();
    vars.insert("chapter_id".to_string(), ch_id.clone());
    vars.insert(
        "user_profile".to_string(),
        serde_json::to_string(&profile_json).unwrap_or_default(),
    );
    vars.insert("curriculum_context".to_string(), "{}".to_string());

    let system_prompt = state
        .prompts
        .load_and_render("chapter_teaching", 1, &vars)
        .map_err(|_| internal_error("Failed to load chapter prompt"))?;

    let user_prompt = format!("Start teaching chapter: {chapter_title}");

    let content = llm_text(&state, &system_prompt, &user_prompt)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Chapter teaching failed");
            internal_error(&e)
        })?;

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

// ---- start_chapter_stream (SSE) ----

async fn start_chapter_stream(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<
    Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>,
    (StatusCode, Json<ErrorResponse>),
> {
    let handle = load_or_404(&state, id).await?;

    let (chapter_title, profile_json) = {
        let s = handle.read().await;
        let current = s.state();
        if current != crate::state::types::SessionState::ChapterLearning {
            return Err(invalid_transition(format!(
                "Cannot start chapter in state {current}"
            )));
        }

        let title = s
            .curriculum
            .as_ref()
            .and_then(|c| c.get("chapters"))
            .and_then(|chapters| chapters.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|ch| ch.get("id").and_then(|v| v.as_str()) == Some(&ch_id))
            })
            .and_then(|c| c.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or(&ch_id)
            .to_string();

        let profile = s.profile.clone().unwrap_or_else(|| {
            json!({
                "experience_level": {"domain_knowledge": "beginner"},
                "learning_style": {"preferred_format": ["text"]},
                "available_time": {"hours_per_week": 5}
            })
        });
        (title, profile)
    };

    let mut vars = HashMap::new();
    vars.insert("chapter_id".to_string(), ch_id.clone());
    vars.insert(
        "user_profile".to_string(),
        serde_json::to_string(&profile_json).unwrap_or_default(),
    );
    vars.insert("curriculum_context".to_string(), "{}".to_string());

    let system_prompt = state
        .prompts
        .load_and_render("chapter_teaching", 1, &vars)
        .map_err(|_| internal_error("Failed to load chapter prompt"))?;

    let user_prompt = format!("Start teaching chapter: {chapter_title}");

    let request = GatewayRequest {
        model: state.config.llm_model.clone(),
        messages: vec![system_msg(&system_prompt), user_msg(&user_prompt)],
        temperature: Some(0.3),
        max_tokens: Some(4096),
        stream: true,
    };

    let ping_interval = std::time::Duration::from_secs(state.config.sse_ping_interval_secs);

    let stream = async_stream::stream! {
        yield Ok(Event::default()
            .id(next_sse_id())
            .data(serde_json::to_string(&SseEvent::Status {
                state: "CHAPTER_LEARNING".to_string(),
                message: format!("Generating content for chapter: {chapter_title}"),
            }).expect("SSE event serialization failed")));

        let chunk_stream = state.llm.stream(request);
        tokio::pin!(chunk_stream);
        let mut full_content = String::new();
        let mut index: u32 = 0;
        while let Some(result) = chunk_stream.next().await {
            match result {
                Ok(chunk) => {
                    full_content.push_str(&chunk.content);
                    yield Ok(Event::default()
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Chunk {
                            content: chunk.content.clone(),
                            index,
                        }).expect("SSE event serialization failed")));
                    index += 1;
                }
                Err(e) => {
                    yield Ok(Event::default()
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Error {
                            code: "STREAM_ERROR".to_string(),
                            message: e.to_string(),
                        }).expect("SSE event serialization failed")));
                    return;
                }
            }
        }

        // Cache content in session
        {
            let mut s = handle.write().await;
            s.current_chapter_id = Some(ch_id.clone());
            s.chapter_contents.insert(ch_id.clone(), full_content.clone());
            s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);
        }

        yield Ok(Event::default()
            .id(next_sse_id())
            .data(serde_json::to_string(&SseEvent::Done {
                result: json!({
                    "chapter_id": ch_id,
                    "content": full_content,
                }),
            }).expect("SSE event serialization failed")));
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(ping_interval)))
}

// ---- ask_question ----

async fn ask_question(
    State(state): State<AppState>,
    axum::extract::Path((id, _ch_id)): axum::extract::Path<(Uuid, String)>,
    Json(question): Json<QuestionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if question.question.is_empty() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: ErrorDetail {
                    code: "VALIDATION_ERROR".to_string(),
                    message: "question must not be empty".into(),
                },
            }),
        ));
    }

    let profile_json = {
        let handle = load_or_404(&state, id).await?;
        let s = handle.read().await;
        let current = s.state();
        if current != crate::state::types::SessionState::ChapterLearning {
            return Err(invalid_transition(format!(
                "Cannot ask question in state {current}"
            )));
        }
        s.profile.clone().unwrap_or_else(|| {
            json!({
                "experience_level": {"domain_knowledge": "beginner"},
                "learning_style": {"preferred_format": ["text"]},
                "available_time": {"hours_per_week": 5}
            })
        })
    };

    let mut vars = HashMap::new();
    vars.insert("question".to_string(), question.question.clone());
    vars.insert(
        "user_profile".to_string(),
        serde_json::to_string(&profile_json).unwrap_or_default(),
    );

    let system_prompt = state
        .prompts
        .load_and_render("question_answering", 1, &vars)
        .map_err(|_| internal_error("Failed to load Q&A prompt"))?;

    let user_prompt = format!("Question: {}", question.question);

    let content = llm_text(&state, &system_prompt, &user_prompt)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Question answering failed");
            internal_error(&e)
        })?;

    let now = chrono::Utc::now().to_rfc3339();
    let user_msg = json!({
        "id": Uuid::new_v4().to_string(),
        "role": "user",
        "content": question.question,
        "chapter_id": _ch_id,
        "timestamp": now,
    });
    let assistant_msg = json!({
        "id": Uuid::new_v4().to_string(),
        "role": "assistant",
        "content": content,
        "chapter_id": _ch_id,
        "timestamp": now,
    });

    {
        let handle = load_or_404(&state, id).await?;
        let mut s = handle.write().await;
        s.messages.push(user_msg);
        s.messages.push(assistant_msg.clone());
        s.updated_at = chrono::Utc::now();
        state.store.persist(s.id);
    }

    Ok(Json(assistant_msg))
}

// ---- complete_chapter ----

async fn complete_chapter(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let handle = load_or_404(&state, id).await?;
    let mut s = handle.write().await;

    s.state_machine
        .transition(Transition::ChapterComplete)
        .map_err(invalid_transition)?;

    let progress = json!({
        "chapter_id": ch_id,
        "status": "completed",
        "completion": 100.0,
        "last_accessed": chrono::Utc::now().to_rfc3339(),
    });

    state
        .validator
        .validate(&progress, "chapter_progress")
        .map_err(|e| {
            tracing::error!(error = %e, "Chapter progress validation failed");
            internal_error("Chapter progress validation failed")
        })?;

    s.current_chapter_id = None;
    s.updated_at = chrono::Utc::now();
    state.store.persist(s.id);

    Ok(Json(progress))
}
