use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::template::types::InputDefinition;

/// Unique identifier for an execution.
pub type ExecutionId = Uuid;

/// Completion status of an execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum CompletionStatus {
    Pass,
    Fail,
    Aborted,
}

/// All events that can occur during a procedure execution.
///
/// Events are internally tagged with `"type"` for JSON serialization,
/// and every event carries an `at` timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    // -- Lifecycle --
    ExecutionStarted {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        procedure_id: String,
        procedure_title: String,
        procedure_version: String,
    },
    ExecutionCompleted {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        status: CompletionStatus,
    },
    ExecutionAborted {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        reason: String,
    },

    // -- Step --
    StepAdded {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        heading: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Insert after this step heading. `None` means append at end.
        #[serde(skip_serializing_if = "Option::is_none")]
        after_step: Option<String>,
        /// Checkbox texts to initialize in this step (from template).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        checkboxes: Vec<String>,
        /// Input definitions for this step (from template).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        inputs: Vec<InputDefinition>,
    },
    StepStarted {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        step_heading: String,
    },
    StepCompleted {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        step_heading: String,
    },
    StepSkipped {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        step_heading: String,
        reason: String,
    },

    // -- Data --
    CheckboxToggled {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        step_heading: String,
        text: String,
        checked: bool,
    },
    InputRecorded {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        step_heading: String,
        label: String,
        value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
    },
    NoteAdded {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        step_heading: Option<String>,
    },

    // -- Attachment --
    AttachmentAdded {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        step_heading: String,
        label: String,
        filename: String,
        path: String,
        content_type: String,
        sha256: String,
    },

    // -- Name --
    ExecutionRenamed {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        name: String,
    },

    // -- Revert --
    /// Marks a previously recorded event as reverted.
    /// State is rebuilt by replaying all events, skipping reverted ones.
    EventReverted {
        at: DateTime<Utc>,
        execution_id: ExecutionId,
        /// Zero-based index of the event in the log to revert.
        reverted_event_index: usize,
        /// Human-readable reason for the revert (audit trail).
        reason: String,
    },
}

/// Whether an event can be reverted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revertibility {
    /// This event can be reverted by the user.
    Revertible,
    /// This event cannot be reverted (structural/lifecycle).
    NotRevertible,
    /// This event is itself a revert marker and cannot be reverted.
    RevertMarker,
}

impl Event {
    /// Classify whether this event can be reverted.
    ///
    /// This match is exhaustive — adding a new `Event` variant without
    /// updating this method will cause a compile error.
    #[must_use]
    pub const fn revertibility(&self) -> Revertibility {
        match self {
            // Lifecycle/structural — not revertible
            Self::ExecutionStarted { .. } | Self::StepAdded { .. } => {
                Revertibility::NotRevertible
            }

            // Everything else (except revert markers) — revertible
            Self::ExecutionCompleted { .. }
            | Self::ExecutionAborted { .. }
            | Self::StepStarted { .. }
            | Self::StepCompleted { .. }
            | Self::StepSkipped { .. }
            | Self::CheckboxToggled { .. }
            | Self::InputRecorded { .. }
            | Self::NoteAdded { .. }
            | Self::AttachmentAdded { .. }
            | Self::ExecutionRenamed { .. } => Revertibility::Revertible,

            // Revert marker — not revertible
            Self::EventReverted { .. } => Revertibility::RevertMarker,
        }
    }

    /// Human-readable description of this event for UI display.
    ///
    /// This match is exhaustive — adding a new `Event` variant without
    /// updating this method will cause a compile error.
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::ExecutionStarted { procedure_id, .. } => {
                format!("Started execution of {procedure_id}")
            }
            Self::ExecutionCompleted { status, .. } => {
                format!("Completed execution: {status:?}")
            }
            Self::ExecutionAborted { reason, .. } => {
                format!("Aborted execution: {reason}")
            }
            Self::StepAdded { heading, .. } => format!("Added step: {heading}"),
            Self::StepStarted { step_heading, .. } => {
                format!("Started step: {step_heading}")
            }
            Self::StepCompleted { step_heading, .. } => {
                format!("Completed step: {step_heading}")
            }
            Self::StepSkipped {
                step_heading,
                reason,
                ..
            } => {
                format!("Skipped step: {step_heading} ({reason})")
            }
            Self::CheckboxToggled {
                step_heading,
                text,
                checked,
                ..
            } => {
                let verb = if *checked { "Checked" } else { "Unchecked" };
                format!("{verb} checkbox '{text}' in {step_heading}")
            }
            Self::InputRecorded {
                step_heading,
                label,
                value,
                ..
            } => {
                format!("Recorded {label} = {value} in {step_heading}")
            }
            Self::NoteAdded {
                text, step_heading, ..
            } => {
                let scope = step_heading
                    .as_ref()
                    .map(|h| format!(" to {h}"))
                    .unwrap_or_default();
                let truncated = if text.len() > 50 {
                    format!("{}...", &text[..50])
                } else {
                    text.clone()
                };
                format!("Added note{scope}: {truncated}")
            }
            Self::AttachmentAdded {
                step_heading,
                label,
                filename,
                ..
            } => {
                format!("Recorded {label} = {filename} in {step_heading}")
            }
            Self::ExecutionRenamed { name, .. } => {
                format!("Renamed execution to: {name}")
            }
            Self::EventReverted {
                reverted_event_index,
                reason,
                ..
            } => {
                format!("Reverted event #{reverted_event_index}: {reason}")
            }
        }
    }
}
