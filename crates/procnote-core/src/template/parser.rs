use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use super::types::{
    ExpectedValue, InputDefinition, InputType, ProcedureMetadata, ProcedureTemplate, Step,
    StepContent,
};

/// Errors that can occur during template parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("missing YAML frontmatter (expected `---` delimiters)")]
    MissingFrontmatter,
    #[error("invalid YAML frontmatter: {0}")]
    InvalidYaml(#[from] serde_yaml_ng::Error),
    #[error("invalid inputs block: {0}")]
    InvalidInputsBlock(String),
}

/// Parse a procedure Markdown file (with YAML frontmatter) into a `ProcedureTemplate`.
pub fn parse_template(source: &str) -> Result<ProcedureTemplate, ParseError> {
    let (frontmatter, body) = split_frontmatter(source)?;
    let metadata: ProcedureMetadata = serde_yaml_ng::from_str(frontmatter)?;
    let steps = parse_body(body)?;
    Ok(ProcedureTemplate { metadata, steps })
}

/// Split `---`-delimited YAML frontmatter from the Markdown body.
fn split_frontmatter(source: &str) -> Result<(&str, &str), ParseError> {
    let trimmed = source.trim_start();
    if !trimmed.starts_with("---") {
        return Err(ParseError::MissingFrontmatter);
    }
    let after_first = &trimmed[3..];
    let end = after_first
        .find("\n---")
        .ok_or(ParseError::MissingFrontmatter)?;
    let frontmatter = &after_first[..end];
    // Skip past the closing `---` and the newline after it.
    let body_start = end + 4; // "\n---".len()
    let body = after_first[body_start..]
        .strip_prefix('\n')
        .unwrap_or(&after_first[body_start..]);
    Ok((frontmatter, body))
}

/// Parse the Markdown body into a list of steps, split on `## ` headings.
fn parse_body(body: &str) -> Result<Vec<Step>, ParseError> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(body, options);
    let events: Vec<Event<'_>> = parser.collect();

    let mut steps: Vec<Step> = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_content: Vec<StepContent> = Vec::new();

    let mut i = 0;
    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::Heading { level, .. })
                if *level == pulldown_cmark::HeadingLevel::H2 =>
            {
                // Flush previous step.
                if let Some(heading) = current_heading.take() {
                    steps.push(Step {
                        heading,
                        content: std::mem::take(&mut current_content),
                    });
                }
                // Collect heading text.
                let heading_text = collect_heading_text(&events, &mut i);
                current_heading = Some(heading_text);
            }
            Event::TaskListMarker(checked) => {
                let checked = *checked;
                // The task list marker is followed by text events inside the list item.
                // Collect the text of this list item.
                let text = collect_task_text(&events, &mut i);
                current_content.push(StepContent::Checkbox {
                    text: text.trim().to_string(),
                    checked,
                });
            }
            Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced(lang)))
                if lang.as_ref() == "inputs" =>
            {
                // Collect the code block content.
                let code = collect_code_block(&events, &mut i);
                let inputs = parse_inputs_block(&code)?;
                current_content.push(StepContent::InputBlock { inputs });
            }
            Event::Start(Tag::Paragraph) => {
                let text = collect_paragraph_text(&events, &mut i);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    current_content.push(StepContent::Prose {
                        text: trimmed.to_string(),
                    });
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    // Flush last step.
    if let Some(heading) = current_heading.take() {
        steps.push(Step {
            heading,
            content: std::mem::take(&mut current_content),
        });
    }

    Ok(steps)
}

