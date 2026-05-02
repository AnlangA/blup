CREATE TABLE IF NOT EXISTS assessments (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    chapter_id TEXT,
    exercise TEXT NOT NULL,
    learner_answer TEXT,
    evaluation TEXT,
    score REAL,
    max_score REAL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    evaluated_at DATETIME
);

CREATE INDEX IF NOT EXISTS idx_assessments_session ON assessments(session_id);
