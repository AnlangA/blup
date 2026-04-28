# Apps Module — Phase 2.5: Desktop Application (Tauri)

## Module Overview

`apps/desktop` is the Tauri-based desktop shell that wraps the Web UI with native capabilities: local file access for imports, Typst/PDF export, local database access, and platform integration. The Web UI from Phase 1 runs inside a Tauri webview; the Rust backend (agent-core) may run as a sidecar process or be embedded in the Tauri Rust process.

## Phase 2.5 Scope

| Deliverable | Description | Status |
|-------------|-------------|--------|
| Tauri shell | Wrap Web UI in Tauri 2 with native window chrome | Planned |
| Tauri commands | Expose import, export, and file operations as Tauri commands | Planned |
| Local storage | SQLite database for session persistence (via `crates/storage`) | Planned |
| File import | Open PDF, Markdown, text files via native file dialog | Planned |
| Export | Save Typst/PDF exports via native save dialog | Planned |
| Auto-update | Tauri updater for app distribution | Planned |

## Architecture

```
┌──────────────────────────────────────────────┐
│                Tauri Desktop Shell             │
│  ┌────────────────────────────────────────┐  │
│  │         Tauri Webview                    │  │
│  │  ┌──────────────────────────────────┐   │  │
│  │  │     Web UI (apps/web-ui)          │   │  │
│  │  │     React/Svelte SPA              │   │  │
│  │  └──────────┬───────────────────────┘   │  │
│  │             │ invoke()                    │  │
│  └─────────────┼───────────────────────────┘  │
│                ▼                                │
│  ┌─────────────────────────────────────────┐  │
│  │        Tauri Rust Backend                │  │
│  │  ┌───────────┐  ┌────────────────────┐  │  │
│  │  │  Tauri     │  │  Agent Core         │  │  │
│  │  │  Commands  │  │  (embedded or       │  │  │
│  │  │  - import  │  │   sidecar)          │  │  │
│  │  │  - export  │  │                     │  │  │
│  │  │  - file_   │  │  HTTP API still     │  │  │
│  │  │   access   │  │  available on       │  │  │
│  │  │            │  │  localhost for      │  │  │
│  │  │            │  │  webview            │  │  │
│  │  └─────┬──────┘  └────────┬───────────┘  │  │
│  │        │                  │               │  │
│  │  ┌─────▼──────────────────▼───────────┐  │  │
│  │  │        crates/                      │  │  │
│  │  │  storage | content-pipeline          │  │  │
│  │  └─────────────────────────────────────┘  │  │
│  └─────────────────────────────────────────┘  │
└──────────────────────────────────────────────┘
```

## File Structure

```
apps/desktop/
├── Cargo.toml                      # Rust dependencies
├── tauri.conf.json                 # Tauri configuration
├── capabilities/
│   └── default.json                # Tauri capability permissions
├── icons/                          # App icons (all platforms)
│   ├── icon.ico
│   ├── icon.icns
│   ├── icon.png
│   ├── 32x32.png
│   ├── 128x128.png
│   └── 128x128@2x.png
├── src/
│   ├── main.rs                     # Tauri entry point
│   ├── lib.rs                      # Tauri plugin registration
│   └── commands/
│       ├── mod.rs
│       ├── import.rs               # File import commands
│       ├── export.rs               # File export commands
│       └── filesystem.rs           # Safe file access commands
├── src-tauri/                      # Alternative Tauri convention
│   └── ... (same as above)
└── tests/
    ├── import_flow_test.rs
    └── export_flow_test.rs
```

## Tauri Configuration

