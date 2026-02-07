use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::template::types::InputDefinition;

/// Unique identifier for an execution.
pub type ExecutionId = Uuid;

/// Completion status of an execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
        filename: String,
        path: String,
        content_type: String,
    },
}
