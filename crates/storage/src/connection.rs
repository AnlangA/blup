use sqlx::migrate::Migrator;
use sqlx::sqlite::SqlitePoolOptions;

use crate::config::StorageConfig;
use crate::Database;

static MIGRATOR: Migrator = sqlx::migrate!("src/migrations");

pub async fn create_pool(config: &StorageConfig) -> Result<Database, crate::error::StorageError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(config.acquire_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(config.max_lifetime)
        .connect(&config.database_url)
        .await
        .map_err(|e| {
            crate::error::StorageError::Connection(format!("Failed to connect to database: {e}"))
        })?;

    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA busy_timeout=5000;")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys=ON;")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA synchronous=NORMAL;")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA cache_size=-64000;")
        .execute(&pool)
        .await?;

    tracing::info!(
        db_type = "sqlite",
        max_connections = config.max_connections,
        "Database connected"
    );

    Ok(Database::Sqlite(pool))
}

pub async fn run_migrations(db: &Database) -> Result<(), crate::error::StorageError> {
    match db {
        Database::Sqlite(pool) => {
            MIGRATOR.run(pool).await?;
        }
    }
    tracing::info!("Database migrations complete");
    Ok(())
}

pub async fn rollback(db: &Database, steps: u32) -> Result<(), crate::error::StorageError> {
    match db {
        Database::Sqlite(pool) => {
            for _ in 0..steps {
                MIGRATOR.undo(pool, 1).await?;
            }
        }
    }
    tracing::info!(steps = steps, "Database rollback complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StorageConfig;

    async fn test_db() -> Database {
        let config = StorageConfig::sqlite(":memory:");
        create_pool(&config).await.unwrap()
    }

    #[tokio::test]
    async fn test_migrations_idempotent() {
        let db = test_db().await;
        // Run migrations twice — both should succeed
        run_migrations(&db).await.unwrap();
        run_migrations(&db).await.unwrap();
    }

    #[tokio::test]
    async fn test_migrations_create_tables() {
        let db = test_db().await;
        run_migrations(&db).await.unwrap();

        // Verify all tables exist
        let Database::Sqlite(pool) = &db;
        let tables: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .fetch_all(pool)
                .await
                .unwrap();

        assert!(tables.contains(&"sessions".to_string()));
        assert!(tables.contains(&"curricula".to_string()));
        assert!(tables.contains(&"chapter_progress".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"assessments".to_string()));
    }

    #[tokio::test]
    async fn test_rollback_and_re_migrate() {
        let db = test_db().await;
        run_migrations(&db).await.unwrap();

        // Rollback the last migration
        rollback(&db, 1).await.unwrap();

        // Re-migrate should succeed
        run_migrations(&db).await.unwrap();
    }
}
