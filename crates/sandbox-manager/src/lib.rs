pub mod config;
pub mod docker;
pub mod error;
pub mod executor;
pub mod generated;
pub mod models;
pub mod session;

pub use config::SandboxConfig;
pub use error::SandboxError;
pub use executor::{DockerExecutor, MockExecutor, SandboxExecutor};
pub use generated::{ExecutionModel, ToolKind};
pub use models::request::SandboxRequest;
pub use models::result::SandboxResult;
pub use models::status::ExecutionStatus;
pub use session::{
    InteractiveOutput, InteractiveSessionInfo, InteractiveSessionManager, InteractiveStartResult,
};
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct SandboxManager {
    executor: Box<dyn SandboxExecutor>,
    interactive: InteractiveSessionManager,
}

impl SandboxManager {
    /// Create a manager with a Docker executor (requires Docker daemon).
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            executor: Box::new(DockerExecutor::new(config.clone())),
            interactive: InteractiveSessionManager::new(config),
        }
    }

    /// Create a manager with a custom executor (e.g. MockExecutor for tests).
    pub fn with_executor(executor: Box<dyn SandboxExecutor>) -> Self {
        Self {
            executor,
            interactive: InteractiveSessionManager::new(SandboxConfig::default()),
        }
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

    pub async fn start_interactive(
        &self,
        request: SandboxRequest,
    ) -> Result<InteractiveStartResult, SandboxError> {
        self.interactive.start(request).await
    }

    pub async fn attach_interactive_output(
        &self,
        interactive_id: Uuid,
    ) -> Result<mpsc::Receiver<InteractiveOutput>, SandboxError> {
        self.interactive.attach_output(interactive_id).await
    }

    pub async fn drain_interactive_output(&self, interactive_id: Uuid) -> Vec<InteractiveOutput> {
        self.interactive.drain_output(interactive_id).await
    }

    pub async fn write_interactive_stdin(
        &self,
        interactive_id: Uuid,
        data: String,
    ) -> Result<(), SandboxError> {
        self.interactive.write_stdin(interactive_id, data).await
    }

    pub async fn kill_interactive(&self, interactive_id: Uuid) -> Result<bool, SandboxError> {
        self.interactive.kill(interactive_id).await
    }

    pub async fn list_interactive(&self) -> Vec<InteractiveSessionInfo> {
        self.interactive.list().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::ToolKind;
    use crate::models::limits::SandboxLimits;
    use crate::models::request::SandboxRequest;
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

    #[tokio::test]
    async fn test_sandbox_manager_unhealthy() {
        let mut mock = MockExecutor::new();
        mock.set_healthy(false);
        let manager = SandboxManager::with_executor(Box::new(mock));
        assert!(manager.health_check().await.is_err());
    }

    #[tokio::test]
    async fn test_sandbox_manager_image_info() {
        let mock = MockExecutor::new();
        let manager = SandboxManager::with_executor(Box::new(mock));
        let images = manager.image_info();
        assert!(!images.is_empty());
    }

    #[tokio::test]
    async fn test_sandbox_manager_with_limits() {
        let mut mock = MockExecutor::new();
        mock.push_response(SandboxResult {
            request_id: uuid::Uuid::nil(),
            session_id: None,
            status: ExecutionStatus::Success,
            exit_code: Some(0),
            stdout: "limited\n".to_string(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 10,
            resource_usage: ResourceUsage::default(),
            error: None,
        });

        let manager = SandboxManager::with_executor(Box::new(mock));
        let limits = SandboxLimits {
            memory_mb: 256,
            run_timeout_secs: 5,
            ..Default::default()
        };

        let result = manager
            .execute(SandboxRequest {
                request_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                tool_kind: ToolKind::PythonExec,
                code: "print('limited')".to_string(),
                language: Some("python".to_string()),
                limits,
                stdin: None,
                environment: None,
            })
            .await
            .unwrap();

        assert_eq!(result.status, ExecutionStatus::Success);
    }

    #[tokio::test]
    async fn test_sandbox_manager_multiple_executions() {
        let mut mock = MockExecutor::new();
        for i in 0..3 {
            mock.push_response(SandboxResult {
                request_id: uuid::Uuid::nil(),
                session_id: None,
                status: ExecutionStatus::Success,
                exit_code: Some(0),
                stdout: format!("output {i}\n"),
                stderr: String::new(),
                stdout_truncated: false,
                stderr_truncated: false,
                duration_ms: i as u64 * 10,
                resource_usage: ResourceUsage::default(),
                error: None,
            });
        }

        let manager = SandboxManager::with_executor(Box::new(mock));

        for i in 0..3 {
            let result = manager
                .execute(SandboxRequest {
                    request_id: uuid::Uuid::new_v4(),
                    session_id: uuid::Uuid::new_v4(),
                    tool_kind: ToolKind::PythonExec,
                    code: format!("print('test {i}')"),
                    language: Some("python".to_string()),
                    limits: SandboxLimits::default(),
                    stdin: None,
                    environment: None,
                })
                .await
                .unwrap();

            assert_eq!(result.status, ExecutionStatus::Success);
            assert_eq!(result.stdout, format!("output {i}\n"));
        }
    }

    #[tokio::test]
    async fn test_sandbox_manager_with_response_fn() {
        let mut mock = MockExecutor::new();
        mock.set_response_fn(Box::new(|req| SandboxResult {
            request_id: req.request_id,
            session_id: Some(req.session_id),
            status: ExecutionStatus::Success,
            exit_code: Some(0),
            stdout: format!("processed: {}", req.code),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 1,
            resource_usage: ResourceUsage::default(),
            error: None,
        }));

        let manager = SandboxManager::with_executor(Box::new(mock));
        let result = manager
            .execute(SandboxRequest {
                request_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                tool_kind: ToolKind::PythonExec,
                code: "test_code".to_string(),
                language: Some("python".to_string()),
                limits: SandboxLimits::default(),
                stdin: None,
                environment: None,
            })
            .await
            .unwrap();

        assert_eq!(result.stdout, "processed: test_code");
    }
}
