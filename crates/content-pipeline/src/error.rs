use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Unsupported file type: {extension}")]
    UnsupportedType { extension: String },

    #[error("PDF extraction failed for {path}: {reason}")]
    ExtractionFailed { path: String, reason: String },

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("URL blocked: {url} — {reason}")]
    UrlBlocked { url: String, reason: String },

    #[error("Website fetch failed: {url} — {reason}")]
    FetchFailed { url: String, reason: String },

    #[error("Content too short ({length} chars) from {origin}")]
    ContentTooShort { origin: String, length: usize },

    #[error("No content found at {0}")]
    NoContent(String),

    #[error("Encoding detection failed for {path}")]
    EncodingError { path: String },

    #[error("Chunking error: {0}")]
    ChunkingError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("PDF extract error: {0}")]
    PdfExtract(#[from] pdf_extract::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Sandbox error: {0}")]
    Sandbox(String),
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Chapter not found: {0}")]
    ChapterNotFound(String),

    #[error("Curriculum not found: {0}")]
    CurriculumNotFound(String),

    #[error("Typst rendering failed: {0}")]
    RenderingFailed(String),

    #[error("Invalid chapter markdown: {0}")]
    InvalidMarkdown(String),

    #[error("Compilation failed: {message}")]
    CompilationFailed {
        message: String,
        diagnostics: Vec<TypstDiagnostic>,
    },

    #[error("Invalid PDF output: {0}")]
    InvalidPdfOutput(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Sandbox error: {0}")]
    Sandbox(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypstDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub source_line: Option<String>,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

impl std::fmt::Display for TypstDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.severity, self.message)?;
        if let Some(line) = self.line {
            write!(f, " at line {}", line)?;
            if let Some(col) = self.column {
                write!(f, ":{}", col)?;
            }
        }
        Ok(())
    }
}
