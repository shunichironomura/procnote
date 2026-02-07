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
        .unwrap_or_else(|| &after_first[body_start..]);
    Ok((frontmatter, body))
}

/// Parse the Markdown body into a list of steps, split on `## ` headings.
#[expect(
    clippy::too_many_lines,
    reason = "event-driven parser is inherently a large match loop"
)]
fn parse_body(body: &str) -> Result<Vec<Step>, ParseError> {
    use std::ops::Range;

    /// Flush accumulated prose from `body[prose_start..boundary]` into `content`.
    fn flush_prose(
        body: &str,
        prose_start: &mut Option<usize>,
        boundary: usize,
        content: &mut Vec<StepContent>,
    ) {
        if let Some(start) = prose_start.take() {
            let raw = body[start..boundary].trim();
            if !raw.is_empty() {
                content.push(StepContent::Prose {
                    text: raw.to_string(),
                });
            }
        }
    }

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(body, options);
    let events: Vec<(Event<'_>, Range<usize>)> = parser.into_offset_iter().collect();

    let mut steps: Vec<Step> = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_content: Vec<StepContent> = Vec::new();
    // Track the start of the current prose region in `body`.
    let mut prose_start: Option<usize> = None;
    // Nesting depth for lists (incremented on Start(List), decremented on End(List)).
    let mut list_depth: usize = 0;
    // Whether we are currently inside a pure task list (all items are checkboxes).
    // While true, we extract TaskListMarker items as Checkbox and suppress prose tracking.
    let mut in_task_list: bool = false;
    // When we determine a top-level list is mixed, record its depth so we skip
    // all TaskListMarker events (including nested ones) until we exit that list.
    let mut skip_markers_until_depth: Option<usize> = None;

    let mut i = 0;
    while i < events.len() {
        match &events[i].0 {
            Event::Start(Tag::Heading { level, .. })
                if *level == pulldown_cmark::HeadingLevel::H2 =>
            {
                // Flush any accumulated prose before this heading.
                flush_prose(
                    body,
                    &mut prose_start,
                    events[i].1.start,
                    &mut current_content,
                );
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
                prose_start = None;
                list_depth = 0;
                in_task_list = false;
                skip_markers_until_depth = None;
            }
            Event::Start(Tag::List(_)) => {
                list_depth += 1;
                i += 1;
            }
            Event::TaskListMarker(checked) => {
                // If we're inside a list that was determined to be mixed, skip.
                if skip_markers_until_depth.is_some() {
                    i += 1;
                    continue;
                }
                let checked = *checked;
                // On the first checkbox, check if the enclosing top-level list
                // is a pure task list.
                if !in_task_list {
                    let list_start_idx = {
                        let mut j = i;
                        while j > 0 {
                            j -= 1;
                            if matches!(&events[j].0, Event::Start(Tag::List(_))) {
                                break;
                            }
                        }
                        j
                    };
                    if is_pure_task_list(&events, list_start_idx) {
                        let list_start = events[list_start_idx].1.start;
                        flush_prose(body, &mut prose_start, list_start, &mut current_content);
                        in_task_list = true;
                    } else {
                        // Mixed list — skip all markers until we exit this list.
                        skip_markers_until_depth = Some(list_depth);
                        i += 1;
                        continue;
                    }
                }
                let text = collect_task_text(&events, &mut i);
                current_content.push(StepContent::Checkbox {
                    text: text.trim().to_string(),
                    checked,
                });
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                // Clear the mixed-list skip flag when we exit that list.
                if let Some(depth) = skip_markers_until_depth
                    && list_depth < depth
                {
                    skip_markers_until_depth = None;
                }
                if list_depth == 0 && in_task_list {
                    // Exiting the outermost task list — prose can start after this event.
                    in_task_list = false;
                    i += 1;
                    if i < events.len() {
                        prose_start = Some(events[i].1.start);
                    }
                } else {
                    i += 1;
                }
            }
            Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced(lang)))
                if lang.as_ref() == "inputs" =>
            {
                // Flush prose before this code block.
                flush_prose(
                    body,
                    &mut prose_start,
                    events[i].1.start,
                    &mut current_content,
                );
                let code = collect_code_block(&events, &mut i);
                let inputs = parse_inputs_block(&code)?;
                current_content.push(StepContent::InputBlock { inputs });
                // Prose resumes after code block.
                if i < events.len() {
                    prose_start = Some(events[i].1.start);
                }
            }
            _ => {
                // Any other event: start tracking as prose if not already,
                // unless we're inside a task list.
                if prose_start.is_none() && current_heading.is_some() && !in_task_list {
                    prose_start = Some(events[i].1.start);
                }
                i += 1;
            }
        }
    }

    // Flush trailing prose and the last step.
    flush_prose(body, &mut prose_start, body.len(), &mut current_content);
    if let Some(heading) = current_heading.take() {
        steps.push(Step {
            heading,
            content: std::mem::take(&mut current_content),
        });
    }

    Ok(steps)
}

