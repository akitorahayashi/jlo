# jo Development Overview

## Project Summary
`jo` is a CLI tool that deploys and manages minimal `.jules/` workspace scaffolding for scheduled LLM agent execution. It creates a simple structure where agents can read project context (via `AGENTS.md` and past reports) and write analysis reports without modifying product code. The v0 design follows a single-scheduled-prompt model: each scheduled task runs one self-contained prompt stored as `.jules/roles/<role>/prompt.yml`. All `.jules/` content is in Japanese, while file/directory names are in English.

## Tech Stack
- **Language**: Rust
- **CLI Parsing**: `clap`
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
- **Functions and Variables**: `snake_case` (e.g., `scaffold_role`, `read_role_prompt`)
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
- **Scaffold embedding**: `src/scaffold.rs` loads static files from `src/scaffold/.jules/` for deployment, plus built-in role definitions from `src/role_kits/`.
- **Workspace abstraction**: `src/workspace.rs` provides a `Workspace` struct for all `.jules/` directory operations, including role discovery and prompt file access.
- **Version management**: `.jo-version` tracks which jo version last deployed the workspace, enabling update detection.

## CLI Commands (v0)
- `jo init` (alias: `i`): Create minimal `.jules/` structure and scaffold the default `taxonomy` role.
- `jo update` (alias: `u`): Update jo-managed files (README, .jo-version).
- `jo role` (alias: `r`): Show interactive menu with existing + built-in roles, scaffold if needed, print the selected role's `prompt.yml` to stdout.

## Workspace Contract (v0)
- `.jules/README.md`: English explanation, jo-managed
- `.jules/.jo-version`: Version marker, jo-managed
- `.jules/roles/<role>/prompt.yml`: Scheduler prompt material, user-owned
- `.jules/roles/<role>/reports/`: Report accumulation directory, user-owned
- `.jules/roles/<role>/reports/.gitkeep`: Structural placeholder, user-owned

## Built-in Roles (v0)
- **taxonomy**: Naming and terminology consistency analysis (only built-in role)

## Prompt Output Logic
When `jo role` is executed:
1. Validate `.jules/` exists (fail early before menu)
2. Discover existing roles via `Workspace::discover_roles()` (scan for `prompt.yml` files)
3. Get built-in role definitions via `scaffold::role_definitions()` (only taxonomy)
4. Show menu: existing roles first, then missing built-ins
5. If user selects built-in that doesn't exist: scaffold it via `Workspace::scaffold_role()`
6. Print `.jules/roles/<role>/prompt.yml` to stdout

## Language Policy
- **Scaffold Content**: English (`prompt.yml`, README.md)
- **File/Directory Names**: English (`roles/`, `reports/`, `prompt.yml`, `.gitkeep`)
- **CLI Messages**: English (stdout/stderr)
- **Code Comments**: English
