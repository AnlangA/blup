use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("LLM provider error: {0}")]
    Llm(#[from] crate::provider::LlmError),

    #[error("Prompt error: {0}")]
    Prompt(#[from] crate::prompt::PromptError),

    #[error("Schema validation failed: {0}")]
    Validation(#[from] crate::schema::ValidationError),

    #[error("LLM response was not valid JSON: {0}")]
    JsonParse(String),

    #[error("Agent step failed: {0}")]
    StepFailed(String),

    #[error("Tool error: {0}")]
    Tool(#[from] crate::tools::ToolError),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
