use serde::{Deserialize, Serialize};

/// A complete procedure template parsed from a Markdown file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcedureTemplate {
    pub metadata: ProcedureMetadata,
    pub steps: Vec<Step>,
}

/// YAML frontmatter metadata for a procedure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcedureMetadata {
    pub id: String,
    pub title: String,
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub equipment: Vec<Equipment>,
    #[serde(default)]
    pub requirement_traces: Vec<String>,
}

/// A piece of equipment referenced by the procedure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Equipment {
    pub id: String,
    pub name: String,
}

/// A single step in the procedure (corresponds to a `## ` heading).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Step {
    pub heading: String,
    pub content: Vec<StepContent>,
}

/// Content items within a step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum StepContent {
    /// Free-form prose text (Markdown source).
    Prose { text: String },
    /// A checkbox item from a task list (`- [ ]` or `- [x]`).
    Checkbox { text: String, checked: bool },
    /// A block of input definitions from a fenced `inputs` code block.
    InputBlock { inputs: Vec<InputDefinition> },
}

/// Definition of an input field that operators fill in during execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InputDefinition {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub input_type: InputType,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub expected: Option<ExpectedValue>,
}

/// The type of input an operator provides.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    Measurement,
    Text,
    Selection,
    Attachment,
}

/// Expected value for validation — either a range or an exact match.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ExpectedValue {
    Range { min: f64, max: f64 },
    Exact(String),
}
