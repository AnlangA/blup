use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// MCP server configuration for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    pub name: String,
    #[serde(flatten)]
    pub transport: McpTransportConfig,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub oauth: Option<McpOAuthEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpTransportConfig {
    #[serde(rename = "local")]
    Local {
        command: Vec<String>,
        #[serde(default)]
        env: Option<HashMap<String, String>>,
    },
    #[serde(rename = "remote")]
    Remote {
        url: String,
        #[serde(default)]
        headers: Option<HashMap<String, String>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpOAuthEntry {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scope: Option<String>,
    pub redirect_uri: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    30_000
}
