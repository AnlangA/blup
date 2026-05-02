use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Connection error: {0}")]
    Connection(String),
}

impl StorageError {
    pub fn not_found(entity: &str, id: &str) -> Self {
        StorageError::NotFound(format!("{} with id {} not found", entity, id))
    }

    pub fn connection(msg: &str) -> Self {
        StorageError::Connection(msg.to_string())
    }
}
