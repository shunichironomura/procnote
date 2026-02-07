# Instructions for Claude

- This project is a procedure execution tool called `procnote`.
- This project is not yet published. So backward compatibility is not a concern. Make the simplest possible implementation.
- `procnote` uses filesystem-based storage with append-only JSONL event logs.
- Use filesystem and avoid in-memory caches as much as possible, to ensure crash safety, git-friendliness, and avoid cache invalidation issues.

## Misc

- The discussions logs can be found in the `.local` directory. Note that some of the documents are ideas that have been discarded later.
