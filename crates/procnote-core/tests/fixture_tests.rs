//! Snapshot fixture tests for backward/forward compatibility of the event log format.
//!
//! These tests deserialize committed JSONL fixture files and assert the resulting
//! `ExecutionState` matches expectations. If a field is renamed, a type changes,
//! or a variant is removed, these tests break immediately.

use std::path::Path;

use procnote_core::event::read_log;
use procnote_core::event::types::LogEntry;
use procnote_core::execution::{ExecutionState, ExecutionStatus, StepStatus};

fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn v1_basic_execution_parses_to_finished_pass() {
    let entries = read_log(&fixture_path("v1_basic_execution.jsonl")).unwrap();
    let state = ExecutionState::from_log_entries(&entries).unwrap();

    assert!(matches!(
        state.status,
        ExecutionStatus::Finished(procnote_core::event::types::CompletionStatus::Pass)
    ));
    assert_eq!(state.step_order.len(), 3);
    assert_eq!(
        state.steps["step-0"].status,
        StepStatus::Completed
    );
    assert_eq!(
        state.steps["step-1"].status,
        StepStatus::Completed
    );
    assert_eq!(
        state.steps["step-2"].status,
        StepStatus::Completed
    );
    // Verify input was recorded
    assert_eq!(
        state.steps["step-1"].inputs["step-1/temp"].value,
        "-39.5"
    );
}

#[test]
fn v1_with_reverts_applies_revert_correctly() {
    let entries = read_log(&fixture_path("v1_with_reverts.jsonl")).unwrap();
    let state = ExecutionState::from_log_entries(&entries).unwrap();

    assert!(matches!(
        state.status,
        ExecutionStatus::Finished(procnote_core::event::types::CompletionStatus::Pass)
    ));
    // The reverted input (999) should not appear; the corrected value (-39.5) should.
    assert_eq!(
        state.steps["step-1"].inputs["step-1/temp"].value,
        "-39.5"
    );
}

#[test]
fn v1_all_event_types_parses_successfully() {
    let entries = read_log(&fixture_path("v1_all_event_types.jsonl")).unwrap();
    let state = ExecutionState::from_log_entries(&entries).unwrap();

    assert!(matches!(
        state.status,
        ExecutionStatus::Finished(procnote_core::event::types::CompletionStatus::Pass)
    ));
    assert_eq!(state.name.as_deref(), Some("Morning run"));
    assert_eq!(state.step_order.len(), 2);
    // step-1 was skipped then reverted, so it ended up completed
    assert_eq!(
        state.steps["step-1"].status,
        StepStatus::Completed
    );
    // Global note was recorded
    assert_eq!(state.global_notes.len(), 1);
    assert_eq!(state.global_notes[0], "Global observation");
    // Step note was recorded
    assert_eq!(state.steps["step-0"].notes.len(), 1);
}

#[test]
fn v1_with_unknown_events_skips_unknown_preserves_state() {
    let entries = read_log(&fixture_path("v1_with_unknown_events.jsonl")).unwrap();

    // Unknown events should be preserved as LogEntry::Unknown.
    let unknown_count = entries
        .iter()
        .filter(|e| matches!(e, LogEntry::Unknown(_)))
        .count();
    assert_eq!(unknown_count, 2, "expected 2 unknown events");

    // State should still reconstruct correctly from known events.
    let state = ExecutionState::from_log_entries(&entries).unwrap();
    assert!(matches!(
        state.status,
        ExecutionStatus::Finished(procnote_core::event::types::CompletionStatus::Pass)
    ));
    assert_eq!(state.step_order.len(), 1);
}

#[test]
fn v1_unknown_events_maintain_correct_indices() {
    let entries = read_log(&fixture_path("v1_with_unknown_events.jsonl")).unwrap();

    // Total entries should include unknowns (they occupy index positions).
    assert_eq!(entries.len(), 8);

    // The unknown event at index 3 should be preserved.
    assert!(matches!(&entries[3], LogEntry::Unknown(v) if v["type"] == "future_event_v2"));
    // The unknown event at index 6 should be preserved.
    assert!(matches!(&entries[6], LogEntry::Unknown(v) if v["type"] == "another_future_event"));
}
