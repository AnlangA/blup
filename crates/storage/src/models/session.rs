use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::StorageError;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: String,
    pub state: String,
    pub previous_state: Option<String>,
    pub goal: Option<String>,
    pub feasibility_result: Option<String>,
    pub user_profile: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn create_session(pool: &SqlitePool) -> Result<Session, StorageError> {
    let id = Uuid::new_v4().to_string();
    create_session_with_id(pool, &id).await
}

pub async fn create_session_with_id(pool: &SqlitePool, id: &str) -> Result<Session, StorageError> {
    let now = Utc::now();

    sqlx::query("INSERT INTO sessions (id, state, created_at, updated_at) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind("IDLE")
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

    Ok(Session {
        id: id.to_string(),
        state: "IDLE".to_string(),
        previous_state: None,
        goal: None,
        feasibility_result: None,
        user_profile: None,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_session(pool: &SqlitePool, id: Uuid) -> Result<Option<Session>, StorageError> {
    let session = sqlx::query_as::<_, Session>(
        "SELECT id, state, previous_state, goal, feasibility_result, user_profile, created_at, updated_at FROM sessions WHERE id = ?"
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(session)
}

pub async fn update_session_state(
    pool: &SqlitePool,
    id: Uuid,
    state: &str,
) -> Result<(), StorageError> {
    let now = Utc::now();
    let result = sqlx::query("UPDATE sessions SET state = ?, updated_at = ? WHERE id = ?")
        .bind(state)
        .bind(now)
        .bind(id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::not_found("Session", &id.to_string()));
    }

    Ok(())
}

pub async fn save_goal(
    pool: &SqlitePool,
    id: Uuid,
    goal: serde_json::Value,
) -> Result<(), StorageError> {
    let now = Utc::now();
    let result = sqlx::query("UPDATE sessions SET goal = ?, updated_at = ? WHERE id = ?")
        .bind(serde_json::to_string(&goal)?)
        .bind(now)
        .bind(id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::not_found("Session", &id.to_string()));
    }

    Ok(())
}

pub async fn save_feasibility_result(
    pool: &SqlitePool,
    id: Uuid,
    result: serde_json::Value,
) -> Result<(), StorageError> {
    let now = Utc::now();
    let result =
        sqlx::query("UPDATE sessions SET feasibility_result = ?, updated_at = ? WHERE id = ?")
            .bind(serde_json::to_string(&result)?)
            .bind(now)
            .bind(id.to_string())
            .execute(pool)
            .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::not_found("Session", &id.to_string()));
    }

    Ok(())
}

pub async fn save_user_profile(
    pool: &SqlitePool,
    id: Uuid,
    profile: serde_json::Value,
) -> Result<(), StorageError> {
    let now = Utc::now();
    let result = sqlx::query("UPDATE sessions SET user_profile = ?, updated_at = ? WHERE id = ?")
        .bind(serde_json::to_string(&profile)?)
        .bind(now)
        .bind(id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::not_found("Session", &id.to_string()));
    }

    Ok(())
}

pub async fn delete_session(pool: &SqlitePool, id: Uuid) -> Result<(), StorageError> {
    let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::not_found("Session", &id.to_string()));
    }

    Ok(())
}

pub async fn list_sessions(pool: &SqlitePool) -> Result<Vec<Session>, StorageError> {
    let sessions = sqlx::query_as::<_, Session>(
        "SELECT id, state, previous_state, goal, feasibility_result, user_profile, created_at, updated_at FROM sessions ORDER BY updated_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(sessions)
}
