use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;

use crate::state::AppState;
use procnote_core::event::types::{CompletionStatus, Event, ExecutionId, Revertibility};
use procnote_core::event::{append_event, read_events};
use procnote_core::execution::{ExecutionState, StepStatus};
use procnote_core::template::parse_template;
use procnote_core::template::types::InputDefinition;

/// Serializable execution state summary for the frontend.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ExecutionSummary {
    pub execution_id: ExecutionId,
    #[ts(optional)]
    pub name: Option<String>,
    pub procedure_id: String,
    pub procedure_title: String,
    pub procedure_version: String,
    pub status: String,
    /// ISO 8601 timestamp of when the execution was started.
    #[ts(optional)]
    pub started_at: Option<String>,
    /// ISO 8601 timestamp of when the execution was finished (completed/aborted).
    #[ts(optional)]
    pub finished_at: Option<String>,
    pub steps: Vec<StepSummary>,
    pub event_history: Vec<EventHistoryEntry>,
}

/// A single entry in the event history, exposed to the frontend.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct EventHistoryEntry {
    pub index: usize,
    pub event_type: String,
    /// ISO 8601 timestamp string.
    pub at: String,
    pub description: String,
    pub revertible: bool,
    pub reverted: bool,
    /// Step heading for step-scoped events, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub step_heading: Option<String>,
    /// Label for input/attachment events, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub label: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct StepSummary {
    pub heading: String,
    #[ts(optional)]
    pub description: Option<String>,
    pub status: String,
    /// ISO 8601 timestamp of the most recent status change (started/completed/skipped).
    #[ts(optional)]
    pub status_at: Option<String>,
    pub checkboxes: Vec<CheckboxState>,
    pub input_definitions: Vec<InputDefinition>,
    pub inputs: Vec<InputState>,
    pub notes: Vec<NoteState>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CheckboxState {
    pub text: String,
    pub checked: bool,
    /// ISO 8601 timestamp of the last toggle, if any.
    #[ts(optional)]
    pub at: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct InputState {
    pub label: String,
    pub value: String,
    #[ts(optional)]
    pub unit: Option<String>,
    /// ISO 8601 timestamp of when the input was recorded.
    #[ts(optional)]
    pub at: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct NoteState {
    pub text: String,
    /// ISO 8601 timestamp of when the note was added.
    #[ts(optional)]
    pub at: Option<String>,
}

fn status_string(status: &procnote_core::execution::ExecutionStatus) -> String {
    match status {
        procnote_core::execution::ExecutionStatus::Pending => "pending".to_string(),
        procnote_core::execution::ExecutionStatus::Active => "active".to_string(),
        procnote_core::execution::ExecutionStatus::Finished(s) => match s {
            CompletionStatus::Pass => "pass".to_string(),
            CompletionStatus::Fail => "fail".to_string(),
            CompletionStatus::Aborted => "aborted".to_string(),
        },
    }
}

fn step_status_string(status: &StepStatus) -> String {
    match status {
        StepStatus::Pending => "pending".to_string(),
        StepStatus::Active => "active".to_string(),
        StepStatus::Completed => "completed".to_string(),
        StepStatus::Skipped => "skipped".to_string(),
    }
}

fn summarize(state: &ExecutionState, events: Option<&[Event]>) -> ExecutionSummary {
    use std::collections::{HashMap, HashSet};

    let events_slice = events.unwrap_or(&[]);

    // Collect reverted event indices so we can skip them.
    let reverted_indices: HashSet<usize> = events_slice
        .iter()
        .filter_map(|e| match e {
            Event::EventReverted {
                reverted_event_index,
                ..
            } => Some(*reverted_event_index),
            _ => None,
        })
        .collect();

    // Build timestamp lookup maps from non-reverted events.
    // Store as RFC3339 strings to avoid depending on chrono in this crate.
    let mut started_at: Option<String> = None;
    let mut finished_at: Option<String> = None;
    // step_heading -> most recent status-change timestamp
    let mut step_status_at: HashMap<&str, String> = HashMap::new();
    // (step_heading, checkbox_text) -> most recent toggle timestamp
    let mut checkbox_at: HashMap<(&str, &str), String> = HashMap::new();
    // (step_heading, input_label) -> most recent record timestamp
    let mut input_at: HashMap<(&str, &str), String> = HashMap::new();
    // (step_heading, note_index_in_step) -> add timestamp
    // We count notes per step to match the index in StepState.notes.
    let mut note_at: HashMap<(&str, usize), String> = HashMap::new();
    let mut note_counts: HashMap<&str, usize> = HashMap::new();

    for (index, event) in events_slice.iter().enumerate() {
        if reverted_indices.contains(&index) {
            continue;
        }
        match event {
            Event::ExecutionStarted { at, .. } => {
                started_at = Some(at.to_rfc3339());
            }
            Event::ExecutionCompleted { at, .. } | Event::ExecutionAborted { at, .. } => {
                finished_at = Some(at.to_rfc3339());
            }
            Event::StepStarted {
                at, step_heading, ..
            }
            | Event::StepCompleted {
                at, step_heading, ..
            }
            | Event::StepSkipped {
                at, step_heading, ..
            } => {
                step_status_at.insert(step_heading, at.to_rfc3339());
            }
            Event::CheckboxToggled {
                at,
                step_heading,
                text,
                ..
            } => {
                checkbox_at.insert((step_heading, text), at.to_rfc3339());
            }
            Event::InputRecorded {
                at,
                step_heading,
                label,
                ..
            } => {
                input_at.insert((step_heading, label), at.to_rfc3339());
            }
            Event::AttachmentAdded {
                at,
                step_heading,
                label,
                ..
            } => {
                input_at.insert((step_heading, label), at.to_rfc3339());
            }
            Event::NoteAdded {
                at,
                step_heading: Some(heading),
                ..
            } => {
                let count = note_counts.entry(heading).or_insert(0);
                note_at.insert((heading, *count), at.to_rfc3339());
                *count += 1;
            }
            _ => {}
        }
    }

    let steps = state
        .step_order
        .iter()
        .filter_map(|heading| {
            state.steps.get(heading).map(|step| {
                let checkboxes = step
                    .checkboxes
                    .iter()
                    .map(|(text, checked)| CheckboxState {
                        text: text.clone(),
                        checked: *checked,
                        at: checkbox_at.get(&(heading.as_str(), text.as_str())).cloned(),
                    })
                    .collect();
                let inputs = step
                    .inputs
                    .values()
                    .map(|input| InputState {
                        label: input.label.clone(),
                        value: input.value.clone(),
                        unit: input.unit.clone(),
                        at: input_at
                            .get(&(heading.as_str(), input.label.as_str()))
                            .cloned(),
                    })
                    .collect();
                let notes = step
                    .notes
                    .iter()
                    .enumerate()
                    .map(|(i, text)| NoteState {
                        text: text.clone(),
                        at: note_at.get(&(heading.as_str(), i)).cloned(),
                    })
                    .collect();
                StepSummary {
                    heading: step.heading.clone(),
                    description: step.description.clone(),
                    status: step_status_string(&step.status),
                    status_at: step_status_at.get(heading.as_str()).cloned(),
                    checkboxes,
                    input_definitions: step.input_definitions.clone(),
                    inputs,
                    notes,
                }
            })
        })
        .collect();

    let event_history = build_event_history(events_slice);

    ExecutionSummary {
        execution_id: state.execution_id.unwrap_or_default(),
        name: state.name.clone(),
        procedure_id: state.procedure_id.clone().unwrap_or_default(),
        procedure_title: state.procedure_title.clone().unwrap_or_default(),
        procedure_version: state.procedure_version.clone().unwrap_or_default(),
        status: status_string(&state.status),
        started_at,
        finished_at,
        steps,
        event_history,
    }
}

fn build_event_history(events: &[Event]) -> Vec<EventHistoryEntry> {
    use std::collections::HashSet;

    // Collect reverted indices.
    let reverted_indices: HashSet<usize> = events
        .iter()
        .filter_map(|e| match e {
            Event::EventReverted {
                reverted_event_index,
                ..
            } => Some(*reverted_event_index),
            _ => None,
        })
        .collect();

    events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let revertible = event.revertibility() == Revertibility::Revertible
                && !reverted_indices.contains(&index);
            let (step_heading, label) = event_step_and_label(event);
            EventHistoryEntry {
                index,
                event_type: event_type_string(event),
                at: event_at(event),
                description: event.description(),
                revertible,
                reverted: reverted_indices.contains(&index),
                step_heading,
                label,
            }
        })
        .collect()
}

