use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

use crate::config::SandboxConfig;
use crate::error::SandboxError;
use crate::generated::{ExecutionModel, ToolKind};
use crate::models::limits::SandboxLimits;
use crate::models::request::SandboxRequest;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InteractiveStatus {
    Starting,
    Running,
    Terminated,
    Failed,
    Killed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InteractiveOutput {
    Stdout { data: String },
    Stderr { data: String },
    Exit { code: Option<i32> },
    Error { code: String, message: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct InteractiveStartResult {
    pub interactive_id: Uuid,
    pub container_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InteractiveSessionInfo {
    pub interactive_id: Uuid,
    pub session_id: Uuid,
    pub language: Option<String>,
    pub status: InteractiveStatus,
    pub idle_seconds: u64,
    pub created_seconds_ago: u64,
    pub container_id: String,
}

struct InteractiveSession {
    interactive_id: Uuid,
    session_id: Uuid,
    container_id: String,
    language: Option<String>,
    status: RwLock<InteractiveStatus>,
    stdin_tx: mpsc::Sender<Vec<u8>>,
    output_rx: Mutex<Option<mpsc::Receiver<InteractiveOutput>>>,
    backlog: Mutex<Vec<InteractiveOutput>>,
    created_at: Instant,
    last_io_at: RwLock<Instant>,
}

#[derive(Clone)]
pub struct InteractiveSessionManager {
    config: SandboxConfig,
    sessions: Arc<RwLock<HashMap<Uuid, Arc<InteractiveSession>>>>,
    max_per_session: usize,
}

impl InteractiveSessionManager {
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_per_session: 5,
        }
    }

    pub async fn start(
        &self,
        request: SandboxRequest,
    ) -> Result<InteractiveStartResult, SandboxError> {
        if self.count_for_session(request.session_id).await >= self.max_per_session {
            return Err(SandboxError::resource_limit(
                "too many active interactive sandboxes for this session",
            ));
        }

        let interactive_id = Uuid::new_v4();
        let container_id = format!("blup-interactive-{interactive_id}");
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<Vec<u8>>(32);
        let (output_tx, output_rx) = mpsc::channel::<InteractiveOutput>(128);

        let mut cmd = self.build_docker_command(&container_id, &request);
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| SandboxError::container(&format!("Failed to spawn docker: {e}")))?;

        let mut child_stdin = child
            .stdin
            .take()
            .ok_or_else(|| SandboxError::container("Failed to open container stdin"))?;
        if let Some(initial_stdin) = request.stdin.clone() {
            child_stdin.write_all(initial_stdin.as_bytes()).await?;
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SandboxError::container("Failed to open container stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| SandboxError::container("Failed to open container stderr"))?;

        let session = Arc::new(InteractiveSession {
            interactive_id,
            session_id: request.session_id,
            container_id: container_id.clone(),
            language: request.language.clone(),
            status: RwLock::new(InteractiveStatus::Running),
            stdin_tx,
            output_rx: Mutex::new(Some(output_rx)),
            backlog: Mutex::new(Vec::new()),
            created_at: Instant::now(),
            last_io_at: RwLock::new(Instant::now()),
        });

        self.sessions
            .write()
            .await
            .insert(interactive_id, session.clone());

        let stdin_session = session.clone();
        tokio::spawn(async move {
            while let Some(bytes) = stdin_rx.recv().await {
                *stdin_session.last_io_at.write().await = Instant::now();
                if child_stdin.write_all(&bytes).await.is_err() {
                    break;
                }
                let _ = child_stdin.flush().await;
            }
        });

        spawn_reader(stdout, output_tx.clone(), true, session.clone());
        spawn_reader(stderr, output_tx.clone(), false, session.clone());

        let wait_sessions = self.sessions.clone();
        let wait_container_id = container_id.clone();
        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => {
                    *session.status.write().await = InteractiveStatus::Terminated;
                    let msg = InteractiveOutput::Exit {
                        code: status.code(),
                    };
                    session.backlog.lock().await.push(msg.clone());
                    let _ = output_tx.send(msg).await;
                }
                Err(e) => {
                    *session.status.write().await = InteractiveStatus::Failed;
                    let msg = InteractiveOutput::Error {
                        code: "PROCESS_ERROR".to_string(),
                        message: e.to_string(),
                    };
                    session.backlog.lock().await.push(msg.clone());
                    let _ = output_tx.send(msg).await;
                }
            }
            force_remove_container(&wait_container_id).await;
            tokio::time::sleep(Duration::from_secs(30)).await;
            wait_sessions.write().await.remove(&interactive_id);
        });

        self.spawn_timeout_watch(interactive_id, request.limits.run_timeout_secs);

        Ok(InteractiveStartResult {
            interactive_id,
            container_id,
        })
    }

    pub async fn attach_output(
        &self,
        interactive_id: Uuid,
    ) -> Result<mpsc::Receiver<InteractiveOutput>, SandboxError> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions
                .get(&interactive_id)
                .cloned()
                .ok_or_else(|| SandboxError::container("interactive session not found"))?
        };
        let output = session
            .output_rx
            .lock()
            .await
            .take()
            .ok_or_else(|| SandboxError::container("interactive output already attached"))?;
        Ok(output)
    }

    pub async fn drain_output(&self, interactive_id: Uuid) -> Vec<InteractiveOutput> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions.get(&interactive_id).cloned()
        };
        let Some(session) = session else {
            return Vec::new();
        };
        let mut backlog = session.backlog.lock().await;
        std::mem::take(&mut *backlog)
    }

    pub async fn write_stdin(
        &self,
        interactive_id: Uuid,
        data: String,
    ) -> Result<(), SandboxError> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&interactive_id)
            .ok_or_else(|| SandboxError::container("interactive session not found"))?;
        *session.last_io_at.write().await = Instant::now();
        session
            .stdin_tx
            .send(data.into_bytes())
            .await
            .map_err(|_| SandboxError::container("interactive stdin closed"))
    }

    pub async fn kill(&self, interactive_id: Uuid) -> Result<bool, SandboxError> {
        let session = self.sessions.write().await.remove(&interactive_id);
        if let Some(session) = session {
            *session.status.write().await = InteractiveStatus::Killed;
            force_remove_container(&session.container_id).await;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn list(&self) -> Vec<InteractiveSessionInfo> {
        let sessions = self.sessions.read().await;
        let mut items = Vec::with_capacity(sessions.len());
        for session in sessions.values() {
            items.push(InteractiveSessionInfo {
                interactive_id: session.interactive_id,
                session_id: session.session_id,
                language: session.language.clone(),
                status: session.status.read().await.clone(),
                idle_seconds: session.last_io_at.read().await.elapsed().as_secs(),
                created_seconds_ago: session.created_at.elapsed().as_secs(),
                container_id: session.container_id.clone(),
            });
        }
        items
    }

    async fn count_for_session(&self, session_id: Uuid) -> usize {
        let sessions = self.sessions.read().await;
        let mut count = 0;
        for session in sessions.values() {
            if session.session_id == session_id {
                count += 1;
            }
        }
        count
    }

    fn spawn_timeout_watch(&self, interactive_id: Uuid, timeout_secs: u64) {
        let manager = self.clone();
        let idle_timeout = Duration::from_secs(timeout_secs.max(180));
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let should_kill = {
                    let sessions = manager.sessions.read().await;
                    if let Some(session) = sessions.get(&interactive_id) {
                        session.last_io_at.read().await.elapsed() > idle_timeout
                    } else {
                        false
                    }
                };
                if should_kill {
                    let _ = manager.kill(interactive_id).await;
                    break;
                }
            }
        });
    }

    fn build_docker_command(&self, container_name: &str, request: &SandboxRequest) -> Command {
        let image = request.tool_kind.to_image();
        let mut cmd = Command::new("docker");
        cmd.args(["run", "-i"])
            .args(["--name", container_name])
            .args(["--env", "PYTHONUNBUFFERED=1"])
            .args(["--memory", &format!("{}m", request.limits.memory_mb)])
            .args(["--cpus", &format!("{}", request.limits.cpu_count)])
            .args(["--pids-limit", &format!("{}", pids_limit(request))])
            .args(["--security-opt", "no-new-privileges:true"])
            .args(["--read-only"])
            .args([
                "--tmpfs",
                "/workspace:rw,nosuid,size=100m,uid=1000,gid=1000",
            ])
            .args(["--tmpfs", "/tmp:rw,nosuid,size=10m,uid=1000,gid=1000"])
            .args(["--cap-drop", "ALL"])
            .args(["--user", "1000:1000"]);

        if request.limits.network_enabled {
            cmd.args(["--network", "bridge"]);
        } else {
            cmd.args(["--network", "none"]);
        }

        if let Some(ref profile) = self.config.seccomp_profile {
            cmd.args(["--security-opt", &format!("seccomp={profile}")]);
        }

        match request.tool_kind.execution_model() {
            ExecutionModel::Interpreted => {
                if matches!(request.tool_kind, ToolKind::PythonExec) {
                    cmd.args(["--entrypoint", "python"]);
                }
                cmd.arg(image);
                if matches!(request.tool_kind, ToolKind::PythonExec) {
                    cmd.arg("-u");
                    cmd.arg("-c");
                }
                cmd.arg(&request.code);
            }
            ExecutionModel::Compiled => {
                if let Some(runner) = request.tool_kind.runner_script() {
                    cmd.args(["--entrypoint", runner]);
                }
                cmd.arg(image);
                cmd.arg(&request.code);
            }
        }
        cmd
    }
}

