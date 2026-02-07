use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::{ActiveExecution, AppState};
use procnote_core::event::types::{CompletionStatus, Event, ExecutionId, Revertibility};
use procnote_core::event::{append_event, read_events};
use procnote_core::execution::{ExecutionState, StepStatus};
use procnote_core::template::parse_template;
use procnote_core::template::types::InputDefinition;

/// Serializable execution state summary for the frontend.
#[derive(Debug, Serialize)]
pub struct ExecutionSummary {
    pub execution_id: ExecutionId,
    pub procedure_id: String,
    pub procedure_version: String,
    pub status: String,
    pub steps: Vec<StepSummary>,
    pub event_history: Vec<EventHistoryEntry>,
}

/// A single entry in the event history, exposed to the frontend.
#[derive(Debug, Serialize)]
pub struct EventHistoryEntry {
    pub index: usize,
    pub event_type: String,
    /// ISO 8601 timestamp string.
    pub at: String,
    pub description: String,
    pub revertible: bool,
    pub reverted: bool,
}

#[derive(Debug, Serialize)]
pub struct StepSummary {
    pub heading: String,
    pub description: Option<String>,
    pub status: String,
    /// ISO 8601 timestamp of the most recent status change (started/completed/skipped).
    pub status_at: Option<String>,
    pub checkboxes: Vec<CheckboxState>,
    pub input_definitions: Vec<InputDefinition>,
    pub inputs: Vec<InputState>,
    pub notes: Vec<NoteState>,
}

#[derive(Debug, Serialize)]
pub struct CheckboxState {
    pub text: String,
    pub checked: bool,
    /// ISO 8601 timestamp of the last toggle, if any.
    pub at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InputState {
    pub label: String,
    pub value: String,
    pub unit: Option<String>,
    /// ISO 8601 timestamp of when the input was recorded.
    pub at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NoteState {
    pub text: String,
    /// ISO 8601 timestamp of when the note was added.
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
        procedure_id: state.procedure_id.clone().unwrap_or_default(),
        procedure_version: state.procedure_version.clone().unwrap_or_default(),
        status: status_string(&state.status),
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
            EventHistoryEntry {
                index,
                event_type: event_type_string(event),
                at: event_at(event),
                description: event.description(),
                revertible,
                reverted: reverted_indices.contains(&index),
            }
        })
        .collect()
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
        | Event::EventReverted { at, .. } => at.to_rfc3339(),
    }
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

    // Create execution directory and log file.
    let exec_dir = state.executions_dir.join(execution_id.to_string());
    std::fs::create_dir_all(&exec_dir).map_err(|e| e.to_string())?;

    // Copy template snapshot.
    let template_snapshot = exec_dir.join("template.md");
    std::fs::copy(&template_path, &template_snapshot).map_err(|e| e.to_string())?;

    // Write events to log.
    let log_path = exec_dir.join("events.jsonl");
    for event in &events {
        append_event(&log_path, event).map_err(|e| e.to_string())?;
    }

    let summary = summarize(&exec_state, Some(&events));

    // Store active execution.
    let mut executions = state.executions.lock().unwrap();
    executions.insert(
        execution_id,
        ActiveExecution {
            state: exec_state,
            log_path,
            events,
        },
    );

    Ok(summary)
}

