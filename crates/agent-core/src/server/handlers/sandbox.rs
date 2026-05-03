use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use serde_json::json;
use uuid::Uuid;

use sandbox_manager::models::limits::SandboxLimits;
use sandbox_manager::models::request::{SandboxRequest, ToolKind};

use super::helpers::next_sse_id;
use super::types::{SandboxExecuteRequest, SseEvent};
use crate::error::ApiError;
use crate::AppState;

// ── sandbox_execute_stream (SSE) ──

pub async fn sandbox_execute_stream(
    State(state): State<AppState>,
    Json(req): Json<SandboxExecuteRequest>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>, ApiError> {
    let tool_kind = language_to_toolkind(&req.language).map_err(|e| {
        ApiError::Validation(format!("Unsupported language '{}': {e}", req.language))
    })?;

    let session_id: Uuid = req
        .session_id
        .parse()
        .map_err(|_| ApiError::Validation("Invalid session_id UUID".to_string()))?;

    let timeout_secs = req.timeout_secs.unwrap_or(30);

    let sandbox_request = SandboxRequest {
        request_id: Uuid::new_v4(),
        session_id,
        tool_kind,
        code: req.code.clone(),
        language: Some(req.language.clone()),
        limits: SandboxLimits {
            run_timeout_secs: timeout_secs,
            memory_mb: 512,
            ..SandboxLimits::default()
        },
        stdin: req.stdin.clone(),
        environment: None,
    };

    let sandbox = state.sandbox_manager.clone();
    let ping_interval = std::time::Duration::from_secs(state.config.sse_ping_interval_secs);

    let stream = async_stream::stream! {
        yield Ok(Event::default()
            .event("status")
            .id(next_sse_id())
            .data(serde_json::to_string(&SseEvent::Status {
                state: "running".to_string(),
                message: format!("Executing {} code...", req.language),
            }).expect("SSE serialize")));

        match sandbox.execute(sandbox_request).await {
            Ok(result) => {
                // Stream stdout in chunks
                for line in result.stdout.lines() {
                    yield Ok(Event::default()
                        .event("stdout")
                        .id(next_sse_id())
                        .data(serde_json::to_string(&SseEvent::Stdout {
                            content: format!("{}\n", line),
                        }).expect("SSE serialize")));
                }

                // Stream stderr in chunks if present
                if !result.stderr.is_empty() {
                    for line in result.stderr.lines() {
                        yield Ok(Event::default()
                            .event("stderr")
                            .id(next_sse_id())
                            .data(serde_json::to_string(&SseEvent::Stderr {
                                content: format!("{}\n", line),
                            }).expect("SSE serialize")));
                    }
                }

                yield Ok(Event::default()
                    .event("done")
                    .id(next_sse_id())
                    .data(serde_json::to_string(&SseEvent::Done {
                        result: json!({
                            "exit_code": result.exit_code,
                            "duration_ms": result.duration_ms,
                            "status": result.status.to_string(),
                        }),
                    }).expect("SSE serialize")));
            }
            Err(e) => {
                yield Ok(Event::default()
                    .event("error")
                    .id(next_sse_id())
                    .data(serde_json::to_string(&SseEvent::Error {
                        code: "SANDBOX_ERROR".to_string(),
                        message: e.to_string(),
                    }).expect("SSE serialize")));
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(ping_interval)))
}

// ── sandbox_health (JSON) ──

pub async fn sandbox_health(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let healthy = state.sandbox_manager.health_check().await.is_ok();
    let images = state
        .sandbox_manager
        .image_info()
        .into_iter()
        .map(|img| {
            json!({
                "name": img.name,
                "version": img.tag,
            })
        })
        .collect::<Vec<_>>();

    Json(json!({
        "healthy": healthy,
        "images": images,
    }))
}

fn language_to_toolkind(lang: &str) -> Result<ToolKind, String> {
    match lang.to_lowercase().as_str() {
        "python" | "py" => Ok(ToolKind::PythonExec),
        "javascript" | "js" | "node" => Ok(ToolKind::NodeExec),
        "rust" | "rs" => Ok(ToolKind::RustCompileRun),
        "typst" => Ok(ToolKind::TypstCompile),
        _ => Err(format!("Unsupported language: {lang}")),
    }
}
