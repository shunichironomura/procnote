use std::path::PathBuf;

/// Application state managed by Tauri.
pub struct AppState {
    /// Root directory containing procedure subdirectories.
    /// Each procedure is a subdirectory with `template.md` and `.executions/`.
    pub procedures_dir: PathBuf,
}
