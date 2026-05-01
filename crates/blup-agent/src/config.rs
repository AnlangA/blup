use std::path::PathBuf;

/// Top-level agent configuration.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub provider: ProviderConfig,
    pub mcp: McpConfig,
    pub memory: MemoryConfig,
    pub audit: AuditConfig,
    pub search: SearchConfig,
    pub prompts_dir: PathBuf,
    pub schemas_dir: PathBuf,
}

/// LLM provider configuration.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub max_retries: u32,
}

/// Supported LLM provider types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Ollama,
    Mock,
}

/// MCP server configuration.
#[derive(Debug, Clone, Default)]
pub struct McpConfig {
    pub servers: Vec<McpServerConfig>,
}

/// Individual MCP server config.
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub server_type: McpServerType,
    pub enabled: bool,
    pub timeout_ms: u64,
    pub oauth: Option<McpOAuthConfig>,
}

/// MCP server transport type.
#[derive(Debug, Clone)]
pub enum McpServerType {
    /// Local stdio process (e.g., ["npx", "-y", "mcp-server-brave-search"])
    Local {
        command: Vec<String>,
        env: Option<std::collections::HashMap<String, String>>,
    },
    /// Remote HTTP endpoint
    Remote {
        url: String,
        headers: Option<std::collections::HashMap<String, String>>,
    },
}

/// OAuth config for remote MCP servers.
#[derive(Debug, Clone)]
pub struct McpOAuthConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scope: Option<String>,
    pub redirect_uri: Option<String>,
}

/// Memory management configuration.
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum tokens to keep in short-term memory window.
    pub max_context_tokens: usize,
    /// Token threshold to trigger compaction.
    pub compaction_threshold: usize,
    /// Directory for persisting long-term memory.
    pub storage_dir: PathBuf,
    /// Whether to enable long-term memory persistence.
    pub enable_long_term: bool,
}

/// Audit logging configuration.
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Whether audit logging is enabled.
    pub enabled: bool,
    /// Directory for audit log files.
    pub storage_dir: PathBuf,
}

/// Web search configuration.
#[derive(Debug, Clone, Default)]
pub struct SearchConfig {
    /// Search provider type.
    pub provider: SearchProvider,
    /// API key for the search provider.
    pub api_key: Option<String>,
    /// Custom search API base URL (for SearXNG etc.).
    pub base_url: Option<String>,
}

/// Supported search providers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SearchProvider {
    #[default]
    None,
    Brave,
    Exa,
    SearXNG,
}

// --- Default implementations ---

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider: ProviderConfig::default(),
            mcp: McpConfig::default(),
            memory: MemoryConfig::default(),
            audit: AuditConfig::default(),
            search: SearchConfig::default(),
            prompts_dir: PathBuf::from("prompts"),
            schemas_dir: PathBuf::from("schemas"),
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_type: ProviderType::OpenAI,
            api_key: None,
            base_url: None,
            model: "gpt-4o".to_string(),
            temperature: 0.3,
            max_tokens: 4096,
            max_retries: 3,
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 128_000,
            compaction_threshold: 100_000,
            storage_dir: PathBuf::from("data/memory"),
            enable_long_term: true,
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            storage_dir: PathBuf::from("data/audit"),
        }
    }
}

impl AgentConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let provider_type = match std::env::var("BLUP_LLM_PROVIDER")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "anthropic" => ProviderType::Anthropic,
            "ollama" => ProviderType::Ollama,
            "mock" => ProviderType::Mock,
            _ => ProviderType::OpenAI,
        };

        let provider = ProviderConfig {
            provider_type,
            api_key: std::env::var("BLUP_LLM_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .ok(),
            base_url: std::env::var("BLUP_LLM_BASE_URL").ok(),
            model: std::env::var("BLUP_LLM_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
            temperature: std::env::var("BLUP_LLM_TEMPERATURE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.3),
            max_tokens: std::env::var("BLUP_LLM_MAX_TOKENS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4096),
            max_retries: 3,
        };

        let search = SearchConfig {
            provider: match std::env::var("BLUP_SEARCH_PROVIDER")
                .unwrap_or_default()
                .to_lowercase()
                .as_str()
            {
                "brave" => SearchProvider::Brave,
                "exa" => SearchProvider::Exa,
                "searxng" => SearchProvider::SearXNG,
                _ => SearchProvider::None,
            },
            api_key: std::env::var("BLUP_SEARCH_API_KEY").ok(),
            base_url: std::env::var("BLUP_SEARCH_BASE_URL").ok(),
        };

        let prompts_dir = std::env::var("BLUP_PROMPTS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("prompts"));

        let schemas_dir = std::env::var("BLUP_SCHEMAS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("schemas"));

        let data_dir = std::env::var("BLUP_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data"));

        Self {
            provider,
            mcp: McpConfig::default(),
            memory: MemoryConfig {
                storage_dir: data_dir.join("memory"),
                ..Default::default()
            },
            audit: AuditConfig {
                storage_dir: data_dir.join("audit"),
                ..Default::default()
            },
            search,
            prompts_dir,
            schemas_dir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AgentConfig::default();
        assert_eq!(config.provider.provider_type, ProviderType::OpenAI);
        assert_eq!(config.provider.model, "gpt-4o");
        assert_eq!(config.provider.temperature, 0.3);
        assert_eq!(config.provider.max_tokens, 4096);
    }

    #[test]
    fn test_provider_config_default() {
        let config = ProviderConfig::default();
        assert_eq!(config.provider_type, ProviderType::OpenAI);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_memory_config_default() {
        let config = MemoryConfig::default();
        assert_eq!(config.max_context_tokens, 128_000);
        assert_eq!(config.compaction_threshold, 100_000);
        assert!(config.enable_long_term);
    }

    #[test]
    fn test_audit_config_default() {
        let config = AuditConfig::default();
        assert!(config.enabled);
        assert_eq!(config.storage_dir, PathBuf::from("data/audit"));
    }

    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert_eq!(config.provider, SearchProvider::None);
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_provider_type_equality() {
        assert_eq!(ProviderType::OpenAI, ProviderType::OpenAI);
        assert_ne!(ProviderType::OpenAI, ProviderType::Anthropic);
    }

    #[test]
    fn test_search_provider_equality() {
        assert_eq!(SearchProvider::None, SearchProvider::None);
        assert_ne!(SearchProvider::Brave, SearchProvider::Exa);
    }

    #[test]
    fn test_mcp_config_default() {
        let config = McpConfig::default();
        assert!(config.servers.is_empty());
    }
}
