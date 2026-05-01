use std::path::PathBuf;

use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

use super::event::AuditEvent;

/// JSONL file-based audit log storage.
pub struct AuditStorage {
    storage_dir: PathBuf,
}

impl AuditStorage {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self { storage_dir }
    }

    /// Append an audit event to the session's log file.
    pub async fn append(&self, event: &AuditEvent) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.storage_dir).await?;

        let file_path = self.file_path(&event.session_id);
        let mut line = serde_json::to_string(event).unwrap_or_default();
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;

        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    /// Read all audit events for a session.
    pub async fn read_events(&self, session_id: &str) -> Result<Vec<AuditEvent>, std::io::Error> {
        let file_path = self.file_path(session_id);
        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&file_path).await?;
        let events = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<AuditEvent>(line).ok())
            .collect();
        Ok(events)
    }

    /// Read events with a filter predicate.
    pub async fn read_filtered<F>(
        &self,
        session_id: &str,
        filter: F,
    ) -> Result<Vec<AuditEvent>, std::io::Error>
    where
        F: Fn(&AuditEvent) -> bool,
    {
        let events = self.read_events(session_id).await?;
        Ok(events.into_iter().filter(|e| filter(e)).collect())
    }

    fn file_path(&self, session_id: &str) -> PathBuf {
        // Sanitize session_id to prevent path traversal
        let safe_id: String = session_id
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        self.storage_dir.join(format!("{safe_id}.jsonl"))
    }
}

/// Compute SHA-256 hash of a string for audit logging (e.g., tool args).
pub fn hash_content(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
