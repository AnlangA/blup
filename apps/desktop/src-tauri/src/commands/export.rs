use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::PathBuf;
use tauri::{command, AppHandle, Emitter, State};

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub path: String,
    pub checksum: String,
    pub size_bytes: u64,
    pub page_count: Option<u32>,
    pub compiled: bool,
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.message, self.code)
    }
}

impl From<content_pipeline::error::ExportError> for ExportError {
    fn from(err: content_pipeline::error::ExportError) -> Self {
        ExportError {
            code: "EXPORT_ERROR".to_string(),
            message: err.to_string(),
        }
    }
}

/// Try to compile Typst source to PDF, falling back through multiple methods:
/// 1. Docker sandbox (if available and sandbox-typst image exists)
/// 2. Host typst CLI (typst compile)
/// 3. Save as .typst source file (always works)
async fn compile_or_save_typst(
    typst_source: &str,
    save_path: &std::path::Path,
) -> Result<ExportResult, ExportError> {
    // Try 1: Docker sandbox compilation
    let sandbox_config = sandbox_manager::SandboxConfig::default()
        .with_image("sandbox-typst:latest")
        .with_timeouts(
            std::time::Duration::from_secs(60),
            std::time::Duration::from_secs(30),
        )
        .with_memory(1024);

    let sandbox = std::sync::Arc::new(sandbox_manager::SandboxManager::new(sandbox_config));

    // Quick health check before attempting compilation
    let docker_available = sandbox.health_check().await.is_ok();

    if docker_available {
        let compiler = content_pipeline::export::TypstCompiler::new(sandbox);
        if let Ok(artifact) = compiler
            .compile_to_pdf(typst_source, &std::collections::HashMap::new())
            .await
        {
            std::fs::write(save_path, &artifact.data).map_err(|e| ExportError {
                code: "WRITE_FAILED".to_string(),
                message: format!("Failed to write PDF: {e}"),
            })?;

            return Ok(ExportResult {
                path: save_path.to_string_lossy().to_string(),
                checksum: artifact.checksum,
                size_bytes: artifact.size_bytes,
                page_count: artifact.page_count,
                compiled: true,
                format: "pdf".to_string(),
            });
        }
    }

    // Try 2: Host typst CLI
    let typst_check = std::process::Command::new("typst")
        .arg("--version")
        .output();

    if let Ok(output) = typst_check {
        if output.status.success() {
            let tmp_dir = tempfile::TempDir::new().map_err(|e| ExportError {
                code: "TEMP_DIR_FAILED".to_string(),
                message: format!("Failed to create temp dir: {e}"),
            })?;
            let input_path = tmp_dir.path().join("input.typ");
            std::fs::write(&input_path, typst_source).map_err(|e| ExportError {
                code: "WRITE_FAILED".to_string(),
                message: format!("Failed to write temp Typst file: {e}"),
            })?;

            let compile_result = std::process::Command::new("typst")
                .args(["compile", input_path.to_str().unwrap_or("input.typ")])
                .arg(save_path)
                .output()
                .map_err(|e| ExportError {
                    code: "COMPILE_FAILED".to_string(),
                    message: format!("Failed to run typst: {e}"),
                })?;

            if compile_result.status.success() {
                let metadata = std::fs::metadata(save_path).map_err(|e| ExportError {
                    code: "READ_FAILED".to_string(),
                    message: format!("Failed to read compiled PDF: {e}"),
                })?;

                let pdf_data = std::fs::read(save_path).map_err(|e| ExportError {
                    code: "READ_FAILED".to_string(),
                    message: format!("Failed to read compiled PDF: {e}"),
                })?;

                let checksum = format!(
                    "sha256:{:x}",
                    sha2::Sha256::digest(&pdf_data)
                );

                return Ok(ExportResult {
                    path: save_path.to_string_lossy().to_string(),
                    checksum,
                    size_bytes: metadata.len(),
                    page_count: None,
                    compiled: true,
                    format: "pdf".to_string(),
                });
            } else {
                let stderr = String::from_utf8_lossy(&compile_result.stderr);
                // Persist the failing Typst source so the user can inspect it
                let debug_path = save_path.with_extension("debug.typ");
                let _ = std::fs::write(&debug_path, typst_source);
                tracing::warn!(
                    "Host typst compilation failed. Source saved to {:?}\n--- stderr ---\n{}\n--- end ---",
                    debug_path,
                    stderr
                );
            }
        }
    }

    // Fallback: Save .typst source file (always works)
    let typst_path = save_path.with_extension("typst");
    std::fs::write(&typst_path, typst_source).map_err(|e| ExportError {
        code: "WRITE_FAILED".to_string(),
        message: format!("Failed to write Typst file: {e}"),
    })?;

    let size = typst_source.len() as u64;
    let checksum = format!(
        "sha256:{:x}",
        sha2::Sha256::digest(typst_source.as_bytes())
    );

    Ok(ExportResult {
        path: typst_path.to_string_lossy().to_string(),
        checksum,
        size_bytes: size,
        page_count: None,
        compiled: false,
        format: "typst".to_string(),
    })
}

