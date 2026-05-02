use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    TimeoutCompile,
    TimeoutRun,
    MemoryExceeded,
    CpuExceeded,
    DiskExceeded,
    NonZeroExit,
    NetworkBlocked,
    InternalError,
}

impl ExecutionStatus {
    pub fn is_timeout(&self) -> bool {
        matches!(
            self,
            ExecutionStatus::TimeoutCompile | ExecutionStatus::TimeoutRun
        )
    }

    pub fn is_resource_exceeded(&self) -> bool {
        matches!(
            self,
            ExecutionStatus::MemoryExceeded
                | ExecutionStatus::CpuExceeded
                | ExecutionStatus::DiskExceeded
        )
    }

    pub fn is_error(&self) -> bool {
        *self != ExecutionStatus::Success
    }

    pub fn to_error_code(&self) -> &str {
        match self {
            ExecutionStatus::Success => "SUCCESS",
            ExecutionStatus::TimeoutCompile => "TIMEOUT_COMPILE",
            ExecutionStatus::TimeoutRun => "TIMEOUT_RUN",
            ExecutionStatus::MemoryExceeded => "MEMORY_EXCEEDED",
            ExecutionStatus::CpuExceeded => "CPU_EXCEEDED",
            ExecutionStatus::DiskExceeded => "DISK_EXCEEDED",
            ExecutionStatus::NonZeroExit => "NON_ZERO_EXIT",
            ExecutionStatus::NetworkBlocked => "NETWORK_BLOCKED",
            ExecutionStatus::InternalError => "INTERNAL_ERROR",
        }
    }
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Success => write!(f, "success"),
            ExecutionStatus::TimeoutCompile => write!(f, "timeout_compile"),
            ExecutionStatus::TimeoutRun => write!(f, "timeout_run"),
            ExecutionStatus::MemoryExceeded => write!(f, "memory_exceeded"),
            ExecutionStatus::CpuExceeded => write!(f, "cpu_exceeded"),
            ExecutionStatus::DiskExceeded => write!(f, "disk_exceeded"),
            ExecutionStatus::NonZeroExit => write!(f, "non_zero_exit"),
            ExecutionStatus::NetworkBlocked => write!(f, "network_blocked"),
            ExecutionStatus::InternalError => write!(f, "internal_error"),
        }
    }
}
