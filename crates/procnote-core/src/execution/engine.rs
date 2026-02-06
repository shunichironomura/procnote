use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use crate::event::types::{CompletionStatus, Event, ExecutionId};
use crate::template::types::{InputDefinition, ProcedureTemplate, StepContent};

/// Errors that can occur during execution state transitions.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExecutionError {
    #[error("execution has not been started")]
    NotStarted,
    #[error("execution has already been started")]
    AlreadyStarted,
    #[error("execution has already finished")]
    AlreadyFinished,
    #[error("step not found: {0}")]
    StepNotFound(String),
    #[error("step already started: {0}")]
    StepAlreadyStarted(String),
    #[error("step not started: {0}")]
    StepNotStarted(String),
    #[error("step already finished: {0}")]
    StepAlreadyFinished(String),
    #[error("duplicate step heading: {0}")]
    DuplicateStepHeading(String),
}

/// Status of the overall execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Not yet started.
    Pending,
    /// In progress.
    Active,
    /// Finished (pass, fail, or aborted).
    Finished(CompletionStatus),
}

/// Status of a single step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Active,
    Completed,
    Skipped,
}

/// Tracked state for a single step during execution.
#[derive(Debug, Clone)]
pub struct StepState {
    pub heading: String,
    pub status: StepStatus,
    /// Checkbox text -> checked state. Insertion order preserved by step_order.
    pub checkboxes: Vec<(String, bool)>,
    /// Input definitions for this step (from template or StepAdded event).
    pub input_definitions: Vec<InputDefinition>,
    /// Recorded input values keyed by label.
    pub inputs: HashMap<String, RecordedInput>,
    pub notes: Vec<String>,
}

/// A recorded input value.
#[derive(Debug, Clone)]
pub struct RecordedInput {
    pub label: String,
    pub value: String,
    pub unit: Option<String>,
}

/// The full state of a procedure execution, reconstructable from events.
#[derive(Debug)]
pub struct ExecutionState {
    pub execution_id: Option<ExecutionId>,
    pub procedure_id: Option<String>,
    pub procedure_version: Option<String>,
    pub operator: Option<String>,
    pub status: ExecutionStatus,
    /// Ordered step headings (preserves insertion order).
    pub step_order: Vec<String>,
    pub steps: HashMap<String, StepState>,
    pub attachments: Vec<Attachment>,
    pub global_notes: Vec<String>,
}

/// A recorded attachment.
#[derive(Debug, Clone)]
pub struct Attachment {
    pub filename: String,
    pub path: String,
    pub content_type: String,
}

impl ExecutionState {
    /// Create a new empty execution state.
    pub fn new() -> Self {
        Self {
            execution_id: None,
            procedure_id: None,
            procedure_version: None,
            operator: None,
            status: ExecutionStatus::Pending,
            step_order: Vec::new(),
            steps: HashMap::new(),
            attachments: Vec::new(),
            global_notes: Vec::new(),
        }
    }

    /// Reconstruct execution state by replaying a sequence of events.
    pub fn from_events(events: &[Event]) -> Result<Self, ExecutionError> {
        let mut state = Self::new();
        for event in events {
            state.apply(event)?;
        }
        Ok(state)
    }