/// Extract optional step_heading and label from an event.
fn event_step_and_label(event: &Event) -> (Option<String>, Option<String>) {
    match event {
        Event::StepStarted { step_heading, .. }
        | Event::StepCompleted { step_heading, .. }
        | Event::StepSkipped { step_heading, .. } => (Some(step_heading.clone()), None),
        Event::CheckboxToggled { step_heading, .. } => (Some(step_heading.clone()), None),
        Event::InputRecorded {
            step_heading,
            label,
            ..
        } => (Some(step_heading.clone()), Some(label.clone())),
        Event::AttachmentAdded {
            step_heading,
            label,
            ..
        } => (Some(step_heading.clone()), Some(label.clone())),
        Event::NoteAdded { step_heading, .. } => (step_heading.clone(), None),
        _ => (None, None),
    }
}

fn event_type_string(event: &Event) -> String {
    match event {
        Event::ExecutionStarted { .. } => "execution_started",
        Event::ExecutionCompleted { .. } => "execution_completed",
        Event::ExecutionAborted { .. } => "execution_aborted",
        Event::StepAdded { .. } => "step_added",
        Event::StepStarted { .. } => "step_started",
        Event::StepCompleted { .. } => "step_completed",
        Event::StepSkipped { .. } => "step_skipped",
        Event::CheckboxToggled { .. } => "checkbox_toggled",
        Event::InputRecorded { .. } => "input_recorded",
        Event::NoteAdded { .. } => "note_added",
        Event::AttachmentAdded { .. } => "attachment_added",
        Event::ExecutionRenamed { .. } => "execution_renamed",
        Event::EventReverted { .. } => "event_reverted",
    }
    .to_string()
}

