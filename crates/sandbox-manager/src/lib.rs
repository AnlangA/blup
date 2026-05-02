pub mod config;
pub mod docker;
pub mod error;
pub mod executor;
pub mod models;

pub use config::SandboxConfig;
pub use error::SandboxError;
pub use executor::{DockerExecutor, MockExecutor, SandboxExecutor};
pub use models::request::SandboxRequest;
pub use models::result::SandboxResult;
pub use models::status::ExecutionStatus;

pub struct SandboxManager {
    executor: Box<dyn SandboxExecutor>,
}

impl SandboxManager {
    /// Create a manager with a Docker executor (requires Docker daemon).
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            executor: Box::new(DockerExecutor::new(config)),
        }
    }

    /// Create a manager with a custom executor (e.g. MockExecutor for tests).
    pub fn with_executor(executor: Box<dyn SandboxExecutor>) -> Self {
        Self { executor }
    }

    pub async fn execute(&self, request: SandboxRequest) -> Result<SandboxResult, SandboxError> {
        self.executor.execute(request).await
    }

    pub async fn health_check(&self) -> Result<(), SandboxError> {
        self.executor.health_check().await
    }

    pub fn image_info(&self) -> Vec<models::image::ImageInfo> {
        self.executor.image_info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::limits::SandboxLimits;
    use crate::models::request::{SandboxRequest, ToolKind};
    use crate::models::result::ResourceUsage;
    use crate::models::status::ExecutionStatus;

    #[tokio::test]
    async fn test_sandbox_manager_with_mock() {
        let mut mock = MockExecutor::new();
        mock.push_response(SandboxResult {
            request_id: uuid::Uuid::nil(),
            session_id: None,
            status: ExecutionStatus::Success,
            exit_code: Some(0),
            stdout: "hi\n".to_string(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 5,
            resource_usage: ResourceUsage::default(),
            error: None,
        });

        let manager = SandboxManager::with_executor(Box::new(mock));
        let result = manager
            .execute(SandboxRequest {
                request_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                tool_kind: ToolKind::PythonExec,
                code: "print('hi')".to_string(),
                language: None,
                limits: SandboxLimits::default(),
                stdin: None,
                environment: None,
            })
            .await
            .unwrap();

        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.stdout, "hi\n");
    }

    #[tokio::test]
    async fn test_sandbox_manager_health() {
        let mock = MockExecutor::new();
        let manager = SandboxManager::with_executor(Box::new(mock));
        assert!(manager.health_check().await.is_ok());
    }
}
