use std::io::{BufRead, Write};
use std::path::Path;

use super::types::{Event, LogEntry};

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

/// Read all log entries from a JSONL event log.
///
/// Distinguishes between:
/// - **Known events**: lines that deserialize into a typed [`Event`].
/// - **Unknown events**: lines that are valid JSON but have an unrecognized
///   `"type"` value. These are preserved as [`LogEntry::Unknown`] so they are
///   never lost.
/// - **Corrupt lines**: lines that are not valid JSON (e.g., truncated writes
///   from a crash). These are skipped with a warning log.
pub fn read_log(path: &Path) -> Result<Vec<LogEntry>, EventLogError> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
    let total_lines = lines.len();
    let mut entries = Vec::new();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Try to deserialize as a known Event.
        match serde_json::from_str::<Event>(trimmed) {
            Ok(event) => {
                entries.push(LogEntry::Event(event));
            }
            Err(_) => {
                // Not a known event — check if it's valid JSON (unknown event type).
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    log::debug!(
                        "Unknown event type at line {}: {}",
                        line_idx + 1,
                        value
                            .get("type")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("<no type>")
                    );
                    entries.push(LogEntry::Unknown(value));
                } else {
                    // Invalid JSON — corrupt/truncated line.
                    let preview: String = trimmed.chars().take(100).collect();
                    if line_idx + 1 == total_lines {
                        log::warn!("Skipping truncated line at end of event log: {preview}");
                    } else {
                        log::warn!(
                            "Skipping corrupt line {} in event log: {preview}",
                            line_idx + 1,
                        );
                    }
                }
            }
        }
    }
    Ok(entries)
}

/// Convenience wrapper: read only the known [`Event`]s from a log, discarding
/// unknown entries. Prefer [`read_log`] when index-correctness matters (e.g.,
/// for reverts).
#[deprecated(note = "Use read_log() to preserve unknown events and correct indices")]
pub fn read_events(path: &Path) -> Result<Vec<Event>, EventLogError> {
    Ok(read_log(path)?
        .into_iter()
        .filter_map(|entry| match entry {
            LogEntry::Event(e) => Some(e),
            LogEntry::Unknown(_) => None,
        })
        .collect())
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "unwrap is acceptable in tests")]
mod tests {
    use super::*;
    use crate::event::types::{CompletionStatus, ExecutionId};
    use crate::template::types::StepContent;
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
                step_id: "step-0".to_string(),
            },
            Event::CheckboxToggled {
                at: now,
                execution_id: id,
                step_id: "step-0".to_string(),
                checkbox_id: "step-0/cb-0".to_string(),
                checked: true,
            },
            Event::StepCompleted {
                at: now,
                execution_id: id,
                step_id: "step-0".to_string(),
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

        let entries = read_log(&path).unwrap();
        assert_eq!(events.len(), entries.len());
        for (original, entry) in events.iter().zip(entries.iter()) {
            assert_eq!(&LogEntry::Event(original.clone()), entry);
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

        let entries = read_log(&path).unwrap();
        // Corrupt line is skipped — only two known events remain.
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], LogEntry::Event(events[0].clone()));
        assert_eq!(entries[1], LogEntry::Event(events[1].clone()));
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

        let entries = read_log(&path).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("events.jsonl");

        let events = sample_events();
        append_event(&path, &events[0]).unwrap();

        assert!(path.exists());
        let entries = read_log(&path).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_unknown_event_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        let events = sample_events();
        append_event(&path, &events[0]).unwrap();

        // Append an unknown event type (valid JSON, unrecognized "type").
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(
            file,
            r#"{{"type":"future_event","at":"2025-01-01T00:00:00Z","data":"hello"}}"#
        )
        .unwrap();
        drop(file);

        append_event(&path, &events[1]).unwrap();

        let entries = read_log(&path).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(matches!(&entries[0], LogEntry::Event(_)));
        assert!(matches!(&entries[1], LogEntry::Unknown(_)));
        assert!(matches!(&entries[2], LogEntry::Event(_)));

        // The unknown entry preserves its raw JSON.
        if let LogEntry::Unknown(raw) = &entries[1] {
            assert_eq!(raw["type"], "future_event");
            assert_eq!(raw["data"], "hello");
        }
    }

    #[test]
    fn test_corrupt_vs_unknown_distinction() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .unwrap();
        // Valid known event
        let event = &sample_events()[0];
        let json = serde_json::to_string(event).unwrap();
        writeln!(file, "{json}").unwrap();
        // Valid JSON but unknown type → should be Unknown
        writeln!(
            file,
            r#"{{"type":"brand_new","at":"2025-01-01T00:00:00Z"}}"#
        )
        .unwrap();
        // Invalid JSON → should be skipped (corrupt)
        writeln!(file, "{{not valid json").unwrap();
        // Valid JSON but no type field → should be Unknown
        writeln!(file, r#"{{"foo":"bar"}}"#).unwrap();
        drop(file);

        let entries = read_log(&path).unwrap();
        // Known event + 2 unknown entries (corrupt line skipped)
        assert_eq!(entries.len(), 3);
        assert!(matches!(&entries[0], LogEntry::Event(_)));
        assert!(matches!(&entries[1], LogEntry::Unknown(_)));
        assert!(matches!(&entries[2], LogEntry::Unknown(_)));
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
                step_id: "dyn-step-1".to_string(),
                heading: "New Step".to_string(),
                content: vec![StepContent::Prose {
                    text: "Added during execution".to_string(),
                }],
                after_step_id: Some("step-0".to_string()),
            },
            Event::StepStarted {
                at: now,
                execution_id: id,
                step_id: "step-0".to_string(),
            },
            Event::StepCompleted {
                at: now,
                execution_id: id,
                step_id: "step-0".to_string(),
            },
            Event::StepSkipped {
                at: now,
                execution_id: id,
                step_id: "step-1".to_string(),
                reason: "Not applicable".to_string(),
            },
            Event::CheckboxToggled {
                at: now,
                execution_id: id,
                step_id: "step-0".to_string(),
                checkbox_id: "step-0/cb-0".to_string(),
                checked: true,
            },
            Event::InputRecorded {
                at: now,
                execution_id: id,
                step_id: "step-0".to_string(),
                input_id: "current-draw".to_string(),
                value: "120".to_string(),
                unit: Some("mA".to_string()),
            },
            Event::NoteAdded {
                at: now,
                execution_id: id,
                text: "Observation noted".to_string(),
                step_id: Some("step-0".to_string()),
            },
            Event::AttachmentAdded {
                at: now,
                execution_id: id,
                step_id: "step-0".to_string(),
                input_id: "log-file".to_string(),
                filename: "photo.jpg".to_string(),
                path: "attachments/photo.jpg".to_string(),
                content_type: "image/jpeg".to_string(),
                sha256: "abc123".to_string(),
            },
            Event::ExecutionRenamed {
                at: now,
                execution_id: id,
                name: "New Name".to_string(),
            },
            Event::EventReverted {
                at: now,
                execution_id: id,
                reverted_event_index: 3,
                reason: "mistake".to_string(),
            },
            Event::LogMeta {
                at: now,
                version: 1,
                tool_version: "0.1.0".to_string(),
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