fn event_at(event: &Event) -> String {
    match event {
        Event::ExecutionStarted { at, .. }
        | Event::ExecutionCompleted { at, .. }
        | Event::ExecutionAborted { at, .. }
        | Event::StepAdded { at, .. }
        | Event::StepStarted { at, .. }
        | Event::StepCompleted { at, .. }
        | Event::StepSkipped { at, .. }
        | Event::CheckboxToggled { at, .. }
        | Event::InputRecorded { at, .. }
        | Event::NoteAdded { at, .. }
        | Event::AttachmentAdded { at, .. }
        | Event::ExecutionRenamed { at, .. }
        | Event::EventReverted { at, .. } => at.to_rfc3339(),
    }
}

/// Compute the SHA-256 hash of a file, returning a lowercase hex string.
fn compute_sha256(path: &str) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path)?;
    let hash = Sha256::digest(&bytes);
    Ok(format!("{hash:x}"))
}

/// Format the execution directory name as `{YYYYMMDD}T{HHMMSS}-{uuid_8}`.
fn execution_dir_name(at: &DateTime<Utc>, execution_id: ExecutionId) -> String {
    format!(
        "{}-{}",
        at.format("%Y%m%dT%H%M%S"),
        &execution_id.to_string()[..8]
    )
}

/// Find the execution directory by matching the short UUID suffix.
fn find_execution_dir(executions_dir: &Path, execution_id: ExecutionId) -> Option<PathBuf> {
    let suffix = format!("-{}", &execution_id.to_string()[..8]);
    let entries = std::fs::read_dir(executions_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.ends_with(&suffix)
        {
            return Some(path);
        }
    }
    None
}