    /// Apply a single event to the state (used by both replay and transitions).
    pub fn apply(&mut self, event: &Event) -> Result<(), ExecutionError> {
        match event {
            Event::ExecutionStarted {
                execution_id,
                procedure_id,
                procedure_version,
                operator,
                ..
            } => {
                if self.status != ExecutionStatus::Pending {
                    return Err(ExecutionError::AlreadyStarted);
                }
                self.execution_id = Some(*execution_id);
                self.procedure_id = Some(procedure_id.clone());
                self.procedure_version = Some(procedure_version.clone());
                self.operator = Some(operator.clone());
                self.status = ExecutionStatus::Active;
            }
            Event::ExecutionCompleted { status, .. } => {
                self.require_active()?;
                self.status = ExecutionStatus::Finished(status.clone());
            }
            Event::ExecutionAborted { .. } => {
                self.require_active()?;
                self.status = ExecutionStatus::Finished(CompletionStatus::Aborted);
            }
            Event::StepAdded {
                heading,
                after_step,
                checkboxes,
                inputs,
                ..
            } => {
                self.require_active()?;
                if self.steps.contains_key(heading) {
                    return Err(ExecutionError::DuplicateStepHeading(heading.clone()));
                }
                let step_state = StepState {
                    heading: heading.clone(),
                    status: StepStatus::Pending,
                    checkboxes: checkboxes.iter().map(|t| (t.clone(), false)).collect(),
                    input_definitions: inputs.clone(),
                    inputs: HashMap::new(),
                    notes: Vec::new(),
                };
                self.steps.insert(heading.clone(), step_state);
                match after_step {
                    Some(after) => {
                        if let Some(pos) = self.step_order.iter().position(|h| h == after) {
                            self.step_order.insert(pos + 1, heading.clone());
                        } else {
                            self.step_order.push(heading.clone());
                        }
                    }
                    None => {
                        self.step_order.push(heading.clone());
                    }
                }
            }
            Event::StepStarted { step_heading, .. } => {
                self.require_active()?;
                let step = self.get_step_mut(step_heading)?;
                match step.status {
                    StepStatus::Pending => step.status = StepStatus::Active,
                    StepStatus::Active => {
                        return Err(ExecutionError::StepAlreadyStarted(step_heading.clone()));
                    }
                    StepStatus::Completed | StepStatus::Skipped => {
                        return Err(ExecutionError::StepAlreadyFinished(step_heading.clone()));
                    }
                }
            }
            Event::StepCompleted { step_heading, .. } => {
                self.require_active()?;
                let step = self.get_step_mut(step_heading)?;
                match step.status {
                    StepStatus::Active => step.status = StepStatus::Completed,
                    StepStatus::Pending => {
                        return Err(ExecutionError::StepNotStarted(step_heading.clone()));
                    }
                    StepStatus::Completed | StepStatus::Skipped => {
                        return Err(ExecutionError::StepAlreadyFinished(step_heading.clone()));
                    }
                }
            }
            Event::StepSkipped { step_heading, .. } => {
                self.require_active()?;
                let step = self.get_step_mut(step_heading)?;
                match step.status {
                    StepStatus::Pending => step.status = StepStatus::Skipped,
                    StepStatus::Active => step.status = StepStatus::Skipped,
                    StepStatus::Completed | StepStatus::Skipped => {
                        return Err(ExecutionError::StepAlreadyFinished(step_heading.clone()));
                    }
                }
            }
            Event::CheckboxToggled {
                step_heading,
                text,
                checked,
                ..
            } => {
                self.require_active()?;
                let step = self.get_step_mut(step_heading)?;
                if let Some(entry) = step.checkboxes.iter_mut().find(|(t, _)| t == text) {
                    entry.1 = *checked;
                } else {
                    // Checkbox not from template — add dynamically.
                    step.checkboxes.push((text.clone(), *checked));
                }
            }
            Event::InputRecorded {
                step_heading,
                label,
                value,
                unit,
                ..
            } => {
                self.require_active()?;
                let step = self.get_step_mut(step_heading)?;
                step.inputs.insert(
                    label.clone(),
                    RecordedInput {
                        label: label.clone(),
                        value: value.clone(),
                        unit: unit.clone(),
                    },
                );
            }
            Event::NoteAdded {
                text, step_heading, ..
            } => {
                self.require_active()?;
                match step_heading {
                    Some(heading) => {
                        let step = self.get_step_mut(heading)?;
                        step.notes.push(text.clone());
                    }
                    None => {
                        self.global_notes.push(text.clone());
                    }
                }
            }
            Event::DeviationRecorded { .. } => {
                // Legacy event: deviation feature has been removed.
                // Accept the event during replay for backward compatibility but do nothing.
            }
            Event::AttachmentAdded {
                filename,
                path,
                content_type,
                ..
            } => {
                self.require_active()?;
                self.attachments.push(Attachment {
                    filename: filename.clone(),
                    path: path.clone(),
                    content_type: content_type.clone(),
                });
            }
        }
        Ok(())
    }

    // -- Transition methods: produce events --

