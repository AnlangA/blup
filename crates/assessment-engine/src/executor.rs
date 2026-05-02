use async_trait::async_trait;

/// Result of executing code in a sandbox.
#[derive(Debug, Clone)]
pub struct CodeExecutionResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

/// Abstraction for executing code in a sandboxed environment.
/// When attached to an AssessmentEngine, coding exercises can be evaluated
/// by actually running the code instead of using static analysis.
#[async_trait]
pub trait CodeExecutor: Send + Sync {
    /// Execute code and return the result.
    async fn execute(
        &self,
        code: &str,
        language: &str,
        stdin: &str,
    ) -> Result<CodeExecutionResult, String>;
}