/// Load an execution from disk by replaying its event log.
fn load_execution_from_disk(
    executions_dir: &Path,
    execution_id: ExecutionId,
) -> Result<(ExecutionState, Vec<Event>, PathBuf), String> {
    let exec_dir = find_execution_dir(executions_dir, execution_id)
        .ok_or_else(|| format!("Execution not found: {execution_id}"))?;
    let log_path = exec_dir.join("events.jsonl");
    if !log_path.exists() {
        return Err(format!("Execution not found: {execution_id}"));
    }
    let events = read_events(&log_path).map_err(|e| e.to_string())?;
    let state = ExecutionState::from_events(&events).map_err(|e| e.to_string())?;
    Ok((state, events, log_path))
}

/// Start a new execution from a template file.
#[tauri::command]
pub fn start_execution(
    state: State<'_, AppState>,
    template_path: String,
) -> Result<ExecutionSummary, String> {
    let source = std::fs::read_to_string(&template_path).map_err(|e| e.to_string())?;
    let template = parse_template(&source).map_err(|e| e.to_string())?;

    let mut exec_state = ExecutionState::new();
    let events = exec_state.start(&template).map_err(|e| e.to_string())?;

    let execution_id = exec_state.execution_id.unwrap();

    // Extract the timestamp from the ExecutionStarted event.
    let started_at = events
        .iter()
        .find_map(|e| match e {
            Event::ExecutionStarted { at, .. } => Some(*at),
            _ => None,
        })
        .expect("start() must produce an ExecutionStarted event");

    // Create execution directory and log file.
    let exec_dir = state
        .executions_dir
        .join(execution_dir_name(&started_at, execution_id));
    std::fs::create_dir_all(&exec_dir).map_err(|e| e.to_string())?;

    // Copy template snapshot.
    let template_snapshot = exec_dir.join("template.md");
    std::fs::copy(&template_path, &template_snapshot).map_err(|e| e.to_string())?;

    // Write events to log.
    let log_path = exec_dir.join("events.jsonl");
    for event in &events {
        append_event(&log_path, event).map_err(|e| e.to_string())?;
    }

    Ok(summarize(&exec_state, Some(&events)))
}

/// Action payload from the frontend for recording events.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ExecutionAction {
    StartStep {
        step_heading: String,
    },
    CompleteStep {
        step_heading: String,
    },
    SkipStep {
        step_heading: String,
        reason: String,
    },
    ToggleCheckbox {
        step_heading: String,
        text: String,
        checked: bool,
    },
    RecordInput {
        step_heading: String,
        label: String,
        value: String,
        #[ts(optional)]
        unit: Option<String>,
    },
    AddNote {
        text: String,
        #[ts(optional)]
        step_heading: Option<String>,
    },
    AddStep {
        heading: String,
        #[ts(optional)]
        description: Option<String>,
        #[ts(optional)]
        after_step: Option<String>,
    },
    AddAttachment {
        step_heading: String,
        label: String,
        filename: String,
        path: String,
        content_type: String,
    },
    Complete {
        status: CompletionStatus,
    },
    Abort {
        reason: String,
    },
    RenameExecution {
        name: String,
    },
    RevertEvent {
        event_index: usize,
        reason: String,
    },
}