    /// Start a new execution from a template.
    pub fn start(
        &mut self,
        template: &ProcedureTemplate,
        operator: &str,
    ) -> Result<Vec<Event>, ExecutionError> {
        if self.status != ExecutionStatus::Pending {
            return Err(ExecutionError::AlreadyStarted);
        }
        let execution_id = Uuid::new_v4();
        let now = Utc::now();

        let mut events = Vec::new();

        // Execution started event.
        let started = Event::ExecutionStarted {
            at: now,
            execution_id,
            procedure_id: template.metadata.id.clone(),
            procedure_version: template.metadata.version.clone(),
            operator: operator.to_string(),
        };
        self.apply(&started)?;
        events.push(started);

        // Add steps from the template, including checkboxes and input definitions.
        for step in &template.steps {
            let mut checkboxes = Vec::new();
            let mut input_defs = Vec::new();
            for content in &step.content {
                match content {
                    StepContent::Checkbox { text, .. } => {
                        checkboxes.push(text.clone());
                    }
                    StepContent::InputBlock { inputs } => {
                        input_defs.extend(inputs.iter().cloned());
                    }
                    StepContent::Prose { .. } => {}
                }
            }
            let step_added = Event::StepAdded {
                at: now,
                execution_id,
                heading: step.heading.clone(),
                description: None,
                after_step: None,
                checkboxes,
                inputs: input_defs,
            };
            self.apply(&step_added)?;
            events.push(step_added);
        }

        Ok(events)
    }

    /// Add a new step during execution.
    pub fn add_step(
        &mut self,
        heading: &str,
        description: Option<&str>,
        after_step: Option<&str>,
    ) -> Result<Event, ExecutionError> {
        self.require_active()?;
        let event = Event::StepAdded {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            heading: heading.to_string(),
            description: description.map(|s| s.to_string()),
            after_step: after_step.map(|s| s.to_string()),
            checkboxes: Vec::new(),
            inputs: Vec::new(),
        };
        self.apply(&event)?;
        Ok(event)
    }

    /// Start a step.
    pub fn start_step(&mut self, heading: &str) -> Result<Event, ExecutionError> {
        self.require_active()?;
        let event = Event::StepStarted {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            step_heading: heading.to_string(),
        };
        self.apply(&event)?;
        Ok(event)
    }

    /// Complete a step.
    pub fn complete_step(&mut self, heading: &str) -> Result<Event, ExecutionError> {
        self.require_active()?;
        let event = Event::StepCompleted {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            step_heading: heading.to_string(),
        };
        self.apply(&event)?;
        Ok(event)
    }

    /// Skip a step.
    pub fn skip_step(&mut self, heading: &str, reason: &str) -> Result<Event, ExecutionError> {
        self.require_active()?;
        let event = Event::StepSkipped {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            step_heading: heading.to_string(),
            reason: reason.to_string(),
        };
        self.apply(&event)?;
        Ok(event)
    }

    /// Toggle a checkbox in a step.
    pub fn toggle_checkbox(
        &mut self,
        step_heading: &str,
        text: &str,
        checked: bool,
    ) -> Result<Event, ExecutionError> {
        self.require_active()?;
        let event = Event::CheckboxToggled {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            step_heading: step_heading.to_string(),
            text: text.to_string(),
            checked,
        };
        self.apply(&event)?;
        Ok(event)
    }

    /// Record an input value.
    pub fn record_input(
        &mut self,
        step_heading: &str,
        label: &str,
        value: &str,
        unit: Option<&str>,
    ) -> Result<Event, ExecutionError> {
        self.require_active()?;
        let event = Event::InputRecorded {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            step_heading: step_heading.to_string(),
            label: label.to_string(),
            value: value.to_string(),
            unit: unit.map(|s| s.to_string()),
        };
        self.apply(&event)?;
        Ok(event)
    }

    /// Add a note.
    pub fn add_note(
        &mut self,
        text: &str,
        step_heading: Option<&str>,
    ) -> Result<Event, ExecutionError> {
        self.require_active()?;
        let event = Event::NoteAdded {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            text: text.to_string(),
            step_heading: step_heading.map(|s| s.to_string()),
        };
        self.apply(&event)?;
        Ok(event)
    }

