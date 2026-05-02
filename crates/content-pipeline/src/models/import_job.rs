use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::source_document::SourceType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportJob {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub source_type: SourceType,
    pub source_path: Option<String>,
    pub source_url: Option<String>,
    pub config: ImportConfig,
    pub status: ImportStatus,
    pub error: Option<ImportError>,
    pub result_document_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    pub ocr_enabled: bool,
    pub max_chunk_size_chars: usize,
    pub chunk_overlap_chars: usize,
    pub timeout_secs: u32,
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            ocr_enabled: false,
            max_chunk_size_chars: 4000,
            chunk_overlap_chars: 200,
            timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    Pending,
    Extracting,
    Chunking,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub code: String,
    pub message: String,
}

impl ImportJob {
    pub fn new_pdf(path: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: None,
            source_type: SourceType::Pdf,
            source_path: Some(path.to_string()),
            source_url: None,
            config: ImportConfig::default(),
            status: ImportStatus::Pending,
            error: None,
            result_document_id: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn new_markdown(path: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: None,
            source_type: SourceType::Markdown,
            source_path: Some(path.to_string()),
            source_url: None,
            config: ImportConfig::default(),
            status: ImportStatus::Pending,
            error: None,
            result_document_id: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn new_text(path: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: None,
            source_type: SourceType::PlainText,
            source_path: Some(path.to_string()),
            source_url: None,
            config: ImportConfig::default(),
            status: ImportStatus::Pending,
            error: None,
            result_document_id: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn new_website(url: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: None,
            source_type: SourceType::Website,
            source_path: None,
            source_url: Some(url.to_string()),
            config: ImportConfig::default(),
            status: ImportStatus::Pending,
            error: None,
            result_document_id: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn mark_completed(&mut self, document_id: Uuid) {
        self.status = ImportStatus::Completed;
        self.result_document_id = Some(document_id);
        self.completed_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self, code: &str, message: &str) {
        self.status = ImportStatus::Failed;
        self.error = Some(ImportError {
            code: code.to_string(),
            message: message.to_string(),
        });
        self.completed_at = Some(Utc::now());
    }
}