// ── Chapter export (PDF) ──

#[command]
pub async fn export_chapter_pdf(
    app: AppHandle,
    chapter: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<ExportResult, ExportError> {
    let chapter_id = chapter
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "rendering", "chapter": chapter_id }),
    );

    let typst_source = state.content_pipeline.render_chapter_to_typst(&chapter)?;

    // Open save dialog
    use tauri_plugin_dialog::DialogExt;
    let save_path = app
        .dialog()
        .file()
        .set_title("Save Chapter")
        .add_filter("All Supported", &["pdf", "typst"])
        .add_filter("PDF Files", &["pdf"])
        .add_filter("Typst Files", &["typst"])
        .blocking_save_file();

    let save_path = match save_path {
        Some(path) => PathBuf::from(path.to_string()),
        None => {
            return Err(ExportError {
                code: "USER_CANCELLED".to_string(),
                message: "User cancelled export".to_string(),
            })
        }
    };

    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "compiling", "chapter": chapter_id }),
    );

    let result = compile_or_save_typst(&typst_source, &save_path).await?;

    let _ = app.emit(
        "export:complete",
        serde_json::json!({
            "chapter": chapter_id,
            "path": result.path,
            "compiled": result.compiled,
            "format": result.format,
        }),
    );

    Ok(result)
}

// ── Curriculum export (PDF) ──

#[command]
pub async fn export_curriculum_pdf(
    app: AppHandle,
    curriculum: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<ExportResult, ExportError> {
    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "rendering" }),
    );

    let typst_source = state
        .content_pipeline
        .render_curriculum_to_typst(&curriculum)?;

    // Open save dialog
    use tauri_plugin_dialog::DialogExt;
    let save_path = app
        .dialog()
        .file()
        .set_title("Save Curriculum")
        .add_filter("All Supported", &["pdf", "typst"])
        .add_filter("PDF Files", &["pdf"])
        .add_filter("Typst Files", &["typst"])
        .blocking_save_file();

    let save_path = match save_path {
        Some(path) => PathBuf::from(path.to_string()),
        None => {
            return Err(ExportError {
                code: "USER_CANCELLED".to_string(),
                message: "User cancelled export".to_string(),
            })
        }
    };

    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "compiling" }),
    );

    let result = compile_or_save_typst(&typst_source, &save_path).await?;

    let _ = app.emit(
        "export:complete",
        serde_json::json!({
            "path": result.path,
            "compiled": result.compiled,
            "format": result.format,
        }),
    );

    Ok(result)
}

// ── Chapter export (Typst) ──