    /// Add an attachment.
    pub fn add_attachment(
        &mut self,
        filename: &str,
        path: &str,
        content_type: &str,
    ) -> Result<Event, ExecutionError> {
        self.require_active()?;
        let event = Event::AttachmentAdded {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            filename: filename.to_string(),
            path: path.to_string(),
            content_type: content_type.to_string(),
        };
        self.apply(&event)?;
        Ok(event)
    }

    /// Complete the execution.
    pub fn complete(&mut self, status: CompletionStatus) -> Result<Event, ExecutionError> {
        self.require_active()?;
        let event = Event::ExecutionCompleted {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            status,
        };
        self.apply(&event)?;
        Ok(event)
    }

    /// Abort the execution.
    pub fn abort(&mut self, reason: &str) -> Result<Event, ExecutionError> {
        self.require_active()?;
        let event = Event::ExecutionAborted {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            reason: reason.to_string(),
        };
        self.apply(&event)?;
        Ok(event)
    }

    // -- Helpers --

    fn require_active(&self) -> Result<(), ExecutionError> {
        match &self.status {
            ExecutionStatus::Pending => Err(ExecutionError::NotStarted),
            ExecutionStatus::Active => Ok(()),
            ExecutionStatus::Finished(_) => Err(ExecutionError::AlreadyFinished),
        }
    }

    fn require_execution_id(&self) -> Result<ExecutionId, ExecutionError> {
        self.execution_id.ok_or(ExecutionError::NotStarted)
    }

    fn get_step_mut(&mut self, heading: &str) -> Result<&mut StepState, ExecutionError> {
        self.steps
            .get_mut(heading)
            .ok_or_else(|| ExecutionError::StepNotFound(heading.to_string()))
    }
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::types::{ProcedureMetadata, ProcedureTemplate, Step};

    fn sample_template() -> ProcedureTemplate {
        ProcedureTemplate {
            metadata: ProcedureMetadata {
                id: "TVT-001".to_string(),
                title: "Thermal Vacuum Test".to_string(),
                version: "1.0".to_string(),
                author: Some("Nomura".to_string()),
                equipment: vec![],
                requirement_traces: vec![],
            },
            steps: vec![
                Step {
                    heading: "Preconditions".to_string(),
                    content: vec![],
                },
                Step {
                    heading: "Step 1: Power On".to_string(),
                    content: vec![],
                },
                Step {
                    heading: "Postconditions".to_string(),
                    content: vec![],
                },
            ],
        }
    }

    #[test]
    fn test_start_execution() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let events = state.start(&template, "Nomura").unwrap();

