use async_trait::async_trait;
use serde_json::Value;

use crate::tools::{AgentTool, ToolError, ToolResult};

/// Adapter that wraps an MCP tool as an AgentTool.
pub struct McpToolAdapter {
    pub tool_name: String,
    pub server_name: String,
    pub description: String,
    pub input_schema: Value,
    // In a real implementation, this would hold a reference to the MCP client
    // for calling the tool. For now, we store the tool info.
}

impl McpToolAdapter {
    pub fn new(server_name: &str, tool_name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            server_name: server_name.to_string(),
            description: description.to_string(),
            input_schema,
        }
    }

    /// Get the full qualified name (server_tool).
    pub fn qualified_name(&self) -> String {
        format!("{}_{}", self.server_name, self.tool_name)
    }
}

#[async_trait]
impl AgentTool for McpToolAdapter {
    fn name(&self) -> &str {
        // We need to return owned string, so we leak slightly
        // In practice, the ToolRegistry owns the name
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.input_schema.clone()
    }

    async fn execute(&self, _args: Value) -> Result<ToolResult, ToolError> {
        // This is a placeholder - in the real implementation,
        // this would call the MCP client's callTool method.
        // The actual execution is handled by McpManager.
        Err(ToolError::ExecutionFailed(
            "MCP tool execution must go through McpManager".to_string(),
        ))
    }
}