#[command]
pub async fn export_typst(
    app: AppHandle,
    chapter: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<ExportResult, ExportError> {
    let chapter_id = chapter
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "rendering", "chapter": chapter_id }),
    );

    let typst_source = state.content_pipeline.render_chapter_to_typst(&chapter)?;

    // Open save dialog
    use tauri_plugin_dialog::DialogExt;
    let save_path = app
        .dialog()
        .file()
        .set_title("Save Typst Source")
        .add_filter("Typst Files", &["typst"])
        .blocking_save_file();

    let save_path = match save_path {
        Some(path) => PathBuf::from(path.to_string()),
        None => {
            return Err(ExportError {
                code: "USER_CANCELLED".to_string(),
                message: "User cancelled export".to_string(),
            })
        }
    };

    // Always ensure .typst extension
    let typst_path = save_path.with_extension("typst");
    std::fs::write(&typst_path, &typst_source).map_err(|e| ExportError {
        code: "WRITE_FAILED".to_string(),
        message: format!("Failed to write Typst file: {e}"),
    })?;

    let checksum = format!(
        "sha256:{:x}",
        sha2::Sha256::digest(typst_source.as_bytes())
    );

    let _ = app.emit(
        "export:complete",
        serde_json::json!({
            "chapter": chapter_id,
            "path": typst_path.to_string_lossy().to_string(),
        }),
    );

    Ok(ExportResult {
        path: typst_path.to_string_lossy().to_string(),
        checksum,
        size_bytes: typst_source.len() as u64,
        page_count: None,
        compiled: false,
        format: "typst".to_string(),
    })
}

// ── Curriculum export (Typst) ──

