use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures::{SinkExt, StreamExt};
use serde_json::json;
use uuid::Uuid;

use sandbox_manager::models::limits::SandboxLimits;
use sandbox_manager::models::request::SandboxRequest;
use sandbox_manager::session::request_for_interactive;
use sandbox_manager::InteractiveOutput;
use sandbox_manager::ToolKind;

use super::helpers::next_sse_id;
use super::types::{
    InteractiveClientMessage, InteractiveServerMessage, InteractiveStartRequest,
    InteractiveStartResponse, SandboxExecuteRequest, SseEvent,
};
use crate::error::ApiError;
use crate::AppState;

// ── sandbox_execute_stream (SSE) ──

pub async fn sandbox_execute_stream(
    State(state): State<AppState>,
    Json(req): Json<SandboxExecuteRequest>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>, ApiError> {
    let tool_kind = ToolKind::from_language(&req.language)
        .ok_or_else(|| ApiError::Validation(format!("Unsupported language '{}'", req.language)))?;

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

pub async fn sandbox_health(State(state): State<AppState>) -> Json<serde_json::Value> {
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

// ── interactive sandbox ──

pub async fn interactive_start(
    State(state): State<AppState>,
    Json(req): Json<InteractiveStartRequest>,
) -> Result<Json<InteractiveStartResponse>, ApiError> {
    let tool_kind = ToolKind::from_language(&req.language)
        .ok_or_else(|| ApiError::Validation(format!("Unsupported language '{}'", req.language)))?;
    let session_id: Uuid = req
        .session_id
        .parse()
        .map_err(|_| ApiError::Validation("Invalid session_id UUID".to_string()))?;

    let sandbox_request = request_for_interactive(
        session_id,
        tool_kind,
        req.language,
        req.code,
        req.stdin,
        req.timeout_secs,
    );

    let started = state
        .sandbox_manager
        .start_interactive(sandbox_request)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(InteractiveStartResponse {
        interactive_id: started.interactive_id.to_string(),
        container_id: started.container_id,
    }))
}

pub async fn interactive_ws(
    State(state): State<AppState>,
    Path(interactive_id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_interactive_socket(state, interactive_id, socket))
}

async fn handle_interactive_socket(state: AppState, interactive_id: Uuid, socket: WebSocket) {
    let output_rx = match state
        .sandbox_manager
        .attach_interactive_output(interactive_id)
        .await
    {
        Ok(rx) => rx,
        Err(e) => {
            let mut socket = socket;
            let _ = send_ws_error(&mut socket, "ATTACH_FAILED", e.to_string()).await;
            return;
        }
    };

    let (mut sender, mut receiver) = socket.split();
    let sandbox_for_input = state.sandbox_manager.clone();
    let input_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            match message {
                Message::Text(text) => {
                    if let Ok(client_msg) = serde_json::from_str::<InteractiveClientMessage>(&text)
                    {
                        match client_msg {
                            InteractiveClientMessage::Stdin { data } => {
                                let _ = sandbox_for_input
                                    .write_interactive_stdin(interactive_id, data)
                                    .await;
                            }
                            InteractiveClientMessage::Resize { .. } => {}
                        }
                    }
                }
                Message::Binary(bytes) => {
                    if let Ok(data) = String::from_utf8(bytes) {
                        let _ = sandbox_for_input
                            .write_interactive_stdin(interactive_id, data)
                            .await;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let mut output_rx = output_rx;
    while let Some(output) = output_rx.recv().await {
        let server_msg = match output {
            InteractiveOutput::Stdout { data } => InteractiveServerMessage::Stdout { data },
            InteractiveOutput::Stderr { data } => InteractiveServerMessage::Stderr { data },
            InteractiveOutput::Exit { code } => InteractiveServerMessage::Exit { code },
            InteractiveOutput::Error { code, message } => {
                InteractiveServerMessage::Error { code, message }
            }
        };

        let Ok(text) = serde_json::to_string(&server_msg) else {
            continue;
        };
        if sender.send(Message::Text(text)).await.is_err() {
            break;
        }
    }

    input_task.abort();
}

async fn send_ws_error(
    socket: &mut WebSocket,
    code: &str,
    message: String,
) -> Result<(), axum::Error> {
    let msg = InteractiveServerMessage::Error {
        code: code.to_string(),
        message,
    };
    socket
        .send(Message::Text(serde_json::to_string(&msg).unwrap_or_else(
            |_| r#"{"type":"error","code":"SERIALIZE_ERROR","message":"failed"}"#.to_string(),
        )))
        .await
}

pub async fn interactive_kill(
    State(state): State<AppState>,
    Path(interactive_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let killed = state
        .sandbox_manager
        .kill_interactive(interactive_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(json!({ "killed": killed })))
}

pub async fn interactive_list(State(state): State<AppState>) -> Json<serde_json::Value> {
    let sessions = state.sandbox_manager.list_interactive().await;
    Json(json!({ "sessions": sessions }))
}
