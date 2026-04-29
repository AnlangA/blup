use std::path::PathBuf;
use std::sync::Arc;

pub mod api;
pub mod llm;
pub mod models;
pub mod prompts;
pub mod server;
pub mod state;
pub mod validation;

/// Application configuration loaded from environment variables.
#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub llm_gateway_url: String,
    pub llm_gateway_secret: String,
    pub llm_model: String,
    pub prompts_dir: PathBuf,
    pub schemas_dir: PathBuf,
    pub log_format: String,
    pub max_sessions: usize,
    pub sse_ping_interval_secs: u64,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("llm_gateway_url", &self.llm_gateway_url)
            .field("llm_gateway_secret", &"[REDACTED]")
            .field("llm_model", &self.llm_model)
            .field("prompts_dir", &self.prompts_dir)
            .field("schemas_dir", &self.schemas_dir)
            .field("log_format", &self.log_format)
            .field("max_sessions", &self.max_sessions)
            .field("sse_ping_interval_secs", &self.sse_ping_interval_secs)
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            llm_gateway_url: "http://127.0.0.1:9000".to_string(),
            llm_gateway_secret: String::new(),
            llm_model: "gpt-4o".to_string(),
            prompts_dir: PathBuf::from("prompts"),
            schemas_dir: PathBuf::from("schemas"),
            log_format: "pretty".to_string(),
            max_sessions: 1000,
            sse_ping_interval_secs: 15,
        }
    }
}

impl Config {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut config = Config::default();

        if let Ok(host) = std::env::var("BLUP_HOST") {
            config.host = host;
        }
        if let Ok(port) = std::env::var("BLUP_PORT") {
            match port.parse::<u16>() {
                Ok(p) => config.port = p,
                Err(_) => tracing::warn!(value = %port, "Invalid BLUP_PORT, using default"),
            }
        }
        if let Ok(url) = std::env::var("BLUP_LLM_GATEWAY_URL") {
            config.llm_gateway_url = url;
        }
        if let Ok(secret) = std::env::var("BLUP_LLM_GATEWAY_SECRET") {
            config.llm_gateway_secret = secret;
        }
        if let Ok(model) = std::env::var("BLUP_LLM_MODEL") {
            config.llm_model = model;
        }
        if let Ok(dir) = std::env::var("BLUP_PROMPTS_DIR") {
            config.prompts_dir = PathBuf::from(dir);
        }
        if let Ok(dir) = std::env::var("BLUP_SCHEMAS_DIR") {
            config.schemas_dir = PathBuf::from(dir);
        }
        if let Ok(fmt) = std::env::var("BLUP_LOG_FORMAT") {
            config.log_format = fmt;
        }

        config
    }
}

/// Shared application state passed to all request handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: state::session::InMemorySessionStore,
    pub llm: llm::client::LlmClient,
    pub prompts: Arc<prompts::loader::PromptLoader>,
    pub validator: Arc<validation::schema_validator::SchemaValidator>,
}
