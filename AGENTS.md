# jo Development Overview

## Project Summary
`jo` is a CLI tool that deploys and manages `.jules/` workspace scaffolding for scheduled LLM agent execution. It implements a **4-layer agent architecture** where specialized agents are organized by their operational responsibilities: Observers analyze code, Deciders screen events, Planners decompose issues, and Implementers execute tasks.

## Tech Stack
- **Language**: Rust
- **CLI Parsing**: `clap`
- **Clipboard**: `arboard`
- **YAML Processing**: `serde`, `serde_yaml`
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
- **Structs and Enums**: `PascalCase` (e.g., `Workspace`, `Commands`, `Layer`)
- **Functions and Variables**: `snake_case` (e.g., `scaffold_role_in_layer`, `find_role_fuzzy`)
- **Modules**: `snake_case` (e.g., `cli_commands.rs`, `generator.rs`)

## Key Commands
- **Build (Debug)**: `cargo build`
- **Build (Release)**: `cargo build --release`
- **Format Check**: `cargo fmt --check`
- **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`
- **Test**: `cargo test --all-targets --all-features`

## Testing Strategy
- **Unit Tests**: Located within the `src/` directory alongside the code they test, covering domain types, services, and workspace operations.
- **Integration Tests**: Housed in the `tests/` directory, covering the public library API and CLI user flows. Separate test crates for API (`tests/commands_api.rs`) and CLI workflows (`tests/cli_commands.rs`, `tests/cli_flow.rs`), with shared fixtures in `tests/common/mod.rs`.

## Architecture

The codebase uses a **layered architecture** with clear separation of concerns:

### Layers

| Layer | Location | Responsibility |
|-------|----------|----------------|
| **Domain** | `src/domain/` | Pure types, enums, validation (no I/O). `Layer`, `RoleId`, `AppError`, constants. |
| **Ports** | `src/ports/` | Trait boundaries defining capabilities. `WorkspaceStore`, `RoleTemplateStore`, `ClipboardWriter`. |
| **Services** | `src/services/` | Concrete implementations with I/O. `FilesystemWorkspaceStore`, `EmbeddedRoleTemplateStore`, `ArboardClipboard`, `PromptGenerator`. |
| **App** | `src/app/` | `AppContext` (DI container) and command orchestration. Commands in `src/app/commands/`. |
| **Testing** | `src/testing/` | Mock implementations of ports for unit testing. |
| **Assets** | `src/assets/` | Static embedded content: scaffold files, role kits, templates. |

### Key Patterns

- **Dependency Injection**: `AppContext<W, R, C>` is generic over port traits, enabling mock injection in tests.
- **Port/Adapter Separation**: Traits in `ports/` define "what", services in `services/` provide "how".
- **Deferred Clipboard Initialization**: Clipboard is only initialized when actually needed (after validation), avoiding failures on headless systems.
- **Embedded Assets**: Static files are compiled into the binary via `include_dir!`.

## CLI Commands
- `jo init` (alias: `i`): Create complete `.jules/` structure with 4-layer architecture and all 6 built-in roles.
- `jo assign <role> [paths...]` (alias: `a`): Generate prompt for a role, inject paths, and copy to clipboard.
- `jo template [-l layer] [-n name]` (alias: `tp`): Create a new custom role from a layer archetype.

## Workspace Contract (v3)

### Directory Structure
```
.jules/
├── README.md           # Workflow documentation (jo-managed)
├── JULES.md            # Agent contract (jo-managed)
├── .jo-version         # Version marker (jo-managed)
│
├── roles/              # 4-Layer agent organization
│   ├── observers/      # Layer 1: Observation
│   │   ├── taxonomy/
│   │   │   ├── prompt.yml
│   │   │   ├── role.yml
│   │   │   └── notes/
│   │   ├── data_arch/
│   │   └── qa/
│   │
│   ├── deciders/       # Layer 2: Decision
│   │   └── triage/
│   │       ├── prompt.yml
│   │       └── role.yml
│   │
│   ├── planners/       # Layer 3: Planning
│   │   └── specifier/
│   │       ├── prompt.yml
│   │       └── role.yml
│   │
│   └── implementers/   # Layer 4: Implementation
│       └── executor/
│           ├── prompt.yml
│           └── role.yml
│
├── events/             # Normalized observations (user-owned)
│   ├── bugs/
│   ├── refacts/
│   ├── updates/
│   ├── tests/
│   └── docs/
│
├── issues/             # Actionable tasks (user-owned, flat)
│   └── *.md
│
└── tasks/              # Executable work items (user-owned, flat)
    └── *.md
```

## Built-in Roles

| Layer | Role | Responsibility |
|-------|------|----------------|
| Observers | `taxonomy` | Naming conventions, terminology consistency |
| Observers | `data_arch` | Data models, data flow efficiency |
| Observers | `qa` | Test coverage, test quality |
| Deciders | `triage` | Event screening, issue creation |
| Planners | `specifier` | Issue analysis, task decomposition |
| Implementers | `executor` | Code implementation, verification |

### Layer Behaviors

**Observers** (Layer 1):
- Read source code and their `notes/` directory
- Update `notes/` with current understanding (declarative state)
- Create normalized events in `.jules/events/<category>/` when issue-worthy observations are found
- Do NOT write to `.jules/issues/` or `.jules/tasks/`

**Deciders** (Layer 2):
- Read events from `.jules/events/**/*.yml`
- Screen critically, merge related observations
- Convert approved items into `.jules/issues/*.md`
- Delete processed events

**Planners** (Layer 3):
- Read issues from `.jules/issues/*.md`
- Decompose into concrete tasks with verification plans
- Create `.jules/tasks/*.md` files
- Delete processed issues

**Implementers** (Layer 4):
- Read tasks from `.jules/tasks/*.md`
- Implement code, tests, documentation
- Run verification
- Delete processed tasks

## Language Policy
- **Scaffold Content**: English (README.md, JULES.md)
- **File/Directory Names**: English (`roles/`, `events/`, `issues/`, `tasks/`, `notes/`, `role.yml`, `prompt.yml`)
- **Role Content**: Japanese (role.yml, prompt.yml, notes, events, issues, tasks)
- **CLI Messages**: English (stdout/stderr)
- **Code Comments**: English
