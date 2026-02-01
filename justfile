# Default recipe: list available commands
default:
    @just --list

# --- Development ---

# Start Tauri development server
dev:
    pnpm tauri dev

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

# Run all checks (fmt, clippy, svelte-check, tests)
check-all: check-fmt check-clippy check-frontend test

# Check Rust formatting
check-fmt:
    cargo fmt --all -- --check

# Run cargo clippy
check-clippy:
    cargo clippy --workspace -- -D warnings

# Run svelte-check
check-frontend:
    pnpm check

# --- Format ---

# Format Rust code
fmt:
    cargo fmt --all

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
