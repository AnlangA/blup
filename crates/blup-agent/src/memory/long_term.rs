use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::fs;

/// Persisted session summary for long-term memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub learning_goal: String,
    pub domain: String,
    pub current_chapter: Option<String>,
    pub completed_chapters: Vec<String>,
    pub key_decisions: Vec<String>,
    pub learner_notes: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Long-term memory: persists session summaries and learner context.
pub struct LongTermMemory {
    storage_dir: PathBuf,
    cache: HashMap<String, SessionSummary>,
}

impl LongTermMemory {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            storage_dir,
            cache: HashMap::new(),
        }
    }

    /// Load a session summary from disk.
    pub async fn load(&mut self, session_id: &str) -> Option<SessionSummary> {
        if let Some(cached) = self.cache.get(session_id) {
            return Some(cached.clone());
        }

        let path = self.file_path(session_id);
        let content = fs::read_to_string(&path).await.ok()?;
        let summary: SessionSummary = serde_json::from_str(&content).ok()?;
        self.cache.insert(session_id.to_string(), summary.clone());
        Some(summary)
    }

    /// Save a session summary to disk.
    pub async fn save(&mut self, summary: &SessionSummary) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.storage_dir).await?;
        let path = self.file_path(&summary.session_id);
        let json = serde_json::to_string_pretty(summary).unwrap_or_default();
        fs::write(&path, json).await?;
        self.cache
            .insert(summary.session_id.clone(), summary.clone());
        Ok(())
    }

    /// Update an existing session summary.
    pub async fn update(
        &mut self,
        session_id: &str,
        updater: impl FnOnce(&mut SessionSummary),
    ) -> Result<(), std::io::Error> {
        let mut summary = self
            .load(session_id)
            .await
            .unwrap_or_else(|| SessionSummary {
                session_id: session_id.to_string(),
                learning_goal: String::new(),
                domain: String::new(),
                current_chapter: None,
                completed_chapters: Vec::new(),
                key_decisions: Vec::new(),
                learner_notes: Vec::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            });

        updater(&mut summary);
        summary.updated_at = chrono::Utc::now();
        self.save(&summary).await
    }

    /// Get all stored session IDs.
    pub async fn list_sessions(&self) -> Result<Vec<String>, std::io::Error> {
        let mut sessions = Vec::new();
        if !self.storage_dir.exists() {
            return Ok(sessions);
        }

        let mut entries = fs::read_dir(&self.storage_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".json") {
                sessions.push(name_str.trim_end_matches(".json").to_string());
            }
        }
        Ok(sessions)
    }

    fn file_path(&self, session_id: &str) -> PathBuf {
        let safe_id: String = session_id
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        self.storage_dir.join(format!("{safe_id}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_summary(session_id: &str) -> SessionSummary {
        SessionSummary {
            session_id: session_id.to_string(),
            learning_goal: "Learn Rust".to_string(),
            domain: "programming".to_string(),
            current_chapter: Some("ch1".to_string()),
            completed_chapters: vec![],
            key_decisions: vec!["Start with basics".to_string()],
            learner_notes: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let mut memory = LongTermMemory::new(tmp.path().to_path_buf());

        let summary = create_test_summary("test-1");
        memory.save(&summary).await.unwrap();

        let loaded = memory.load("test-1").await.unwrap();
        assert_eq!(loaded.session_id, "test-1");
        assert_eq!(loaded.learning_goal, "Learn Rust");
    }

    #[tokio::test]
    async fn test_load_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let mut memory = LongTermMemory::new(tmp.path().to_path_buf());

        let result = memory.load("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update() {
        let tmp = TempDir::new().unwrap();
        let mut memory = LongTermMemory::new(tmp.path().to_path_buf());

        let summary = create_test_summary("test-2");
        memory.save(&summary).await.unwrap();

        memory
            .update("test-2", |s| {
                s.current_chapter = Some("ch2".to_string());
                s.completed_chapters.push("ch1".to_string());
            })
            .await
            .unwrap();

        let loaded = memory.load("test-2").await.unwrap();
        assert_eq!(loaded.current_chapter, Some("ch2".to_string()));
        assert_eq!(loaded.completed_chapters, vec!["ch1".to_string()]);
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let tmp = TempDir::new().unwrap();
        let mut memory = LongTermMemory::new(tmp.path().to_path_buf());

        memory.save(&create_test_summary("sess-a")).await.unwrap();
        memory.save(&create_test_summary("sess-b")).await.unwrap();

        let sessions = memory.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&"sess-a".to_string()));
        assert!(sessions.contains(&"sess-b".to_string()));
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let tmp = TempDir::new().unwrap();
        let mut memory = LongTermMemory::new(tmp.path().to_path_buf());

        let summary = create_test_summary("cached");
        memory.save(&summary).await.unwrap();

        // First load caches the result
        let _ = memory.load("cached").await.unwrap();

        // Delete the file to prove cache is used
        let path = memory.file_path("cached");
        tokio::fs::remove_file(&path).await.unwrap();

        // Second load should still work from cache
        let loaded = memory.load("cached").await.unwrap();
        assert_eq!(loaded.session_id, "cached");
    }
}
