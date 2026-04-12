use std::path::PathBuf;

use clap::Parser;

/// procnote - Procedure execution tool for hardware testing.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Workspace directory containing procedure subdirectories.
    /// Defaults to the current working directory.
    #[arg(default_value = ".")]
    workspace: PathBuf,
}

fn main() {
    let args = Args::parse();
    procnote_tauri_lib::run(&args.workspace);
}
