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
  input_type: InputType;
  unit?: string;
  options: string[];
  expected?: ExpectedValue;
}

export type InputType = "measurement" | "text" | "selection";

export type ExpectedValue = { min: number; max: number } | string;

// -- Execution types --

export interface ExecutionSummary {
  execution_id: string;
  procedure_id: string;
  procedure_version: string;
  operator: string;
  status: string;
  steps: StepSummary[];
}

export interface StepSummary {
  heading: string;
  status: string;
  checkboxes: CheckboxState[];
  input_definitions: InputDefinition[];
  inputs: InputState[];
  notes: string[];
}

export interface CheckboxState {
  text: string;
  checked: boolean;
}

export interface InputState {
  label: string;
  value: string;
  unit?: string;
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
  | { action: "record_deviation"; description: string; justification: string }
  | {
      action: "add_attachment";
      filename: string;
      path: string;
      content_type: string;
    }
  | { action: "complete"; status: "pass" | "fail" | "aborted" }
  | { action: "abort"; reason: string };
