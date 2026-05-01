pub mod builtin;
pub mod web_search;

use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Errors from tool execution.
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

/// Result of tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
    pub metadata: Option<Value>,
}

impl ToolResult {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
            metadata: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: message.into(),
            is_error: true,
            metadata: None,
        }
    }
}

/// Agent tool trait.
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// Tool name (unique identifier).
    fn name(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// JSON Schema for the tool's parameters.
    fn parameters_schema(&self) -> Value;

    /// Execute the tool with given arguments.
    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError>;
}

/// Concurrent registry of available tools using DashMap.
///
/// DashMap provides lock-free concurrent reads and fine-grained write locking,
/// making it ideal for a tool registry that is read-heavy (tool execution)
/// with occasional writes (tool registration).
pub struct ToolRegistry {
    tools: DashMap<String, Arc<dyn AgentTool>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
        }
    }

    /// Register a tool. This acquires a write lock on a single shard.
    pub fn register(&self, tool: Arc<dyn AgentTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name. This is lock-free for concurrent reads.
    pub fn get(&self, name: &str) -> Option<Arc<dyn AgentTool>> {
        self.tools.get(name).map(|r| Arc::clone(r.value()))
    }

    /// List all tool names.
    pub fn list_names(&self) -> Vec<String> {
        self.tools.iter().map(|r| r.key().clone()).collect()
    }

    /// List all tools with their info.
    pub fn list_tools(&self) -> Vec<ToolInfo> {
        self.tools
            .iter()
            .map(|r| {
                let t = r.value();
                ToolInfo {
                    name: t.name().to_string(),
                    description: t.description().to_string(),
                    parameters: t.parameters_schema(),
                }
            })
            .collect()
    }

    /// Execute a tool by name.
    pub async fn execute(&self, name: &str, args: Value) -> Result<ToolResult, ToolError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        tool.execute(args).await
    }

    /// Get the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// Serializable tool information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}
