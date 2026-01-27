# jlo

`jlo` is a CLI tool that deploys and manages `.jules/` workspace scaffolding for scheduled LLM agent execution. It implements a **4-layer agent architecture** where specialized agents are organized by their operational responsibilities: Observers analyze code, Deciders screen events, Planners decompose issues, and Implementers execute tasks.

## Design Philosophy

- **4-Layer Architecture**: Observers → Deciders → Planners → Implementers
- **Declarative Memory**: Observer agents record "what is" (not "what was done") in persistent `notes/` directories
- **Centralized Decision Making**: All events flow through Deciders for quality control before becoming issues
- **Dynamic Prompt Generation**: Prompts are composed at runtime and copied to clipboard for scheduler use

## `.jules/` Structure

```text
.jules/
├── README.md           # Workflow documentation (jlo-managed)
├── JULES.md            # Agent contract (jlo-managed)
├── .jlo-version         # Version marker (jlo-managed)
│
├── roles/              # [4-Layer] Agent organization
│   ├── observers/      # Layer 1: Observation
│   │   ├── taxonomy/   # Naming consistency specialist
│   │   ├── data_arch/  # Data model specialist
│   │   └── qa/         # Quality assurance specialist
│   │
│   ├── deciders/       # Layer 2: Decision
│   │   └── triage/     # Event screening, issue creation
│   │
│   ├── planners/       # Layer 3: Planning
│   │   └── specifier/  # Issue decomposition into tasks
│   │
│   └── implementers/   # Layer 4: Implementation
│       └── executor/   # Code implementation
│

├── events/             # [Inbox] Normalized observations
│   ├── bugs/
│   ├── refacts/
│   ├── updates/
│   ├── tests/
│   └── docs/
│
├── issues/             # [Transit] Approved actionable tasks
│   └── *.md
│
└── tasks/              # [Outbox] Executable work items
    └── *.md
```

## Quick Start

```bash
cargo install --path .
cd your-project
jlo init
jlo assign taxonomy src/
```

The `jlo init` command creates the complete `.jules/` structure with all 6 built-in roles organized in 4 layers. The `jlo assign` command generates a prompt for a role and copies it to your clipboard.

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jlo init` | `i` | Create `.jules/` structure with 4-layer architecture |
| `jlo assign <role> [paths...]` | `a` | Generate prompt and copy to clipboard |
| `jlo template [-l layer] [-n name]` | `tp` | Create a new role from layer template |

### Examples

```bash
# Initialize workspace
jlo init

# Assign taxonomy role with specific paths
jlo assign taxonomy src/models/ src/controllers/

# Assign using fuzzy matching
jlo assign tax src/

# Create a new custom observer role
jlo template -l observers -n security

# Create a new implementer role
jlo template --layer implementers --name frontend
```

## Workflow

### Agent Workflow

1. **Observer Agents** (scheduled):
   - Read contracts.yml (layer behavior) and role.yml (specialized focus)
   - Read source code and their `notes/` directory
   - Update `notes/` with current understanding (declarative state)
   - Record observations as normalized events in `exchange/events/*.yml` when issue-worthy

2. **Decider Agent** (scheduled):
   - Read contracts.yml (layer behavior)
   - Read events from `exchange/events/`
   - Screen critically and merge related observations
   - Convert approved items to flat `exchange/issues/*.md` with frontmatter
   - Delete processed events

3. **Planner Agent** (on-demand):
   - Read contracts.yml (layer behavior)
   - Read issues from `exchange/issues/`
   - Decompose into concrete tasks with verification plans
   - Create `exchange/tasks/*.md` files
   - Delete processed issues

4. **Implementer Agent** (on-demand):
   - Read contracts.yml (layer behavior)
   - Read tasks from `exchange/tasks/`
   - Implement code changes
   - Run verification
   - Delete processed tasks

### CLI Workflow

1. **Initialize**: Run `jlo init` to create `.jules/` with 4-layer structure
2. **Assign Role**: Run `jlo assign <role>` to generate prompt and copy to clipboard
3. **Schedule**: Paste prompt into scheduler
4. **Monitor**: Review `exchange/events/`, `exchange/issues/`, and `exchange/tasks/` for agent activity

## Built-in Roles

| Layer | Role | Responsibility |
|-------|------|----------------|
| Observers | `taxonomy` | Naming conventions, terminology consistency |
| Observers | `data_arch` | Data models, data flow efficiency |
| Observers | `qa` | Test coverage, test quality |
| Deciders | `triage` | Event screening, issue creation |
| Planners | `specifier` | Issue analysis, task decomposition |
| Implementers | `executor` | Code implementation, verification |

## Language Policy

- **File/Directory Names**: English only
- **File Contents**: Japanese (role.yml, prompt.yml, notes, events, issues)
- **CLI Output**: English

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
jlo/
├── src/
│   ├── main.rs            # CLI parsing (clap)
│   ├── lib.rs             # Public API
│   ├── domain/            # Pure domain types (no I/O)
│   │   ├── error.rs       # AppError definitions
│   │   ├── layer.rs       # 4-layer architecture (Layer enum)
│   │   ├── role_id.rs     # Role identifier validation
│   │   ├── workspace_layout.rs  # Constants (.jules/ structure)
│   │   └── generated_prompt.rs  # Prompt value object
│   ├── ports/             # Trait boundaries
│   │   ├── clipboard_writer.rs  # Clipboard abstraction
│   │   ├── workspace_store.rs   # Workspace operations
│   │   └── role_template_store.rs # Template loading
│   ├── services/          # Implementations (with I/O)
│   │   ├── clipboard_arboard.rs    # arboard clipboard
│   │   ├── workspace_filesystem.rs # Filesystem operations
│   │   ├── role_template_service.rs # Embedded templates
│   │   └── prompt_generator.rs     # Dynamic YAML generation
│   ├── app/               # Application layer
│   │   ├── context.rs     # AppContext (DI container)
│   │   └── commands/      # Command implementations
│   │       ├── init.rs
│   │       └── template.rs
│   ├── assets/            # Embedded static content
│   │   └── scaffold/      # .jules/ scaffold files (roles)
│   └── testing/           # Test-only mock implementations
│       ├── mock_clipboard.rs
│       ├── mock_workspace_store.rs
│       └── mock_role_template_store.rs
└── tests/
    ├── common/            # Shared test fixtures
    ├── cli_commands.rs    # CLI command tests
    ├── cli_flow.rs        # CLI workflow tests
    ├── commands_api.rs    # Library API tests
    └── commands_core.rs   # Error handling tests
```
