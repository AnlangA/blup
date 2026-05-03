CREATE TABLE IF NOT EXISTS source_documents (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    source_type TEXT NOT NULL,
    title TEXT NOT NULL,
    origin TEXT NOT NULL,
    checksum TEXT NOT NULL,
    language TEXT,
    license_or_usage_note TEXT,
    metadata TEXT NOT NULL,
    extracted_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS source_chunks (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    heading_path TEXT NOT NULL,
    token_count INTEGER NOT NULL,
    overlap_with_previous BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (document_id) REFERENCES source_documents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS import_jobs (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    source_type TEXT NOT NULL,
    source_path TEXT,
    source_url TEXT,
    config TEXT NOT NULL,
    status TEXT NOT NULL,
    error TEXT,
    result_document_id TEXT,
    created_at DATETIME NOT NULL,
    completed_at DATETIME,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (result_document_id) REFERENCES source_documents(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS export_jobs (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    export_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    config TEXT NOT NULL,
    status TEXT NOT NULL,
    error TEXT,
    result_artifact_id TEXT,
    created_at DATETIME NOT NULL,
    completed_at DATETIME,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS citations (
    id TEXT PRIMARY KEY,
    source_chunk_id TEXT NOT NULL,
    target_message_id TEXT NOT NULL,
    relevance_score REAL NOT NULL,
    usage_type TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (source_chunk_id) REFERENCES source_chunks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_source_documents_session_id ON source_documents(session_id);
CREATE INDEX IF NOT EXISTS idx_source_documents_checksum ON source_documents(checksum);
CREATE INDEX IF NOT EXISTS idx_source_chunks_document_id ON source_chunks(document_id);
CREATE INDEX IF NOT EXISTS idx_import_jobs_session_id ON import_jobs(session_id);
CREATE INDEX IF NOT EXISTS idx_import_jobs_status ON import_jobs(status);
CREATE INDEX IF NOT EXISTS idx_export_jobs_session_id ON export_jobs(session_id);
CREATE INDEX IF NOT EXISTS idx_export_jobs_status ON export_jobs(status);
CREATE INDEX IF NOT EXISTS idx_citations_target_message_id ON citations(target_message_id);
