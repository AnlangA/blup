use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::domain::*;
use super::machine::{StateMachine, TransitionRecord};
use super::types::SessionState;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub state_machine: StateMachine,
    pub version: u64,
    pub goal: Option<LearningGoal>,
    pub feasibility_result: Option<FeasibilityResult>,
    pub profile: Option<UserProfile>,
    pub profile_rounds: u32,
    pub curriculum: Option<CurriculumPlan>,
    pub current_chapter_id: Option<String>,
    pub chapter_contents: HashMap<String, String>,
    pub messages: Vec<SessionMessage>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Session {
    pub fn state(&self) -> SessionState {
        self.state_machine.current_state()
    }
}

/// Serializable snapshot of a session for disk persistence.
#[derive(serde::Serialize, serde::Deserialize)]
struct SessionSnapshot {
    id: Uuid,
    current_state: SessionState,
    previous_state: Option<SessionState>,
    #[serde(default)]
    version: u64,
    #[serde(default)]
    transition_history: Vec<TransitionRecord>,
    goal: Option<LearningGoal>,
    feasibility_result: Option<FeasibilityResult>,
    profile: Option<UserProfile>,
    profile_rounds: u32,
    curriculum: Option<CurriculumPlan>,
    current_chapter_id: Option<String>,
    chapter_contents: HashMap<String, String>,
    messages: Vec<SessionMessage>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&Session> for SessionSnapshot {
    fn from(s: &Session) -> Self {
        Self {
            id: s.id,
            current_state: s.state(),
            previous_state: s.state_machine.previous_state(),
            version: s.version,
            transition_history: s.state_machine.history().to_vec(),
            goal: s.goal.clone(),
            feasibility_result: s.feasibility_result.clone(),
            profile: s.profile.clone(),
            profile_rounds: s.profile_rounds,
            curriculum: s.curriculum.clone(),
            current_chapter_id: s.current_chapter_id.clone(),
            chapter_contents: s.chapter_contents.clone(),
            messages: s.messages.clone(),
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

impl SessionSnapshot {
    fn into_session(self) -> Session {
        let mut sm = StateMachine::with_state(self.current_state);
        if let Some(prev) = self.previous_state {
            sm.set_previous_state(prev);
        }
        // Replay transition history if available (backward compat: old files lack this)
        for record in &self.transition_history {
            sm.replay_record(record);
        }
        Session {
            id: self.id,
            state_machine: sm,
            version: self.version,
            goal: self.goal,
            feasibility_result: self.feasibility_result,
            profile: self.profile,
            profile_rounds: self.profile_rounds,
            curriculum: self.curriculum,
            current_chapter_id: self.current_chapter_id,
            chapter_contents: self.chapter_contents,
            messages: self.messages,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Shared handle to a session behind a read-write lock.
pub type SessionHandle = Arc<RwLock<Session>>;

#[derive(Debug, Clone)]
pub struct InMemorySessionStore {
    sessions: Arc<RwLock<HashMap<Uuid, SessionHandle>>>,
    max_sessions: usize,
    data_dir: Option<PathBuf>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_sessions: 1000,
            data_dir: None,
        }
    }

    pub fn with_limit(limit: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_sessions: limit,
            data_dir: None,
        }
    }

    /// Enable file persistence. Sessions will be saved to and loaded from
    /// the given directory as individual JSON files.
    pub fn with_persistence(mut self, dir: PathBuf) -> Self {
        self.data_dir = Some(dir);
        self
    }

    /// Load all persisted sessions from disk. Called once at startup.
    pub async fn load_from_disk(&self) {
        let dir = match &self.data_dir {
            Some(d) => d.clone(),
            None => return,
        };

        let _ = tokio::fs::create_dir_all(&dir).await;
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut count = 0usize;
        let mut sessions = self.sessions.write().await;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if !path.extension().is_some_and(|e| e == "json") {
                continue;
            }

            let content = match tokio::fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(_) => continue,
            };

            let snapshot: SessionSnapshot = match serde_json::from_str(&content) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "Failed to deserialize session");
                    continue;
                }
            };

            let session = snapshot.into_session();
            sessions.insert(session.id, Arc::new(RwLock::new(session)));
            count += 1;
        }

