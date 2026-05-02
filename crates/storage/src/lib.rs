pub mod config;
pub mod connection;
pub mod error;
pub mod models;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

pub use config::StorageConfig;
pub use error::StorageError;

#[derive(Debug, Clone)]
pub enum Database {
    Sqlite(SqlitePool),
}

#[derive(Debug, Clone)]
pub struct Storage {
    db: Database,
    #[allow(dead_code)]
    config: StorageConfig,
}

impl Storage {
    pub async fn connect(config: StorageConfig) -> Result<Self, StorageError> {
        let db = connection::create_pool(&config).await?;
        Ok(Self { db, config })
    }

    pub async fn run_migrations(&self) -> Result<(), StorageError> {
        connection::run_migrations(&self.db).await
    }

    pub async fn rollback(&self, steps: u32) -> Result<(), StorageError> {
        connection::rollback(&self.db, steps).await
    }

    /// Backup the database to a file path using SQLite VACUUM INTO.
    pub async fn backup(&self, path: &str) -> Result<(), StorageError> {
        let pool = self.pool();
        let query = format!("VACUUM INTO '{}'", path.replace('\'', "''"));
        sqlx::query(&query)
            .execute(pool)
            .await
            .map_err(|e| StorageError::Connection(format!("Backup failed: {e}")))?;
        tracing::info!(path = path, "Database backup completed");
        Ok(())
    }

    /// Restore the database from a backup file by closing current connections
    /// and replacing the database file. Returns a new Storage connected to the
    /// restored database.
    pub async fn restore(config: &StorageConfig, backup_path: &str) -> Result<Self, StorageError> {
        if !config.is_sqlite() {
            return Err(StorageError::UnsupportedOperation(
                "Restore is only supported for SQLite".into(),
            ));
        }

        let db_path = config
            .database_url
            .strip_prefix("sqlite:")
            .unwrap_or(&config.database_url);

        // Close existing connections by dropping any active pool, then copy
        tokio::fs::copy(backup_path, db_path)
            .await
            .map_err(|e| StorageError::Connection(format!("Failed to restore from backup: {e}")))?;

        tracing::info!(
            backup = backup_path,
            target = db_path,
            "Database restored from backup"
        );
        Self::connect(config.clone()).await
    }

    fn pool(&self) -> &SqlitePool {
        match &self.db {
            Database::Sqlite(pool) => pool,
        }
    }

    // Session operations
    pub async fn create_session(&self) -> Result<models::session::Session, StorageError> {
        models::session::create_session(self.pool()).await
    }

    pub async fn create_session_with_id(
        &self,
        id: Uuid,
    ) -> Result<models::session::Session, StorageError> {
        models::session::create_session_with_id(self.pool(), &id.to_string()).await
    }

    pub async fn get_session(
        &self,
        id: Uuid,
    ) -> Result<Option<models::session::Session>, StorageError> {
        models::session::get_session(self.pool(), id).await
    }

    pub async fn update_session_state(&self, id: Uuid, state: &str) -> Result<(), StorageError> {
        models::session::update_session_state(self.pool(), id, state).await
    }

    pub async fn save_goal(&self, id: Uuid, goal: serde_json::Value) -> Result<(), StorageError> {
        models::session::save_goal(self.pool(), id, goal).await
    }

    pub async fn save_feasibility_result(
        &self,
        id: Uuid,
        result: serde_json::Value,
    ) -> Result<(), StorageError> {
        models::session::save_feasibility_result(self.pool(), id, result).await
    }

    pub async fn save_user_profile(
        &self,
        id: Uuid,
        profile: serde_json::Value,
    ) -> Result<(), StorageError> {
        models::session::save_user_profile(self.pool(), id, profile).await
    }

    pub async fn delete_session(&self, id: Uuid) -> Result<(), StorageError> {
        models::session::delete_session(self.pool(), id).await
    }

    pub async fn list_sessions(&self) -> Result<Vec<models::session::Session>, StorageError> {
        models::session::list_sessions(self.pool()).await
    }

    // Curriculum operations
    pub async fn save_curriculum(
        &self,
        session_id: Uuid,
        curriculum: serde_json::Value,
    ) -> Result<(), StorageError> {
        models::curriculum::save_curriculum(self.pool(), session_id, curriculum).await
    }

    pub async fn get_curriculum(
        &self,
        session_id: Uuid,
    ) -> Result<Option<serde_json::Value>, StorageError> {
        models::curriculum::get_curriculum(self.pool(), session_id).await
    }

    // Progress operations
    pub async fn upsert_progress(
        &self,
        session_id: Uuid,
        chapter_id: &str,
        progress: serde_json::Value,
    ) -> Result<(), StorageError> {
        models::progress::upsert_progress(self.pool(), session_id, chapter_id, progress).await
    }

    pub async fn get_progress(
        &self,
        session_id: Uuid,
        chapter_id: &str,
    ) -> Result<Option<serde_json::Value>, StorageError> {
        models::progress::get_progress(self.pool(), session_id, chapter_id).await
    }

    pub async fn get_all_progress(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<serde_json::Value>, StorageError> {
        models::progress::get_all_progress(self.pool(), session_id).await
    }

    // Message operations
    pub async fn save_message(
        &self,
        session_id: Uuid,
        chapter_id: Option<&str>,
        role: &str,
        content: &str,
    ) -> Result<(), StorageError> {
        models::message::save_message(self.pool(), session_id, chapter_id, role, content).await
    }

    pub async fn get_messages(
        &self,
        session_id: Uuid,
        limit: i64,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<serde_json::Value>, StorageError> {
        models::message::get_messages(self.pool(), session_id, limit, before).await
    }

    // Assessment operations
    pub async fn save_assessment(
        &self,
        session_id: Uuid,
        chapter_id: Option<&str>,
        input: &models::assessment::AssessmentInput,
    ) -> Result<(), StorageError> {
        models::assessment::save_assessment(self.pool(), session_id, chapter_id, input).await
    }

    pub async fn get_assessments(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<serde_json::Value>, StorageError> {
        models::assessment::get_assessments(self.pool(), session_id).await
    }
}
