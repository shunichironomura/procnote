use std::collections::{HashMap, HashSet};

use chrono::Utc;
use uuid::Uuid;

use crate::event::types::{CompletionStatus, Event, ExecutionId, Revertibility};
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
    #[error("event index out of range: {0}")]
    EventIndexOutOfRange(usize),
    #[error("event at index {0} is not revertible")]
    EventNotRevertible(usize),
    #[error("event at index {0} has already been reverted")]
    EventAlreadyReverted(usize),
    #[error("reverting event at index {0} would produce an invalid state: {1}")]
    RevertWouldInvalidateState(usize, String),
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
    pub description: Option<String>,
    pub status: StepStatus,
    /// Checkbox text -> checked state. Insertion order preserved by `step_order`.
    pub checkboxes: Vec<(String, bool)>,
    /// Input definitions for this step (from template or `StepAdded` event).
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
    pub procedure_title: Option<String>,
    pub procedure_version: Option<String>,
    pub name: Option<String>,

    pub status: ExecutionStatus,
    /// Ordered step headings (preserves insertion order).
    pub step_order: Vec<String>,
    pub steps: HashMap<String, StepState>,
    pub global_notes: Vec<String>,
}

impl ExecutionState {
    /// Create a new empty execution state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            execution_id: None,
            procedure_id: None,
            procedure_title: None,
            procedure_version: None,
            name: None,
            status: ExecutionStatus::Pending,
            step_order: Vec::new(),
            steps: HashMap::new(),
            global_notes: Vec::new(),
        }
    }

    /// Reconstruct execution state by replaying a sequence of events,
    /// respecting `EventReverted` markers.
    ///
    /// This collects all reverted indices first, then replays only
    /// non-reverted events. `EventReverted` events themselves are skipped.
    pub fn from_events(events: &[Event]) -> Result<Self, ExecutionError> {
        // First pass: collect all reverted event indices.
        let reverted_indices: HashSet<usize> = events
            .iter()
            .filter_map(|event| match event {
                Event::EventReverted {
                    reverted_event_index,
                    ..
                } => Some(*reverted_event_index),
                _ => None,
            })
            .collect();

        // Second pass: replay non-reverted, non-marker events.
        let mut state = Self::new();
        for (index, event) in events.iter().enumerate() {
            if reverted_indices.contains(&index) {
                continue;
            }
            if matches!(event, Event::EventReverted { .. }) {
                continue;
            }
            state.apply(event)?;
        }
        Ok(state)
    }

    /// Apply a single event to the state (used by both replay and transitions).
    #[expect(
        clippy::too_many_lines,
        reason = "exhaustive match over all Event variants for state machine"
    )]
    pub fn apply(&mut self, event: &Event) -> Result<(), ExecutionError> {
        match event {
            Event::ExecutionStarted {
                execution_id,
                procedure_id,
                procedure_title,
                procedure_version,
                ..
            } => {
                if self.status != ExecutionStatus::Pending {
                    return Err(ExecutionError::AlreadyStarted);
                }
                self.execution_id = Some(*execution_id);
                self.procedure_id = Some(procedure_id.clone());
                self.procedure_title = Some(procedure_title.clone());
                self.procedure_version = Some(procedure_version.clone());
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
                description,
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
                    description: description.clone(),
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
                    StepStatus::Pending | StepStatus::Active => {
                        step.status = StepStatus::Skipped;
                    }
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

            Event::AttachmentAdded {
                step_heading,
                label,
                filename,
                ..
            } => {
                self.require_active()?;
                let step = self.get_step_mut(step_heading)?;
                step.inputs.insert(
                    label.clone(),
                    RecordedInput {
                        label: label.clone(),
                        value: filename.clone(),
                        unit: None,
                    },
                );
            }

            Event::ExecutionRenamed { name, .. } => {
                if self.execution_id.is_none() {
                    return Err(ExecutionError::NotStarted);
                }
                self.name = Some(name.clone());
            }

            // EventReverted is handled at the from_events() level by skipping
            // reverted events. It should not be applied directly.
            Event::EventReverted { .. } => {}
        }
        Ok(())
    }

    // -- Transition methods: produce events --

    /// Start a new execution from a template.
    pub fn start(&mut self, template: &ProcedureTemplate) -> Result<Vec<Event>, ExecutionError> {
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
            procedure_title: template.metadata.title.clone(),
            procedure_version: template.metadata.version.clone(),
        };
        self.apply(&started)?;
        events.push(started);

        // Auto-generate a name for the execution.
        let auto_name = names::Generator::default()
            .next()
            .unwrap_or_else(|| format!("execution-{}", &execution_id.to_string()[..8]));
        let named = Event::ExecutionRenamed {
            at: now,
            execution_id,
            name: auto_name,
        };
        self.apply(&named)?;
        events.push(named);

        // Add steps from the template, including checkboxes and input definitions.
        for step in &template.steps {
            let mut checkboxes = Vec::new();
            let mut input_defs = Vec::new();
            let mut prose_parts = Vec::new();
            for content in &step.content {
                match content {
                    StepContent::Checkbox { text, .. } => {
                        checkboxes.push(text.clone());
                    }
                    StepContent::InputBlock { inputs } => {
                        input_defs.extend(inputs.iter().cloned());
                    }
                    StepContent::Prose { text } => {
                        prose_parts.push(text.clone());
                    }
                }
            }
            let description = if prose_parts.is_empty() {
                None
            } else {
                Some(prose_parts.join("\n\n"))
            };
            let step_added = Event::StepAdded {
                at: now,
                execution_id,
                heading: step.heading.clone(),
                description,
                after_step: None,
                checkboxes,
                inputs: input_defs,
            };
            self.apply(&step_added)?;
            events.push(step_added);
        }

        Ok(events)
    }

    /// Rename the execution.
    ///
    /// Unlike most actions, this works on both active and finished executions
    /// (it's metadata, not a state transition).
    pub fn rename(&mut self, name: &str) -> Result<Event, ExecutionError> {
        let event = Event::ExecutionRenamed {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            name: name.to_string(),
        };
        self.apply(&event)?;
        Ok(event)
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
            description: description.map(std::string::ToString::to_string),
            after_step: after_step.map(std::string::ToString::to_string),
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
            unit: unit.map(std::string::ToString::to_string),
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
            step_heading: step_heading.map(std::string::ToString::to_string),
        };
        self.apply(&event)?;
        Ok(event)
    }

    /// Add an attachment.
    pub fn add_attachment(
        &mut self,
        step_heading: &str,
        label: &str,
        filename: &str,
        path: &str,
        content_type: &str,
        sha256: &str,
    ) -> Result<Event, ExecutionError> {
        self.require_active()?;
        let event = Event::AttachmentAdded {
            at: Utc::now(),
            execution_id: self.require_execution_id()?,
            step_heading: step_heading.to_string(),
            label: label.to_string(),
            filename: filename.to_string(),
            path: path.to_string(),
            content_type: content_type.to_string(),
            sha256: sha256.to_string(),
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

    // -- Revert --

    /// Produce an `EventReverted` marker for the event at the given index.
    ///
    /// Validates that the event is revertible, not already reverted, and that
    /// the resulting state would be consistent (via trial replay).
    pub fn revert_event(
        all_events: &[Event],
        event_index: usize,
        reason: &str,
    ) -> Result<Event, ExecutionError> {
        // Validate index is in range.
        let target_event = all_events
            .get(event_index)
            .ok_or(ExecutionError::EventIndexOutOfRange(event_index))?;

        // Validate the event is revertible.
        match target_event.revertibility() {
            Revertibility::Revertible => {}
            Revertibility::NotRevertible | Revertibility::RevertMarker => {
                return Err(ExecutionError::EventNotRevertible(event_index));
            }
        }

        // Check it hasn't already been reverted.
        let already_reverted = all_events.iter().any(|e| {
            matches!(
                e,
                Event::EventReverted {
                    reverted_event_index,
                    ..
                } if *reverted_event_index == event_index
            )
        });
        if already_reverted {
            return Err(ExecutionError::EventAlreadyReverted(event_index));
        }

        // Extract execution_id from the first event.
        let execution_id = match &all_events[0] {
            Event::ExecutionStarted { execution_id, .. } => *execution_id,
            _ => return Err(ExecutionError::NotStarted),
        };

        let revert_marker = Event::EventReverted {
            at: Utc::now(),
            execution_id,
            reverted_event_index: event_index,
            reason: reason.to_string(),
        };

        // Validate by trial replay: append the marker and rebuild.
        let mut trial_events = all_events.to_vec();
        trial_events.push(revert_marker.clone());
        Self::from_events(&trial_events)
            .map_err(|e| ExecutionError::RevertWouldInvalidateState(event_index, e.to_string()))?;

        Ok(revert_marker)
    }

    // -- Helpers --

    const fn require_active(&self) -> Result<(), ExecutionError> {
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
#[expect(clippy::unwrap_used, reason = "unwrap is acceptable in tests")]
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
        let events = state.start(&template).unwrap();

        // 1 ExecutionStarted + 1 ExecutionRenamed + 3 StepAdded
        assert_eq!(events.len(), 5);
        assert_eq!(state.status, ExecutionStatus::Active);
        assert!(state.name.is_some());
        assert_eq!(state.step_order.len(), 3);
        assert_eq!(state.step_order[0], "Preconditions");
        assert_eq!(state.step_order[1], "Step 1: Power On");
        assert_eq!(state.step_order[2], "Postconditions");
    }

    #[test]
    fn test_rename_execution() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template).unwrap();

        let original_name = state.name.clone().unwrap();
        state.rename("my-custom-name").unwrap();
        assert_eq!(state.name.as_deref(), Some("my-custom-name"));
        assert_ne!(state.name.as_deref(), Some(original_name.as_str()));
    }

    #[test]
    fn test_rename_finished_execution() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template).unwrap();
        state.complete(CompletionStatus::Pass).unwrap();

        // Renaming should work even after completion.
        state.rename("post-finish-name").unwrap();
        assert_eq!(state.name.as_deref(), Some("post-finish-name"));
    }

    #[test]
    fn test_cannot_rename_before_start() {
        let mut state = ExecutionState::new();
        let result = state.rename("some-name");
        assert_eq!(result.unwrap_err(), ExecutionError::NotStarted);
    }

    #[test]
    fn test_full_execution_flow() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let mut all_events: Vec<Event> = Vec::new();

        // Start
        all_events.extend(state.start(&template).unwrap());

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
        state.start(&template).unwrap();

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
        state.start(&template).unwrap();
        let result = state.start(&template);
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
        state.start(&template).unwrap();
        state.complete(CompletionStatus::Pass).unwrap();

        let result = state.start_step("Preconditions");
        assert_eq!(result.unwrap_err(), ExecutionError::AlreadyFinished);
    }

    #[test]
    fn test_cannot_complete_unstarted_step() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template).unwrap();

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
        state.start(&template).unwrap();
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
        state.start(&template).unwrap();
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
        state.start(&template).unwrap();
        state.start_step("Step 1: Power On").unwrap();

        state
            .add_attachment(
                "Step 1: Power On",
                "Log file",
                "photo.jpg",
                "attachments/photo.jpg",
                "image/jpeg",
                "abc123",
            )
            .unwrap();

        let input = &state.steps["Step 1: Power On"].inputs["Log file"];
        assert_eq!(input.value, "photo.jpg");
    }

    #[test]
    fn test_global_note() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template).unwrap();

        state.add_note("General observation", None).unwrap();

        assert_eq!(state.global_notes.len(), 1);
        assert_eq!(state.global_notes[0], "General observation");
    }

    #[test]
    fn test_duplicate_step_heading() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        state.start(&template).unwrap();

        let result = state.add_step("Preconditions", None, None);
        assert_eq!(
            result.unwrap_err(),
            ExecutionError::DuplicateStepHeading("Preconditions".to_string())
        );
    }

    // -- Revert tests --

    /// Helper: build events for a started-and-completed step scenario.
    fn events_with_completed_step() -> Vec<Event> {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let mut events: Vec<Event> = Vec::new();
        events.extend(state.start(&template).unwrap());
        // indices 0..4: ExecutionStarted + ExecutionRenamed + 3 StepAdded
        events.push(state.start_step("Preconditions").unwrap()); // index 5
        events.push(state.complete_step("Preconditions").unwrap()); // index 6
        events
    }

    #[test]
    fn test_revert_step_completed() {
        let mut events = events_with_completed_step();
        // Revert StepCompleted at index 6
        let revert = ExecutionState::revert_event(&events, 6, "mistake").unwrap();
        events.push(revert);

        let state = ExecutionState::from_events(&events).unwrap();
        // Step should be back to Active (StepStarted still applies)
        assert_eq!(state.steps["Preconditions"].status, StepStatus::Active);
    }

    #[test]
    fn test_revert_step_started() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let mut events: Vec<Event> = Vec::new();
        events.extend(state.start(&template).unwrap());
        events.push(state.start_step("Preconditions").unwrap()); // index 5

        let revert = ExecutionState::revert_event(&events, 5, "wrong step").unwrap();
        events.push(revert);

        let state = ExecutionState::from_events(&events).unwrap();
        assert_eq!(state.steps["Preconditions"].status, StepStatus::Pending);
    }

    #[test]
    fn test_revert_step_skipped() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let mut events: Vec<Event> = Vec::new();
        events.extend(state.start(&template).unwrap());
        events.push(state.skip_step("Preconditions", "N/A").unwrap()); // index 5

        let revert = ExecutionState::revert_event(&events, 5, "actually needed").unwrap();
        events.push(revert);

        let state = ExecutionState::from_events(&events).unwrap();
        assert_eq!(state.steps["Preconditions"].status, StepStatus::Pending);
    }

    #[test]
    fn test_revert_input_recorded() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let mut events: Vec<Event> = Vec::new();
        events.extend(state.start(&template).unwrap());
        events.push(state.start_step("Preconditions").unwrap());
        events.push(
            state
                .record_input("Preconditions", "Voltage", "5.0", Some("V"))
                .unwrap(),
        ); // index 6

        let revert = ExecutionState::revert_event(&events, 6, "wrong value").unwrap();
        events.push(revert);

        let state = ExecutionState::from_events(&events).unwrap();
        assert!(!state.steps["Preconditions"].inputs.contains_key("Voltage"));
    }

    #[test]
    fn test_revert_note_added() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let mut events: Vec<Event> = Vec::new();
        events.extend(state.start(&template).unwrap());
        events.push(state.add_note("oops", None).unwrap()); // index 5

        let revert = ExecutionState::revert_event(&events, 5, "typo").unwrap();
        events.push(revert);

        let state = ExecutionState::from_events(&events).unwrap();
        assert!(state.global_notes.is_empty());
    }

    #[test]
    fn test_revert_checkbox_toggled() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let mut events: Vec<Event> = Vec::new();
        events.extend(state.start(&template).unwrap());
        events.push(state.start_step("Preconditions").unwrap());
        events.push(
            state
                .toggle_checkbox("Preconditions", "Check A", true)
                .unwrap(),
        ); // index 6

        let revert = ExecutionState::revert_event(&events, 6, "undo check").unwrap();
        events.push(revert);

        let state = ExecutionState::from_events(&events).unwrap();
        // The checkbox was dynamically added by the toggle; reverting removes it entirely
        // since it was not in the template.
        assert!(
            !state.steps["Preconditions"]
                .checkboxes
                .iter()
                .any(|(t, _)| t == "Check A")
        );
    }

    #[test]
    fn test_revert_execution_completed() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let mut events: Vec<Event> = Vec::new();
        events.extend(state.start(&template).unwrap());
        events.push(state.complete(CompletionStatus::Pass).unwrap()); // index 5

        let revert = ExecutionState::revert_event(&events, 5, "not done yet").unwrap();
        events.push(revert);

        let state = ExecutionState::from_events(&events).unwrap();
        assert_eq!(state.status, ExecutionStatus::Active);
    }

    #[test]
    fn test_revert_attachment_added() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let mut events: Vec<Event> = Vec::new();
        events.extend(state.start(&template).unwrap());
        events.push(state.start_step("Step 1: Power On").unwrap()); // index 5
        events.push(
            state
                .add_attachment(
                    "Step 1: Power On",
                    "Log file",
                    "photo.jpg",
                    "path/photo.jpg",
                    "image/jpeg",
                    "abc123",
                )
                .unwrap(),
        ); // index 6

        let revert = ExecutionState::revert_event(&events, 6, "wrong file").unwrap();
        events.push(revert);

        let state = ExecutionState::from_events(&events).unwrap();
        assert!(
            !state.steps["Step 1: Power On"]
                .inputs
                .contains_key("Log file")
        );
    }

    #[test]
    fn test_cannot_revert_execution_started() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let events: Vec<Event> = state.start(&template).unwrap();

        let result = ExecutionState::revert_event(&events, 0, "nope");
        assert_eq!(result.unwrap_err(), ExecutionError::EventNotRevertible(0));
    }

    #[test]
    fn test_cannot_revert_step_added() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let events: Vec<Event> = state.start(&template).unwrap();
        // index 2 is the first StepAdded

        let result = ExecutionState::revert_event(&events, 2, "nope");
        assert_eq!(result.unwrap_err(), ExecutionError::EventNotRevertible(2));
    }

    #[test]
    fn test_cannot_revert_already_reverted() {
        let mut events = events_with_completed_step();
        let revert = ExecutionState::revert_event(&events, 6, "first").unwrap();
        events.push(revert);

        let result = ExecutionState::revert_event(&events, 6, "second");
        assert_eq!(result.unwrap_err(), ExecutionError::EventAlreadyReverted(6));
    }

    #[test]
    fn test_cannot_revert_out_of_range() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let events: Vec<Event> = state.start(&template).unwrap();

        let result = ExecutionState::revert_event(&events, 999, "nope");
        assert_eq!(
            result.unwrap_err(),
            ExecutionError::EventIndexOutOfRange(999)
        );
    }

    #[test]
    fn test_cannot_revert_step_started_when_completed_follows() {
        let events = events_with_completed_step();
        // Try to revert StepStarted (index 5), but StepCompleted (index 6) still exists.
        // Replay without StepStarted would fail because StepCompleted requires Active status.
        let result = ExecutionState::revert_event(&events, 5, "nope");
        assert!(matches!(
            result.unwrap_err(),
            ExecutionError::RevertWouldInvalidateState(5, _)
        ));
    }

    #[test]
    fn test_revert_then_redo_step() {
        let mut events = events_with_completed_step();
        // Revert StepCompleted at index 6
        let revert = ExecutionState::revert_event(&events, 6, "redo").unwrap();
        events.push(revert);

        // Now rebuild state and complete the step again
        let mut state = ExecutionState::from_events(&events).unwrap();
        assert_eq!(state.steps["Preconditions"].status, StepStatus::Active);
        events.push(state.complete_step("Preconditions").unwrap());

        let final_state = ExecutionState::from_events(&events).unwrap();
        assert_eq!(
            final_state.steps["Preconditions"].status,
            StepStatus::Completed
        );
    }

    #[test]
    fn test_revert_serialization_roundtrip() {
        let mut events = events_with_completed_step();
        let revert = ExecutionState::revert_event(&events, 6, "test reason").unwrap();
        events.push(revert.clone());

        // Serialize and deserialize
        let json = serde_json::to_string(&revert).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(revert, deserialized);

        // Rebuild state from deserialized events
        let jsons: Vec<String> = events
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect();
        let deserialized_events: Vec<Event> = jsons
            .iter()
            .map(|j| serde_json::from_str(j).unwrap())
            .collect();
        let state = ExecutionState::from_events(&deserialized_events).unwrap();
        assert_eq!(state.steps["Preconditions"].status, StepStatus::Active);
    }

    #[test]
    fn test_from_events_with_interleaved_reverts() {
        let template = sample_template();
        let mut state = ExecutionState::new();
        let mut events: Vec<Event> = Vec::new();
        events.extend(state.start(&template).unwrap());

        // Start and complete Preconditions
        events.push(state.start_step("Preconditions").unwrap()); // index 5
        events.push(state.complete_step("Preconditions").unwrap()); // index 6

        // Start and complete Step 1
        events.push(state.start_step("Step 1: Power On").unwrap()); // index 7
        events.push(
            state
                .record_input("Step 1: Power On", "Current", "120", Some("mA"))
                .unwrap(),
        ); // index 8
        events.push(state.complete_step("Step 1: Power On").unwrap()); // index 9

        // Revert Step 1 completion (index 9)
        let revert1 = ExecutionState::revert_event(&events, 9, "redo step 1").unwrap();
        events.push(revert1);

        // Also revert the input (index 8)
        let revert2 = ExecutionState::revert_event(&events, 8, "wrong reading").unwrap();
        events.push(revert2);

        let rebuilt = ExecutionState::from_events(&events).unwrap();
        assert_eq!(rebuilt.steps["Preconditions"].status, StepStatus::Completed);
        assert_eq!(rebuilt.steps["Step 1: Power On"].status, StepStatus::Active);
        assert!(
            !rebuilt.steps["Step 1: Power On"]
                .inputs
                .contains_key("Current")
        );
    }
}
