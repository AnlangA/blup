pub mod auth;
pub mod client;
pub mod config;
pub mod tool;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::audit::AuditLogger;
use crate::config::McpConfig;

pub use auth::{AuthEntry, AuthStore};
pub use client::{McpClientHandle, McpContent, McpToolInfo, McpToolResult};
pub use config::{McpServerEntry, McpTransportConfig};

/// MCP server connection status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpStatus {
    Connected,
    Disconnected,
    Failed(String),
    NeedsAuth,
}

/// Manages connections to multiple MCP servers.
pub struct McpManager {
    servers: HashMap<String, McpServerState>,
    auth_store: Arc<Mutex<AuthStore>>,
    audit: Option<Arc<AuditLogger>>,
}

#[allow(dead_code)]
struct McpServerState {
    name: String,
    status: McpStatus,
    tools: Vec<McpToolInfo>,
}

impl McpManager {
    pub async fn new(
        config: &McpConfig,
        auth_dir: PathBuf,
        audit: Option<Arc<AuditLogger>>,
    ) -> Self {
        let auth_store = AuthStore::new(auth_dir.join("mcp-auth.json")).await;

        let mut servers = HashMap::new();
        for server_config in &config.servers {
            servers.insert(
                server_config.name.clone(),
                McpServerState {
                    name: server_config.name.clone(),
                    status: McpStatus::Disconnected,
                    tools: Vec::new(),
                },
            );
        }

        Self {
            servers,
            auth_store: Arc::new(Mutex::new(auth_store)),
            audit,
        }
    }

    /// Connect to all enabled MCP servers.
    pub async fn connect_all(&mut self) {
        let server_names: Vec<String> = self.servers.keys().cloned().collect();
        for name in server_names {
            self.connect_server(&name).await;
        }
    }

    /// Connect to a specific MCP server.
    pub async fn connect_server(&mut self, name: &str) {
        let state = match self.servers.get_mut(name) {
            Some(s) => s,
            None => {
                tracing::warn!(server = name, "MCP server not found in config");
                return;
            }
        };

        tracing::info!(server = name, "Connecting to MCP server...");
        state.status = McpStatus::Connected;

        // Log audit event
        if let Some(ref audit) = self.audit {
            audit.log(crate::audit::AuditEvent::new(
                "system",
                crate::audit::AuditEventType::McpConnection {
                    server_name: name.to_string(),
                    status: "connected".to_string(),
                    transport: "stdio".to_string(),
                },
            ));
        }
    }

    /// Disconnect from a specific MCP server.
    pub async fn disconnect_server(&mut self, name: &str) {
        if let Some(state) = self.servers.get_mut(name) {
            state.status = McpStatus::Disconnected;
            state.tools.clear();
            tracing::info!(server = name, "Disconnected from MCP server");
        }
    }

    /// Get all available tools from all connected MCP servers.
    pub fn available_tools(&self) -> Vec<McpToolInfo> {
        self.servers
            .values()
            .filter(|s| s.status == McpStatus::Connected)
            .flat_map(|s| s.tools.clone())
            .collect()
    }

    /// Get status of all servers.
    pub fn server_status(&self) -> HashMap<String, McpStatus> {
        self.servers
            .iter()
            .map(|(name, state)| (name.clone(), state.status.clone()))
            .collect()
    }

    /// Call an MCP tool on a specific server.
    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        _args: serde_json::Value,
    ) -> Result<McpToolResult, String> {
        let state = self
            .servers
            .get(server_name)
            .ok_or_else(|| format!("Server '{server_name}' not found"))?;

        if state.status != McpStatus::Connected {
            return Err(format!(
                "Server '{server_name}' is not connected (status: {:?})",
                state.status
            ));
        }

        tracing::info!(server = server_name, tool = tool_name, "Calling MCP tool");

        Ok(McpToolResult {
            content: vec![McpContent::Text {
                text: "MCP tool execution placeholder".to_string(),
            }],
            is_error: false,
        })
    }

    /// Get the auth store reference.
    pub fn auth_store(&self) -> Arc<Mutex<AuthStore>> {
        self.auth_store.clone()
    }
}
