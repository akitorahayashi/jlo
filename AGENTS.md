# jlo Development Overview

## Critical Design Principles

### 1. Prompts are Static Files, Never Generated in Rust
**Problem**: Dynamically generating prompts in Rust code leads to unpredictable structure and makes it impossible for users to see/audit what agents will execute.

**Rule**: All prompts must exist as `.yml` files in `src/assets/scaffold/` or `src/assets/templates/`. Rust code may only do simple string replacement (e.g., `ROLE_NAME` → actual role name), never compose or generate prompt content.

### 2. JULES.md is the Single Source of Truth
**Problem**: Multiple sources of behavioral specification (role.yml, prompt.yml, JULES.md) create confusion about authority.

**Rule**:
- JULES.md defines complete behavioral contracts for all layers
- role.yml (observers only) defines specialized focus WITHIN the observer contract
- prompt.yml is the composed, executable prompt that references JULES.md
- Agents always read JULES.md first for their behavioral contract

### 3. Minimal Duplication in Prompts
**Problem**: Repeating global policy and layer behavior in every role's prompt.yml creates maintenance burden and inconsistency.

**Rule**:
- Common rules belong in JULES.md (one place)
- prompt.yml should be minimal: role identity + "read JULES.md for your contract"
- Template files in `src/assets/templates/` follow this minimalism

## Project Summary
`jlo` is a CLI tool that deploys and manages `.jules/` workspace scaffolding for scheduled LLM agent execution. It implements a **4-layer agent architecture** where specialized agents are organized by their operational responsibilities: Observers analyze code, Deciders screen events, Planners decompose issues, and Implementers execute tasks.

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
- `jlo init` (alias: `i`): Create complete `.jules/` structure with 4-layer architecture and all 6 built-in roles.
- `jlo assign <role> [paths...]` (alias: `a`): Read a role's prompt.yml and copy to clipboard. Optional paths are added to the YAML at execution time.
- `jlo template [-l layer] [-n name]` (alias: `tp`): Create a new custom role from a layer archetype.

## Workspace Contract (v3)

### Directory Structure
```
.jules/
├── README.md           # Workflow documentation (jlo-managed)
├── JULES.md            # Agent contract (jlo-managed)
├── .jlo-version         # Version marker (jlo-managed)
│
├── roles/              # 4-Layer agent organization
│   ├── observers/      # Layer 1: Observation (stateful)
│   │   ├── contracts.yml     # Shared observer contract
│   │   ├── taxonomy/
│   │   │   ├── prompt.yml    # Execution parameters
│   │   │   ├── role.yml      # Specialized focus
│   │   │   ├── notes/        # Declarative state
│   │   │   └── feedbacks/    # Decider rejection feedback
│   │   ├── data_arch/
│   │   │   ├── prompt.yml
│   │   │   ├── role.yml
│   │   │   ├── notes/
│   │   │   └── feedbacks/
│   │   └── qa/
│   │       ├── prompt.yml
│   │       ├── role.yml
│   │       ├── notes/
│   │       └── feedbacks/
│   │
│   ├── deciders/       # Layer 2: Decision (stateless)
│   │   ├── contracts.yml     # Shared decider contract
│   │   └── triage/
│   │       └── prompt.yml
│   │
│   ├── planners/       # Layer 3: Planning (stateless)
│   │   ├── contracts.yml     # Shared planner contract
│   │   └── specifier/
│   │       └── prompt.yml
│   │
│   └── implementers/   # Layer 4: Implementation (stateless)
│       ├── contracts.yml     # Shared implementer contract
│       └── executor/
│           └── prompt.yml
│
└── exchange/           # Transient data flow (user-owned)
    ├── events/         # Normalized observations
    │   ├── bugs/
    │   ├── refacts/
    │   ├── updates/
    │   ├── tests/
    │   └── docs/
    │
    ├── issues/         # Actionable tasks (flat)
    │   └── *.md
    │
    └── tasks/          # Executable work items (flat)
        └── *.md
```

## Built-in Roles

| Layer | Role | Responsibility |
|-------|------|----------------|
| Observers | `taxonomy` | Naming conventions, terminology consistency |
| Observers | `data_arch` | Data models, data flow efficiency |
| Observers | `qa` | Test coverage, test quality |
| Deciders | `triage` | Event screening, issue creation, feedback writing |
| Planners | `specifier` | Issue analysis, task decomposition |
| Implementers | `executor` | Code implementation, verification |

### Layer Behaviors

**Observers** (Layer 1):
- Read contracts.yml (layer behavior), role.yml (specialized focus), notes/, and feedbacks/
- **Initialization**: Read all feedback files, abstract patterns, update `role.yml` to reduce noise
- Update `notes/` with current understanding (declarative state: describe "what is", not "what was done")
- Create normalized events in `.jules/exchange/events/<category>/` when issue-worthy observations are found
- **Stateful**: Maintain persistent `notes/` and receive feedback via `feedbacks/`
- Do NOT write to `.jules/exchange/issues/` or `.jules/exchange/tasks/`

**Deciders** (Layer 2):
- Read contracts.yml (layer behavior) and events from `.jules/exchange/events/**/*.yml`
- Screen critically (verify observations actually exist in codebase)
- Merge related observations that share root cause
- Convert approved items into `.jules/exchange/issues/*.md`
- **Write feedback**: When rejecting recurring patterns, create `feedbacks/<date>_<description>.yml` in observer's directory
- Delete processed events (both accepted and rejected)
- **Stateless**: All behavior defined in contracts.yml

**Planners** (Layer 3):
- Read contracts.yml (layer behavior) and target issue from `.jules/exchange/issues/*.md`
- Decompose into concrete tasks with verification plans
- Create `.jules/exchange/tasks/*.md` files
- Delete processed issues
- **Stateless**: All behavior defined in contracts.yml

**Implementers** (Layer 4):
- Read contracts.yml (layer behavior) and target task from `.jules/exchange/tasks/*.md`
- Implement code, tests, documentation
- Run verification (or reliable alternative if environment constraints exist)
- Delete processed tasks
- **Stateless**: All behavior defined in contracts.yml

## Configuration Hierarchy

- contracts.yml: Layer-level shared constraints (at each layer directory)
- JULES.md: Overall workflow and file semantics
- role.yml: Specialized focus for observers (dynamic, evolves with feedback)
- prompt.yml: Execution parameters and references to contracts.yml

## Feedback Loop

- Observer creates events in exchange/events/
- Decider reviews events, rejects if needed, writes feedback to observer's feedbacks/
- Observer reads feedback at next execution, updates role.yml to reduce noise

## Language Policy
- **Scaffold Content**: English (README.md, JULES.md, all YAML configuration files)
- **File/Directory Names**: English (`roles/`, `exchange/`, `events/`, `issues/`, `tasks/`, `notes/`, `feedbacks/`, `contracts.yml`, `role.yml`, `prompt.yml`)
- **Role Content**: User-defined (events, issues, tasks, notes can be in any language)
- **CLI Messages**: English (stdout/stderr)
- **Code Comments**: English
