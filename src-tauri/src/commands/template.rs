use serde::Serialize;
use tauri::State;

use crate::state::AppState;
use procnote_core::template::{ProcedureTemplate, parse_template};

/// Summary of a template for listing.
#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export)]
pub struct TemplateSummary {
    pub id: String,
    pub title: String,
    pub version: String,
    pub path: String,
}

/// List all procedure templates found in the procedures directory.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri command handlers require owned parameters"
)]
pub fn list_templates(state: State<'_, AppState>) -> Result<Vec<TemplateSummary>, String> {
    let dir = &state.procedures_dir;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let source = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            match parse_template(&source) {
                Ok(template) => {
                    summaries.push(TemplateSummary {
                        id: template.metadata.id,
                        title: template.metadata.title,
                        version: template.metadata.version,
                        path: path.to_string_lossy().to_string(),
                    });
                }
                Err(e) => {
                    log::warn!("Skipping invalid template {}: {e}", path.display());
                }
            }
        }
    }

    Ok(summaries)
}

/// Load and parse a specific procedure template.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri command handlers require owned parameters"
)]
pub fn load_template(path: String) -> Result<ProcedureTemplate, String> {
    let source = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    parse_template(&source).map_err(|e| e.to_string())
}
