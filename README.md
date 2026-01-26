# jo

`jo` is a CLI tool that deploys and manages minimal `.jules/` workspace scaffolding for scheduled LLM agent execution. It creates a simple, consistent structure where agents can read project context and write analysis reports without modifying product code.

## Design Philosophy

- **Single-Scheduled-Prompt**: Each scheduled task runs one self-contained prompt
- **Stateless Execution**: Agents read repo files for context (no in-memory state)
- **Japanese Content**: All `.jules/` content is in Japanese; file/directory names are English

## `.jules/` Structure

```text
.jules/
  README.md           # English explanation of workspace
  .jo-version         # Version marker for updates
  roles/              # Role-scoped workspaces
    <role>/
      prompt.yml      # Role prompt material (Japanese, pasteable as-is)
      reports/        # Accumulated analysis reports
        YYYY-MM-DD_HHMMSS.md
```

## Quick Start

```bash
cargo install --path .
cd your-project
jo init
jo role
```

The `jo init` command creates the minimal `.jules/` structure. The `jo role` command shows an interactive menu to select a role and prints a ready-to-paste scheduler prompt.

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jo init` | `i` | Create minimal `.jules/` structure |
| `jo update` | `u` | Update jo-managed files (README, version) |
| `jo role` | `r` | Interactive role selection and prompt output to stdout |

## Workflow

1. **Initialize**: Run `jo init` to create `.jules/`
2. **Select Role**: Run `jo role` to see available roles
3. **Get Prompt**: Select a role (scaffolds if built-in) and copy the printed `prompt.yml`
4. **Schedule**: Paste prompt into scheduler (e.g., Jules GUI)
5. **Execute**: Scheduler runs prompt; agent writes report to `.jules/roles/<role>/reports/`

## Built-in Roles

- **taxonomy**: Analyzes naming and terminology consistency across the repository
`jo init` scaffolds `taxonomy` by default so the structure is visible immediately.

## Version Management

The `.jules/.jo-version` file tracks which version of `jo` last managed the workspace. Running `jo update` refreshes managed files and checks for modifications.

## Language Policy

- **File/Directory Names**: English only (e.g., `roles/`, `reports/`, `prompt.yml`)
- **File Contents**: Japanese only (`prompt.yml`, reports)
- **CLI Output**: English (command-line messages, errors)

## Managed Files

`jo update` only touches these files:
- `.jules/README.md`
- `.jules/.jo-version`

Everything else (user reports, user-created roles) is never touched by `jo`.

## Development Commands

- `cargo build` — build a debug binary
- `cargo build --release` — build the optimized release binary
- `cargo fmt` — format code using rustfmt
- `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings` — lint
- `cargo test --all-targets --all-features` — run all tests

## Testing Culture

- **Unit Tests**: Live alongside their modules inside `src/`
- **Command Logic Tests**: Each command module includes `#[cfg(test)]` tests
- **Integration Tests**: Located in `tests/` directory covering CLI workflows and library API

## Project Structure

```
jo/
├── src/
│   ├── main.rs           # CLI parsing (clap)
│   ├── lib.rs            # Public API
│   ├── scaffold.rs       # Embedded scaffold loader
│   ├── scaffold/         # Embedded .jules content
│   │   └── .jules/       # Scaffold files
│   ├── role_kits/        # Built-in role definitions
│   │   └── taxonomy/
│   ├── error.rs          # AppError definitions
│   ├── workspace.rs      # Workspace filesystem operations
│   └── commands/         # Command implementations
│       ├── mod.rs
│       ├── init.rs
│       ├── update.rs
│       └── role.rs
└── tests/
    ├── common/           # Shared test fixtures
    ├── cli_commands.rs   # CLI command tests
    ├── cli_flow.rs       # CLI workflow tests
    ├── commands_api.rs   # Library API tests
    └── commands_core.rs  # Error handling tests
```
