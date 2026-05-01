use serde_json::Value;

/// Result of calling an MCP tool.
#[derive(Debug, Clone)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

/// MCP content block.
#[derive(Debug, Clone)]
pub enum McpContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { uri: String, text: Option<String> },
}

/// Information about an MCP tool.
#[derive(Debug, Clone)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// MCP client handle for a connected server.
///
/// This wraps the rmcp client and provides a simpler interface.
/// The actual rmcp client is stored in McpManager for lifecycle management.
pub struct McpClientHandle {
    pub server_name: String,
    pub tools: Vec<McpToolInfo>,
    pub connected: bool,
}

impl McpClientHandle {
    pub fn new(server_name: String) -> Self {
        Self {
            server_name,
            tools: Vec::new(),
            connected: false,
        }
    }
}
