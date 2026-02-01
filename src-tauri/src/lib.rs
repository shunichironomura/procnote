mod commands;
mod state;

use std::collections::HashMap;
use std::sync::Mutex;

use tauri::Manager;

use commands::execution::{get_execution_state, list_executions, record_action, start_execution};
use commands::template::{list_templates, load_template};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Resolve data directories relative to the app's resource dir.
            let resource_dir = app
                .path()
                .resource_dir()
                .expect("failed to resolve resource dir");
            let procedures_dir = resource_dir.join("procedures");
            let executions_dir = resource_dir.join(".executions");

            // Ensure directories exist.
            let _ = std::fs::create_dir_all(&procedures_dir);
            let _ = std::fs::create_dir_all(&executions_dir);

            app.manage(AppState {
                procedures_dir,
                executions_dir,
                executions: Mutex::new(HashMap::new()),
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
