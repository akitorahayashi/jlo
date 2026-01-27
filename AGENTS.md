# jo Development Overview

## Critical Design Principles

### 1. Prompts are Static Files, Never Generated in Rust
**Problem**: Dynamically generating prompts in Rust code leads to unpredictable structure and makes it impossible for users to see/audit what agents will execute.

**Rule**: All prompts must exist as `.yml` files in `src/assets/scaffold/` or `src/assets/templates/`. Rust code may only do simple string replacement (e.g., `ROLE_NAME` → actual role name), never compose or generate prompt content.

### 2. Archetypes are Build-Time Only, Never Deployed
**Problem**: If archetypes are deployed to `.jules/archetypes/`, agents might read them instead of JULES.md, breaking the single source of truth contract.

**Rule**: 
- Archetypes live in `src/assets/archetypes/` (NOT in scaffold)
- They are used ONLY during `jo init` to generate the scaffold
- Agents read JULES.md for behavioral contracts, never archetypes
- `.jules/` workspace contains NO archetypes directory after deployment

### 3. JULES.md is the Single Source of Truth
**Problem**: Multiple sources of behavioral specification (archetypes, role.yml, prompt.yml, JULES.md) create confusion about authority.

**Rule**:
- JULES.md defines complete behavioral contracts for all layers
- role.yml (observers only) defines specialized focus WITHIN the observer contract
- prompt.yml is the composed, executable prompt that references JULES.md
- Agents always read JULES.md first for their behavioral contract

### 4. Minimal Duplication in Prompts
**Problem**: Repeating global policy and layer behavior in every role's prompt.yml creates maintenance burden and inconsistency.

**Rule**:
- Common rules belong in JULES.md (one place)
- prompt.yml should be minimal: role identity + "read JULES.md for your contract"
- Template files in `src/assets/templates/` follow this minimalism

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
- `jo assign <role> [paths...]` (alias: `a`): Read a role's prompt.yml and copy to clipboard. Optional paths are added to the YAML at execution time.
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
│   ├── observers/      # Layer 1: Observation (stateful)
│   │   ├── taxonomy/
│   │   │   ├── prompt.yml    # Execution parameters only
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
│   │   └── triage/
│   │       └── prompt.yml    # No role.yml (behavior in archetype)
│   │
│   ├── planners/       # Layer 3: Planning (stateless)
│   │   └── specifier/
│   │       └── prompt.yml    # No role.yml (behavior in archetype)
│   │
│   └── implementers/   # Layer 4: Implementation (stateless)
│       └── executor/
│           └── prompt.yml    # No role.yml (behavior in archetype)
│
├── archetypes/         # Layer behavior definitions
│   ├── layers/
│   │   ├── observer.yml      # Complete observer behavior
│   │   ├── decider.yml       # Complete decider behavior
│   │   ├── planner.yml       # Complete planner behavior
│   │   └── implementer.yml   # Complete implementer behavior
│   └── policy.yml
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
| Deciders | `triage` | Event screening, issue creation, feedback writing |
| Planners | `specifier` | Issue analysis, task decomposition |
| Implementers | `executor` | Code implementation, verification |

### Layer Behaviors

**Observers** (Layer 1):
- Read source code, `notes/`, and `feedbacks/` directories
- **Initialization**: Read all feedback files, abstract patterns, update `role.yml` to reduce noise
- Update `notes/` with current understanding (declarative state: describe "what is", not "what was done")
- Create normalized events in `.jules/events/<category>/` when issue-worthy observations are found
- **Stateful**: Maintain persistent `notes/` and receive feedback via `feedbacks/`
- Do NOT write to `.jules/issues/` or `.jules/tasks/`

**Deciders** (Layer 2):
- Read events from `.jules/events/**/*.yml`
- Screen critically (verify observations actually exist in codebase)
- Merge related observations that share root cause
- Convert approved items into `.jules/issues/*.md`
- **Write feedback**: When rejecting recurring patterns, create `feedbacks/<date>_<description>.yml` in observer's directory
- Delete processed events (both accepted and rejected)
- **Stateless**: All behavior defined in `.jules/archetypes/layers/decider.yml`

**Planners** (Layer 3):
- Read target issue from `.jules/issues/*.md`
- Decompose into concrete tasks with verification plans
- Create `.jules/tasks/*.md` files
- Delete processed issues
- **Stateless**: All behavior defined in `.jules/archetypes/layers/planner.yml`

**Implementers** (Layer 4):
- Read target task from `.jules/tasks/*.md`
- Implement code, tests, documentation
- Run verification (or reliable alternative if environment constraints exist)
- Delete processed tasks
- **Stateless**: All behavior defined in `.jules/archetypes/layers/implementer.yml`

## Configuration Hierarchy

The configuration follows a **single source of truth** hierarchy:

```
JULES.md (contract, schemas)
  └── archetypes/layers/*.yml (layer default behavior)
       └── roles/observers/*/role.yml (specialized focus, only for observers)
            └── prompt.yml (execution-time parameters only)
```

- **JULES.md**: Defines contracts, schemas, and workflows
- **Archetypes**: Define complete behavior for each layer
- **role.yml**: Only exists for observers (stateful roles); defines specialized analytical focus
- **prompt.yml**: Contains only execution-time parameters (paths for observers, target for planners/implementers)

## Feedback Loop

The feedback mechanism enables continuous improvement:

1. **Observer** creates events based on observations
2. **Decider** reviews events and may reject some due to recurring patterns
3. **Decider** writes feedback files to `.jules/roles/observers/<role>/feedbacks/`
4. **Observer** reads feedback files on next execution, abstracts patterns
5. **Observer** updates its own `role.yml` to refine focus and prevent noise

Feedback files are preserved for audit (never deleted). This self-improvement loop reduces false positives over time.

## Language Policy
- **Scaffold Content**: English (README.md, JULES.md, all YAML configuration files)
- **File/Directory Names**: English (`roles/`, `events/`, `issues/`, `tasks/`, `notes/`, `feedbacks/`, `role.yml`, `prompt.yml`)
- **Role Content**: User-defined (events, issues, tasks, notes can be in any language)
- **CLI Messages**: English (stdout/stderr)
- **Code Comments**: English