```json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-cli/schema.json",
  "productName": "Blup",
  "version": "0.2.0",
  "identifier": "dev.blup.desktop",
  "build": {
    "frontendDist": "../web-ui/dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "title": "Blup Learning Agent",
    "windows": [
      {
        "title": "Blup",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; connect-src 'self' http://localhost:3000; style-src 'self' 'unsafe-inline'; script-src 'self'; img-src 'self' data:; font-src 'self'"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "resources": ["../prompts/*.md"],
    "publisher": "Blup Project",
    "category": "Education",
    "shortDescription": "AI interactive learning agent",
    "longDescription": "Blup is an AI interactive learning-agent platform that creates personalized curricula and teaches through structured dialogue."
  },
  "plugins": {
    "updater": {
      "endpoints": ["https://releases.blup.dev/{{target}}/{{current_version}}"],
      "pubkey": "<update-public-key>"
    },
    "sql": {
      "preload": {
        "db": "sqlite:blup.db"
      }
    }
  }
}
```

## Tauri Commands

### Import Commands

```rust
// commands/import.rs (conceptual)
use tauri::command;

#[command]
async fn import_file(
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<ImportResult, ImportError> {
    // 1. Open native file dialog (filter: PDF, Markdown, text)
    let file_path = tauri::api::dialog::blocking::FileDialogBuilder::new()
        .add_filter("Documents", &["pdf", "md", "txt", "markdown"])
        .pick_file();

    // 2. Read file through Tauri's safe file access
    // 3. Hash content for deduplication
    // 4. Send to content-pipeline for extraction
    // 5. Store SourceDocument in storage
    // 6. Return ImportResult with metadata
}

#[command]
async fn import_website(
    url: String,
    state: tauri::State<'_, AppState>,
) -> Result<ImportResult, ImportError> {
    // 1. Validate URL
    // 2. Fetch through controlled fetch/extract pipeline
    // 3. Record URL, title, access time, content hash
    // 4. Return structured SourceDocument
}
```

### Export Commands (Full Implementation)

```rust
// commands/export.rs
use tauri::{command, State, AppHandle, Manager};
use tauri::api::dialog::FileDialogBuilder;
use std::path::PathBuf;

#[command]
async fn export_chapter_pdf(
    app: AppHandle,
    chapter_id: String,
    state: State<'_, AppState>,
) -> Result<ExportResult, ExportError> {
    // 1. Load chapter from storage
    let chapter = state.storage.get_chapter(&chapter_id).await
        .map_err(|e| ExportError::ChapterNotFound(chapter_id.clone()))?
        .ok_or(ExportError::ChapterNotFound(chapter_id.clone()))?;

    // 2. Open native save dialog
    let default_name = format!("{}.pdf", slugify(&chapter.title));
    let save_path: Option<PathBuf> = FileDialogBuilder::new()
        .set_title("Save Chapter as PDF")
        .set_file_name(&default_name)
        .add_filter("PDF Files", &["pdf"])
        .save_file();

    let save_path = match save_path {
        Some(path) => path,
        None => return Err(ExportError::UserCancelled),
    };

    // 3. Render chapter to Typst
    let typst_source = state.content_pipeline
        .render_chapter_to_typst(&chapter)
        .await?;

    // 4. Compile via sandbox
    let pdf = state.sandbox_manager
        .compile_typst(&typst_source, &chapter.assets)
        .await?;

    // 5. Write to chosen path
    tokio::fs::write(&save_path, &pdf.data).await?;

    // 6. Notify frontend
    app.emit_all("export:complete", ExportEvent {
        chapter_id: chapter.id.clone(),
        path: save_path.to_string_lossy().to_string(),
        size_bytes: pdf.size_bytes,
    })?;

    Ok(ExportResult {
        path: save_path,
        checksum: pdf.checksum,
        size_bytes: pdf.size_bytes,
        page_count: pdf.page_count,
    })
}

#[command]
async fn export_curriculum_pdf(
    app: AppHandle,
    session_id: Uuid,
    state: State<'_, AppState>,
) -> Result<ExportResult, ExportError> {
    // 1. Load full curriculum with all chapters
    let curriculum = state.storage.get_curriculum_with_chapters(&session_id).await?;

    // 2. Open save dialog
    let default_name = format!("{}-curriculum.pdf", slugify(&curriculum.title));
    let save_path: Option<PathBuf> = FileDialogBuilder::new()
        .set_title("Save Curriculum as PDF")
        .set_file_name(&default_name)
        .add_filter("PDF Files", &["pdf"])
        .save_file();

    let save_path = match save_path {
        Some(path) => path,
        None => return Err(ExportError::UserCancelled),
    };

    // 3. Show progress in UI (multi-chapter export can be slow)
    let total = curriculum.chapters.len() as u32;
    for (i, chapter) in curriculum.chapters.iter().enumerate() {
        app.emit_all("export:progress", ExportProgress {
            current: i as u32 + 1,
            total,
            chapter: chapter.title.clone(),
        })?;
    }

    // 4. Render + compile full curriculum
    let typst_source = state.content_pipeline
        .render_curriculum_to_typst(&curriculum)
        .await?;
    let pdf = state.sandbox_manager
        .compile_typst(&typst_source, &curriculum.assets)
        .await?;

    tokio::fs::write(&save_path, &pdf.data).await?;

    app.emit_all("export:complete", ExportEvent {
        chapter_id: "curriculum".into(),
        path: save_path.to_string_lossy().to_string(),
        size_bytes: pdf.size_bytes,
    })?;

    Ok(ExportResult {
        path: save_path,
        checksum: pdf.checksum,
        size_bytes: pdf.size_bytes,
        page_count: pdf.page_count,
    })
}

fn slugify(title: &str) -> String {
    title.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
```