/// Collect the text content of a heading, advancing `i` past the heading end.
fn collect_heading_text(events: &[Event<'_>], i: &mut usize) -> String {
    let mut text = String::new();
    *i += 1; // skip Start(Heading)
    while *i < events.len() {
        match &events[*i] {
            Event::End(TagEnd::Heading(pulldown_cmark::HeadingLevel::H2)) => {
                *i += 1;
                break;
            }
            Event::Text(t) | Event::Code(t) => {
                text.push_str(t);
                *i += 1;
            }
            _ => {
                *i += 1;
            }
        }
    }
    text
}

/// Collect the text of a task list item after the `TaskListMarker`, advancing `i`.
fn collect_task_text(events: &[Event<'_>], i: &mut usize) -> String {
    let mut text = String::new();
    *i += 1; // skip TaskListMarker
    while *i < events.len() {
        match &events[*i] {
            Event::End(TagEnd::Item) => {
                *i += 1;
                break;
            }
            Event::Text(t) | Event::Code(t) => {
                text.push_str(t);
                *i += 1;
            }
            Event::SoftBreak | Event::HardBreak => {
                text.push(' ');
                *i += 1;
            }
            // Nested tags (e.g., emphasis) — just skip the tag markers.
            Event::Start(_) | Event::End(_) => {
                *i += 1;
            }
            _ => {
                *i += 1;
            }
        }
    }
    text
}

/// Collect content of a fenced code block, advancing `i` past the block end.
fn collect_code_block(events: &[Event<'_>], i: &mut usize) -> String {
    let mut code = String::new();
    *i += 1; // skip Start(CodeBlock)
    while *i < events.len() {
        match &events[*i] {
            Event::End(TagEnd::CodeBlock) => {
                *i += 1;
                break;
            }
            Event::Text(t) => {
                code.push_str(t);
                *i += 1;
            }
            _ => {
                *i += 1;
            }
        }
    }
    code
}

/// Collect text inside a paragraph, advancing `i` past the paragraph end.
fn collect_paragraph_text(events: &[Event<'_>], i: &mut usize) -> String {
    let mut text = String::new();
    *i += 1; // skip Start(Paragraph)
    while *i < events.len() {
        match &events[*i] {
            Event::End(TagEnd::Paragraph) => {
                *i += 1;
                break;
            }
            Event::Text(t) | Event::Code(t) => {
                text.push_str(t);
                *i += 1;
            }
            Event::SoftBreak => {
                text.push(' ');
                *i += 1;
            }
            Event::HardBreak => {
                text.push('\n');
                *i += 1;
            }
            // Inline elements (emphasis, strong, etc.)
            Event::Start(_) | Event::End(_) => {
                *i += 1;
            }
            _ => {
                *i += 1;
            }
        }
    }
    text
}

/// Parse a YAML inputs block into a list of `InputDefinition`s.
fn parse_inputs_block(code: &str) -> Result<Vec<InputDefinition>, ParseError> {
    // The inputs block is a YAML list of input definitions.
    // We need to handle the `expected` field specially since it can be a range or exact value.
    let raw: Vec<RawInputDefinition> =
        serde_yaml_ng::from_str(code).map_err(|e| ParseError::InvalidInputsBlock(e.to_string()))?;

    raw.into_iter()
        .map(std::convert::TryInto::try_into)
        .collect()
}

/// Intermediate representation for deserializing input definitions with flexible `expected`.
#[derive(Debug, Deserialize)]
struct RawInputDefinition {
    id: String,
    label: String,
    #[serde(rename = "type")]
    input_type: InputType,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    options: Vec<String>,
    #[serde(default)]
    expected: Option<serde_yaml_ng::Value>,
}

use serde::Deserialize;

impl TryFrom<RawInputDefinition> for InputDefinition {
    type Error = ParseError;

    fn try_from(raw: RawInputDefinition) -> Result<Self, Self::Error> {
        let expected = match raw.expected {
            None => None,
            Some(serde_yaml_ng::Value::Mapping(map)) => {
                let min = map
                    .get(serde_yaml_ng::Value::String("min".to_string()))
                    .and_then(serde_yaml_ng::Value::as_f64);
                let max = map
                    .get(serde_yaml_ng::Value::String("max".to_string()))
                    .and_then(serde_yaml_ng::Value::as_f64);
                match (min, max) {
                    (Some(min), Some(max)) => Some(ExpectedValue::Range { min, max }),
                    _ => {
                        return Err(ParseError::InvalidInputsBlock(
                            "expected range must have both `min` and `max`".to_string(),
                        ));
                    }
                }
            }
            Some(serde_yaml_ng::Value::String(s)) => Some(ExpectedValue::Exact(s)),
            Some(other) => Some(ExpectedValue::Exact(format!("{other:?}"))),
        };

        Ok(Self {
            id: raw.id,
            label: raw.label,
            input_type: raw.input_type,
            unit: raw.unit,
            options: raw.options,
            expected,
        })
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "unwrap is acceptable in tests")]
mod tests {
    use super::*;

    const SAMPLE_TEMPLATE: &str = r#"---
id: TVT-001
title: "Thermal Vacuum Test - Reaction Wheel Unit"
version: "1.0"
author: "Nomura"
equipment:
  - id: CHAMBER-A
    name: "Thermal Vacuum Chamber A"
requirement_traces:
  - REQ-RWU-TEMP-001
---

## Preconditions

- [ ] Chamber pressure < 1e-5 Pa
- [ ] DUT temperature stabilized at 25 deg C +/- 2 deg C
- [ ] EGSE connected and nominal

## Step 1: Power On Sequence

Connect PSU to DUT J1 connector. Set voltage to 5.0V. Enable output.

```inputs
- id: current-draw
  label: "Measure current draw"
  type: measurement
  unit: "mA"
  expected:
    min: 100
    max: 150
```

## Step 2: Functional Check

Execute self-test command via EGSE.

```inputs
- id: selftest-result
  label: "Self-test response"
  type: selection
  options: ["PASS", "FAIL", "TIMEOUT"]
  expected: "PASS"
```

## Postconditions

- [ ] DUT powered off
- [ ] Chamber returned to ambient
"#;

    #[test]
    fn test_parse_frontmatter() {
        let (fm, body) = split_frontmatter(SAMPLE_TEMPLATE).unwrap();
        assert!(fm.contains("TVT-001"));
        assert!(body.trim_start().starts_with("## Preconditions"));
    }

    #[test]
    fn test_parse_metadata() {
        let template = parse_template(SAMPLE_TEMPLATE).unwrap();
        assert_eq!(template.metadata.id, "TVT-001");
        assert_eq!(
            template.metadata.title,
            "Thermal Vacuum Test - Reaction Wheel Unit"
        );
        assert_eq!(template.metadata.version, "1.0");
        assert_eq!(template.metadata.author, Some("Nomura".to_string()));
        assert_eq!(template.metadata.equipment.len(), 1);
        assert_eq!(template.metadata.equipment[0].id, "CHAMBER-A");
        assert_eq!(template.metadata.requirement_traces.len(), 1);
    }

    #[test]
    fn test_parse_steps() {
        let template = parse_template(SAMPLE_TEMPLATE).unwrap();
        assert_eq!(template.steps.len(), 4);

        assert_eq!(template.steps[0].heading, "Preconditions");
        assert_eq!(template.steps[1].heading, "Step 1: Power On Sequence");
        assert_eq!(template.steps[2].heading, "Step 2: Functional Check");
        assert_eq!(template.steps[3].heading, "Postconditions");
    }

    #[test]
    fn test_parse_checkboxes() {
        let template = parse_template(SAMPLE_TEMPLATE).unwrap();
        let preconditions = &template.steps[0];

        let checkboxes: Vec<_> = preconditions
            .content
            .iter()
            .filter_map(|c| match c {
                StepContent::Checkbox { text, checked } => Some((text.clone(), *checked)),
                _ => None,
            })
            .collect();

        assert_eq!(checkboxes.len(), 3);
        assert_eq!(checkboxes[0].0, "Chamber pressure < 1e-5 Pa");
        assert!(!checkboxes[0].1);
        assert_eq!(
            checkboxes[1].0,
            "DUT temperature stabilized at 25 deg C +/- 2 deg C"
        );
        assert!(!checkboxes[1].1);
    }

    #[test]
    fn test_parse_prose() {
        let template = parse_template(SAMPLE_TEMPLATE).unwrap();
        let step1 = &template.steps[1];

        let prose: Vec<_> = step1
            .content
            .iter()
            .filter_map(|c| match c {
                StepContent::Prose { text } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(prose.len(), 1);
        assert!(prose[0].contains("Connect PSU to DUT J1 connector"));
    }

    #[test]
    fn test_parse_measurement_input() {
        let template = parse_template(SAMPLE_TEMPLATE).unwrap();
        let step1 = &template.steps[1];

        let inputs: Vec<_> = step1
            .content
            .iter()
            .filter_map(|c| match c {
                StepContent::InputBlock { inputs } => Some(inputs.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].len(), 1);

        let input = &inputs[0][0];
        assert_eq!(input.id, "current-draw");
        assert_eq!(input.label, "Measure current draw");
        assert_eq!(input.input_type, InputType::Measurement);
        assert_eq!(input.unit, Some("mA".to_string()));
        assert_eq!(
            input.expected,
            Some(ExpectedValue::Range {
                min: 100.0,
                max: 150.0
            })
        );
    }

    #[test]
    fn test_parse_selection_input() {
        let template = parse_template(SAMPLE_TEMPLATE).unwrap();
        let step2 = &template.steps[2];

        let inputs: Vec<_> = step2
            .content
            .iter()
            .filter_map(|c| match c {
                StepContent::InputBlock { inputs } => Some(inputs.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(inputs.len(), 1);
        let input = &inputs[0][0];
        assert_eq!(input.id, "selftest-result");
        assert_eq!(input.input_type, InputType::Selection);
        assert_eq!(input.options, vec!["PASS", "FAIL", "TIMEOUT"]);
        assert_eq!(
            input.expected,
            Some(ExpectedValue::Exact("PASS".to_string()))
        );
    }

    #[test]
    fn test_missing_frontmatter() {
        let result = parse_template("# No frontmatter\nSome text.");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::MissingFrontmatter
        ));
    }

    #[test]
    fn test_minimal_template() {
        let source = r#"---
id: MIN-001
title: "Minimal"
version: "0.1"
---

## Only Step

Just some text.
"#;
        let template = parse_template(source).unwrap();
        assert_eq!(template.metadata.id, "MIN-001");
        assert_eq!(template.steps.len(), 1);
        assert_eq!(template.steps[0].heading, "Only Step");
    }
}