/// Action payload from the frontend for recording events.
#[derive(Debug, Deserialize)]
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
        unit: Option<String>,
    },
    AddNote {
        text: String,
        step_heading: Option<String>,
    },
    AddStep {
        heading: String,
        description: Option<String>,
        after_step: Option<String>,
    },
    AddAttachment {
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
    let mut executions = state.executions.lock().unwrap();
    let active = executions
        .get_mut(&execution_id)
        .ok_or_else(|| format!("Execution not found: {execution_id}"))?;

    // Revert is a special case: it rebuilds state from events.
    if let ExecutionAction::RevertEvent {
        event_index,
        reason,
    } = action
    {
        let revert_marker = ExecutionState::revert_event(&active.events, event_index, &reason)
            .map_err(|e| e.to_string())?;

        // Persist the revert marker.
        append_event(&active.log_path, &revert_marker).map_err(|e| e.to_string())?;
        active.events.push(revert_marker);

        // Rebuild state from the full event log.
        active.state = ExecutionState::from_events(&active.events).map_err(|e| e.to_string())?;

        return Ok(summarize(&active.state, Some(&active.events)));
    }

    let event: Event = match action {
        ExecutionAction::StartStep { step_heading } => active
            .state
            .start_step(&step_heading)
            .map_err(|e| e.to_string())?,
        ExecutionAction::CompleteStep { step_heading } => active
            .state
            .complete_step(&step_heading)
            .map_err(|e| e.to_string())?,
        ExecutionAction::SkipStep {
            step_heading,
            reason,
        } => active
            .state
            .skip_step(&step_heading, &reason)
            .map_err(|e| e.to_string())?,
        ExecutionAction::ToggleCheckbox {
            step_heading,
            text,
            checked,
        } => active
            .state
            .toggle_checkbox(&step_heading, &text, checked)
            .map_err(|e| e.to_string())?,
        ExecutionAction::RecordInput {
            step_heading,
            label,
            value,
            unit,
        } => active
            .state
            .record_input(&step_heading, &label, &value, unit.as_deref())
            .map_err(|e| e.to_string())?,
        ExecutionAction::AddNote { text, step_heading } => active
            .state
            .add_note(&text, step_heading.as_deref())
            .map_err(|e| e.to_string())?,
        ExecutionAction::AddStep {
            heading,
            description,
            after_step,
        } => active
            .state
            .add_step(&heading, description.as_deref(), after_step.as_deref())
            .map_err(|e| e.to_string())?,
        ExecutionAction::AddAttachment {
            filename,
            path,
            content_type,
        } => active
            .state
            .add_attachment(&filename, &path, &content_type)
            .map_err(|e| e.to_string())?,
        ExecutionAction::Complete { status } => {
            active.state.complete(status).map_err(|e| e.to_string())?
        }
        ExecutionAction::Abort { reason } => {
            active.state.abort(&reason).map_err(|e| e.to_string())?
        }
        ExecutionAction::RevertEvent { .. } => unreachable!("handled above"),
    };

    // Persist event.
    append_event(&active.log_path, &event).map_err(|e| e.to_string())?;
    active.events.push(event);

    Ok(summarize(&active.state, Some(&active.events)))
}

/// Get the current state of an execution.
#[tauri::command]
pub fn get_execution_state(
    state: State<'_, AppState>,
    execution_id: ExecutionId,
) -> Result<ExecutionSummary, String> {
    let executions = state.executions.lock().unwrap();
    let active = executions
        .get(&execution_id)
        .ok_or_else(|| format!("Execution not found: {execution_id}"))?;

    Ok(summarize(&active.state, Some(&active.events)))
}

/// List all executions (active in memory + completed on disk).
#[tauri::command]
pub fn list_executions(state: State<'_, AppState>) -> Result<Vec<ExecutionSummary>, String> {
    let executions = state.executions.lock().unwrap();

    // Return active in-memory executions.
    let mut summaries: Vec<ExecutionSummary> = executions
        .values()
        .map(|active| summarize(&active.state, Some(&active.events)))
        .collect();

    // Also check disk for completed executions not in memory.
    if state.executions_dir.exists() {
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
            // Parse the directory name as UUID.
            let dir_name = dir_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let exec_id: ExecutionId = match dir_name.parse() {
                Ok(id) => id,
                Err(_) => continue,
            };
            // Skip if already in memory.
            if executions.contains_key(&exec_id) {
                continue;
            }
            // Replay from disk.
            match read_events(&log_path) {
                Ok(events) => match ExecutionState::from_events(&events) {
                    Ok(exec_state) => summaries.push(summarize(&exec_state, Some(&events))),
                    Err(e) => log::warn!("Failed to replay execution {exec_id}: {e}"),
                },
                Err(e) => log::warn!("Failed to read events for {exec_id}: {e}"),
            }
        }
    }

    Ok(summaries)
}
