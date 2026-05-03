#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tracing_subscriber::EnvFilter;

use blup_desktop::AppState;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let state = AppState::new();
            app.manage(state);
            tracing::info!("Blup desktop started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            blup_desktop::commands::import::import_file,
            blup_desktop::commands::import::import_website,
            blup_desktop::commands::export::export_chapter_pdf,
            blup_desktop::commands::export::export_curriculum_pdf,
            blup_desktop::commands::export::export_typst,
            blup_desktop::commands::export::export_curriculum_typst,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
