# Crates Module — Phase 2: Storage, Assessment Engine, LLM Gateway

## Module Overview

Phase 2 splits `agent-core` into specialized components. Two new Rust crates join the workspace: `storage` and `assessment-engine`. The `llm-gateway` evolves as an enhanced Python service (continuing from Phase 1's Python LLM Gateway) rather than a Rust crate — it gains caching, advanced retry, and multi-model routing using the official `openai` and `anthropic` packages.

## Phase 2 Scope

| Component | Language | Purpose | Status |
|-----------|----------|---------|--------|
| `agent-core` | Rust | Core orchestration, HTTP API, state machine (continues) | Evolving |
| `storage` | Rust | Persistent database access (SQLite for dev, PostgreSQL for prod) | Planned |
| `assessment-engine` | Rust | Exercise generation, rubric-based answer evaluation, scoring | Planned |
| `llm-gateway` | Python | Enhanced LLM gateway: caching, advanced retry, multi-model routing, cost tracking | Evolving |

## Crate: storage

### Purpose

Persistent storage for sessions, user profiles, curriculum plans, chapter progress, messages, and assessment results. Phase 1 used in-memory storage; Phase 2 introduces durable persistence.

### File Structure

```
crates/storage/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── config.rs              # Database URL, connection pool settings
│   ├── connection.rs          # Pool creation, migration runner
│   ├── models/
│   │   ├── mod.rs
│   │   ├── session.rs         # Session row, CRUD
│   │   ├── curriculum.rs      # CurriculumPlan + Chapter rows
│   │   ├── progress.rs        # ChapterProgress rows
│   │   ├── message.rs         # Message rows
│   │   └── assessment.rs      # AssessmentResult rows
│   ├── migrations/
│   │   ├── 0001_create_sessions.sql
│   │   ├── 0002_create_curricula.sql
│   │   ├── 0003_create_progress.sql
│   │   ├── 0004_create_messages.sql
│   │   └── 0005_create_assessments.sql
│   └── error.rs
└── tests/
    ├── session_crud_test.rs
    ├── curriculum_crud_test.rs
    ├── progress_test.rs
    └── migration_test.rs
```

### Schema Design

```sql
-- 0001_create_sessions.sql
CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    state VARCHAR(32) NOT NULL DEFAULT 'IDLE',
    previous_state VARCHAR(32),
    goal JSONB,
    feasibility_result JSONB,
    user_profile JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_state ON sessions(state);
CREATE INDEX idx_sessions_updated_at ON sessions(updated_at);

-- 0002_create_curricula.sql
CREATE TABLE curricula (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    title VARCHAR(500) NOT NULL,
    description TEXT,
    estimated_duration VARCHAR(200),
    learning_objectives JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE chapters (
    id VARCHAR(100) PRIMARY KEY,
    curriculum_id UUID NOT NULL REFERENCES curricula(id) ON DELETE CASCADE,
    title VARCHAR(300) NOT NULL,
    "order" INTEGER NOT NULL,
    objectives JSONB NOT NULL,
    prerequisites JSONB,
    content TEXT,
    estimated_minutes INTEGER,
    key_concepts JSONB,
    UNIQUE(curriculum_id, "order")
);

CREATE INDEX idx_chapters_curriculum ON chapters(curriculum_id);

-- 0003_create_progress.sql
CREATE TABLE chapter_progress (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    chapter_id VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'not_started',
    completion REAL NOT NULL DEFAULT 0 CHECK (completion >= 0 AND completion <= 100),
    time_spent_minutes INTEGER DEFAULT 0,
    exercises_completed INTEGER DEFAULT 0,
    exercises_total INTEGER DEFAULT 0,
    difficulty_rating INTEGER CHECK (difficulty_rating >= 1 AND difficulty_rating <= 5),
    last_accessed TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(session_id, chapter_id)
);

-- 0004_create_messages.sql
CREATE TABLE messages (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    chapter_id VARCHAR(100),
    role VARCHAR(16) NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    content_type VARCHAR(32),
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_messages_session ON messages(session_id, created_at);

-- 0005_create_assessments.sql
CREATE TABLE assessments (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    chapter_id VARCHAR(100),
    exercise JSONB NOT NULL,
    learner_answer JSONB,
    evaluation JSONB,
    score REAL,
    max_score REAL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    evaluated_at TIMESTAMPTZ
);

CREATE INDEX idx_assessments_session ON assessments(session_id);
```

### Migration Scripts

Each migration has up and down scripts:

```sql
-- 0001_create_sessions.up.sql
CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    state VARCHAR(32) NOT NULL DEFAULT 'IDLE',
    previous_state VARCHAR(32),
    goal JSONB,
    feasibility_result JSONB,
    user_profile JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_sessions_state ON sessions(state);
CREATE INDEX idx_sessions_updated_at ON sessions(updated_at);

-- 0001_create_sessions.down.sql
DROP TABLE IF EXISTS sessions;
```

```
crates/storage/src/migrations/
├── 0001_create_sessions.up.sql
├── 0001_create_sessions.down.sql
├── 0002_create_curricula.up.sql
├── 0002_create_curricula.down.sql
├── 0003_create_progress.up.sql
├── 0003_create_progress.down.sql
├── 0004_create_messages.up.sql
├── 0004_create_messages.down.sql
├── 0005_create_assessments.up.sql
└── 0005_create_assessments.down.sql
```

**Migration runner:**

```rust
// connection.rs
use sqlx::migrate::Migrator;

static MIGRATOR: Migrator = sqlx::migrate!("src/migrations");

impl Storage {
    pub async fn run_migrations(&self) -> Result<(), StorageError> {
        match &self.db {
            Database::Sqlite(pool) => {
                MIGRATOR.run(pool).await?;
            }
            Database::Postgres(pool) => {
                MIGRATOR.run(pool).await?;
            }
        }
        tracing::info!("Database migrations complete");
        Ok(())
    }

    /// Roll back the last N migrations (development only).
    pub async fn rollback(&self, steps: u32) -> Result<(), StorageError> {
        for _ in 0..steps {
            match &self.db {
                Database::Sqlite(pool) => { MIGRATOR.undo(pool, 1).await?; }
                Database::Postgres(pool) => { MIGRATOR.undo(pool, 1).await?; }
            }
        }
        Ok(())
    }
}
```

### API

```rust
// storage/src/lib.rs (conceptual)
use sqlx::{PgPool, SqlitePool};

pub enum Database {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

pub struct Storage {
    db: Database,
}

impl Storage {
    pub async fn connect(config: StorageConfig) -> Result<Self, StorageError> { ... }
    pub async fn run_migrations(&self) -> Result<(), StorageError> { ... }

    // Sessions
    pub async fn create_session(&self) -> Result<Session, StorageError> { ... }
    pub async fn get_session(&self, id: Uuid) -> Result<Option<Session>, StorageError> { ... }
    pub async fn update_session_state(&self, id: Uuid, state: SessionState) -> Result<(), StorageError> { ... }
    pub async fn save_goal(&self, id: Uuid, goal: LearningGoal) -> Result<(), StorageError> { ... }

    // Curricula
    pub async fn save_curriculum(&self, curriculum: CurriculumPlan, session_id: Uuid) -> Result<(), StorageError> { ... }
    pub async fn get_curriculum(&self, session_id: Uuid) -> Result<Option<CurriculumPlan>, StorageError> { ... }

    // Progress
    pub async fn upsert_progress(&self, progress: ChapterProgress) -> Result<(), StorageError> { ... }
    pub async fn get_all_progress(&self, session_id: Uuid) -> Result<Vec<ChapterProgress>, StorageError> { ... }

    // Messages
    pub async fn save_message(&self, message: Message) -> Result<(), StorageError> { ... }
    pub async fn get_messages(&self, session_id: Uuid, limit: u32, before: Option<DateTime<Utc>>) -> Result<Vec<Message>, StorageError> { ... }

    // Assessments
    pub async fn save_assessment(&self, assessment: Assessment) -> Result<(), StorageError> { ... }
    pub async fn get_assessments(&self, session_id: Uuid) -> Result<Vec<Assessment>, StorageError> { ... }
}
```

### Cargo Dependencies

```toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "sqlite", "uuid", "chrono", "json"] }
uuid = { version = "1", features = ["v4", "serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
tokio = "1"
tracing = "0.1"
thiserror = "1"
```

### Connection Pool Configuration

```rust
// connection.rs
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
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

impl Storage {
    pub async fn connect(config: StorageConfig) -> Result<Self, StorageError> {
        let db = if config.database_url.starts_with("sqlite:") {
            let pool = SqlitePoolOptions::new()
                .max_connections(config.max_connections)
                .acquire_timeout(config.acquire_timeout)
                .idle_timeout(config.idle_timeout)
                .max_lifetime(config.max_lifetime)
                .connect(&config.database_url)
                .await?;

            // Enable WAL mode for better concurrent read performance
            sqlx::query("PRAGMA journal_mode=WAL;")
                .execute(&pool).await?;
            sqlx::query("PRAGMA busy_timeout=5000;")
                .execute(&pool).await?;
            sqlx::query("PRAGMA foreign_keys=ON;")
                .execute(&pool).await?;

            Database::Sqlite(pool)
        } else {
            let pool = PgPoolOptions::new()
                .max_connections(config.max_connections)
                .acquire_timeout(config.acquire_timeout)
                .idle_timeout(config.idle_timeout)
                .max_lifetime(config.max_lifetime)
                .connect(&config.database_url)
                .await?;

            Database::Postgres(pool)
        };

        tracing::info!(
            db_type = %if config.database_url.starts_with("sqlite:") { "sqlite" } else { "postgres" },
            max_connections = config.max_connections,
            "Database connected"
        );

        Ok(Self { db, config })
    }
}
```

### Backup and Restore

```rust
// backup.rs
impl Storage {
    /// Backup SQLite database to a file (Phase 2, SQLite only).
    pub async fn backup_sqlite(&self, output_path: &Path) -> Result<(), StorageError> {
        let pool = match &self.db {
            Database::Sqlite(pool) => pool,
            Database::Postgres(_) => {
                // PostgreSQL uses pg_dump, not in-process backup
                return Err(StorageError::UnsupportedOperation(
                    "In-process backup only supported for SQLite. Use pg_dump for PostgreSQL.".into()
                ));
            }
        };

        // Use SQLite backup API
        let mut src_conn = pool.acquire().await?;
        let dst_path = output_path.to_string_lossy().to_string();

        // Create backup connection to destination file
        let mut dst_conn = sqlx::SqliteConnection::connect(&format!("sqlite:{}?mode=rwc", dst_path)).await?;

        // Run backup
        sqlx::query("VACUUM INTO ?")
            .bind(&dst_path)
            .execute(&mut *src_conn)
            .await?;

        tracing::info!(path = %output_path.display(), "SQLite backup complete");
        Ok(())
    }

    /// Create a periodic backup task.
    pub async fn start_periodic_backup(
        pool: SqlitePool,
        backup_dir: PathBuf,
        interval: Duration,
    ) {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let path = backup_dir.join(format!("blup_backup_{}.db", timestamp));

                match Self::backup_to_path(&pool, &path).await {
                    Ok(()) => {
                        // Keep only last 7 daily backups
                        Self::cleanup_old_backups(&backup_dir, 7).await;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Backup failed");
                    }
                }
            }
        });
    }

    async fn cleanup_old_backups(backup_dir: &Path, keep: usize) {
        let mut backups: Vec<_> = std::fs::read_dir(backup_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("blup_backup_"))
            .collect();

        backups.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

        // Delete oldest beyond keep limit
        for entry in backups.iter().take(backups.len().saturating_sub(keep)) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
```

### Common Query Patterns

```rust
// queries.rs — Example query patterns used across storage
impl Storage {
    /// Get session with all related data in one query (avoids N+1).
    pub async fn get_session_full(&self, id: Uuid) -> Result<Option<SessionFull>, StorageError> {
        let session = sqlx::query_as::<_, SessionRow>(
            "SELECT * FROM sessions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool()?)
        .await?;

        if session.is_none() { return Ok(None); }
        let session = session.unwrap();

        let chapters = sqlx::query_as::<_, ChapterRow>(
            "SELECT c.* FROM chapters c
             JOIN curricula cur ON c.curriculum_id = cur.id
             WHERE cur.session_id = ?
             ORDER BY c.\"order\""
        )
        .bind(id)
        .fetch_all(&self.pool()?)
        .await?;

        let progress = sqlx::query_as::<_, ChapterProgressRow>(
            "SELECT * FROM chapter_progress WHERE session_id = ?"
        )
        .bind(id)
        .fetch_all(&self.pool()?)
        .await?;

        let messages = sqlx::query_as::<_, MessageRow>(
            "SELECT * FROM messages WHERE session_id = ? ORDER BY created_at LIMIT 100"
        )
        .bind(id)
        .fetch_all(&self.pool()?)
        .await?;

        Ok(Some(SessionFull { session, chapters, progress, messages }))
    }

    /// Batch-insert messages for performance.
    pub async fn save_messages_batch(&self, messages: &[Message]) -> Result<(), StorageError> {
        if messages.is_empty() { return Ok(()); }

        // SQLite: use transaction for batch insert
        let mut tx = self.pool()?.begin().await?;

        for msg in messages {
            sqlx::query(
                "INSERT INTO messages (id, session_id, chapter_id, role, content, content_type, metadata, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(msg.id)
            .bind(msg.session_id)
            .bind(&msg.chapter_id)
            .bind(&msg.role)
            .bind(&msg.content)
            .bind(&msg.content_type)
            .bind(&msg.metadata)
            .bind(msg.created_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Paginated message query.
    pub async fn get_messages_paginated(
        &self,
        session_id: Uuid,
        before: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<Message>, StorageError> {
        let messages = if let Some(before_time) = before {
            sqlx::query_as::<_, MessageRow>(
                "SELECT * FROM messages
                 WHERE session_id = ? AND created_at < ?
                 ORDER BY created_at DESC
                 LIMIT ?"
            )
            .bind(session_id)
            .bind(before_time)
            .bind(limit)
            .fetch_all(&self.pool()?)
            .await?
        } else {
            sqlx::query_as::<_, MessageRow>(
                "SELECT * FROM messages
                 WHERE session_id = ?
                 ORDER BY created_at DESC
                 LIMIT ?"
            )
            .bind(session_id)
            .bind(limit)
            .fetch_all(&self.pool()?)
            .await?
        };

        Ok(messages.into_iter().map(Into::into).collect())
    }
}
```

## Crate: assessment-engine

### Purpose

Generates exercises, evaluates learner answers against rubrics, and computes scores. Ensures exercise quality and answer evaluation are deterministic and auditable.

### File Structure

```
crates/assessment-engine/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── generation/
│   │   ├── mod.rs
│   │   ├── generator.rs       # LLM-assisted exercise generation
│   │   ├── templates.rs       # Exercise templates by type
│   │   └── difficulty.rs      # Difficulty calibration
│   ├── evaluation/
│   │   ├── mod.rs
│   │   ├── rubric.rs          # Rubric definition and matching
│   │   ├── scorer.rs          # Score computation
│   │   ├── multiple_choice.rs # MC evaluation
│   │   ├── short_answer.rs    # Short answer (semantic comparison via LLM)
│   │   ├── coding.rs          # Code submission (delegates to sandbox)
│   │   └── reflection.rs      # Reflection (completeness check)
│   ├── models/
│   │   ├── mod.rs
│   │   ├── exercise.rs
│   │   ├── rubric.rs
│   │   └── evaluation.rs
│   └── error.rs
└── tests/
    ├── generator_test.rs
    ├── mc_evaluation_test.rs
    ├── short_answer_test.rs
    └── rubric_test.rs
```

### Exercise Model

```rust
// models/exercise.rs (conceptual)
pub struct Exercise {
    pub id: Uuid,
    pub chapter_id: String,
    pub question: String,          // Markdown
    pub exercise_type: ExerciseType,
    pub difficulty: Difficulty,
    pub rubric: Rubric,
    pub max_score: f64,
    pub hints: Vec<String>,
    pub explanation: String,       // Shown after answering
}

pub enum ExerciseType {
    MultipleChoice {
        options: Vec<String>,
        correct_index: usize,
    },
    ShortAnswer {
        model_answer: String,      // Reference answer
        key_points: Vec<String>,   // Required points for full credit
    },
    Coding {
        language: String,
        starter_code: Option<String>,
        test_cases: Vec<TestCase>,
    },
    Reflection {
        prompt: String,
        min_length: usize,
        rubric_dimensions: Vec<RubricDimension>,
    },
}
```

### Evaluation Model

```rust
// models/evaluation.rs (conceptual)
pub struct Evaluation {
    pub exercise_id: Uuid,
    pub learner_answer: serde_json::Value,
    pub score: f64,
    pub max_score: f64,
    pub feedback: String,             // Personalized feedback in Markdown
    pub rubric_results: Vec<RubricResult>,
    pub is_correct: bool,             // true if score >= threshold
}

pub struct RubricResult {
    pub dimension: String,
    pub score: f64,
    pub max_score: f64,
    pub comment: String,
}
```

### Evaluation Flows

**Multiple Choice:** Fully deterministic — compare selected index to correct index. No LLM needed.

**Short Answer:** LLM-assisted semantic comparison. The prompt compares the learner's answer to the model answer and key points. The LLM produces structured rubric results. Core validates the output.

**Coding:** Sandbox execution. Submit code + test cases to the sandbox. Parse test results. Compute score from pass/fail ratio.

**Reflection:** LLM evaluates completeness and depth against rubric dimensions.

```rust
impl AssessmentEngine {
    pub async fn evaluate(
        &self,
        exercise: &Exercise,
        answer: &serde_json::Value,
        llm: &LlmGateway,           // LLM for semantic evaluation
        sandbox: Option<&SandboxManager>,  // Sandbox for code evaluation
    ) -> Result<Evaluation, AssessmentError> {
        match exercise.exercise_type {
            ExerciseType::MultipleChoice { correct_index, .. } => {
                self.evaluate_multiple_choice(exercise, answer, correct_index)
            }
            ExerciseType::ShortAnswer { ref model_answer, ref key_points } => {
                self.evaluate_short_answer(exercise, answer, model_answer, key_points, llm).await
            }
            ExerciseType::Coding { ref test_cases, .. } => {
                self.evaluate_coding(exercise, answer, test_cases, sandbox).await
            }
            ExerciseType::Reflection { ref rubric_dimensions, .. } => {
                self.evaluate_reflection(exercise, answer, rubric_dimensions, llm).await
            }
        }
    }
}
```

### Evaluation Edge Cases

```rust
// assessment-engine/src/evaluation/multiple_choice.rs
impl AssessmentEngine {
    fn evaluate_multiple_choice(
        &self,
        exercise: &Exercise,
        answer: &serde_json::Value,
        correct_index: usize,
    ) -> Result<Evaluation, AssessmentError> {
        let selected = answer["selected_index"].as_u64()
            .ok_or(AssessmentError::InvalidAnswer("Missing 'selected_index'".into()))? as usize;

        let score = if selected == correct_index { exercise.max_score } else { 0.0 };
        let is_correct = selected == correct_index;

        let feedback = if is_correct {
            "Correct!".to_string()
        } else {
            format!("The correct answer was option {}. {}", correct_index + 1, exercise.explanation)
        };

        Ok(Evaluation {
            exercise_id: exercise.id,
            learner_answer: answer.clone(),
            score,
            max_score: exercise.max_score,
            feedback,
            rubric_results: vec![],
            is_correct,
        })
    }
}

// Short answer evaluation with key point matching
fn evaluate_short_answer_key_points(
    learner_answer: &str,
    key_points: &[String],
) -> (f64, Vec<String>) {
    let mut matched = Vec::new();
    let mut missed = Vec::new();
    let learner_lower = learner_answer.to_lowercase();

    for point in key_points {
        // Check if the key concept appears in the learner's answer
        // (simple keyword matching; LLM-based semantic matching is more accurate)
        let keywords: Vec<&str> = point.split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();

        let match_count = keywords.iter()
            .filter(|kw| learner_lower.contains(&kw.to_lowercase()))
            .count();

        if match_count as f64 / keywords.len() as f64 > 0.5 {
            matched.push(point.clone());
        } else {
            missed.push(point.clone());
        }
    }

    let score = if key_points.is_empty() {
        exercise.max_score
    } else {
        (matched.len() as f64 / key_points.len() as f64) * exercise.max_score
    };

    (score, missed)
}

// Coding evaluation with test case result parsing
async fn evaluate_coding(
    &self,
    exercise: &Exercise,
    answer: &serde_json::Value,
    test_cases: &[TestCase],
    sandbox: Option<&SandboxManager>,
) -> Result<Evaluation, AssessmentError> {
    let sandbox = sandbox.ok_or(AssessmentError::SandboxRequired)?;
    let code = answer["code"].as_str()
        .ok_or(AssessmentError::InvalidAnswer("Missing 'code'".into()))?;

    // Build test harness: wrap learner's code + test cases into executable script
    let harness = build_test_harness(code, test_cases, &exercise.language);

    let result = sandbox.execute(SandboxRequest {
        tool_kind: ToolKind::CodeExecution { language: exercise.language.clone() },
        code: harness,
        ..Default::default()
    }).await?;

    // Parse test results from stdout (e.g., "TEST 1: PASS", "TEST 2: FAIL: expected 5 got 3")
    let test_results = parse_test_output(&result.stdout, test_cases.len());

    let passed = test_results.iter().filter(|r| r.passed).count();
    let score = (passed as f64 / test_cases.len() as f64) * exercise.max_score;

    let feedback = build_coding_feedback(&test_results, score, exercise.max_score);

    Ok(Evaluation {
        exercise_id: exercise.id,
        learner_answer: answer.clone(),
        score,
        max_score: exercise.max_score,
        feedback,
        rubric_results: vec![],
        is_correct: passed == test_cases.len(),
    })
}

fn build_test_harness(code: &str, test_cases: &[TestCase], language: &str) -> String {
    match language {
        "python" => format!(r#"
{code}

import json, sys
_results = []
{test_cases}

print(json.dumps([{{"name": r["name"], "passed": r["passed"], "message": r.get("message", "")}} for r in _results]))
"#),
        _ => unimplemented!("Test harness for {}", language),
    }
}
```

### Quality Gates

- [ ] Multiple choice evaluation is 100% deterministic (same input → same score after 1000 runs)
- [ ] Multiple choice with invalid answer format returns `AssessmentError::InvalidAnswer`
- [ ] Coding evaluation delegates to sandbox; never runs code in-process
- [ ] Coding evaluation with no sandbox available returns `AssessmentError::SandboxRequired`
- [ ] Short answer evaluation produces structured rubric results with matched/missed key points
- [ ] Short answer with empty answer returns score 0.0 with helpful feedback
- [ ] All evaluation outputs are schema-validated before returning
- [ ] No LLM claims of code execution appear in feedback text

## Service: llm-gateway (Python, Evolved from Phase 1)

### Purpose

The Python LLM Gateway evolves from its Phase 1 role (provider abstraction using `openai` and `anthropic` packages) into a full-featured gateway service with:
- **Prompt caching**: Anthropic's `cache_control` breakpoints and OpenAI's automatic caching.
- **Advanced retry**: Exponential backoff with jitter, circuit breaker pattern.
- **Multi-model routing**: Route requests to the best available model based on capability, cost, or load.
- **Rate limiting**: Token-bucket rate limiter per provider and per model.
- **Cost tracking**: Per-request and aggregate cost attribution by session.
- **Response streaming**: Efficient SSE streaming from both providers back to agent-core.

### Phase 2 File Structure (Evolved)

```
services/llm-gateway/
├── pyproject.toml
├── requirements.txt
├── src/
│   ├── __init__.py
│   ├── main.py                     # FastAPI entry point
│   ├── config.py                   # Multi-provider configuration
│   ├── providers/
│   │   ├── __init__.py
│   │   ├── base.py                 # Abstract provider interface
│   │   ├── openai_provider.py      # Uses `openai` package
│   │   ├── anthropic_provider.py   # Uses `anthropic` package
│   │   └── openai_compatible.py    # Generic OpenAI-compatible (local models)
│   ├── gateway.py                  # Model router + provider selection
│   ├── cache/
│   │   ├── __init__.py
│   │   ├── prompt_cache.py         # Anthropic cache_control breakpoint injection
│   │   └── response_cache.py       # Optional Redis-backed response cache
│   ├── routing/
│   │   ├── __init__.py
│   │   ├── router.py               # Model selection by name/pattern
│   │   └── fallback.py             # Fallback chain (primary → secondary model)
│   ├── middleware/
│   │   ├── __init__.py
│   │   ├── rate_limiter.py         # Token-bucket per provider
│   │   ├── cost_tracker.py         # Per-request cost calculation
│   │   └── circuit_breaker.py      # Open circuit on repeated failures
│   ├── stream.py                   # SSE streaming helpers
│   └── logging_config.py           # structlog configuration
└── tests/
    ├── test_openai_provider.py
    ├── test_anthropic_provider.py
    ├── test_cache.py
    ├── test_retry.py
    ├── test_rate_limiter.py
    └── test_gateway.py
```

### Prompt Caching Integration

The Python gateway leverages Anthropic's prompt caching natively through the SDK:

```python
# cache/prompt_cache.py (conceptual)
from anthropic.types import MessageParam

class PromptCacheManager:
    """Injects cache_control breakpoints for Anthropic prompt caching."""

    @staticmethod
    def prepare_messages(
        messages: list[dict],
        cache_system_prompt: bool = True,
    ) -> list[MessageParam]:
        result = []
        for i, msg in enumerate(reversed(messages)):
            original_index = len(messages) - 1 - i
            content = msg["content"]

            # Cache breakpoints on last N messages (oldest = most cacheable)
            if original_index <= 3:  # Cache last 4 messages
                result.insert(0, {
                    "role": msg["role"],
                    "content": [{
                        "type": "text",
                        "text": content,
                        "cache_control": {"type": "ephemeral"}
                    }]
                })
            else:
                result.insert(0, msg)

        return result

    @staticmethod
    def extract_cache_usage(response) -> dict:
        """Extract cache read/write token counts from Anthropic response."""
        usage = response.usage
        return {
            "cache_read_tokens": getattr(usage, 'cache_read_input_tokens', 0),
            "cache_write_tokens": getattr(usage, 'cache_creation_input_tokens', 0),
        }
```

### Circuit Breaker

```python
# middleware/circuit_breaker.py (conceptual)
from dataclasses import dataclass
from enum import Enum
import time
import asyncio

class CircuitState(Enum):
    CLOSED = "closed"           # Normal operation
    OPEN = "open"               # Failing, reject requests
    HALF_OPEN = "half_open"     # Testing if service recovered

@dataclass
class CircuitBreaker:
    failure_threshold: int = 5
    recovery_timeout: float = 30.0  # Seconds before trying HALF_OPEN
    half_open_max_requests: int = 1

    def __init__(self, ...):
        self.state = CircuitState.CLOSED
        self.failure_count = 0
        self.last_failure_time = 0.0

    async def call(self, provider_name: str, fn, *args, **kwargs):
        if self.state == CircuitState.OPEN:
            if time.monotonic() - self.last_failure_time > self.recovery_timeout:
                self.state = CircuitState.HALF_OPEN
            else:
                raise CircuitBreakerOpenError(f"Circuit open for {provider_name}")

        try:
            result = await fn(*args, **kwargs)
            if self.state == CircuitState.HALF_OPEN:
                self.state = CircuitState.CLOSED
                self.failure_count = 0
            return result
        except Exception as e:
            self.failure_count += 1
            self.last_failure_time = time.monotonic()
            if self.failure_count >= self.failure_threshold:
                self.state = CircuitState.OPEN
            raise e
```

### Fallback Routing

```python
# routing/fallback.py (conceptual)
class FallbackRouter:
    """Routes to primary model; falls back to secondary on failure."""

    def __init__(self, fallback_chains: dict[str, list[str]]):
        self.chains = fallback_chains
        # Example: {"gpt-4o": ["gpt-4o", "claude-sonnet-4-6", "gpt-4o-mini"]}

    async def route(self, model: str, request: GatewayRequest) -> GatewayResponse:
        chain = self.chains.get(model, [model])
        last_error = None

        for model_name in chain:
            try:
                provider = select_provider(model_name)
                return await provider.complete(request)
            except (RateLimitError, ProviderUnavailableError) as e:
                last_error = e
                logger.warning("fallback_triggered",
                    from_model=model_name, error=str(e))
                continue

        raise last_error or AllProvidersFailedError(chain)
```

## Agent-Core Phase 2 Changes

`agent-core` in Phase 2 changes from Phase 1:

1. **SessionStore** → delegates to `storage` crate (SQLite/Postgres).
2. **LlmClient** → continues calling the Python LLM Gateway (enhanced with Phase 2 features). The Rust client is unchanged; the gateway handles the new capabilities internally.
3. **New dependency**: `assessment-engine` crate for exercise evaluation.
4. **New API endpoints**:
   - `POST /api/session/{id}/chapter/{ch_id}/exercise/{ex_id}/submit` — submit exercise answer.
   - `GET /api/session/{id}/progress` — get all chapter progress.

### Updated Cargo.toml for agent-core in Phase 2

```toml
[dependencies]
storage = { path = "../storage" }
assessment-engine = { path = "../assessment-engine" }
# llm-gateway is NOT a Rust crate — it's the Python service on localhost
# agent-core continues calling it via HTTP as in Phase 1
# ... Phase 1 dependencies continue
```

## Testing Strategy

| Test Category | Crate | Method |
|---------------|-------|--------|
| SQL migrations | storage | Run migrations up/down; verify schema |
| CRUD operations | storage | Each model: create, read, update, delete |
| Connection pooling | storage | Concurrent read/write; pool exhaustion recovery |
| Exercise generation | assessment-engine | Mock LLM → generated exercise validates against schema |
| MC evaluation | assessment-engine | Determinism: same answer, same score (100 runs) |
| Short answer eval | assessment-engine | Mock LLM with known rubric outputs |
| Provider mapping | llm-gateway | Request → provider-specific format is correct |
| Retry logic | llm-gateway | Mock HTTP failures; verify backoff timing |
| Provider fallback | llm-gateway | Primary provider fails → secondary (if configured) |

## Quality Gates

- [ ] All storage migrations run and roll back cleanly
- [ ] SQLite is the default for dev; PostgreSQL for CI and production
- [ ] Session data survives restarts
- [ ] Assessment engine never runs learner code directly (delegates to sandbox)
- [ ] LLM gateway supports both OpenAI and Anthropic providers
- [ ] Retry logic has exponential backoff with jitter
- [ ] No API keys or raw prompts in gateway logs
- [ ] Crate dependency graph is acyclic (no circular dependencies)
- [ ] All crates pass `cargo fmt --check`, `cargo clippy`, `cargo test`