#[command]
pub async fn export_curriculum_typst(
    app: AppHandle,
    curriculum: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<ExportResult, ExportError> {
    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "rendering" }),
    );

    let typst_source = state
        .content_pipeline
        .render_curriculum_to_typst(&curriculum)?;

    // Open save dialog
    use tauri_plugin_dialog::DialogExt;
    let save_path = app
        .dialog()
        .file()
        .set_title("Save Curriculum as Typst")
        .add_filter("Typst Files", &["typst"])
        .blocking_save_file();

    let save_path = match save_path {
        Some(path) => PathBuf::from(path.to_string()),
        None => {
            return Err(ExportError {
                code: "USER_CANCELLED".to_string(),
                message: "User cancelled export".to_string(),
            })
        }
    };

    let typst_path = save_path.with_extension("typst");
    std::fs::write(&typst_path, &typst_source).map_err(|e| ExportError {
        code: "WRITE_FAILED".to_string(),
        message: format!("Failed to write Typst file: {e}"),
    })?;

    let checksum = format!(
        "sha256:{:x}",
        sha2::Sha256::digest(typst_source.as_bytes())
    );

    let _ = app.emit(
        "export:complete",
        serde_json::json!({
            "path": typst_path.to_string_lossy().to_string(),
        }),
    );

    Ok(ExportResult {
        path: typst_path.to_string_lossy().to_string(),
        checksum,
        size_bytes: typst_source.len() as u64,
        page_count: None,
        compiled: false,
        format: "typst".to_string(),
    })
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that content_pipeline renders Typst correctly from chapter JSON.
    #[test]
    fn test_render_chapter_to_typst() {
        let pipeline = content_pipeline::ContentPipeline::new();

        let chapter_json = serde_json::json!({
            "title": "Introduction",
            "content": "# Welcome\n\nThis is a test chapter.\n\n## Section 1\n\nSome content here.",
        });

        let result = pipeline.render_chapter_to_typst(&chapter_json);
        assert!(
            result.is_ok(),
            "render_chapter_to_typst should succeed: {:?}",
            result.err()
        );

        let typst_source = result.unwrap();
        assert!(!typst_source.is_empty());
        assert!(
            typst_source.contains("Introduction"),
            "Typst output should contain chapter title. Got: {}",
            &typst_source[..200.min(typst_source.len())]
        );
    }

    /// Test that content_pipeline renders curriculum to Typst correctly.
    #[test]
    fn test_render_curriculum_to_typst() {
        let pipeline = content_pipeline::ContentPipeline::new();

        let curriculum_json = serde_json::json!({
            "title": "Python Basics",
            "description": "A beginner course in Python",
            "chapters": [
                {
                    "id": "ch1",
                    "title": "Getting Started",
                    "order": 1,
                    "objectives": ["Install Python", "Write first program"],
                    "estimated_minutes": 30,
                    "prerequisites": [],
                    "key_concepts": ["python", "installation"],
                    "exercises": []
                }
            ],
            "estimated_duration": "2 hours",
            "prerequisites_summary": [],
            "learning_objectives": ["Learn Python basics"]
        });

        let result = pipeline.render_curriculum_to_typst(&curriculum_json);
        assert!(
            result.is_ok(),
            "render_curriculum_to_typst should succeed: {:?}",
            result.err()
        );

        let typst_source = result.unwrap();
        assert!(!typst_source.is_empty());
        assert!(
            typst_source.contains("Python Basics"),
            "Typst output should contain curriculum title"
        );
    }

    /// Test the full export pipeline: render → write typst file → verify file exists.
    #[test]
    fn test_export_typst_file_written() {
        let pipeline = content_pipeline::ContentPipeline::new();

        let chapter_json = serde_json::json!({
            "title": "Test Chapter",
            "content": "# Test\n\nHello world.",
        });

        let typst_source = pipeline
            .render_chapter_to_typst(&chapter_json)
            .expect("render should succeed");

        let tmp_dir = tempfile::TempDir::new().expect("create temp dir");
        let file_path = tmp_dir.path().join("test_chapter.typst");

        std::fs::write(&file_path, &typst_source).expect("write should succeed");

        assert!(file_path.exists(), "Typst file should exist at {:?}", file_path);
        assert!(file_path.metadata().unwrap().len() > 0, "File should not be empty");

        let read_back = std::fs::read_to_string(&file_path).expect("read should succeed");
        assert_eq!(read_back, typst_source, "Round-trip should match");

        assert!(read_back.contains("Test Chapter"), "Should contain title");
    }

    /// Test that typst output is valid UTF-8 and contains expected structure markers.
    #[test]
    fn test_typst_output_structure() {
        let pipeline = content_pipeline::ContentPipeline::new();

        let chapter_json = serde_json::json!({
            "title": "Structured Chapter",
            "content": "# Heading 1\n\nContent para.\n\n## Heading 2\n\nMore content.\n\n```python\nprint('hello')\n```",
        });

        let typst_source = pipeline
            .render_chapter_to_typst(&chapter_json)
            .expect("render should succeed");

        assert!(
            typst_source.contains("Heading"),
            "Should contain heading text"
        );
        assert!(
            typst_source.lines().count() > 3,
            "Should have multiple lines of output, got {} lines:\n{}",
            typst_source.lines().count(),
            typst_source
        );
    }

    /// Test rendering and saving curriculum-level Typst output.
    #[test]
    fn test_curriculum_typst_file_written() {
        let pipeline = content_pipeline::ContentPipeline::new();

        let curriculum_json = serde_json::json!({
            "title": "Full Course",
            "description": "A complete course for testing",
            "chapters": [
                {
                    "id": "ch1",
                    "title": "Chapter One",
                    "order": 1,
                    "objectives": ["Goal 1"],
                    "estimated_minutes": 20,
                    "prerequisites": [],
                    "key_concepts": ["concept"],
                    "exercises": []
                },
            ],
            "estimated_duration": "45 minutes",
            "prerequisites_summary": [],
            "learning_objectives": ["Learn testing"]
        });

        let typst_source = pipeline
            .render_curriculum_to_typst(&curriculum_json)
            .expect("render should succeed");

        let tmp_dir = tempfile::TempDir::new().expect("create temp dir");
        let file_path = tmp_dir.path().join("full_course.typ");

        std::fs::write(&file_path, &typst_source).expect("write should succeed");

        assert!(file_path.exists());
        let read_back = std::fs::read_to_string(&file_path).expect("read should succeed");
        assert_eq!(read_back, typst_source);
        assert!(read_back.contains("Full Course"));
        assert!(read_back.contains("A complete course for testing"));
        assert!(read_back.contains("45 minutes"));
    }

    /// Test ExportResult serialization round-trip.
    #[test]
    fn test_export_result_serialization() {
        let result = ExportResult {
            path: "/tmp/test.typst".to_string(),
            checksum: "sha256:abcdef1234567890".to_string(),
            size_bytes: 1024,
            page_count: None,
            compiled: false,
            format: "typst".to_string(),
        };

        let json = serde_json::to_string(&result).expect("serialize should succeed");
        let parsed: ExportResult =
            serde_json::from_str(&json).expect("deserialize should succeed");

        assert_eq!(parsed.path, "/tmp/test.typst");
        assert_eq!(parsed.checksum, "sha256:abcdef1234567890");
        assert_eq!(parsed.size_bytes, 1024);
        assert!(!parsed.compiled);
        assert_eq!(parsed.format, "typst");
        assert_eq!(parsed.page_count, None);
    }
}