/// Record an action on an active execution.
#[tauri::command]
pub fn record_action(
    state: State<'_, AppState>,
    execution_id: ExecutionId,
    action: ExecutionAction,
) -> Result<ExecutionSummary, String> {
    let (mut exec_state, mut events, log_path) =
        load_execution_from_disk(&state.executions_dir, execution_id)?;

    // Revert is a special case: it rebuilds state from events.
    if let ExecutionAction::RevertEvent {
        event_index,
        reason,
    } = action
    {
        let revert_marker = ExecutionState::revert_event(&events, event_index, &reason)
            .map_err(|e| e.to_string())?;

        // Persist the revert marker.
        append_event(&log_path, &revert_marker).map_err(|e| e.to_string())?;
        events.push(revert_marker);

        // Rebuild state from the full event log.
        let exec_state = ExecutionState::from_events(&events).map_err(|e| e.to_string())?;

        return Ok(summarize(&exec_state, Some(&events)));
    }

    let event: Event = match action {
        ExecutionAction::StartStep { step_heading } => exec_state
            .start_step(&step_heading)
            .map_err(|e| e.to_string())?,
        ExecutionAction::CompleteStep { step_heading } => exec_state
            .complete_step(&step_heading)
            .map_err(|e| e.to_string())?,
        ExecutionAction::SkipStep {
            step_heading,
            reason,
        } => exec_state
            .skip_step(&step_heading, &reason)
            .map_err(|e| e.to_string())?,
        ExecutionAction::ToggleCheckbox {
            step_heading,
            text,
            checked,
        } => exec_state
            .toggle_checkbox(&step_heading, &text, checked)
            .map_err(|e| e.to_string())?,
        ExecutionAction::RecordInput {
            step_heading,
            label,
            value,
            unit,
        } => exec_state
            .record_input(&step_heading, &label, &value, unit.as_deref())
            .map_err(|e| e.to_string())?,
        ExecutionAction::AddNote { text, step_heading } => exec_state
            .add_note(&text, step_heading.as_deref())
            .map_err(|e| e.to_string())?,
        ExecutionAction::AddStep {
            heading,
            description,
            after_step,
        } => exec_state
            .add_step(&heading, description.as_deref(), after_step.as_deref())
            .map_err(|e| e.to_string())?,
        ExecutionAction::AddAttachment {
            step_heading,
            label,
            filename,
            path,
            content_type,
        } => {
            let sha256 = compute_sha256(&path).map_err(|e| e.to_string())?;

            // Copy file into <exec_dir>/attachments/<hash7>-<filename>.
            let short_hash = &sha256[..7];
            let stored_name = format!("{short_hash}-{filename}");
            let exec_dir = log_path.parent().expect("log_path must have a parent");
            let attachments_dir = exec_dir.join("attachments");
            std::fs::create_dir_all(&attachments_dir).map_err(|e| e.to_string())?;
            let dest = attachments_dir.join(&stored_name);
            std::fs::copy(&path, &dest).map_err(|e| e.to_string())?;
            let relative_path = format!("attachments/{stored_name}");

            exec_state
                .add_attachment(
                    &step_heading,
                    &label,
                    &filename,
                    &relative_path,
                    &content_type,
                    &sha256,
                )
                .map_err(|e| e.to_string())?
        }
        ExecutionAction::Complete { status } => {
            exec_state.complete(status).map_err(|e| e.to_string())?
        }
        ExecutionAction::Abort { reason } => {
            exec_state.abort(&reason).map_err(|e| e.to_string())?
        }
        ExecutionAction::RenameExecution { name } => {
            exec_state.rename(&name).map_err(|e| e.to_string())?
        }
        ExecutionAction::RevertEvent { .. } => unreachable!("handled above"),
    };

    // Persist event.
    append_event(&log_path, &event).map_err(|e| e.to_string())?;
    events.push(event);

    Ok(summarize(&exec_state, Some(&events)))
}

/// Get the current state of an execution.
#[tauri::command]
pub fn get_execution_state(
    state: State<'_, AppState>,
    execution_id: ExecutionId,
) -> Result<ExecutionSummary, String> {
    let (exec_state, events, _) = load_execution_from_disk(&state.executions_dir, execution_id)?;
    Ok(summarize(&exec_state, Some(&events)))
}

/// List all executions by scanning the executions directory on disk.
#[tauri::command]
pub fn list_executions(state: State<'_, AppState>) -> Result<Vec<ExecutionSummary>, String> {
    let mut summaries = Vec::new();

    if !state.executions_dir.exists() {
        return Ok(summaries);
    }

    let entries = std::fs::read_dir(&state.executions_dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let dir_path = entry.path();
        if !dir_path.is_dir() {
            continue;
        }
        let log_path = dir_path.join("events.jsonl");
        if !log_path.exists() {
            continue;
        }
        let events = match read_events(&log_path) {
            Ok(events) => events,
            Err(e) => {
                log::warn!("Failed to read events from {}: {e}", log_path.display());
                continue;
            }
        };
        let exec_state = match ExecutionState::from_events(&events) {
            Ok(state) => state,
            Err(e) => {
                log::warn!("Failed to replay events from {}: {e}", log_path.display());
                continue;
            }
        };
        summaries.push(summarize(&exec_state, Some(&events)));
    }

    Ok(summaries)
}
