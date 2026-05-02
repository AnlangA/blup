use std::time::Duration;

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite:blup.db?mode=rwc".into(),
            max_connections: if cfg!(test) { 1 } else { 10 },
            acquire_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

impl StorageConfig {
    pub fn sqlite(path: &str) -> Self {
        Self {
            database_url: format!("sqlite:{}?mode=rwc", path),
            ..Default::default()
        }
    }

    pub fn postgres(url: &str) -> Self {
        Self {
            database_url: url.to_string(),
            ..Default::default()
        }
    }

    pub fn is_sqlite(&self) -> bool {
        self.database_url.starts_with("sqlite:")
    }

    pub fn is_postgres(&self) -> bool {
        self.database_url.starts_with("postgres:") || self.database_url.starts_with("postgresql:")
    }
}
