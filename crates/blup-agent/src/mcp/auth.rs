use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::fs;

/// OAuth token information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<f64>,
    pub scope: Option<String>,
}

/// OAuth client information (from dynamic registration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClientInfo {
    pub client_id: String,
    pub client_secret: Option<String>,
}

/// A stored auth entry for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthEntry {
    pub tokens: Option<OAuthTokens>,
    pub client_info: Option<OAuthClientInfo>,
    pub code_verifier: Option<String>,
    pub oauth_state: Option<String>,
    pub server_url: Option<String>,
}

/// Persistent OAuth token storage.
pub struct AuthStore {
    file_path: PathBuf,
    entries: HashMap<String, AuthEntry>,
}

impl AuthStore {
    pub async fn new(file_path: PathBuf) -> Self {
        let entries = Self::load_from_disk(&file_path).await.unwrap_or_default();
        Self { file_path, entries }
    }

    /// Get auth entry for an MCP server.
    pub fn get(&self, name: &str) -> Option<&AuthEntry> {
        self.entries.get(name)
    }

    /// Set auth entry for an MCP server.
    pub async fn set(&mut self, name: &str, entry: AuthEntry) -> Result<(), std::io::Error> {
        self.entries.insert(name.to_string(), entry);
        self.save_to_disk().await
    }

    /// Remove auth entry.
    pub async fn remove(&mut self, name: &str) -> Result<(), std::io::Error> {
        self.entries.remove(name);
        self.save_to_disk().await
    }

    /// Update tokens for an MCP server.
    pub async fn update_tokens(
        &mut self,
        name: &str,
        tokens: OAuthTokens,
    ) -> Result<(), std::io::Error> {
        let entry = self
            .entries
            .entry(name.to_string())
            .or_insert_with(|| AuthEntry {
                tokens: None,
                client_info: None,
                code_verifier: None,
                oauth_state: None,
                server_url: None,
            });
        entry.tokens = Some(tokens);
        self.save_to_disk().await
    }

    /// Update OAuth state for PKCE flow.
    pub async fn update_oauth_state(
        &mut self,
        name: &str,
        state: String,
    ) -> Result<(), std::io::Error> {
        let entry = self
            .entries
            .entry(name.to_string())
            .or_insert_with(|| AuthEntry {
                tokens: None,
                client_info: None,
                code_verifier: None,
                oauth_state: None,
                server_url: None,
            });
        entry.oauth_state = Some(state);
        self.save_to_disk().await
    }

    /// Get OAuth state.
    pub fn get_oauth_state(&self, name: &str) -> Option<&str> {
        self.entries
            .get(name)
            .and_then(|e| e.oauth_state.as_deref())
    }

    /// Update code verifier for PKCE.
    pub async fn update_code_verifier(
        &mut self,
        name: &str,
        verifier: String,
    ) -> Result<(), std::io::Error> {
        let entry = self
            .entries
            .entry(name.to_string())
            .or_insert_with(|| AuthEntry {
                tokens: None,
                client_info: None,
                code_verifier: None,
                oauth_state: None,
                server_url: None,
            });
        entry.code_verifier = Some(verifier);
        self.save_to_disk().await
    }

    /// Get code verifier.
    pub fn get_code_verifier(&self, name: &str) -> Option<&str> {
        self.entries
            .get(name)
            .and_then(|e| e.code_verifier.as_deref())
    }

    /// Check if tokens are expired.
    pub fn is_token_expired(&self, name: &str) -> Option<bool> {
        let entry = self.entries.get(name)?;
        let tokens = entry.tokens.as_ref()?;
        match tokens.expires_at {
            Some(expires_at) => Some(expires_at < chrono::Utc::now().timestamp() as f64),
            None => Some(false),
        }
    }

    async fn load_from_disk(path: &PathBuf) -> Result<HashMap<String, AuthEntry>, std::io::Error> {
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let content = fs::read_to_string(path).await?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    async fn save_to_disk(&self) -> Result<(), std::io::Error> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_string_pretty(&self.entries).unwrap_or_default();
        fs::write(&self.file_path, json).await
    }
}