        if count > 0 {
            tracing::info!(count, dir = %dir.display(), "Loaded persisted sessions");
        }
    }

    /// Save a single session to disk asynchronously.
    async fn persist_one(&self, id: Uuid) {
        let dir = match &self.data_dir {
            Some(d) => d.clone(),
            None => return,
        };

        let handle = match self.get(id).await {
            Some(h) => h,
            None => return,
        };

        let snapshot = SessionSnapshot::from(&*handle.read().await);
        let path = dir.join(format!("{id}.json"));

        match serde_json::to_string(&snapshot) {
            Ok(json) => {
                if let Err(e) = tokio::fs::write(&path, &json).await {
                    tracing::warn!(path = %path.display(), error = %e, "Failed to persist session, retrying");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    if let Err(e2) = tokio::fs::write(&path, &json).await {
                        tracing::error!(path = %path.display(), error = %e2, "Persist failed after retry");
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, session_id = %id, "Failed to serialize session for persistence");
            }
        }
    }

    /// Persist a session to disk (non-blocking fire-and-forget).
    pub fn persist(&self, id: Uuid) {
        let store = self.clone();
        tokio::spawn(async move {
            store.persist_one(id).await;
        });
    }

    /// Create a new session and return a handle to it.
    pub async fn create(&self) -> Option<SessionHandle> {
        let mut sessions = self.sessions.write().await;
        if sessions.len() >= self.max_sessions {
            return None;
        }
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let session = Session {
            id,
            state_machine: StateMachine::new(),
            version: 0,
            goal: None,
            feasibility_result: None,
            profile: None,
            profile_rounds: 0,
            curriculum: None,
            current_chapter_id: None,
            chapter_contents: HashMap::new(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        let handle = Arc::new(RwLock::new(session));
        sessions.insert(id, Arc::clone(&handle));
        drop(sessions);

        // Persist to disk if enabled
        if self.data_dir.is_some() {
            self.persist(id);
        }

        Some(handle)
    }

    /// Get a handle to an existing session.
    pub async fn get(&self, id: Uuid) -> Option<SessionHandle> {
        self.sessions.read().await.get(&id).cloned()
    }

    /// Get the current version of a session without holding the lock.
    pub async fn version(&self, id: Uuid) -> Option<u64> {
        let handle = self.get(id).await?;
        let v = handle.read().await.version;
        Some(v)
    }

    /// Attempt a version-checked mutation. Returns the result of `f`
    /// if the version matches, or `None` if a conflict is detected.
    pub async fn try_mutate<F, R>(&self, id: Uuid, expected_version: u64, f: F) -> Option<R>
    where
        F: FnOnce(&mut Session) -> R,
    {
        let handle = self.get(id).await?;
        let mut s = handle.write().await;
        if s.version != expected_version {
            tracing::warn!(
                session_id = %id,
                expected = expected_version,
                actual = s.version,
                "Version conflict detected"
            );
            return None;
        }
        let result = f(&mut s);
        s.version += 1;
        s.updated_at = chrono::Utc::now();
        self.persist(s.id);
        Some(result)
    }

    /// Remove a session from the store and disk.
    pub async fn delete(&self, id: Uuid) {
        self.sessions.write().await.remove(&id);
        if let Some(dir) = &self.data_dir {
            let path = dir.join(format!("{id}.json"));
            let _ = tokio::fs::remove_file(&path).await;
        }
    }

    pub async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// List lightweight metadata for all sessions.
    pub async fn list(&self) -> Vec<SessionListEntry> {
        let sessions = self.sessions.read().await;
        let mut entries: Vec<SessionListEntry> = sessions
            .iter()
            .map(|(id, handle)| {
                // Best-effort: try read lock, skip if contended
                let s = handle.try_read();
                match s {
                    Ok(session) => SessionListEntry {
                        id: session.id.to_string(),
                        state: session.state().to_string(),
                        goal_description: session
                            .goal
                            .as_ref()
                            .map(|g| g.description.as_str())
                            .unwrap_or("Untitled")
                            .to_string(),
                        domain: session
                            .goal
                            .as_ref()
                            .map(|g| g.domain.as_str())
                            .unwrap_or("")
                            .to_string(),
                        updated_at: session.updated_at.to_rfc3339(),
                    },
                    Err(_) => SessionListEntry {
                        id: id.to_string(),
                        state: "UNKNOWN".to_string(),
                        goal_description: "Untitled".to_string(),
                        domain: String::new(),
                        updated_at: String::new(),
                    },
                }
            })
            .collect();
        entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        entries
    }

    /// Start a background task that evicts sessions older than `ttl`.
    pub fn start_eviction_task(&self, ttl: Duration, interval: Duration) {
        let store = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let cutoff = chrono::Utc::now()
                    - chrono::Duration::from_std(ttl).expect("Invalid TTL duration");
                let mut to_remove = Vec::new();

                {
                    let sessions = store.sessions.read().await;
                    for (id, handle) in sessions.iter() {
                        if let Ok(s) = handle.try_read() {
                            if s.updated_at < cutoff {
                                to_remove.push(*id);
                            }
                        }
                    }
                }

                for id in &to_remove {
                    store.delete(*id).await;
                }

                if !to_remove.is_empty() {
                    tracing::info!(count = to_remove.len(), "Evicted stale sessions");
                }
            }
        });
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Lightweight session metadata for listing.
#[derive(Debug, serde::Serialize)]
pub struct SessionListEntry {
    pub id: String,
    pub state: String,
    pub goal_description: String,
    pub domain: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::types::SessionState;
    use std::time::Duration;

    #[tokio::test]
    async fn test_create_and_get_session() {
        let store = InMemorySessionStore::new();
        let handle = store.create().await.unwrap();
        let id = handle.read().await.id;

        let retrieved = store.get(id).await.unwrap();
        assert_eq!(retrieved.read().await.id, id);
        assert_eq!(retrieved.read().await.state(), SessionState::Idle);
    }

    #[tokio::test]
    async fn test_create_at_capacity_limit() {
        let store = InMemorySessionStore::with_limit(3);
        assert!(store.create().await.is_some());
        assert!(store.create().await.is_some());
        assert!(store.create().await.is_some());
        // Next create should fail
        assert!(store.create().await.is_none());
        assert_eq!(store.count().await, 3);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let store = InMemorySessionStore::new();
        let handle = store.create().await.unwrap();
        let id = handle.read().await.id;

        store.delete(id).await;
        assert!(store.get(id).await.is_none());
    }

    #[tokio::test]
    async fn test_version_tracking() {
        let store = InMemorySessionStore::new();
        let handle = store.create().await.unwrap();
        let id = handle.read().await.id;
        assert_eq!(store.version(id).await, Some(0));

        // try_mutate bumps version internally
        store.try_mutate(id, 0, |s| s.state().to_string()).await;
        assert_eq!(store.version(id).await, Some(1));
    }

    #[tokio::test]
    async fn test_version_conflict_detection() {
        let store = InMemorySessionStore::new();
        let handle = store.create().await.unwrap();
        let id = handle.read().await.id;

        // First mutation succeeds
        let result = store.try_mutate(id, 0, |_s| "ok").await;
        assert_eq!(result, Some("ok"));

        // Second mutation with stale version 0 fails (version is now 1)
        let result = store.try_mutate(id, 0, |_s| "should_not_happen").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_session_creates() {
        let store = InMemorySessionStore::with_limit(100);
        let mut handles = vec![];

        for _ in 0..20 {
            let s = store.clone();
            handles.push(tokio::spawn(async move { s.create().await }));
        }

        let mut ids = Vec::new();
        for h in handles {
            if let Some(handle) = h.await.unwrap() {
                ids.push(handle.read().await.id);
            }
        }

        assert_eq!(ids.len(), 20);
        // Verify all IDs are unique
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 20);
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let store = InMemorySessionStore::new();
        let handle1 = store.create().await.unwrap();
        let handle2 = store.create().await.unwrap();
        let id1 = handle1.read().await.id;
        let id2 = handle2.read().await.id;

        let list = store.list().await;
        assert!(list.len() >= 2);

        let ids: Vec<&str> = list.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&id1.to_string().as_str()));
        assert!(ids.contains(&id2.to_string().as_str()));
    }

    #[tokio::test]
    async fn test_disk_persistence_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_path_buf();

        // Create and populate a session
        let store = InMemorySessionStore::with_limit(10).with_persistence(dir_path.clone());
        let handle = store.create().await.unwrap();
        let id = {
            let mut s = handle.write().await;
            s.goal = Some(LearningGoal {
                description: "Learn Rust".to_string(),
                domain: "programming".to_string(),
                context: None,
                current_level: None,
            });
            s.version = 5;
            s.id
        };
        store.persist(id);

        // Give persistence a moment
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Create a new store and load from disk
        let store2 = InMemorySessionStore::with_limit(10).with_persistence(dir_path);
        store2.load_from_disk().await;

        let loaded = store2.get(id).await.unwrap();
        let s = loaded.read().await;
        assert_eq!(s.id, id);
        assert!(s.goal.is_some());
        assert_eq!(s.goal.as_ref().unwrap().description, "Learn Rust");
    }

    #[tokio::test]
    async fn test_eviction_stale_sessions() {
        let store = InMemorySessionStore::new();
        let handle = store.create().await.unwrap();
        let id = handle.read().await.id;

        // Artificially age the session
        {
            let mut s = handle.write().await;
            s.updated_at = chrono::Utc::now() - chrono::Duration::hours(48);
        }

        assert_eq!(store.count().await, 1);

        // Start eviction with 1-hour TTL, 50ms interval
        store.start_eviction_task(Duration::from_secs(3600), Duration::from_millis(50));

        // Wait for eviction to fire
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Session should have been evicted
        assert_eq!(store.count().await, 0);
        assert!(store.get(id).await.is_none());
    }

    #[tokio::test]
    async fn test_corrupt_snapshot_file_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_path_buf();

        // Write a valid session
        let store = InMemorySessionStore::with_limit(10).with_persistence(dir_path.clone());
        let handle = store.create().await.unwrap();
        let id = handle.read().await.id;
        store.persist(id);
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Write a corrupt file alongside the valid one
        let corrupt_path = dir_path.join("corrupt-session.json");
        tokio::fs::write(&corrupt_path, "not valid json {{{")
            .await
            .unwrap();

        // Load — should skip corrupt file and load valid one
        let store2 = InMemorySessionStore::with_limit(10).with_persistence(dir_path);
        store2.load_from_disk().await;
        assert!(store2.get(id).await.is_some());
    }

    #[tokio::test]
    async fn test_persist_nonexistent_session_no_panic() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_path_buf();
        let store = InMemorySessionStore::with_limit(10).with_persistence(dir_path);

        // Persisting a non-existent session should not panic
        store.persist(uuid::Uuid::new_v4());
        // Give async persist a moment
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_load_from_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_path_buf();
        let store = InMemorySessionStore::with_limit(10).with_persistence(dir_path);
        store.load_from_disk().await;
        assert_eq!(store.count().await, 0);
    }

    #[tokio::test]
    async fn test_load_skips_non_json_files() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_path_buf();
        // Write a .txt file (should be ignored)
        tokio::fs::write(dir_path.join("notes.txt"), "not json")
            .await
            .unwrap();

        let store = InMemorySessionStore::with_limit(10).with_persistence(dir_path);
        store.load_from_disk().await;
        assert_eq!(store.count().await, 0);
    }
}
