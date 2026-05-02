use thiserror::Error;

#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Docker error: {0}")]
    Docker(String),

    #[error("Container error: {0}")]
    Container(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl SandboxError {
    pub fn docker(msg: &str) -> Self {
        SandboxError::Docker(msg.to_string())
    }

    pub fn container(msg: &str) -> Self {
        SandboxError::Container(msg.to_string())
    }

    pub fn timeout(msg: &str) -> Self {
        SandboxError::Timeout(msg.to_string())
    }

    pub fn resource_limit(msg: &str) -> Self {
        SandboxError::ResourceLimit(msg.to_string())
    }
}
