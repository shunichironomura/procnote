# Default recipe: list available commands
default:
    @just --list

# --- Development ---

# Start Tauri development server (uses project-root procedures/ by default)
dev *args='':
    pnpm tauri dev -- -- --procedures-dir {{ justfile_directory() }}/procedures {{ args }}

# Start frontend-only dev server
dev-frontend:
    pnpm dev

# --- Build ---

# Build the full Tauri application
build:
    pnpm tauri build

# Build frontend only
build-frontend:
    pnpm build

# --- Check & Lint ---

# Run all checks (fmt, clippy, biome, svelte-check, type bindings, tests)
check-all: check-fmt check-clippy check-biome check-frontend check-types test

# Check Rust formatting
check-fmt:
    cargo fmt --all -- --check

# Run cargo clippy
check-clippy:
    cargo clippy --workspace -- -D warnings

# Run biome check
check-biome:
    pnpm run biome:check

# Run svelte-check
check-frontend:
    pnpm check

# --- Format ---

# Format Rust code
fmt:
    cargo fmt --all

# --- Type Bindings ---

# Generate TypeScript type bindings from Rust types
generate-types:
    cargo test --workspace export_bindings_

# Check that generated TypeScript types are up-to-date
check-types:
    cargo test --workspace export_bindings_
    git diff --exit-code src/lib/types/generated/

# --- Test ---

# Run all tests
test: test-rust

# Run Rust tests
test-rust:
    cargo test --workspace

# --- Utility ---

# Run pre-commit hooks on all files
pre-commit:
    pre-commit run --all-files

# Clean build artifacts
clean:
    cargo clean
    rm -rf build .svelte-kit node_modules/.vite
