use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

use crate::config::SandboxConfig;
use crate::error::SandboxError;
use crate::generated::{ExecutionModel, ToolKind};
use crate::models::request::SandboxRequest;
use crate::models::result::{ResourceUsage, SandboxResult};
use crate::models::status::ExecutionStatus;

pub struct ContainerExecutor {
    config: SandboxConfig,
}

impl ContainerExecutor {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    pub async fn execute(&self, request: SandboxRequest) -> Result<SandboxResult, SandboxError> {
        let container_name = format!("blup-sandbox-{}", request.request_id);
        let image = request.tool_kind.to_image();
        let total_timeout = Duration::from_secs(
            request.limits.compile_timeout_secs + request.limits.run_timeout_secs,
        );

        // Typst needs more PIDs for rayon parallelism
        let needs_extra_pids = matches!(request.tool_kind, ToolKind::TypstCompile);
        let pids_limit = if request.limits.max_processes < 64 && needs_extra_pids {
            64
        } else {
            request.limits.max_processes
        };

        let execution_model = request.tool_kind.execution_model();
        let code = request.code.clone();

        // Build docker run command (without --rm so we can inspect after exit)
        let mut cmd = Command::new("docker");
        cmd.args(["run"])
            .args(["--name", &container_name])
            .args(["--memory", &format!("{}m", request.limits.memory_mb)])
            .args(["--cpus", &format!("{}", request.limits.cpu_count)])
            .args(["--pids-limit", &format!("{}", pids_limit)])
            .args(["--security-opt", "no-new-privileges:true"])
            .args(["--read-only"])
            .args([
                "--tmpfs",
                "/workspace:rw,nosuid,exec,size=100m,uid=1000,gid=1000",
            ])
            .args(["--tmpfs", "/tmp:rw,nosuid,exec,size=10m,uid=1000,gid=1000"])
            .args(["--cap-drop", "ALL"])
            .args(["--user", "1000:1000"]);

        // Network configuration
        if request.limits.network_enabled {
            cmd.args(["--network", "bridge"]);
        } else {
            cmd.args(["--network", "none"]);
        }

        // Seccomp profile
        if let Some(ref profile) = self.config.seccomp_profile {
            cmd.args(["--security-opt", &format!("seccomp={}", profile)]);
        }

        match execution_model {
            ExecutionModel::Interpreted => {
                // Interpreted-language images have ENTRYPOINT set (e.g. ["python", "-c"]).
                // Just pass the code as the sole argument — the ENTRYPOINT handles the rest.
                cmd.arg(image);
                cmd.arg(&code);
            }
            ExecutionModel::Compiled => {
                // docker run -i --entrypoint <runner_script> <image>
                // code piped via stdin — -i keeps stdin open so the runner can read it
                cmd.arg("-i");
                if let Some(runner) = request.tool_kind.runner_script() {
                    cmd.args(["--entrypoint", runner]);
                }
                cmd.arg(image);
                cmd.stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
            }
        }

        // Execute with timeout
        let start = std::time::Instant::now();
        let result = timeout(total_timeout, async {
            let output = match execution_model {
                ExecutionModel::Interpreted => tokio::task::spawn_blocking(move || cmd.output())
                    .await
                    .map_err(|e| SandboxError::container(&format!("Failed to spawn task: {}", e)))?
                    .map_err(|e| {
                        SandboxError::container(&format!("Failed to execute docker: {}", e))
                    })?,
                ExecutionModel::Compiled => {
                    // Spawn the process, pipe code to stdin, then wait
                    let mut child = tokio::task::spawn_blocking(move || {
                        cmd.spawn().map_err(|e| {
                            SandboxError::container(&format!("Failed to spawn docker: {}", e))
                        })
                    })
                    .await
                    .map_err(|e| SandboxError::container(&format!("Spawn task failed: {}", e)))??;

                    {
                        let stdin = child.stdin.take();
                        if let Some(mut stdin) = stdin {
                            stdin.write_all(code.as_bytes()).map_err(|e| {
                                SandboxError::container(&format!("Failed to write stdin: {}", e))
                            })?;
                        }
                        // stdin dropped here -> closes pipe -> runner reads EOF
                    }

                    tokio::task::spawn_blocking(move || child.wait_with_output())
                        .await
                        .map_err(|e| SandboxError::container(&format!("Wait task failed: {}", e)))?
                        .map_err(|e| {
                            SandboxError::container(&format!(
                                "Failed to wait for docker output: {}",
                                e
                            ))
                        })?
                }
            };

            Ok::<_, SandboxError>(output)
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                let duration_ms = start.elapsed().as_millis() as u64;

                // Inspect container to capture resource usage before removing
                let resource_usage = self.inspect_container(&container_name);

                // Clean up container
                self.force_remove_container(&container_name);

                self.parse_output(
                    request.request_id,
                    output,
                    &request,
                    duration_ms,
                    resource_usage,
                )
            }
            Ok(Err(e)) => {
                self.force_remove_container(&container_name);
                Err(e)
            }
            Err(_elapsed) => {
                // Timeout - force kill, then inspect before removing
                let _ = Command::new("docker")
                    .args(["kill", "--signal=SIGKILL", &container_name])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();

                std::thread::sleep(Duration::from_millis(500));

                let resource_usage = self.inspect_container(&container_name);
                self.force_remove_container(&container_name);

                Ok(SandboxResult {
                    request_id: request.request_id,
                    session_id: Some(request.session_id),
                    status: ExecutionStatus::TimeoutRun,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "Execution timed out".to_string(),
                    stdout_truncated: false,
                    stderr_truncated: false,
                    duration_ms: total_timeout.as_millis() as u64,
                    resource_usage,
                    error: Some(crate::models::result::ErrorDetails {
                        code: "TIMEOUT".to_string(),
                        message: "Execution timed out".to_string(),
                    }),
                })
            }
        }
    }

