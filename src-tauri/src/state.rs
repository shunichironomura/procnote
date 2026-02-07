use std::path::PathBuf;

/// Application state managed by Tauri.
pub struct AppState {
    /// Directory containing procedure template `.md` files.
    pub procedures_dir: PathBuf,
    /// Base directory for execution records.
    pub executions_dir: PathBuf,
}
