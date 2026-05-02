use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::PathBuf;
use tauri::{command, AppHandle, Emitter, State};

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportResult {
    pub path: String,
    pub checksum: String,
    pub size_bytes: u64,
    pub page_count: Option<u32>,
    pub compiled: bool,
}

#[derive(Debug, Serialize)]
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

/// Fetch the curriculum JSON from agent-core.
async fn fetch_curriculum(
    agent_core_url: &str,
    session_id: &str,
) -> Result<serde_json::Value, ExportError> {
    let url = format!("{agent_core_url}/api/session/{session_id}/curriculum");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ExportError {
            code: "FETCH_FAILED".to_string(),
            message: format!(
                "Failed to connect to agent-core at {agent_core_url}: {e}"
            ),
        })?;

    if !resp.status().is_success() {
        return Err(ExportError {
            code: "API_ERROR".to_string(),
            message: format!(
                "agent-core returned {} for GET {url}",
                resp.status()
            ),
        });
    }

    resp.json().await.map_err(|e| ExportError {
        code: "PARSE_ERROR".to_string(),
        message: format!("Failed to parse curriculum JSON: {e}"),
    })
}

/// Extract a single chapter from the curriculum JSON by chapter_id.
fn find_chapter<'a>(
    curriculum: &'a serde_json::Value,
    chapter_id: &str,
) -> Option<&'a serde_json::Value> {
    curriculum
        .get("chapters")
        .and_then(|chapters| chapters.as_array())
        .and_then(|chapters| chapters.iter().find(|ch| ch.get("id").map_or(false, |id| id.as_str() == Some(chapter_id))))
}

/// Compile Typst source to PDF via sandbox, falling back to saving the .typst
/// source if Docker is not available.
fn compile_or_save_typst(
    typst_source: &str,
    save_path: &std::path::Path,
) -> Result<ExportResult, ExportError> {
    // Try sandbox compilation first
    let sandbox = sandbox_manager::SandboxManager::new(
        sandbox_manager::SandboxConfig::default(),
    );

    let compiled = if let Ok(result) =
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let compiler =
                    content_pipeline::export::TypstCompiler::new(sandbox);
                compiler
                    .compile_to_pdf(typst_source, &std::collections::HashMap::new())
                    .await
            })
        }) {
        // Write PDF bytes to the chosen path
        std::fs::write(save_path, &result.data).map_err(|e| ExportError {
            code: "WRITE_FAILED".to_string(),
            message: format!("Failed to write PDF: {e}"),
        })?;

        ExportResult {
            path: save_path.to_string_lossy().to_string(),
            checksum: result.checksum,
            size_bytes: result.size_bytes,
            page_count: result.page_count,
            compiled: true,
        }
    } else {
        // Sandbox unavailable — save .typst source instead
        let typst_path = save_path.with_extension("typst");
        std::fs::write(&typst_path, typst_source).map_err(|e| ExportError {
            code: "WRITE_FAILED".to_string(),
            message: format!("Failed to write Typst file: {e}"),
        })?;

        let size = typst_source.len() as u64;
        ExportResult {
            path: typst_path.to_string_lossy().to_string(),
            checksum: format!(
                "sha256:{:x}",
                sha2::Sha256::digest(typst_source.as_bytes())
            ),
            size_bytes: size,
            page_count: None,
            compiled: false,
        }
    };

    Ok(compiled)
}

#[command]
pub async fn export_chapter_pdf(
    app: AppHandle,
    session_id: String,
    chapter_id: String,
    state: State<'_, AppState>,
) -> Result<ExportResult, ExportError> {
    // 1. Fetch curriculum from agent-core
    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "fetching", "chapter": chapter_id }),
    );

    let curriculum = fetch_curriculum(&state.agent_core_url, &session_id).await?;

    // 2. Extract chapter
    let chapter = find_chapter(&curriculum, &chapter_id).ok_or_else(|| {
        ExportError {
            code: "CHAPTER_NOT_FOUND".to_string(),
            message: format!("Chapter {chapter_id} not found in curriculum"),
        }
    })?;

    // 3. Render to Typst
    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "rendering", "chapter": chapter_id }),
    );

    let typst_source = state
        .content_pipeline
        .render_chapter_to_typst(chapter)?;

    // 4. Open save dialog
    use tauri_plugin_dialog::DialogExt;
    let save_path = app
        .dialog()
        .file()
        .set_title("Save Chapter as PDF")
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

    // 5. Compile and save
    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "compiling", "chapter": chapter_id }),
    );

    let result = compile_or_save_typst(&typst_source, &save_path)?;

    let _ = app.emit(
        "export:complete",
        serde_json::json!({
            "chapter": chapter_id,
            "path": result.path,
            "compiled": result.compiled,
        }),
    );

    Ok(result)
}

#[command]
pub async fn export_curriculum_pdf(
    app: AppHandle,
    session_id: String,
    state: State<'_, AppState>,
) -> Result<ExportResult, ExportError> {
    // 1. Fetch curriculum from agent-core
    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "fetching" }),
    );

    let curriculum = fetch_curriculum(&state.agent_core_url, &session_id).await?;

    // 2. Render to Typst
    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "rendering" }),
    );

    let typst_source = state
        .content_pipeline
        .render_curriculum_to_typst(&curriculum)?;

    // 3. Open save dialog
    use tauri_plugin_dialog::DialogExt;
    let save_path = app
        .dialog()
        .file()
        .set_title("Save Curriculum as PDF")
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

    // 4. Compile and save
    let _ = app.emit(
        "export:progress",
        serde_json::json!({ "stage": "compiling" }),
    );

    let result = compile_or_save_typst(&typst_source, &save_path)?;

    let _ = app.emit(
        "export:complete",
        serde_json::json!({
            "session": session_id,
            "path": result.path,
            "compiled": result.compiled,
        }),
    );

    Ok(result)
}

#[command]
pub async fn export_typst(
    app: AppHandle,
    session_id: String,
    chapter_id: String,
    state: State<'_, AppState>,
) -> Result<String, ExportError> {
    // 1. Fetch curriculum from agent-core
    let curriculum = fetch_curriculum(&state.agent_core_url, &session_id).await?;

    // 2. Extract chapter
    let chapter = find_chapter(&curriculum, &chapter_id).ok_or_else(|| {
        ExportError {
            code: "CHAPTER_NOT_FOUND".to_string(),
            message: format!("Chapter {chapter_id} not found in curriculum"),
        }
    })?;

    // 3. Render to Typst
    let typst_source = state
        .content_pipeline
        .render_chapter_to_typst(chapter)?;

    // 4. Open save dialog
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

    // 5. Save Typst source
    std::fs::write(&save_path, &typst_source).map_err(|e| ExportError {
        code: "WRITE_FAILED".to_string(),
        message: format!("Failed to write Typst file: {e}"),
    })?;

    Ok(save_path.to_string_lossy().to_string())
}