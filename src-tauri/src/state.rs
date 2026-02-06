use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use procnote_core::event::types::ExecutionId;
use procnote_core::execution::ExecutionState;

/// Application state managed by Tauri.
pub struct AppState {
    /// Directory containing procedure template `.md` files.
    pub procedures_dir: PathBuf,
    /// Base directory for execution records.
    pub executions_dir: PathBuf,
    /// Active executions keyed by execution ID.
    pub executions: Mutex<HashMap<ExecutionId, ActiveExecution>>,
}

/// An active execution with its state and log path.
pub struct ActiveExecution {
    pub state: ExecutionState,
    pub log_path: PathBuf,
}
