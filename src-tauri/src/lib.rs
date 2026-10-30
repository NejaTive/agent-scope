//! Agent Scope — read-only viewer for Claude Code configuration.

pub mod commands;
pub mod errors;
pub mod models;
pub mod parser;
pub mod paths;
pub mod resolver;
pub mod scanner;
pub mod state;
pub mod storage;
pub mod watcher;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let storage = storage::Storage::open(&dir.join("agent-scope.sqlite"))?;
            let state = AppState::new(paths::Paths::resolve(), storage);
            let watch_on_start = state
                .db
                .lock()
                .get_preferences()
                .map(|p| p.watch_on_start)
                .unwrap_or(false);
            app.manage(state.clone());
            if watch_on_start {
                let _ = state.start_watching(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_dashboard,
            commands::run_scan,
            commands::list_scan_roots,
            commands::add_scan_root,
            commands::remove_scan_root,
            commands::list_projects,
            commands::add_project,
            commands::remove_project,
            commands::pick_directory,
            commands::get_artifact_content,
            commands::get_effective_settings,
            commands::get_conflicts,
            commands::get_settings_diff,
            commands::get_recent_changes,
            commands::start_watching,
            commands::stop_watching,
            commands::search_artifacts,
            commands::get_preferences,
            commands::set_preferences,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