    /// Inspect a container with `docker inspect` to extract resource usage metrics.
    fn inspect_container(&self, name: &str) -> ResourceUsage {
        // Safely read OOMKilled from docker inspect
        let _oom_killed = Command::new("docker")
            .args(["inspect", "--format", "{{json .State}}", name])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("OOMKilled").and_then(|v| v.as_bool()))
            .unwrap_or(false);

        // Try to get memory stats from docker stats (one-shot)
        let stats = Command::new("docker")
            .args(["stats", "--no-stream", "--format", "{{json .}}", name])
            .output();

        let (peak_memory_mb, cpu_time_ms) = stats
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .map(|v| {
                let mem_str = v
                    .get("MemUsage")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0MiB / 0MiB");
                // Parse "12.5MiB / 256MiB" → peak in MB
                let mem_mb = mem_str
                    .split(" / ")
                    .next()
                    .and_then(|used| {
                        let used = used.trim();
                        if used.ends_with("GiB") {
                            used.trim_end_matches("GiB")
                                .trim()
                                .parse::<f64>()
                                .ok()
                                .map(|v| v * 1024.0)
                        } else if used.ends_with("MiB") {
                            used.trim_end_matches("MiB").trim().parse::<f64>().ok()
                        } else if used.ends_with("KiB") {
                            used.trim_end_matches("KiB")
                                .trim()
                                .parse::<f64>()
                                .ok()
                                .map(|v| v / 1024.0)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.0);

                let cpu_str = v.get("CPUPerc").and_then(|v| v.as_str()).unwrap_or("0.00%");
                let cpu_pct = cpu_str
                    .trim_end_matches('%')
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(0.0);

                (mem_mb, (cpu_pct * 10.0) as u64)
            })
            .unwrap_or((0.0, 0));

        ResourceUsage {
            peak_memory_mb,
            cpu_time_ms,
            disk_used_kb: 0,
        }
    }

    fn parse_output(
        &self,
        request_id: Uuid,
        output: std::process::Output,
        request: &SandboxRequest,
        duration_ms: u64,
        resource_usage: ResourceUsage,
    ) -> Result<SandboxResult, SandboxError> {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code();

        // Truncate output if too large (64KB)
        let max_size = 64 * 1024;
        let (stdout, stdout_truncated) = if stdout.len() > max_size {
            (stdout[..max_size].to_string(), true)
        } else {
            (stdout, false)
        };
        let (stderr, stderr_truncated) = if stderr.len() > max_size {
            (stderr[..max_size].to_string(), true)
        } else {
            (stderr, false)
        };

        // Determine status: prioritize resource limit signals over exit code
        let status = match exit_code {
            Some(0) => ExecutionStatus::Success,
            Some(137) => {
                // SIGKILL (137 = 128 + 9) — typically OOM or timeout signal
                if resource_usage.peak_memory_mb >= request.limits.memory_mb as f64 {
                    ExecutionStatus::MemoryExceeded
                } else {
                    ExecutionStatus::NonZeroExit
                }
            }
            Some(_code) => ExecutionStatus::NonZeroExit,
            None => ExecutionStatus::InternalError,
        };

        Ok(SandboxResult {
            request_id,
            session_id: Some(request.session_id),
            status,
            exit_code,
            stdout,
            stderr,
            stdout_truncated,
            stderr_truncated,
            duration_ms,
            resource_usage,
            error: None,
        })
    }

    fn force_remove_container(&self, name: &str) {
        let _ = Command::new("docker")
            .args(["rm", "-f", name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}