### Import Commands (Full Implementation)

```rust
// commands/import.rs
#[command]
async fn import_file(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ImportResult, ImportError> {
    // 1. Open native file dialog with filter
    let file_path: Option<PathBuf> = FileDialogBuilder::new()
        .set_title("Import Learning Material")
        .add_filter("Documents", &["pdf", "md", "txt", "markdown"])
        .add_filter("PDF Files", &["pdf"])
        .add_filter("Markdown Files", &["md", "markdown"])
        .add_filter("Text Files", &["txt"])
        .pick_file();

    let file_path = match file_path {
        Some(path) => path,
        None => return Err(ImportError::UserCancelled),
    };

    // 2. Validate file size (reject > 50MB)
    let metadata = tokio::fs::metadata(&file_path).await?;
    if metadata.len() > 50 * 1024 * 1024 {
        return Err(ImportError::FileTooLarge {
            path: file_path,
            size_bytes: metadata.len(),
            max_allowed: 50 * 1024 * 1024,
        });
    }

    // 3. Emit progress
    app.emit_all("import:progress", ImportProgress {
        stage: "extracting".into(),
        path: file_path.to_string_lossy().to_string(),
    })?;

    // 4. Import via content pipeline
    let source_doc = state.content_pipeline
        .import_file(&file_path)
        .await?;

    // 5. Store in database
    let doc_id = state.storage.save_source_document(&source_doc).await?;

    // 6. Emit completion
    app.emit_all("import:complete", ImportEvent {
        doc_id: doc_id.to_string(),
        title: source_doc.title.clone(),
        chunks: source_doc.chunks.len() as u32,
        word_count: source_doc.metadata.word_count,
    })?;

    Ok(ImportResult {
        doc_id,
        title: source_doc.title,
        source_type: source_doc.source_type,
        checksum: source_doc.checksum,
        chunks: source_doc.chunks.len() as u32,
        word_count: source_doc.metadata.word_count,
        language: source_doc.language,
    })
}

#[command]
async fn import_website(
    app: AppHandle,
    url: String,
    state: State<'_, AppState>,
) -> Result<ImportResult, ImportError> {
    // 1. Validate URL format
    let parsed = url::Url::parse(&url)
        .map_err(|_| ImportError::InvalidUrl(url.clone()))?;

    // 2. Security: reject internal URLs
    if is_private_host(parsed.host_str().unwrap_or("")) {
        return Err(ImportError::UrlBlocked {
            url,
            reason: "Cannot import from internal/private URLs".into(),
        });
    }

    app.emit_all("import:progress", ImportProgress {
        stage: "fetching".into(),
        path: url.clone(),
    })?;

    // 3. Fetch via sandboxed import pipeline
    let source_doc = state.content_pipeline
        .import_website(&url)
        .await?;

    let doc_id = state.storage.save_source_document(&source_doc).await?;

    app.emit_all("import:complete", ImportEvent {
        doc_id: doc_id.to_string(),
        title: source_doc.title.clone(),
        chunks: source_doc.chunks.len() as u32,
        word_count: source_doc.metadata.word_count,
    })?;

    Ok(ImportResult {
        doc_id,
        title: source_doc.title,
        source_type: SourceType::Website,
        checksum: source_doc.checksum,
        chunks: source_doc.chunks.len() as u32,
        word_count: source_doc.metadata.word_count,
        language: source_doc.language,
    })
}
```