        // 1 ExecutionStarted + 3 StepAdded
        assert_eq!(events.len(), 4);
        assert_eq!(state.status, ExecutionStatus::Active);
        assert_eq!(state.step_order.len(), 3);
        assert_eq!(state.step_order[0], "Preconditions");
        assert_eq!(state.step_order[1], "Step 1: Power On");
        assert_eq!(state.step_order[2], "Postconditions");
    }

    #[test]
    fn test_full_execution_flow() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let mut all_events: Vec<Event> = Vec::new();

        // Start
        all_events.extend(state.start(&template, "Nomura").unwrap());

        // Step through preconditions
        all_events.push(state.start_step("Preconditions").unwrap());
        all_events.push(
            state
                .toggle_checkbox("Preconditions", "Check 1", true)
                .unwrap(),
        );
        all_events.push(state.complete_step("Preconditions").unwrap());

        // Step 1
        all_events.push(state.start_step("Step 1: Power On").unwrap());
        all_events.push(
            state
                .record_input("Step 1: Power On", "Current", "120", Some("mA"))
                .unwrap(),
        );
        all_events.push(
            state
                .add_note("Voltage stable", Some("Step 1: Power On"))
                .unwrap(),
        );
        all_events.push(state.complete_step("Step 1: Power On").unwrap());

        // Skip postconditions
        all_events.push(state.skip_step("Postconditions", "Not applicable").unwrap());

        // Complete
        all_events.push(state.complete(CompletionStatus::Pass).unwrap());

        assert_eq!(
            state.status,
            ExecutionStatus::Finished(CompletionStatus::Pass)
        );
        assert_eq!(state.steps["Preconditions"].status, StepStatus::Completed);
        assert_eq!(
            state.steps["Step 1: Power On"].status,
            StepStatus::Completed
        );
        assert_eq!(state.steps["Postconditions"].status, StepStatus::Skipped);
        assert!(
            state.steps["Preconditions"]
                .checkboxes
                .iter()
                .any(|(t, c)| t == "Check 1" && *c)
        );
        assert_eq!(
            state.steps["Step 1: Power On"].inputs["Current"].value,
            "120"
        );
        assert_eq!(state.steps["Step 1: Power On"].notes.len(), 1);

        // Replay from events
        let replayed = ExecutionState::from_events(&all_events).unwrap();
        assert_eq!(
            replayed.status,
            ExecutionStatus::Finished(CompletionStatus::Pass)
        );
        assert_eq!(replayed.step_order.len(), 3);
        assert!(
            replayed.steps["Preconditions"]
                .checkboxes
                .iter()
                .any(|(t, c)| t == "Check 1" && *c)
        );
    }

    #[test]
    fn test_add_step_during_execution() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template, "Nomura").unwrap();

        // Add a step after "Step 1: Power On"
        state
            .add_step(
                "Step 1.5: Verification",
                Some("Extra verification step"),
                Some("Step 1: Power On"),
            )
            .unwrap();

        assert_eq!(state.step_order.len(), 4);
        assert_eq!(state.step_order[0], "Preconditions");
        assert_eq!(state.step_order[1], "Step 1: Power On");
        assert_eq!(state.step_order[2], "Step 1.5: Verification");
        assert_eq!(state.step_order[3], "Postconditions");
    }

    #[test]
    fn test_cannot_start_twice() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template, "Nomura").unwrap();
        let result = state.start(&template, "Nomura");
        assert_eq!(result.unwrap_err(), ExecutionError::AlreadyStarted);
    }

    #[test]
    fn test_cannot_act_before_start() {
        let mut state = ExecutionState::new();
        let result = state.start_step("Step 1");
        assert_eq!(result.unwrap_err(), ExecutionError::NotStarted);
    }

    #[test]
    fn test_cannot_act_after_finish() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template, "Nomura").unwrap();
        state.complete(CompletionStatus::Pass).unwrap();

        let result = state.start_step("Preconditions");
        assert_eq!(result.unwrap_err(), ExecutionError::AlreadyFinished);
    }

    #[test]
    fn test_cannot_complete_unstarted_step() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template, "Nomura").unwrap();

        let result = state.complete_step("Preconditions");
        assert_eq!(
            result.unwrap_err(),
            ExecutionError::StepNotStarted("Preconditions".to_string())
        );
    }

    #[test]
    fn test_cannot_start_completed_step() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template, "Nomura").unwrap();
        state.start_step("Preconditions").unwrap();
        state.complete_step("Preconditions").unwrap();

        let result = state.start_step("Preconditions");
        assert_eq!(
            result.unwrap_err(),
            ExecutionError::StepAlreadyFinished("Preconditions".to_string())
        );
    }

    #[test]
    fn test_abort_execution() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template, "Nomura").unwrap();
        state.abort("Power failure").unwrap();

        assert_eq!(
            state.status,
            ExecutionStatus::Finished(CompletionStatus::Aborted)
        );
    }

    #[test]
    fn test_attachment() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template, "Nomura").unwrap();

        state
            .add_attachment("photo.jpg", "attachments/photo.jpg", "image/jpeg")
            .unwrap();

        assert_eq!(state.attachments.len(), 1);
        assert_eq!(state.attachments[0].filename, "photo.jpg");
    }

    #[test]
    fn test_global_note() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template, "Nomura").unwrap();

        state.add_note("General observation", None).unwrap();

        assert_eq!(state.global_notes.len(), 1);
        assert_eq!(state.global_notes[0], "General observation");
    }

    #[test]
    fn test_duplicate_step_heading() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template, "Nomura").unwrap();

        let result = state.add_step("Preconditions", None, None);
        assert_eq!(
            result.unwrap_err(),
            ExecutionError::DuplicateStepHeading("Preconditions".to_string())
        );
    }
}
