use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

use crate::config::SandboxConfig;
use crate::error::SandboxError;
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

        // Build docker run command
        let mut cmd = Command::new("docker");
        cmd.args(["run", "--rm"])
            .args(["--name", &container_name])
            .args(["--memory", &format!("{}m", request.limits.memory_mb)])
            .args(["--cpus", &format!("{}", request.limits.cpu_count)])
            .args(["--pids-limit", &format!("{}", request.limits.max_processes)])
            .args(["--security-opt", "no-new-privileges:true"])
            .args(["--read-only"])
            .args(["--tmpfs", "/workspace:rw,noexec,nosuid,size=100m"])
            .args(["--tmpfs", "/tmp:rw,noexec,nosuid,size=10m"])
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

        // Image and command
        cmd.arg(image);
        cmd.arg(&request.code);

        // Execute with timeout
        let start = std::time::Instant::now();
        let result = timeout(total_timeout, async {
            let output = tokio::task::spawn_blocking(move || cmd.output())
                .await
                .map_err(|e| SandboxError::container(&format!("Failed to spawn task: {}", e)))?
                .map_err(|e| {
                    SandboxError::container(&format!("Failed to execute docker: {}", e))
                })?;

            Ok::<_, SandboxError>(output)
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                // Clean up container
                self.force_remove_container(&container_name);

                // Parse output with measured duration
                let duration_ms = start.elapsed().as_millis() as u64;
                self.parse_output(request.request_id, output, &request, duration_ms)
            }
            Ok(Err(e)) => {
                self.force_remove_container(&container_name);
                Err(e)
            }
            Err(_elapsed) => {
                // Timeout - force kill container
                self.force_kill_container(&container_name);
                Ok(SandboxResult::timeout(
                    request.request_id,
                    total_timeout.as_millis() as u64,
                ))
            }
        }
    }

    fn parse_output(
        &self,
        request_id: Uuid,
        output: std::process::Output,
        request: &SandboxRequest,
        duration_ms: u64,
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

        let status = match exit_code {
            Some(0) => ExecutionStatus::Success,
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
            resource_usage: ResourceUsage::default(),
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

    fn force_kill_container(&self, name: &str) {
        let _ = Command::new("docker")
            .args(["kill", "--signal=SIGKILL", name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        // Wait briefly for the container to stop — this runs on the blocking
        // thread pool (called from spawn_blocking) so thread::sleep is fine.
        std::thread::sleep(Duration::from_millis(500));

        self.force_remove_container(name);
    }
}
