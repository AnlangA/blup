use thiserror::Error;

#[derive(Error, Debug)]
pub enum AssessmentError {
    #[error("Invalid answer format: {0}")]
    InvalidAnswer(String),

    #[error("Exercise validation error: {0}")]
    ValidationError(String),

    #[error("Evaluation error: {0}")]
    EvaluationError(String),

    #[error("Sandbox required for this exercise type")]
    SandboxRequired,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
