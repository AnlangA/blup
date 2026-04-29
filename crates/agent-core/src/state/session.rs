use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::machine::StateMachine;
use super::types::SessionState;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub state_machine: StateMachine,
    pub goal: Option<serde_json::Value>,
    pub feasibility_result: Option<serde_json::Value>,
    pub profile: Option<serde_json::Value>,
    pub curriculum: Option<serde_json::Value>,
    pub current_chapter_id: Option<String>,
    pub messages: Vec<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Session {
    pub fn state(&self) -> SessionState {
        self.state_machine.current_state()
    }
}

#[derive(Debug, Clone)]
pub struct InMemorySessionStore {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create(&self) -> Session {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let session = Session {
            id,
            state_machine: StateMachine::new(),
            goal: None,
            feasibility_result: None,
            profile: None,
            curriculum: None,
            current_chapter_id: None,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        self.sessions.write().await.insert(id, session.clone());
        session
    }

    pub async fn get(&self, id: Uuid) -> Option<Session> {
        self.sessions.read().await.get(&id).cloned()
    }

    pub async fn update(&self, session: Session) {
        self.sessions.write().await.insert(session.id, session);
    }

    pub async fn delete(&self, id: Uuid) {
        self.sessions.write().await.remove(&id);
    }

    pub async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}
