# jo Development Overview

## Project Summary
`jo` is a CLI tool that deploys and manages `.jules/` workspace scaffolding for scheduled LLM agent execution. It implements a **PM/Worker agent organization layer** where specialized worker agents maintain persistent memory (`notes/`), propose improvements (`reports/`), and a PM agent screens and converts proposals into actionable issues (`issues/`).

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
- **Functions and Variables**: `snake_case` (e.g., `scaffold_role`, `read_role_config`)
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
- **Workspace abstraction**: `src/workspace.rs` provides a `Workspace` struct for all `.jules/` directory operations, including role discovery and config access.
- **Version management**: `.jo-version` tracks which jo version last deployed the workspace, enabling update detection.

## CLI Commands
- `jo init` (alias: `i`): Create complete `.jules/` structure with all 4 built-in roles.
- `jo update` (alias: `u`): Update jo-managed files (README, .jo-version).
- `jo role` (alias: `r`): Show interactive menu with roles, print selected role's `role.yml` to stdout.

## Workspace Contract (v1)

### Directory Structure
```
.jules/
├── README.md           # Workflow documentation (jo-managed)
├── .jo-version         # Version marker (jo-managed)
│
├── roles/              # Agent workspaces
│   ├── <role>/         # Worker role
│   │   ├── role.yml    # Role definition (user-owned)
│   │   └── notes/      # Persistent memory (user-owned)
│   └── pm/             # PM role (special)
│       ├── role.yml    # PM definition (user-owned)
│       └── policy.md   # Decision criteria (user-owned)
│
├── reports/            # Proposals from Workers (user-owned)
│   └── YYYY-MM-DD_<role>_<title>.md
│
└── issues/             # Approved tasks from PM (user-owned)
    ├── bugs/           # Bug fixes
    ├── refacts/        # Refactoring
    ├── updates/        # New features
    ├── tests/          # Test-only changes
    └── docs/           # Documentation-only changes
```

### File Ownership
- **jo-managed**: `README.md`, `.jo-version` (overwritten by `jo update`)
- **user-owned**: Everything else (never modified by jo)

## Built-in Roles

| Role | Type | Responsibility |
|------|------|----------------|
| `taxonomy` | Worker | Naming conventions, terminology consistency |
| `data_arch` | Worker | Data models, data flow efficiency |
| `qa` | Worker | Test coverage, test quality |
| `pm` | Manager | Proposal review, issue creation |

### Worker Behavior
Workers read source code and their `notes/` directory, update notes with current understanding (declarative state), and create proposals in `reports/` when improvements are found. Workers do NOT write to `issues/`.

### PM Behavior
PM reads proposals from `reports/`, screens against `policy.md` criteria, and converts approved proposals to `issues/<category>/*.md`. Only PM writes to `issues/`.

## Role Configuration Schema
Each role has a `role.yml` file defining:
- `role`: Identifier
- `type`: `worker` or `manager`
- `goal`: Purpose description
- `memory`: Notes directory configuration (workers only)
- `reporting`: How to create proposals
- `behavior`: Read/write patterns and constraints

## Language Policy
- **Scaffold Content**: English (README.md)
- **File/Directory Names**: English (`roles/`, `reports/`, `issues/`, `notes/`, `role.yml`)
- **Role Content**: Japanese (role.yml, notes, reports, issues)
- **CLI Messages**: English (stdout/stderr)
- **Code Comments**: English
