use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::{ActiveExecution, AppState};
use procnote_core::event::types::{CompletionStatus, Event, ExecutionId};
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
}

#[derive(Debug, Serialize)]
pub struct StepSummary {
    pub heading: String,
    pub description: Option<String>,
    pub status: String,
    pub checkboxes: Vec<CheckboxState>,
    pub input_definitions: Vec<InputDefinition>,
    pub inputs: Vec<InputState>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckboxState {
    pub text: String,
    pub checked: bool,
}

#[derive(Debug, Serialize)]
pub struct InputState {
    pub label: String,
    pub value: String,
    pub unit: Option<String>,
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

fn summarize(state: &ExecutionState) -> ExecutionSummary {
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
                    })
                    .collect();
                let inputs = step
                    .inputs
                    .values()
                    .map(|input| InputState {
                        label: input.label.clone(),
                        value: input.value.clone(),
                        unit: input.unit.clone(),
                    })
                    .collect();
                StepSummary {
                    heading: step.heading.clone(),
                    description: step.description.clone(),
                    status: step_status_string(&step.status),
                    checkboxes,
                    input_definitions: step.input_definitions.clone(),
                    inputs,
                    notes: step.notes.clone(),
                }
            })
        })
        .collect();

    ExecutionSummary {
        execution_id: state.execution_id.unwrap_or_default(),
        procedure_id: state.procedure_id.clone().unwrap_or_default(),
        procedure_version: state.procedure_version.clone().unwrap_or_default(),
        status: status_string(&state.status),
        steps,
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

    let summary = summarize(&exec_state);

    // Store active execution.
    let mut executions = state.executions.lock().unwrap();
    executions.insert(
        execution_id,
        ActiveExecution {
            state: exec_state,
            log_path,
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
    };

    // Persist event.
    append_event(&active.log_path, &event).map_err(|e| e.to_string())?;

    Ok(summarize(&active.state))
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

    Ok(summarize(&active.state))
}

/// List all executions (active in memory + completed on disk).
#[tauri::command]
pub fn list_executions(state: State<'_, AppState>) -> Result<Vec<ExecutionSummary>, String> {
    let executions = state.executions.lock().unwrap();

    // Return active in-memory executions.
    let mut summaries: Vec<ExecutionSummary> = executions
        .values()
        .map(|active| summarize(&active.state))
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
                    Ok(exec_state) => summaries.push(summarize(&exec_state)),
                    Err(e) => log::warn!("Failed to replay execution {exec_id}: {e}"),
                },
                Err(e) => log::warn!("Failed to read events for {exec_id}: {e}"),
            }
        }
    }

    Ok(summaries)
}
