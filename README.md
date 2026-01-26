# jo

`jo` is a CLI tool that deploys and manages `.jules/` workspace scaffolding for scheduled LLM agent execution. It implements a **PM/Worker agent organization layer** where specialized worker agents maintain persistent memory, propose improvements, and a PM agent screens and converts proposals into actionable issues.

## Design Philosophy

- **Distributed Agent Organization**: Worker agents analyze code from specialized perspectives; PM agent gates issue creation
- **Declarative Memory**: Agents record "what is" (not "what was done") in persistent `notes/` directories
- **Centralized Decision Making**: All proposals flow through PM for quality control before becoming issues
- **Structured Output**: Clear categorization of issues (bugs, refacts, updates, tests, docs)

## `.jules/` Structure

```text
.jules/
├── README.md           # Workflow documentation (jo-managed)
├── .jo-version         # Version marker (jo-managed)
│
├── roles/              # [Worker Layer] Agent workspaces
│   ├── taxonomy/       # Naming consistency specialist
│   │   ├── role.yml    # Role definition and behavior
│   │   └── notes/      # Persistent declarative memory
│   ├── data_arch/      # Data model specialist
│   │   ├── role.yml
│   │   └── notes/
│   ├── qa/             # Quality assurance specialist
│   │   ├── role.yml
│   │   └── notes/
│   └── pm/             # [Manager] Project Manager
│       ├── role.yml
│       └── policy.md   # Decision criteria
│
├── reports/            # [Inbox] Proposals from Workers
│   └── YYYY-MM-DD_<role>_<title>.md
│
└── issues/             # [Outbox] Approved actionable tasks
    ├── bugs/           # Bug fixes (+tests, +docs)
    ├── refacts/        # Refactoring (+tests, +docs)
    ├── updates/        # New features (+tests, +docs)
    ├── tests/          # Test-only changes
    └── docs/           # Documentation-only changes
```

## Quick Start

```bash
cargo install --path .
cd your-project
jo init
jo role
```

The `jo init` command creates the complete `.jules/` structure with all 4 built-in roles. The `jo role` command shows an interactive menu to select a role and prints the role configuration.

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jo init` | `i` | Create `.jules/` structure with all built-in roles |
| `jo update` | `u` | Update jo-managed files (README, version) |
| `jo role` | `r` | Interactive role selection and config output |

## Workflow

### Agent Workflow

1. **Worker Agents** (scheduled):
   - Read source code and their `notes/` directory
   - Update `notes/` with current understanding (declarative state)
   - Create proposals in `reports/` when improvements are found

2. **PM Agent** (scheduled):
   - Read proposals from `reports/`
   - Screen against `policy.md` criteria
   - Convert approved proposals to `issues/<category>/*.md`

3. **Human Execution**:
   - Review issues in `issues/`
   - Select and execute (or delegate to coding agent)
   - Archive completed issues

### CLI Workflow

1. **Initialize**: Run `jo init` to create `.jules/` with all roles
2. **Select Role**: Run `jo role` to get role configuration
3. **Schedule**: Paste `role.yml` content into scheduler
4. **Monitor**: Review `reports/` and `issues/` for agent activity

## Built-in Roles

| Role | Type | Responsibility |
|------|------|----------------|
| `taxonomy` | Worker | Naming conventions, terminology consistency |
| `data_arch` | Worker | Data models, data flow efficiency |
| `qa` | Worker | Test coverage, test quality |
| `pm` | Manager | Proposal review, issue creation |

## Version Management

The `.jules/.jo-version` file tracks which version of `jo` last managed the workspace. Running `jo update` refreshes managed files.

## Language Policy

- **File/Directory Names**: English only
- **File Contents**: Japanese (role.yml, notes, reports, issues)
- **CLI Output**: English

## Managed Files

`jo update` only touches:
- `.jules/README.md`
- `.jules/.jo-version`

All other files (roles, notes, reports, issues) are user-owned.

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
│   │   └── pm/
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
