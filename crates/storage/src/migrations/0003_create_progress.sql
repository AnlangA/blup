CREATE TABLE IF NOT EXISTS chapter_progress (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    chapter_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'not_started',
    completion REAL NOT NULL DEFAULT 0 CHECK (completion >= 0 AND completion <= 100),
    time_spent_minutes INTEGER DEFAULT 0,
    exercises_completed INTEGER DEFAULT 0,
    exercises_total INTEGER DEFAULT 0,
    difficulty_rating INTEGER CHECK (difficulty_rating >= 1 AND difficulty_rating <= 5),
    last_accessed DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(session_id, chapter_id)
);

CREATE INDEX IF NOT EXISTS idx_chapter_progress_session ON chapter_progress(session_id);
