// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use clap::Parser;

/// procnote - Procedure execution tool for hardware testing.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Directory containing procedure template (.md) files.
    #[arg(long)]
    procedures_dir: Option<PathBuf>,

    /// Directory for storing execution data (event logs, snapshots).
    #[arg(long)]
    executions_dir: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    procnote_tauri_lib::run(args.procedures_dir, args.executions_dir);
}
