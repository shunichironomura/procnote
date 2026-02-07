// Types mirroring Rust structs for IPC communication.

// -- Template types --

export interface TemplateSummary {
  id: string;
  title: string;
  version: string;
  path: string;
}

export interface ProcedureTemplate {
  metadata: ProcedureMetadata;
  steps: Step[];
}

export interface ProcedureMetadata {
  id: string;
  title: string;
  version: string;
  author?: string;
  equipment: Equipment[];
  requirement_traces: string[];
}

export interface Equipment {
  id: string;
  name: string;
}

export interface Step {
  heading: string;
  content: StepContent[];
}

export type StepContent =
  | { type: "Prose"; text: string }
  | { type: "Checkbox"; text: string; checked: boolean }
  | { type: "InputBlock"; inputs: InputDefinition[] };

export interface InputDefinition {
  id: string;
  label: string;
  type: InputType;
  unit?: string;
  options: string[];
  expected?: ExpectedValue;
}

export type InputType = "measurement" | "text" | "selection" | "attachment";

export type ExpectedValue = { min: number; max: number } | string;

// -- Execution types --

export interface ExecutionSummary {
  execution_id: string;
  name?: string;
  procedure_id: string;
  procedure_title: string;
  procedure_version: string;
  status: string;
  started_at?: string;
  finished_at?: string;
  steps: StepSummary[];
  event_history: EventHistoryEntry[];
}

export interface EventHistoryEntry {
  index: number;
  event_type: string;
  at: string;
  description: string;
  revertible: boolean;
  reverted: boolean;
  step_heading?: string;
  label?: string;
}

export interface StepSummary {
  heading: string;
  description?: string;
  status: string;
  status_at?: string;
  checkboxes: CheckboxState[];
  input_definitions: InputDefinition[];
  inputs: InputState[];
  notes: NoteState[];
}

export interface CheckboxState {
  text: string;
  checked: boolean;
  at?: string;
}

export interface InputState {
  label: string;
  value: string;
  unit?: string;
  at?: string;
}

export interface NoteState {
  text: string;
  at?: string;
}

// -- Action types (sent to backend) --

export type ExecutionAction =
  | { action: "start_step"; step_heading: string }
  | { action: "complete_step"; step_heading: string }
  | { action: "skip_step"; step_heading: string; reason: string }
  | {
      action: "toggle_checkbox";
      step_heading: string;
      text: string;
      checked: boolean;
    }
  | {
      action: "record_input";
      step_heading: string;
      label: string;
      value: string;
      unit?: string;
    }
  | { action: "add_note"; text: string; step_heading?: string }
  | {
      action: "add_step";
      heading: string;
      description?: string;
      after_step?: string;
    }
  | {
      action: "add_attachment";
      step_heading: string;
      label: string;
      filename: string;
      path: string;
      content_type: string;
    }
  | { action: "complete"; status: "pass" | "fail" | "aborted" }
  | { action: "abort"; reason: string }
  | { action: "rename_execution"; name: string }
  | { action: "revert_event"; event_index: number; reason: string };
