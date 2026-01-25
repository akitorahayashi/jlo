# jo Development Overview

## Project Summary
`jo` is a CLI tool that deploys and manages `.jules/` workspace scaffolding for organizational memory. It standardizes a versioned policy/docs bundle into `.jules/` so scheduled LLM agents and humans read consistent structure in-repo. The tool scaffolds paths and files so outputs land in consistent directories without defining domain-specific roles. `jo update` overwrites jo-managed files (`.jules/.jo/`, `.jules/README.md`, and `.jules/**/.gitkeep`) and never overwrites user-owned content.

## Tech Stack
- **Language**: Rust
- **CLI Parsing**: `clap`
- **Date/Time**: `chrono`
- **Hashing**: `sha2`
- **Embedded scaffold**: `include_dir`
- **Interactive prompts**: `dialoguer`
- **Development Dependencies**:
  - `assert_cmd`
  - `assert_fs`
  - `predicates`
  - `serial_test`
  - `tempfile`

## Coding Standards
- **Formatter**: `rustfmt` is used for code formatting. Key rules include a maximum line width of 100 characters, crate-level import granularity, and grouping imports by standard, external, and crate modules.
- **Linter**: `clippy` is used for linting, with a strict policy of treating all warnings as errors (`-D warnings`).

## Naming Conventions
- **Structs and Enums**: `PascalCase` (e.g., `Workspace`, `Commands`)
- **Functions and Variables**: `snake_case` (e.g., `create_role`, `session_path`)
- **Modules**: `snake_case` (e.g., `cli_commands.rs`)

## Key Commands
- **Build (Debug)**: `cargo build`
- **Build (Release)**: `cargo build --release`
- **Format Check**: `cargo fmt --check`
- **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`
- **Test**: `cargo test --all-targets --all-features`

## Testing Strategy
- **Unit Tests**: Located within the `src/` directory alongside the code they test, covering helper utilities and workspace operations.
- **Command Logic Tests**: Found in `src/commands/`, each command module includes `#[cfg(test)]` tests.
- **Integration Tests**: Housed in the `tests/` directory, these tests cover the public library API and CLI user flows from an external perspective. Separate crates for API (`tests/commands_api.rs`) and CLI workflows (`tests/cli_commands.rs`, `tests/cli_flow.rs`), with shared fixtures in `tests/common/mod.rs`.

## Architectural Highlights
- **Two-tier structure**: `src/main.rs` handles CLI parsing, `src/lib.rs` exposes public APIs, and `src/commands/` keeps command logic testable.
- **Scaffold embedding**: `src/scaffold.rs` loads static files from `src/scaffold/.jules/` for deployment and role kits from `src/role_kits/`.
- **Workspace abstraction**: `src/workspace.rs` provides a `Workspace` struct for all `.jules/` directory operations.
- **Version management**: `.jo-version` tracks which jo version last deployed the workspace, enabling update detection.

## CLI Commands
- `jo init` (alias: `i`): Create `.jules/` skeleton and source-of-truth docs.
- `jo update` (alias: `u`): Update jo-managed docs/templates and structural placeholders.
- `jo update --force` (alias: `u -f`): Force overwrite jo-managed files.
- `jo status` (alias: `st`): Print version info and detect local modifications.
- `jo role [role_id]` (alias: `r`): Scaffold `.jules/roles/<role_id>/` workspace (interactive when omitted).
- `jo session <role_id> [--slug]` (alias: `s`): Create new session file.
