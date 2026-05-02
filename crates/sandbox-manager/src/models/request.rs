use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::limits::SandboxLimits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxRequest {
    pub request_id: Uuid,
    pub session_id: Uuid,
    pub tool_kind: ToolKind,
    pub code: String,
    pub language: Option<String>,
    pub limits: SandboxLimits,
    pub stdin: Option<String>,
    pub environment: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    PythonExec,
    NodeExec,
    RustCompileRun,
    MathEval,
    TypstCompile,
}

impl ToolKind {
    pub fn to_image(&self) -> &str {
        match self {
            ToolKind::PythonExec => "sandbox-python:latest",
            ToolKind::NodeExec => "sandbox-node:latest",
            ToolKind::RustCompileRun => "sandbox-rust:latest",
            ToolKind::MathEval => "sandbox-math:latest",
            ToolKind::TypstCompile => "sandbox-typst:latest",
        }
    }

    pub fn to_language(&self) -> &str {
        match self {
            ToolKind::PythonExec => "python",
            ToolKind::NodeExec => "javascript",
            ToolKind::RustCompileRun => "rust",
            ToolKind::MathEval => "math",
            ToolKind::TypstCompile => "typst",
        }
    }
}

impl SandboxRequest {
    pub fn new_python(session_id: Uuid, code: &str) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            session_id,
            tool_kind: ToolKind::PythonExec,
            code: code.to_string(),
            language: Some("python".to_string()),
            limits: SandboxLimits::default(),
            stdin: None,
            environment: None,
        }
    }

    pub fn new_node(session_id: Uuid, code: &str) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            session_id,
            tool_kind: ToolKind::NodeExec,
            code: code.to_string(),
            language: Some("javascript".to_string()),
            limits: SandboxLimits::default(),
            stdin: None,
            environment: None,
        }
    }

    pub fn with_limits(mut self, limits: SandboxLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_stdin(mut self, stdin: &str) -> Self {
        self.stdin = Some(stdin.to_string());
        self
    }

    pub fn with_environment(mut self, env: std::collections::HashMap<String, String>) -> Self {
        self.environment = Some(env);
        self
    }
}
