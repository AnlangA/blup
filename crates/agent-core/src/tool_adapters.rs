use std::sync::Arc;

use assessment_engine::executor::{CodeExecutionResult, CodeExecutor};
use async_trait::async_trait;
use sandbox_manager::models::limits::SandboxLimits;
use sandbox_manager::{ExecutionStatus, SandboxManager, SandboxRequest, ToolKind};

#[derive(Clone)]
pub struct SandboxCodeExecutor {
    sandbox: Arc<SandboxManager>,
}

impl SandboxCodeExecutor {
    pub fn new(sandbox: Arc<SandboxManager>) -> Self {
        Self { sandbox }
    }
}

#[async_trait]
impl CodeExecutor for SandboxCodeExecutor {
    async fn execute(
        &self,
        code: &str,
        language: &str,
        stdin: &str,
    ) -> Result<CodeExecutionResult, String> {
        let tool_kind = ToolKind::from_language(language)
            .ok_or_else(|| format!("Unsupported assessment language: {language}"))?;

        let result = self
            .sandbox
            .execute(SandboxRequest {
                request_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                tool_kind,
                code: code.to_string(),
                language: Some(language.to_string()),
                limits: SandboxLimits::strict(),
                stdin: Some(stdin.to_string()),
                environment: None,
            })
            .await
            .map_err(|e| e.to_string())?;

        Ok(CodeExecutionResult {
            success: result.status == ExecutionStatus::Success && result.exit_code == Some(0),
            stdout: result.stdout,
            stderr: result.stderr,
            exit_code: result.exit_code,
            duration_ms: result.duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assessment_engine::executor::CodeExecutor;
    use sandbox_manager::models::result::SandboxResult;
    use sandbox_manager::{MockExecutor, SandboxManager};

    #[tokio::test]
    async fn maps_sandbox_result_to_code_execution_result() {
        let mut mock = MockExecutor::new();
        mock.set_response_fn(Box::new(|req| SandboxResult {
            request_id: req.request_id,
            session_id: Some(req.session_id),
            status: ExecutionStatus::Success,
            exit_code: Some(0),
            stdout: req.stdin.clone().unwrap_or_default(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 7,
            resource_usage: Default::default(),
            error: None,
        }));
        let sandbox = Arc::new(SandboxManager::with_executor(Box::new(mock)));
        let executor = SandboxCodeExecutor::new(sandbox);

        let result = executor
            .execute("print(input())", "python", "hello")
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.stdout, "hello");
        assert_eq!(result.duration_ms, 7);
    }
}