/// Check whether the top-level list starting at `list_start_idx` is a *pure* task list
/// (every direct item has a `TaskListMarker`). Mixed lists (some items with markers,
/// some without) return `false` and should be treated as prose.
fn is_pure_task_list(
    events: &[(Event<'_>, std::ops::Range<usize>)],
    list_start_idx: usize,
) -> bool {
    let mut depth: usize = 0;
    let mut item_has_marker = false;
    let mut seen_item = false;
    let mut j = list_start_idx;

    while j < events.len() {
        match &events[j].0 {
            Event::Start(Tag::List(_)) => {
                depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                depth -= 1;
                if depth == 0 {
                    // End of the list we're inspecting.
                    // Check the last item.
                    if seen_item && !item_has_marker {
                        return false;
                    }
                    return seen_item; // empty list → false
                }
            }
            Event::Start(Tag::Item) if depth == 1 => {
                // Starting a new direct child item.
                // Check the previous item (if any).
                if seen_item && !item_has_marker {
                    return false;
                }
                seen_item = true;
                item_has_marker = false;
            }
            Event::TaskListMarker(_) if depth == 1 => {
                item_has_marker = true;
            }
            _ => {}
        }
        j += 1;
    }
    // Shouldn't reach here (list not properly closed), but be safe.
    false
}

/// Collect the text content of a heading, advancing `i` past the heading end.
fn collect_heading_text(events: &[(Event<'_>, std::ops::Range<usize>)], i: &mut usize) -> String {
    let mut text = String::new();
    *i += 1; // skip Start(Heading)
    while *i < events.len() {
        match &events[*i].0 {
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
fn collect_task_text(events: &[(Event<'_>, std::ops::Range<usize>)], i: &mut usize) -> String {
    let mut text = String::new();
    *i += 1; // skip TaskListMarker
    while *i < events.len() {
        match &events[*i].0 {
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
            // Nested tags (e.g., emphasis) and other events — just advance.
            _ => {
                *i += 1;
            }
        }
    }
    text
}

/// Collect content of a fenced code block, advancing `i` past the block end.
fn collect_code_block(events: &[(Event<'_>, std::ops::Range<usize>)], i: &mut usize) -> String {
    let mut code = String::new();
    *i += 1; // skip Start(CodeBlock)
    while *i < events.len() {
        match &events[*i].0 {
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

    #[test]
    fn test_prose_preserves_markdown() {
        let source = r#"---
id: MD-001
title: "Markdown Test"
version: "0.1"
---

## Step with rich prose

Here is a paragraph with **bold** and *italic* text.

- bullet point 1
- bullet point 2

### A sub-heading

```python
print("hello")
```

Some trailing text.

```inputs
- id: val
  label: "Value"
  type: measurement
  unit: "V"
```
"#;
        let template = parse_template(source).unwrap();
        assert_eq!(template.steps.len(), 1);

        let prose_parts: Vec<_> = template.steps[0]
            .content
            .iter()
            .filter_map(|c| match c {
                StepContent::Prose { text } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(prose_parts.len(), 1);
        let prose = &prose_parts[0];
        // All Markdown elements should be preserved as raw text.
        assert!(prose.contains("**bold**"), "bold not preserved: {prose}");
        assert!(prose.contains("*italic*"), "italic not preserved: {prose}");
        assert!(
            prose.contains("- bullet point 1"),
            "bullet list not preserved: {prose}"
        );
        assert!(
            prose.contains("### A sub-heading"),
            "sub-heading not preserved: {prose}"
        );
        assert!(
            prose.contains("```python"),
            "code block not preserved: {prose}"
        );
        assert!(
            prose.contains("Some trailing text."),
            "trailing text not preserved: {prose}"
        );
    }

    #[test]
    fn test_prose_between_checkboxes_and_inputs() {
        let source = r#"---
id: MIX-001
title: "Mixed Content"
version: "0.1"
---

## Mixed Step

- [ ] First check
- [ ] Second check

Some prose between checkboxes and inputs.

```inputs
- id: val
  label: "Value"
  type: measurement
  unit: "V"
```
"#;
        let template = parse_template(source).unwrap();
        assert_eq!(template.steps.len(), 1);

        let content = &template.steps[0].content;
        // Should have: Checkbox, Checkbox, Prose, InputBlock
        assert_eq!(content.len(), 4, "expected 4 content items: {content:?}");
        assert!(matches!(content[0], StepContent::Checkbox { .. }));
        assert!(matches!(content[1], StepContent::Checkbox { .. }));
        assert!(matches!(content[2], StepContent::Prose { .. }));
        assert!(matches!(content[3], StepContent::InputBlock { .. }));

        if let StepContent::Prose { text } = &content[2] {
            assert!(text.contains("Some prose between checkboxes and inputs"));
        }
    }

    #[test]
    fn test_mixed_regular_and_checkbox_items_as_prose() {
        let source = r#"---
id: MIX-002
title: "Mixed List Test"
version: "0.1"
---

## Step with mixed list

- bullet point 1
- [ ] a checkbox item
  - [ ] a nested checkbox item
"#;
        let template = parse_template(source).unwrap();
        assert_eq!(template.steps.len(), 1);

        let content = &template.steps[0].content;
        // Mixed list should be captured entirely as prose (not interactive checkboxes).
        assert_eq!(
            content.len(),
            1,
            "expected 1 content item (prose): {content:?}"
        );
        assert!(matches!(content[0], StepContent::Prose { .. }));

        if let StepContent::Prose { text } = &content[0] {
            assert!(
                text.contains("bullet point 1"),
                "should contain bullet: {text}"
            );
            assert!(
                text.contains("[ ] a checkbox item"),
                "should contain checkbox text: {text}"
            );
        }
    }

    #[test]
    fn test_nested_pure_checkboxes() {
        let source = r#"---
id: NEST-001
title: "Nested Checkbox Test"
version: "0.1"
---

## Step with nested checkboxes

- [ ] parent checkbox
  - [ ] nested checkbox
"#;
        let template = parse_template(source).unwrap();
        assert_eq!(template.steps.len(), 1);

        let checkboxes: Vec<_> = template.steps[0]
            .content
            .iter()
            .filter_map(|c| match c {
                StepContent::Checkbox { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        // Both should be captured as checkboxes (the outer list is pure task list).
        assert!(
            !checkboxes.is_empty(),
            "expected at least 1 checkbox: {checkboxes:?}"
        );
        assert!(
            checkboxes.iter().any(|t| t.contains("parent checkbox")),
            "should have parent checkbox: {checkboxes:?}"
        );
    }

    #[test]
    fn test_regular_bullet_list_as_prose() {
        let source = r#"---
id: BULLET-001
title: "Bullet List Test"
version: "0.1"
---

## Step with bullets

- item 1
- item 2
- item 3
"#;
        let template = parse_template(source).unwrap();
        assert_eq!(template.steps.len(), 1);

        let content = &template.steps[0].content;
        // Pure bullet list (no checkboxes) should be prose.
        assert_eq!(content.len(), 1, "expected 1 content item: {content:?}");
        assert!(matches!(content[0], StepContent::Prose { .. }));

        if let StepContent::Prose { text } = &content[0] {
            assert!(
                text.contains("- item 1"),
                "should contain bullet items: {text}"
            );
            assert!(
                text.contains("- item 3"),
                "should contain all items: {text}"
            );
        }
    }

    #[test]
    fn test_pure_task_list_then_regular_list() {
        let source = r#"---
id: SEQ-001
title: "Sequential Lists"
version: "0.1"
---

## Step with sequential lists

- [ ] check 1
- [ ] check 2

Some text between.

- bullet A
- bullet B
"#;
        let template = parse_template(source).unwrap();
        assert_eq!(template.steps.len(), 1);

        let content = &template.steps[0].content;
        // Should have: Checkbox, Checkbox, Prose (text + bullet list)
        let checkbox_count = content
            .iter()
            .filter(|c| matches!(c, StepContent::Checkbox { .. }))
            .count();
        let prose_count = content
            .iter()
            .filter(|c| matches!(c, StepContent::Prose { .. }))
            .count();

        assert_eq!(checkbox_count, 2, "expected 2 checkboxes: {content:?}");
        assert!(
            prose_count >= 1,
            "expected at least 1 prose block: {content:?}"
        );
    }

    #[test]
    fn test_same_marker_adjacent_lists_merged_as_prose() {
        // pulldown-cmark merges adjacent lists using the same bullet marker into
        // a single list. When that list contains both task items and regular items,
        // the whole list becomes prose.
        let source = r#"---
id: MERGE-001
title: "Merged List"
version: "0.1"
---

## Step

- [ ] check 1
- [ ] check 2

- bullet A
- bullet B
"#;
        let template = parse_template(source).unwrap();
        assert_eq!(template.steps.len(), 1);

        let content = &template.steps[0].content;
        // pulldown-cmark treats all 4 items as one list → mixed → prose
        assert_eq!(content.len(), 1, "expected 1 prose block: {content:?}");
        assert!(matches!(content[0], StepContent::Prose { .. }));
    }
}
