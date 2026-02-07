use std::io::{BufRead, Write};
use std::path::Path;

use super::types::Event;

/// Errors that can occur during event log operations.
#[derive(Debug, thiserror::Error)]
pub enum EventLogError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Append a single event to a JSONL file.
///
/// Creates the file (and parent directories) if it does not exist.
pub fn append_event(path: &Path, event: &Event) -> Result<(), EventLogError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let json = serde_json::to_string(event)?;
    writeln!(file, "{json}")?;
    Ok(())
}

/// Read all events from a JSONL file.
///
/// Skips empty lines and lines that fail to parse (for crash recovery).
pub fn read_events(path: &Path) -> Result<Vec<Event>, EventLogError> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Event>(trimmed) {
            Ok(event) => events.push(event),
            Err(_) => {
                // Skip corrupt/partial lines (crash recovery).
                continue;
            }
        }
    }
    Ok(events)
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "unwrap is acceptable in tests")]
mod tests {
    use super::*;
    use crate::event::types::{CompletionStatus, ExecutionId};
    use chrono::Utc;
    use uuid::Uuid;

    fn sample_execution_id() -> ExecutionId {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

    fn sample_events() -> Vec<Event> {
        let id = sample_execution_id();
        let now = Utc::now();
        vec![
            Event::ExecutionStarted {
                at: now,
                execution_id: id,
                procedure_id: "TVT-001".to_string(),
                procedure_title: "Thermal Vacuum Test".to_string(),
                procedure_version: "1.0".to_string(),
            },
            Event::StepStarted {
                at: now,
                execution_id: id,
                step_heading: "Preconditions".to_string(),
            },
            Event::CheckboxToggled {
                at: now,
                execution_id: id,
                step_heading: "Preconditions".to_string(),
                text: "Chamber pressure < 1e-5 Pa".to_string(),
                checked: true,
            },
            Event::StepCompleted {
                at: now,
                execution_id: id,
                step_heading: "Preconditions".to_string(),
            },
            Event::ExecutionCompleted {
                at: now,
                execution_id: id,
                status: CompletionStatus::Pass,
            },
        ]
    }

    #[test]
    fn test_round_trip_single_event() {
        let event = &sample_events()[0];
        let json = serde_json::to_string(event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(*event, deserialized);
    }

    #[test]
    fn test_append_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        let events = sample_events();
        for event in &events {
            append_event(&path, event).unwrap();
        }

        let read_back = read_events(&path).unwrap();
        assert_eq!(events.len(), read_back.len());
        for (original, read) in events.iter().zip(read_back.iter()) {
            assert_eq!(original, read);
        }
    }

    #[test]
    fn test_skip_corrupt_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        // Write a valid event, then a corrupt line, then another valid event.
        let events = sample_events();
        append_event(&path, &events[0]).unwrap();

        // Append a corrupt line directly.
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(file, "{{corrupt json line").unwrap();
        drop(file);

        append_event(&path, &events[1]).unwrap();

        let read_back = read_events(&path).unwrap();
        assert_eq!(read_back.len(), 2);
        assert_eq!(read_back[0], events[0]);
        assert_eq!(read_back[1], events[1]);
    }

    #[test]
    fn test_empty_lines_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        let events = sample_events();
        append_event(&path, &events[0]).unwrap();

        // Append empty lines.
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(file).unwrap();
        writeln!(file, "   ").unwrap();
        drop(file);

        append_event(&path, &events[1]).unwrap();

        let read_back = read_events(&path).unwrap();
        assert_eq!(read_back.len(), 2);
    }

    #[test]
    fn test_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("events.jsonl");

        let events = sample_events();
        append_event(&path, &events[0]).unwrap();

        assert!(path.exists());
        let read_back = read_events(&path).unwrap();
        assert_eq!(read_back.len(), 1);
    }

    #[test]
    fn test_all_event_types_serialize() {
        let id = sample_execution_id();
        let now = Utc::now();

        let all_events = vec![
            Event::ExecutionStarted {
                at: now,
                execution_id: id,
                procedure_id: "P-001".to_string(),
                procedure_title: "Procedure 001".to_string(),
                procedure_version: "1.0".to_string(),
            },
            Event::ExecutionCompleted {
                at: now,
                execution_id: id,
                status: CompletionStatus::Pass,
            },
            Event::ExecutionAborted {
                at: now,
                execution_id: id,
                reason: "Power failure".to_string(),
            },
            Event::StepAdded {
                at: now,
                execution_id: id,
                heading: "New Step".to_string(),
                description: Some("Added during execution".to_string()),
                after_step: Some("Preconditions".to_string()),
                checkboxes: vec![],
                inputs: vec![],
            },
            Event::StepStarted {
                at: now,
                execution_id: id,
                step_heading: "Step 1".to_string(),
            },
            Event::StepCompleted {
                at: now,
                execution_id: id,
                step_heading: "Step 1".to_string(),
            },
            Event::StepSkipped {
                at: now,
                execution_id: id,
                step_heading: "Step 2".to_string(),
                reason: "Not applicable".to_string(),
            },
            Event::CheckboxToggled {
                at: now,
                execution_id: id,
                step_heading: "Step 1".to_string(),
                text: "Check item".to_string(),
                checked: true,
            },
            Event::InputRecorded {
                at: now,
                execution_id: id,
                step_heading: "Step 1".to_string(),
                label: "Current".to_string(),
                value: "120".to_string(),
                unit: Some("mA".to_string()),
            },
            Event::NoteAdded {
                at: now,
                execution_id: id,
                text: "Observation noted".to_string(),
                step_heading: Some("Step 1".to_string()),
            },
            Event::AttachmentAdded {
                at: now,
                execution_id: id,
                step_heading: "Step 1".to_string(),
                label: "Log file".to_string(),
                filename: "photo.jpg".to_string(),
                path: "attachments/photo.jpg".to_string(),
                content_type: "image/jpeg".to_string(),
                sha256: "abc123".to_string(),
            },
        ];

        // Round-trip all event types through JSON.
        for event in &all_events {
            let json = serde_json::to_string(event).unwrap();
            let deserialized: Event = serde_json::from_str(&json).unwrap();
            assert_eq!(*event, deserialized);
        }
    }
}
