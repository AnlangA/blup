use std::collections::HashMap;

use axum::{
    extract::State,
    http::StatusCode,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use uuid::Uuid;

use crate::llm::client::{GatewayMessage, GatewayRequest};
use crate::models::types::*;
use crate::state::types::Transition;
use crate::AppState;

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
) -> Result<crate::state::session::Session, (StatusCode, Json<ErrorResponse>)> {
    state.store.get(id).await.ok_or_else(not_found)
}

async fn save(state: &AppState, mut session: crate::state::session::Session) {
    session.updated_at = chrono::Utc::now();
    state.store.update(session).await;
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

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

async fn health() -> Json<serde_json::Value> {
    Json(json!({"status": "ok", "version": "0.1.0"}))
}

async fn create_session(
    State(state): State<AppState>,
) -> Result<Json<CreateSessionResponse>, StatusCode> {
    let session = state.store.create().await;
    Ok(Json(CreateSessionResponse {
        session_id: session.id.to_string(),
        state: session.state().to_string(),
    }))
}

async fn get_session_status(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let session = load_or_404(&state, id).await?;
    Ok(Json(json!({
        "session_id": session.id.to_string(),
        "state": session.state().to_string(),
        "goal": session.goal,
        "feasibility_result": session.feasibility_result,
        "profile": session.profile,
        "curriculum": session.curriculum,
        "current_chapter_id": session.current_chapter_id,
        "chapter_contents": serde_json::to_value(&session.chapter_contents).unwrap_or(json!({})),
        "messages": session.messages,
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

    let mut s = load_or_404(&state, id).await?;

    // Transition: Idle → GoalInput → FeasibilityCheck
    let current = s.state();
    if current == crate::state::types::SessionState::Idle {
        s.state_machine
            .transition(Transition::SubmitGoal)
            .map_err(invalid_transition)?;
    }
    s.state_machine
        .transition(Transition::SubmitGoal)
        .map_err(invalid_transition)?;

    s.goal = Some(
        serde_json::to_value(&goal)
            .map_err(|e| internal_error(&format!("Failed to serialize goal: {e}")))?,
    );
    save(&state, s).await;

    let goal_desc = goal.description.clone();
    let goal_domain = goal.domain.clone();
    let goal_context = goal.context.clone().unwrap_or_default();

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

    let mut session = state.store.get(id).await.ok_or_else(not_found)?;
    let feasible = feasibility
        .get("feasible")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    session.feasibility_result = Some(feasibility.clone());
    if feasible {
        let _ = session.state_machine.transition(Transition::GoalFeasible);
    } else {
        let _ = session.state_machine.transition(Transition::GoalInfeasible);
    }
    save(&state, session).await;

    Ok(Json(json!({
        "feasibility": feasibility,
        "state": if feasible { "PROFILE_COLLECTION" } else { "GOAL_INPUT" }
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

    let mut s = load_or_404(&state, id).await?;

    // Transition: Idle → GoalInput → FeasibilityCheck
    let current = s.state();
    if current == crate::state::types::SessionState::Idle {
        s.state_machine
            .transition(Transition::SubmitGoal)
            .map_err(invalid_transition)?;
    }
    s.state_machine
        .transition(Transition::SubmitGoal)
        .map_err(invalid_transition)?;

    s.goal = Some(
        serde_json::to_value(&goal)
            .map_err(|e| internal_error(&format!("Failed to serialize goal: {e}")))?,
    );
    save(&state, s).await;

    let goal_desc = goal.description.clone();
    let goal_domain = goal.domain.clone();
    let goal_context = goal.context.clone().unwrap_or_default();

    let mut vars = HashMap::new();
    vars.insert("learning_goal".to_string(), goal_desc.clone());
    vars.insert("domain".to_string(), goal_domain.clone());
    vars.insert("context".to_string(), goal_context);

    let system_prompt = state
        .prompts
        .load_and_render("feasibility_check", 1, &vars)
        .map_err(|_| internal_error("Failed to load prompt"))?;

    let user_prompt = format!("Learning goal: {goal_desc}\nDomain: {goal_domain}");

    // Create SSE stream
    let stream = async_stream::stream! {
        // Send status event
        yield Ok(Event::default().data(serde_json::to_string(&SseEvent::Status {
            state: "FEASIBILITY_CHECK".to_string(),
            message: "Checking goal feasibility...".to_string(),
        }).unwrap()));

        // Call LLM
        match llm_json(&state, &system_prompt, &user_prompt, "feasibility_result").await {
            Ok(feasibility) => {
                // Update session
                let mut session = state.store.get(id).await.unwrap();
                let feasible = feasibility
                    .get("feasible")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                session.feasibility_result = Some(feasibility.clone());
                if feasible {
                    let _ = session.state_machine.transition(Transition::GoalFeasible);
                } else {
                    let _ = session.state_machine.transition(Transition::GoalInfeasible);
                }
                save(&state, session).await;

                // Send done event
                yield Ok(Event::default().data(serde_json::to_string(&SseEvent::Done {
                    result: json!({
                        "feasibility": feasibility,
                        "state": if feasible { "PROFILE_COLLECTION" } else { "GOAL_INPUT" }
                    }),
                }).unwrap()));
            }
            Err(e) => {
                tracing::error!(error = %e, "Feasibility check failed");
                yield Ok(Event::default().data(serde_json::to_string(&SseEvent::Error {
                    code: "LLM_ERROR".to_string(),
                    message: e,
                }).unwrap()));
            }
        }
    };

    Ok(Sse::new(stream))
}

// ---- submit_profile_answer ----

async fn submit_profile_answer(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(answer): Json<ProfileAnswer>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut session = load_or_404(&state, id).await?;

    session
        .state_machine
        .transition(Transition::ProfileComplete)
        .map_err(invalid_transition)?;
    save(&state, session).await;

    let goal_desc = state
        .store
        .get(id)
        .await
        .and_then(|s| s.goal)
        .and_then(|g| g.get("description").cloned())
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "Unknown goal".to_string());

    let goal_domain = state
        .store
        .get(id)
        .await
        .and_then(|s| s.goal)
        .and_then(|g| g.get("domain").cloned())
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "general".to_string());

    let mut vars = HashMap::new();
    vars.insert("learning_goal".to_string(), goal_desc);
    vars.insert("domain".to_string(), goal_domain);
    vars.insert("answer".to_string(), answer.answer.clone());

    let system_prompt = state
        .prompts
        .load_and_render("profile_collection", 1, &vars)
        .map_err(|_| internal_error("Failed to load profile prompt"))?;

    let user_prompt = format!(
        "The user was asked about their experience level. Their answer: {}",
        answer.answer
    );

    let profile = llm_json(&state, &system_prompt, &user_prompt, "user_profile")
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Profile collection failed");
            internal_error(&e)
        })?;

    let mut session = state.store.get(id).await.ok_or_else(not_found)?;
    session.profile = Some(profile.clone());
    save(&state, session).await;

    Ok(Json(json!({
        "is_complete": true,
        "profile": profile,
        "state": "CURRICULUM_PLANNING"
    })))
}

// ---- get_curriculum ----

async fn get_curriculum(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let session = load_or_404(&state, id).await?;

    // Allow both states
    let current = session.state();
    if current != crate::state::types::SessionState::ChapterLearning
        && current != crate::state::types::SessionState::CurriculumPlanning
    {
        return Err(invalid_transition(format!(
            "Cannot get curriculum in state {current}"
        )));
    }

    if let Some(ref curriculum) = session.curriculum {
        return Ok(Json(curriculum.clone()));
    }

    let goal_desc = session
        .goal
        .as_ref()
        .and_then(|g| g.get("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown goal");

    let profile_json = session.profile.clone().unwrap_or_else(|| {
        json!({
            "experience_level": {"domain_knowledge": "beginner"},
            "learning_style": {"preferred_format": ["text"]},
            "available_time": {"hours_per_week": 5}
        })
    });

    let mut vars = HashMap::new();
    vars.insert("learning_goal".to_string(), goal_desc.to_string());
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

    let mut session = state.store.get(id).await.ok_or_else(not_found)?;
    session.curriculum = Some(curriculum.clone());
    let _ = session
        .state_machine
        .transition(Transition::CurriculumReady);
    save(&state, session).await;

    Ok(Json(curriculum))
}

// ---- start_chapter ----

async fn start_chapter(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let session = load_or_404(&state, id).await?;

    let current = session.state();
    if current != crate::state::types::SessionState::ChapterLearning {
        return Err(invalid_transition(format!(
            "Cannot start chapter in state {current}"
        )));
    }

    // Check cache first
    if let Some(cached_content) = session.chapter_contents.get(&ch_id).cloned() {
        tracing::info!(chapter_id = %ch_id, "Returning cached chapter content");
        let mut session = session;
        session.current_chapter_id = Some(ch_id);
        save(&state, session).await;

        return Ok(Json(json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "role": "assistant",
            "content": cached_content,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })));
    }

    // Get chapter info from curriculum
    let chapter_info = session.curriculum.as_ref().and_then(|c| {
        c.get("chapters").and_then(|chapters| {
            chapters.as_array().and_then(|arr| {
                arr.iter()
                    .find(|ch| ch.get("id").and_then(|v| v.as_str()) == Some(&ch_id))
            })
        })
    });

    let profile = session.profile.clone().unwrap_or_else(|| {
        json!({
            "experience_level": {"domain_knowledge": "beginner"},
            "learning_style": {"preferred_format": ["text"]},
            "available_time": {"hours_per_week": 5}
        })
    });

    let mut vars = HashMap::new();
    vars.insert("chapter_id".to_string(), ch_id.clone());
    vars.insert(
        "user_profile".to_string(),
        serde_json::to_string(&profile).unwrap_or_default(),
    );
    vars.insert("curriculum_context".to_string(), "{}".to_string());

    let system_prompt = state
        .prompts
        .load_and_render("chapter_teaching", 1, &vars)
        .map_err(|_| internal_error("Failed to load chapter prompt"))?;

    let chapter_title = chapter_info
        .and_then(|c| c.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or(&ch_id);

    let user_prompt = format!("Start teaching chapter: {chapter_title}");

    // Use llm_text for markdown content
    let content = llm_text(&state, &system_prompt, &user_prompt)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Chapter teaching failed");
            internal_error(&e)
        })?;

    // Cache the content
    let mut session = state.store.get(id).await.ok_or_else(not_found)?;
    session.current_chapter_id = Some(ch_id.clone());
    session.chapter_contents.insert(ch_id, content.clone());
    save(&state, session).await;

    // Return as message format
    Ok(Json(json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "role": "assistant",
        "content": content,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
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

    let session = load_or_404(&state, id).await?;

    let current = session.state();
    if current != crate::state::types::SessionState::ChapterLearning {
        return Err(invalid_transition(format!(
            "Cannot ask question in state {current}"
        )));
    }

    let profile = session.profile.clone().unwrap_or_else(|| {
        json!({
            "experience_level": {"domain_knowledge": "beginner"},
            "learning_style": {"preferred_format": ["text"]},
            "available_time": {"hours_per_week": 5}
        })
    });

    let mut vars = HashMap::new();
    vars.insert("question".to_string(), question.question.clone());
    vars.insert(
        "user_profile".to_string(),
        serde_json::to_string(&profile).unwrap_or_default(),
    );

    let system_prompt = state
        .prompts
        .load_and_render("question_answering", 1, &vars)
        .map_err(|_| internal_error("Failed to load Q&A prompt"))?;

    let user_prompt = format!("Question: {}", question.question);

    // Use llm_text for markdown content
    let content = llm_text(&state, &system_prompt, &user_prompt)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Question answering failed");
            internal_error(&e)
        })?;

    let message = json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "role": "assistant",
        "content": content,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let mut session = state.store.get(id).await.ok_or_else(not_found)?;
    session.messages.push(message.clone());
    save(&state, session).await;

    Ok(Json(message))
}

// ---- complete_chapter ----

async fn complete_chapter(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut session = load_or_404(&state, id).await?;

    session
        .state_machine
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

    session.current_chapter_id = None;
    save(&state, session).await;

    Ok(Json(progress))
}
