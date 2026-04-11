---
icon: material/shield-check
---

# Crash Safety & Auditability

Procnote is designed for environments where data integrity and traceability are non-negotiable.

## Crash Safety

### Append-Only Event Log

The event log (`events.jsonl`) is append-only. Events are never modified or deleted. If the app crashes mid-operation, the log contains all events that were successfully written. On restart, the app re-reads the log and replays it to reconstruct the execution state.

### No In-Memory Cache

Every action re-reads and replays the full event log from disk. There is no in-memory cache that can become stale or inconsistent. This design trades a small amount of performance for strong consistency guarantees.

### Template Snapshots

When an execution starts, the procedure template is copied into the execution directory. The execution record is self-contained and unaffected by later template edits.

## Auditability

### Immutable Event History

Every operator action -- toggling a checkbox, entering a measurement, completing a step -- is recorded as an event with a timestamp. The event log is a complete, ordered record of everything that happened during the execution.

### Revert Audit Trail

When an action is reverted, the original event is not deleted. Instead, an `EventReverted` marker is appended with the target event index and a reason. This means you can always answer:

- What was the original value?
- When was it changed?
- Why was it reverted?

### File Integrity

Attachments are stored with a SHA-256 hash prefix in their filename (e.g., `a1b2c3d-report.pdf`), providing a built-in integrity check.

### Git-Friendly

The entire execution record -- event log, template snapshot, and attachments -- lives on the filesystem as plain files. This means executions can be committed to Git, diff'd, reviewed, and shared like any other project artifact.
