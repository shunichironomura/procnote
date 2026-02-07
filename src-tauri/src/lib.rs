mod commands;
mod state;

use std::path::PathBuf;

use tauri::Manager;

use commands::execution::{get_execution_state, list_executions, record_action, start_execution};
use commands::template::{list_templates, load_template};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(procedures_dir: Option<PathBuf>, executions_dir: Option<PathBuf>) {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Debug)
                        .target(tauri_plugin_log::Target::new(
                            tauri_plugin_log::TargetKind::Stdout,
                        ))
                        .target(tauri_plugin_log::Target::new(
                            tauri_plugin_log::TargetKind::LogDir {
                                file_name: None,
                            },
                        ))
                        .target(tauri_plugin_log::Target::new(
                            tauri_plugin_log::TargetKind::Webview,
                        ))
                        .build(),
                )?;
            }

            // Use CLI-provided paths, or fall back to the app's resource directory.
            let default_base = app
                .path()
                .resource_dir()
                .expect("failed to resolve resource dir");

            let procedures_dir = procedures_dir.unwrap_or_else(|| default_base.join("procedures"));
            let executions_dir = executions_dir.unwrap_or_else(|| default_base.join(".executions"));

            // Ensure directories exist.
            let _ = std::fs::create_dir_all(&procedures_dir);
            let _ = std::fs::create_dir_all(&executions_dir);

            app.manage(AppState {
                procedures_dir,
                executions_dir,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_templates,
            load_template,
            start_execution,
            record_action,
            get_execution_state,
            list_executions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
