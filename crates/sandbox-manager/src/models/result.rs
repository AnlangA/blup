use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::status::ExecutionStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    pub request_id: Uuid,
    pub session_id: Option<Uuid>,
    pub status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub duration_ms: u64,
    pub resource_usage: ResourceUsage,
    pub error: Option<ErrorDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub peak_memory_mb: f64,
    pub cpu_time_ms: u64,
    pub disk_used_kb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
}

impl SandboxResult {
    pub fn success(request_id: Uuid, stdout: &str, stderr: &str, duration_ms: u64) -> Self {
        Self {
            request_id,
            session_id: None,
            status: ExecutionStatus::Success,
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms,
            resource_usage: ResourceUsage::default(),
            error: None,
        }
    }

    pub fn timeout(request_id: Uuid, duration_ms: u64) -> Self {
        Self {
            request_id,
            session_id: None,
            status: ExecutionStatus::TimeoutRun,
            exit_code: None,
            stdout: String::new(),
            stderr: "Execution timed out".to_string(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms,
            resource_usage: ResourceUsage::default(),
            error: Some(ErrorDetails {
                code: "TIMEOUT".to_string(),
                message: "Execution timed out".to_string(),
            }),
        }
    }

    pub fn error(request_id: Uuid, message: &str) -> Self {
        Self {
            request_id,
            session_id: None,
            status: ExecutionStatus::InternalError,
            exit_code: None,
            stdout: String::new(),
            stderr: message.to_string(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 0,
            resource_usage: ResourceUsage::default(),
            error: Some(ErrorDetails {
                code: "INTERNAL_ERROR".to_string(),
                message: message.to_string(),
            }),
        }
    }

    pub fn is_success(&self) -> bool {
        self.status == ExecutionStatus::Success
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            peak_memory_mb: 0.0,
            cpu_time_ms: 0,
            disk_used_kb: 0,
        }
    }
}