### Window Management and Menu

```rust
// commands/window.rs
use tauri::{Window, WindowMenuEvent, Manager};

#[command]
async fn toggle_sidebar(window: Window) -> Result<(), String> {
    // Toggle sidebar visibility via JavaScript eval
    window.eval("window.__BLUP_TOGGLE_SIDEBAR__()")
        .map_err(|e| e.to_string())
}

fn build_app_menu() -> tauri::menu::Menu {
    use tauri::menu::{MenuBuilder, SubmenuBuilder, MenuItemBuilder};

    let file_menu = SubmenuBuilder::new("File")
        .item(&MenuItemBuilder::with_id("import_file", "Import File...").build())
        .item(&MenuItemBuilder::with_id("import_url", "Import from URL...").build())
        .separator()
        .item(&MenuItemBuilder::with_id("export_chapter", "Export Chapter as PDF...").build())
        .item(&MenuItemBuilder::with_id("export_curriculum", "Export Curriculum as PDF...").build())
        .build();

    let edit_menu = SubmenuBuilder::new("Edit")
        .item(&MenuItemBuilder::with_id("undo", "Undo").build())
        .item(&MenuItemBuilder::with_id("redo", "Redo").build())
        .build();

    let help_menu = SubmenuBuilder::new("Help")
        .item(&MenuItemBuilder::with_id("about", "About Blup").build())
        .build();

    MenuBuilder::new()
        .item(&file_menu)
        .item(&edit_menu)
        .item(&help_menu)
        .build()
}

fn handle_menu_event(app: &AppHandle, event: WindowMenuEvent) {
    match event.id().as_ref() {
        "import_file" => { let _ = import_file_dialog(app.clone()); }
        "import_url" => { /* open URL input dialog */ }
        "export_chapter" => { /* trigger export for current chapter */ }
        "export_curriculum" => { /* trigger export for full curriculum */ }
        "about" => { /* show about dialog */ }
        _ => {}
    }
}
```

### Auto-Update

```rust
// commands/update.rs
use tauri_plugin_updater::UpdaterExt;

async fn check_for_updates(app: AppHandle) -> Result<Option<UpdateInfo>, Box<dyn Error>> {
    let updater = app.updater()?;
    let update = updater.check().await?;

    if let Some(update) = update {
        let info = UpdateInfo {
            version: update.version.clone(),
            body: update.body.clone().unwrap_or_default(),
            date: update.date.clone(),
            download_size: update.content_length,
        };

        // Notify frontend
        app.emit("update:available", &info)?;

        Ok(Some(info))
    } else {
        Ok(None)
    }
}

#[command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;

    if let Some(update) = update {
        app.emit("update:installing", ()).map_err(|e| e.to_string())?;
        update.download_and_install().await.map_err(|e| e.to_string())?;
        // App will restart automatically after install
    }

    Ok(())
}
```

