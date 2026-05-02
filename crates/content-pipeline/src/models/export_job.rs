use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::TypstDiagnostic;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub export_type: ExportType,
    pub source_id: String,
    pub config: ExportConfig,
    pub status: ExportStatus,
    pub error: Option<ExportError>,
    pub result_artifact_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExportType {
    Chapter,
    Curriculum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub template: ExportTemplate,
    pub include_toc: bool,
    pub include_title_page: bool,
    pub compile_timeout_secs: u32,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            template: ExportTemplate::Chapter,
            include_toc: false,
            include_title_page: true,
            compile_timeout_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExportTemplate {
    Chapter,
    Curriculum,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    Pending,
    Rendering,
    Compiling,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportError {
    pub code: String,
    pub message: String,
    pub diagnostics: Option<Vec<TypstDiagnostic>>,
}

impl ExportJob {
    pub fn new_chapter(chapter_id: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: None,
            export_type: ExportType::Chapter,
            source_id: chapter_id.to_string(),
            config: ExportConfig::default(),
            status: ExportStatus::Pending,
            error: None,
            result_artifact_id: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn new_curriculum(curriculum_id: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: None,
            export_type: ExportType::Curriculum,
            source_id: curriculum_id.to_string(),
            config: ExportConfig {
                template: ExportTemplate::Curriculum,
                include_toc: true,
                ..ExportConfig::default()
            },
            status: ExportStatus::Pending,
            error: None,
            result_artifact_id: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn mark_completed(&mut self, artifact_id: Uuid) {
        self.status = ExportStatus::Completed;
        self.result_artifact_id = Some(artifact_id);
        self.completed_at = Some(Utc::now());
    }

    pub fn mark_failed(
        &mut self,
        code: &str,
        message: &str,
        diagnostics: Option<Vec<TypstDiagnostic>>,
    ) {
        self.status = ExportStatus::Failed;
        self.error = Some(ExportError {
            code: code.to_string(),
            message: message.to_string(),
            diagnostics,
        });
        self.completed_at = Some(Utc::now());
    }
}
