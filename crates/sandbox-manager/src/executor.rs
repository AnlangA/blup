use async_trait::async_trait;

use crate::config::SandboxConfig;
use crate::docker::client::DockerClient;
use crate::docker::container::ContainerExecutor;
use crate::error::SandboxError;
use crate::models::image::ImageInfo;
use crate::models::request::SandboxRequest;
use crate::models::result::{ResourceUsage, SandboxResult};
use crate::models::status::ExecutionStatus;

/// Abstraction over sandbox execution backends (Docker, mock, etc.).
#[async_trait]
pub trait SandboxExecutor: Send + Sync {
    async fn execute(&self, request: SandboxRequest) -> Result<SandboxResult, SandboxError>;
    async fn health_check(&self) -> Result<(), SandboxError>;
    fn image_info(&self) -> Vec<ImageInfo>;
}

// ── Docker Executor ──

pub struct DockerExecutor {
    config: SandboxConfig,
    client: DockerClient,
}

impl DockerExecutor {
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            client: DockerClient::new(),
        }
    }
}

#[async_trait]
impl SandboxExecutor for DockerExecutor {
    async fn execute(&self, request: SandboxRequest) -> Result<SandboxResult, SandboxError> {
        self.client.health_check()?;
        let executor = ContainerExecutor::new(self.config.clone());
        executor.execute(request).await
    }

    async fn health_check(&self) -> Result<(), SandboxError> {
        self.client.health_check()
    }

    fn image_info(&self) -> Vec<ImageInfo> {
        vec![ImageInfo::new(&self.config.default_image, "latest")]
    }
}

// ── Mock Executor (for testing without Docker) ──

pub type MockResponseFn = Box<dyn Fn(&SandboxRequest) -> SandboxResult + Send + Sync>;

pub struct MockExecutor {
    responses: Vec<SandboxResult>,
    counter: std::sync::atomic::AtomicUsize,
    response_fn: Option<MockResponseFn>,
    healthy: bool,
}

fn default_result() -> SandboxResult {
    SandboxResult {
        request_id: uuid::Uuid::nil(),
        session_id: None,
        status: ExecutionStatus::Success,
        exit_code: Some(0),
        stdout: "mock output\n".to_string(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        duration_ms: 0,
        resource_usage: ResourceUsage::default(),
        error: None,
    }
}

impl MockExecutor {
    pub fn new() -> Self {
        Self {
            responses: Vec::new(),
            counter: std::sync::atomic::AtomicUsize::new(0),
            response_fn: None,
            healthy: true,
        }
    }

    pub fn push_response(&mut self, result: SandboxResult) {
        self.responses.push(result);
    }

    pub fn set_response_fn(&mut self, f: MockResponseFn) {
        self.response_fn = Some(f);
    }

    pub fn set_healthy(&mut self, healthy: bool) {
        self.healthy = healthy;
    }

    pub fn success_default() -> Self {
        let mut mock = Self::new();
        mock.push_response(SandboxResult {
            request_id: uuid::Uuid::nil(),
            session_id: None,
            status: ExecutionStatus::Success,
            exit_code: Some(0),
            stdout: "Hello, World!\n".to_string(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 100,
            resource_usage: ResourceUsage::default(),
            error: None,
        });
        mock
    }
}

impl Default for MockExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SandboxExecutor for MockExecutor {
    async fn execute(&self, request: SandboxRequest) -> Result<SandboxResult, SandboxError> {
        if let Some(ref f) = self.response_fn {
            return Ok(f(&request));
        }

        let idx = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if idx < self.responses.len() {
            let mut result = self.responses[idx].clone();
            result.request_id = request.request_id;
            return Ok(result);
        }

        let mut result = default_result();
        result.request_id = request.request_id;
        Ok(result)
    }

    async fn health_check(&self) -> Result<(), SandboxError> {
        if self.healthy {
            Ok(())
        } else {
            Err(SandboxError::Docker("mock: unhealthy".to_string()))
        }
    }

    fn image_info(&self) -> Vec<ImageInfo> {
        vec![
            ImageInfo::new("sandbox-python", "mock"),
            ImageInfo::new("sandbox-node", "mock"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::limits::SandboxLimits;
    use crate::models::request::ToolKind;
    use crate::models::result::ErrorDetails;

    #[tokio::test]
    async fn test_mock_executor_success() {
        let mut mock = MockExecutor::new();
        mock.push_response(SandboxResult {
            request_id: uuid::Uuid::nil(),
            session_id: None,
            status: ExecutionStatus::Success,
            exit_code: Some(0),
            stdout: "test output\n".to_string(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 42,
            resource_usage: ResourceUsage::default(),
            error: None,
        });

        let result = mock
            .execute(SandboxRequest {
                request_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                tool_kind: ToolKind::PythonExec,
                code: "print('hello')".to_string(),
                language: Some("python".to_string()),
                limits: SandboxLimits::default(),
                stdin: None,
                environment: None,
            })
            .await
            .unwrap();

        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout, "test output\n");
        assert_eq!(result.duration_ms, 42);
    }

    #[tokio::test]
    async fn test_mock_executor_timeout() {
        let mut mock = MockExecutor::new();
        mock.push_response(SandboxResult {
            request_id: uuid::Uuid::nil(),
            session_id: None,
            status: ExecutionStatus::TimeoutRun,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 10000,
            resource_usage: ResourceUsage::default(),
            error: Some(ErrorDetails {
                code: "TIMEOUT".to_string(),
                message: "Execution timed out after 10 seconds".to_string(),
            }),
        });

        let result = mock
            .execute(SandboxRequest {
                request_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                tool_kind: ToolKind::PythonExec,
                code: "while True: pass".to_string(),
                language: Some("python".to_string()),
                limits: SandboxLimits::default(),
                stdin: None,
                environment: None,
            })
            .await
            .unwrap();

        assert!(result.status.is_timeout());
    }

    #[tokio::test]
    async fn test_mock_executor_health() {
        let mock = MockExecutor::new();
        assert!(mock.health_check().await.is_ok());

        let mut unhealthy = MockExecutor::new();
        unhealthy.set_healthy(false);
        assert!(unhealthy.health_check().await.is_err());
    }

    #[tokio::test]
    async fn test_mock_executor_response_fn() {
        let mut mock = MockExecutor::new();
        mock.set_response_fn(Box::new(|req| SandboxResult {
            request_id: req.request_id,
            session_id: None,
            status: ExecutionStatus::Success,
            exit_code: Some(0),
            stdout: format!("echo: {}", req.code),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 1,
            resource_usage: ResourceUsage::default(),
            error: None,
        }));

        let result = mock
            .execute(SandboxRequest {
                request_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                tool_kind: ToolKind::PythonExec,
                code: "print('x')".to_string(),
                language: Some("python".to_string()),
                limits: SandboxLimits::default(),
                stdin: None,
                environment: None,
            })
            .await
            .unwrap();

        assert_eq!(result.stdout, "echo: print('x')");
    }

    #[tokio::test]
    async fn test_mock_executor_default_response() {
        let mock = MockExecutor::default();
        let result = mock
            .execute(SandboxRequest {
                request_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                tool_kind: ToolKind::PythonExec,
                code: String::new(),
                language: None,
                limits: SandboxLimits::default(),
                stdin: None,
                environment: None,
            })
            .await
            .unwrap();

        assert_eq!(result.status, ExecutionStatus::Success);
    }
}