### Error Types for Desktop

```rust
// error.rs
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
    #[error("File too large: {size_bytes} bytes (max {max_allowed})")]
    FileTooLarge { path: PathBuf, size_bytes: u64, max_allowed: u64 },
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("URL blocked: {url} — {reason}")]
    UrlBlocked { url: String, reason: String },
    #[error("Import pipeline error: {0}")]
    PipelineError(String),
    #[error("User cancelled")]
    UserCancelled,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Chapter not found: {0}")]
    ChapterNotFound(String),
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
    #[error("User cancelled")]
    UserCancelled,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Capability Permissions

Tauri 2 uses a capability-based permission system:

```json
// capabilities/default.json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-utils/schema/capability.json",
  "identifier": "default",
  "description": "Default capabilities for Blup desktop",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "dialog:allow-open",
    "dialog:allow-save",
    "fs:default",
    "fs:allow-read",
    "fs:allow-write",
    "path:default",
    "updater:default"
  ]
}
```

**Important:** The `fs` permission is scoped to user-selected files and the app data directory. Never grant broad filesystem access.

## Agent Core Integration

Two options for running the Rust backend in the desktop app:

### Option A: Embedded (Recommended for Phase 2.5)

Agent core runs as a library within the Tauri Rust process:

```rust
// main.rs (conceptual)
use tauri::Manager;

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Start agent-core HTTP server on localhost:random_port
            let port = find_available_port();
            let agent_core = agent_core::start_server(port).await?;

            // Store port in app state so webview knows where to connect
            app.manage(AppState {
                agent_core_port: port,
                agent_core_handle: agent_core,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            import_file,
            import_website,
            export_chapter_pdf,
            export_curriculum_pdf,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Pros:** Single binary, no sidecar management.
**Cons:** Tauri and agent-core share a Tokio runtime (compatibility check needed).

### Option B: Sidecar Process

Agent core runs as a separate process managed by Tauri:

```json
// tauri.conf.json excerpt
{
  "bundle": {
    "externalBin": ["binaries/agent-core"]
  }
}
```

**Pros:** Process isolation, crash independence.
**Cons:** Two processes to manage, inter-process communication overhead.

**Recommendation:** Start with Option A (embedded) for simplicity. Fall back to Option B only if the shared Tokio runtime causes issues.

## Security Considerations

| Concern | Mitigation |
|---------|------------|
| File system access | Tauri capability system — only allow user-selected files and app data dir |
| CSP bypass in webview | Strict CSP in tauri.conf.json; no `unsafe-eval` |
| IPC spoofing | Tauri's invoke system is type-safe; validate all command inputs |
| SQL injection | Use parameterized queries through SQLx (not string formatting) |
| Update tampering | Signed updates with public key pinning in tauri.conf.json |
| Local data leakage | SQLite database in app data dir; no cloud sync unless explicitly enabled |

## Testing Strategy

| Test Category | Method | Scope |
|---------------|--------|-------|
| Command tests | Rust unit tests | Each Tauri command |
| Import flow | Integration test | PDF → SourceDocument → storage |
| Export flow | Integration test | Chapter → Typst → PDF output |
| File dialog | Manual test (can't easily automate native dialogs) | File open/save UX |
| Update flow | Tauri updater test | Check for updates, download, verify signature |
| CSP | Security test | Verify webview doesn't make unauthorized requests |

## Quality Gates

- [ ] Desktop app builds for macOS, Windows, and Linux
- [ ] File import works (PDF, Markdown, text)
- [ ] File export works (PDF via Typst)
- [ ] Web UI loads correctly in Tauri webview
- [ ] Agent-core API is reachable from webview
- [ ] Tauri commands are properly permissioned
- [ ] CSP blocks unauthorized requests
- [ ] Auto-update is configured and tested
- [ ] App icons present for all platforms
- [ ] No un-scoped filesystem access in capability config