fn pids_limit(request: &SandboxRequest) -> u32 {
    let needs_extra_pids = matches!(request.tool_kind, ToolKind::TypstCompile);
    if request.limits.max_processes < 64 && needs_extra_pids {
        64
    } else {
        request.limits.max_processes
    }
}

fn spawn_reader<R>(
    mut reader: R,
    tx: mpsc::Sender<InteractiveOutput>,
    is_stdout: bool,
    session: Arc<InteractiveSession>,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut buf = vec![0_u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    *session.last_io_at.write().await = Instant::now();
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let msg = if is_stdout {
                        InteractiveOutput::Stdout { data }
                    } else {
                        InteractiveOutput::Stderr { data }
                    };
                    session.backlog.lock().await.push(msg.clone());
                    if tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let msg = InteractiveOutput::Error {
                        code: "STREAM_ERROR".to_string(),
                        message: e.to_string(),
                    };
                    session.backlog.lock().await.push(msg.clone());
                    let _ = tx.send(msg).await;
                    break;
                }
            }
        }
    });
}

async fn force_remove_container(name: &str) {
    let name = name.to_string();
    let _ = tokio::task::spawn_blocking(move || {
        std::process::Command::new("docker")
            .args(["rm", "-f", &name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    })
    .await;
}

pub fn request_for_interactive(
    session_id: Uuid,
    tool_kind: ToolKind,
    language: String,
    code: String,
    stdin: Option<String>,
    timeout_secs: Option<u64>,
) -> SandboxRequest {
    SandboxRequest {
        request_id: Uuid::new_v4(),
        session_id,
        tool_kind,
        code,
        language: Some(language),
        limits: SandboxLimits {
            run_timeout_secs: timeout_secs.unwrap_or(180),
            memory_mb: 512,
            ..SandboxLimits::default()
        },
        stdin,
        environment: None,
    }
}
