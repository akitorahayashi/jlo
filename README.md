# jo

`jo` is a CLI tool that deploys and manages `.jules/` workspace scaffolding for scheduled LLM agent execution. It implements a **Worker/Triage agent organization layer** where specialized worker agents maintain persistent memory, record observations as events, and a triage agent screens and converts events into actionable issues.

## Design Philosophy

- **Distributed Agent Organization**: Worker agents analyze code from specialized perspectives; triage agent gates issue creation
- **Declarative Memory**: Agents record "what is" (not "what was done") in persistent `notes/` directories
- **Centralized Decision Making**: All events flow through triage for quality control before becoming issues
- **Structured Output**: Normalized events and flat, frontmatter-based issues

## `.jules/` Structure

```text
.jules/
├── README.md           # Workflow documentation (jo-managed)
├── AGENTS.md           # Agent contract (jo-managed)
├── .jo-version         # Version marker (jo-managed)
│
├── roles/              # [Worker Layer] Agent workspaces
│   ├── taxonomy/       # Naming consistency specialist
│   │   ├── prompt.yml  # Scheduler prompt template (jo-managed)
│   │   ├── role.yml    # Role definition and behavior
│   │   └── notes/      # Persistent declarative memory
│   ├── data_arch/      # Data model specialist
│   │   ├── prompt.yml
│   │   ├── role.yml
│   │   └── notes/
│   ├── qa/             # Quality assurance specialist
│   │   ├── prompt.yml
│   │   ├── role.yml
│   │   └── notes/
│   └── triage/         # [Manager] Triage gatekeeper
│       ├── prompt.yml
│       └── role.yml
│
├── events/             # [Inbox] Normalized observations
│   ├── bugs/
│   ├── refacts/
│   ├── updates/
│   ├── tests/
│   └── docs/
│
└── issues/             # [Outbox] Approved actionable tasks (flat)
    └── *.md
```

## Quick Start

```bash
cargo install --path .
cd your-project
jo init
jo role
```

The `jo init` command creates the complete `.jules/` structure with all 4 built-in roles. The `jo role` command shows an interactive menu to select a role and prints the scheduler prompt.

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jo init` | `i` | Create `.jules/` structure with all built-in roles |
| `jo update` | `u` | Update jo-managed files (README, AGENTS, prompt.yml, version) |
| `jo role` | `r` | Interactive role selection and scheduler prompt output |

## Workflow

### Agent Workflow

1. **Worker Agents** (scheduled):
   - Read source code and their `notes/` directory
   - Update `notes/` with current understanding (declarative state)
   - Record observations as normalized `events/*.yml` when issue-worthy

2. **Triage Agent** (scheduled):
   - Read events from `events/`
   - Screen critically and merge related observations
   - Convert approved items to flat `issues/*.md` with frontmatter
   - Delete processed events

3. **Human Execution**:
   - Review issues in `issues/`
   - Select and execute (or delegate to coding agent)
   - Archive completed issues

### CLI Workflow

1. **Initialize**: Run `jo init` to create `.jules/` with all roles
2. **Select Role**: Run `jo role` to get scheduler prompt
3. **Schedule**: Paste `prompt.yml` content into scheduler
4. **Monitor**: Review `events/` and `issues/` for agent activity

## Built-in Roles

| Role | Type | Responsibility |
|------|------|----------------|
| `taxonomy` | Worker | Naming conventions, terminology consistency |
| `data_arch` | Worker | Data models, data flow efficiency |
| `qa` | Worker | Test coverage, test quality |
| `triage` | Manager | Event screening, issue creation |

## Version Management

The `.jules/.jo-version` file tracks which version of `jo` last managed the workspace. Running `jo update` refreshes managed files.

## Language Policy

- **File/Directory Names**: English only
- **File Contents**: Japanese (role.yml, prompt.yml, notes, events, issues)
- **CLI Output**: English

## Managed Files

`jo update` only touches:
- `.jules/README.md`
- `.jules/AGENTS.md`
- `.jules/roles/*/prompt.yml`
- `.jules/.jo-version`

All other files (roles, notes, events, issues) are user-owned.

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
│   │   ├── taxonomy/
│   │   ├── data_arch/
│   │   ├── qa/
│   │   └── triage/
│   ├── error.rs          # AppError definitions
│   ├── workspace.rs      # Workspace filesystem operations
│   └── commands/         # Command implementations
└── tests/
    ├── common/           # Shared test fixtures
    ├── cli_commands.rs   # CLI command tests
    ├── cli_flow.rs       # CLI workflow tests
    ├── commands_api.rs   # Library API tests
    └── commands_core.rs  # Error handling tests
```
