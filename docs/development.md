---
icon: material/code-braces
---

# Development

## Prerequisites

- Rust stable toolchain
- Node.js
- [pnpm](https://pnpm.io/) package manager
- [just](https://github.com/casey/just) command runner
- [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)

## Setup

```bash
# Install frontend dependencies
pnpm install

# Run the development server
just dev
```

This starts the Tauri dev server with the `procedures/` directory in the project root as the workspace. Hot-reloading is enabled for both the Rust backend and the Svelte frontend.

## Architecture

Procnote is a three-layer application:

| Layer | Location | Purpose |
|-------|----------|---------|
| **Core** | `crates/procnote-core/` | Pure Rust domain logic (events, state machine, template parser) |
| **Tauri shell** | `src-tauri/` | Bridges core to desktop via IPC commands, filesystem I/O |
| **Frontend** | `src/` | SvelteKit + Svelte 5 UI |

Dependency direction is strictly one-way: Frontend -> Tauri shell -> Core.

## Common Commands

```bash
# Run all Rust tests
cargo test --workspace

# TypeScript type checking
npx svelte-check

# Frontend linting and formatting
pnpm biome check .

# Regenerate TypeScript types from Rust
cargo test --workspace export_bindings_

# Format Rust code
cargo fmt

# Run all checks
just check-all
```

## TypeScript Type Generation

TypeScript types in `src/lib/types/generated/` are auto-generated from Rust structs via [ts-rs](https://github.com/Aleph-Alpha/ts-rs). After changing Rust DTOs, regenerate them:

```bash
cargo test --workspace export_bindings_
```

CI enforces that generated types stay in sync with Rust definitions.

## Logging

The Tauri log plugin writes to stdout, the log directory, and the webview console. In debug mode, the log level is `Debug`.

```bash
# Tail the log file
tail -f ~/Library/Logs/com.github.shunichironomura.procnote/procnote.log
```
