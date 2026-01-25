# jo

`jo` deploys and manages `.jules/` workspace scaffolding for organizational memory. It standardizes
a versioned policy/docs bundle into `.jules/` so scheduled agents and humans read consistent
structure in-repo.

## What is `.jules/`

The `.jules/` directory is repository-local organizational memory and a workflow contract for
scheduled LLM agents and humans. It persists direction, decisions, and per-role session outputs
so each scheduled run starts fresh while still regaining context by reading `.jules/`.

## Quick Start

```bash
cargo install --path .
cd your-project
jo init
```

This creates a `.jules/` workspace with:
- Source-of-truth documents in `org/`
- Role workspaces under `roles/`
- Decision records in `decisions/`
- Inter-role communication in `exchange/`
- jo-managed policy and templates in `.jo/`

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jo init` | `i` | Create `.jules/` skeleton and source-of-truth docs |
| `jo update` | `u` | Update jo-managed docs/templates under `.jules/.jo/` |
| `jo update --force` | `u -f` | Force overwrite jo-managed files |
| `jo status` | `st` | Print version info and detect local modifications |
| `jo role [role_id]` | `r` | Scaffold `.jules/roles/<role_id>/` workspace (interactive when omitted) |
| `jo session <role_id> [--slug <slug>]` | `s` | Create new session file |

## Usage Examples

```bash
# Initialize a new workspace
jo init

# Check status
jo status

# Create a role (interactive selection)
jo role

# Create a role by selecting from the menu
jo role

# Create a session for a role
jo session taxonomy --slug initial-analysis

# Update jo-managed files after upgrading jo
jo update

# Force update even if local modifications exist
jo update --force
```

## Directory Layout

```text
.jules/
  README.md                  # Entry point for navigating the workspace
  .jo-version                # jo version that last deployed .jo/
  .jo/                       # jo-managed policy and templates
    policy/
      contract.md
      layout.md
      run-bootstrap.md
      run-output.md
      role-boundaries.md
      exchange.md
      decisions.md
    templates/
      session.md
      decision.md
      weekly-synthesis.md
      role-charter.md
      role-direction.md
    roles/
      .gitkeep
  org/                       # Source-of-truth direction (human-managed)
    north_star.md
    constraints.md
    current_priorities.md
  decisions/                 # Decision records by year
    YYYY/
      YYYY-MM-DD_<slug>.md
  roles/                     # Per-role workspaces
    <role_id>/
      charter.md
      direction.md
      sessions/
        YYYY-MM-DD/
          HHMMSS_<slug>.md
  exchange/                  # Inter-role communication
    inbox/
      <role_id>/
    threads/
      <thread_id>/
  synthesis/                 # Periodic synthesis outputs
    weekly/
      YYYY-WW.md
  state/                     # Machine-readable state
    lenses.json
    open_threads.json
```

## Ownership Rules

| Path | Owner | Notes |
|------|-------|-------|
| `.jules/.jo/` | jo | Overwritten by `jo update` |
| `.jules/.jo-version` | jo | Version marker |
| Everything else | Human/Agent | Never overwritten by jo |

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
│   ├── error.rs          # AppError definitions
│   ├── workspace.rs      # Workspace filesystem operations
│   └── commands/         # Command implementations
│       ├── mod.rs
│       ├── init.rs
│       ├── update.rs
│       ├── status.rs
│       ├── role.rs
│       └── session.rs
└── tests/
    ├── common/           # Shared test fixtures
    ├── cli_commands.rs   # CLI command tests
    ├── cli_flow.rs       # CLI workflow tests
    ├── commands_api.rs   # Library API tests
    └── commands_core.rs  # Error handling tests
```
