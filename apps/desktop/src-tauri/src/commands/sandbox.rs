use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Emitter, State};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct SandboxExecuteRequest {
    pub session_id: String,
    pub language: String,
    pub code: String,
    pub stdin: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SandboxExecuteResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct InteractiveStartResponse {
    pub interactive_id: String,
    pub container_id: String,
}

#[command]
pub async fn sandbox_execute(
    state: State<'_, AppState>,
    req: SandboxExecuteRequest,
) -> Result<SandboxExecuteResponse, String> {
    let tool_kind = sandbox_manager::ToolKind::from_language(&req.language)
        .ok_or_else(|| format!("Unsupported language '{}'", req.language))?;
    let session_id = req
        .session_id
        .parse()
        .map_err(|_| "Invalid session_id UUID".to_string())?;

    let result = state
        .sandbox_manager
        .execute(sandbox_manager::SandboxRequest {
            request_id: uuid::Uuid::new_v4(),
            session_id,
            tool_kind,
            code: req.code,
            language: Some(req.language),
            limits: sandbox_manager::models::limits::SandboxLimits {
                run_timeout_secs: req.timeout_secs.unwrap_or(30),
                memory_mb: 512,
                ..Default::default()
            },
            stdin: req.stdin,
            environment: None,
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(SandboxExecuteResponse {
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
        duration_ms: result.duration_ms,
        status: result.status.to_string(),
    })
}

#[command]
pub async fn sandbox_interactive_start(
    app: AppHandle,
    state: State<'_, AppState>,
    req: SandboxExecuteRequest,
) -> Result<InteractiveStartResponse, String> {
    let tool_kind = sandbox_manager::ToolKind::from_language(&req.language)
        .ok_or_else(|| format!("Unsupported language '{}'", req.language))?;
    let session_id = req
        .session_id
        .parse()
        .map_err(|_| "Invalid session_id UUID".to_string())?;

    let request = sandbox_manager::session::request_for_interactive(
        session_id,
        tool_kind,
        req.language,
        req.code,
        req.stdin,
        req.timeout_secs,
    );

    let started = state
        .sandbox_manager
        .start_interactive(request)
        .await
        .map_err(|e| e.to_string())?;

    let interactive_id = started.interactive_id;
    let mut output_rx = state
        .sandbox_manager
        .attach_interactive_output(interactive_id)
        .await
        .map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn(async move {
        while let Some(output) = output_rx.recv().await {
            let payload = match output {
                sandbox_manager::InteractiveOutput::Stdout { data } => {
                    serde_json::json!({ "interactive_id": interactive_id, "type": "stdout", "data": data })
                }
                sandbox_manager::InteractiveOutput::Stderr { data } => {
                    serde_json::json!({ "interactive_id": interactive_id, "type": "stderr", "data": data })
                }
                sandbox_manager::InteractiveOutput::Exit { code } => {
                    serde_json::json!({ "interactive_id": interactive_id, "type": "exit", "code": code })
                }
                sandbox_manager::InteractiveOutput::Error { code, message } => {
                    serde_json::json!({ "interactive_id": interactive_id, "type": "error", "code": code, "message": message })
                }
            };
            let _ = app.emit("sandbox:interactive", payload);
        }
    });

    Ok(InteractiveStartResponse {
        interactive_id: started.interactive_id.to_string(),
        container_id: started.container_id,
    })
}

#[command]
pub async fn sandbox_interactive_stdin(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] interactiveId: String,
    data: String,
) -> Result<Vec<serde_json::Value>, String> {
    let interactive_id = interactiveId
        .parse()
        .map_err(|_| "Invalid interactive_id UUID".to_string())?;
    state
        .sandbox_manager
        .write_interactive_stdin(interactive_id, data)
        .await
        .map_err(|e| e.to_string())?;
    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    Ok(drain_interactive_output(&state, interactive_id).await)
}

#[command]
pub async fn sandbox_interactive_kill(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] interactiveId: String,
) -> Result<bool, String> {
    let interactive_id = interactiveId
        .parse()
        .map_err(|_| "Invalid interactive_id UUID".to_string())?;
    state
        .sandbox_manager
        .kill_interactive(interactive_id)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn sandbox_interactive_poll(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] interactiveId: String,
) -> Result<Vec<serde_json::Value>, String> {
    let interactive_id = interactiveId
        .parse()
        .map_err(|_| "Invalid interactive_id UUID".to_string())?;
    Ok(drain_interactive_output(&state, interactive_id).await)
}

async fn drain_interactive_output(
    state: &State<'_, AppState>,
    interactive_id: uuid::Uuid,
) -> Vec<serde_json::Value> {
    let mut messages = Vec::new();
    for output in state
        .sandbox_manager
        .drain_interactive_output(interactive_id)
        .await
    {
        messages.push(match output {
            sandbox_manager::InteractiveOutput::Stdout { data } => {
                serde_json::json!({ "interactive_id": interactive_id, "type": "stdout", "data": data })
            }
            sandbox_manager::InteractiveOutput::Stderr { data } => {
                serde_json::json!({ "interactive_id": interactive_id, "type": "stderr", "data": data })
            }
            sandbox_manager::InteractiveOutput::Exit { code } => {
                serde_json::json!({ "interactive_id": interactive_id, "type": "exit", "code": code })
            }
            sandbox_manager::InteractiveOutput::Error { code, message } => {
                serde_json::json!({ "interactive_id": interactive_id, "type": "error", "code": code, "message": message })
            }
        });
    }
    messages
}
