# jlo Development Overview

## Critical Design Principles

### 1. Prompts are Static Files, Never Generated in Rust
All prompts exist as `.yml` files in `src/assets/scaffold/` or `src/assets/templates/`. Rust code only does simple string replacement (e.g., `ROLE_NAME` -> actual role name).

### 2. JULES.md is the Single Source of Truth
- JULES.md defines complete behavioral contracts for all layers
- role.yml (observers only) defines specialized focus WITHIN the observer contract
- prompt.yml references JULES.md for behavioral instructions

### 3. Minimal Duplication in Prompts
Common rules belong in JULES.md. Template files in `src/assets/templates/` follow this minimalism.

## Project Summary
`jlo` is a CLI tool that deploys and manages `.jules/` workspace scaffolding for scheduled LLM agent execution. Specialized agents are organized by their operational responsibilities: Observers analyze code, Deciders screen events, Planners decompose issues, and Mergers consolidate parallel work. Implementation is invoked via GitHub Issues with `jules` label.

## Tech Stack
- **Language**: Rust
- **CLI Parsing**: `clap`
- **Clipboard**: `arboard`
- **YAML Processing**: `serde`, `serde_yaml`
- **Hashing**: `sha2`
- **Embedded scaffold**: `include_dir`
- **Interactive prompts**: `dialoguer`
- **Date/Time**: `chrono`

## Coding Standards
- **Formatter**: `rustfmt` (100 char width, crate-level import granularity)
- **Linter**: `clippy` with `-D warnings`

## Key Commands
- **Build**: `cargo build`
- **Format**: `cargo fmt --check`
- **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`
- **Test**: `cargo test --all-targets --all-features`

## Project Structure

```
src/
├── main.rs            # CLI (clap)
├── lib.rs             # Public API
├── domain/            # Pure types (Layer, RoleId, AppError)
├── ports/             # Trait boundaries
├── services/          # I/O implementations
├── app/
│   ├── context.rs     # AppContext (DI container)
│   └── commands/      # init, assign, template, prune
├── assets/
│   ├── scaffold/      # Embedded .jules/ structure
│   └── templates/     # Role templates by layer
└── testing/           # Mock implementations
tests/
├── common/            # Shared test fixtures
├── cli_commands.rs    # CLI tests
├── cli_flow.rs        # Workflow tests
└── commands_api.rs    # Library API tests
```

## CLI Commands
- `jlo init` (alias: `i`): Create `.jules/` structure
- `jlo assign <role> [paths...]` (alias: `a`): Copy prompt to clipboard
- `jlo template [-l layer] [-n name]` (alias: `tp`): Create custom role
- `jlo prune -d <days>` (alias: `prn`): Delete old jules/* branches

## Built-in Roles

| Layer | Role | Responsibility |
|-------|------|----------------|
| Observers | `taxonomy` | Naming conventions |
| Observers | `data_arch` | Data models |
| Observers | `qa` | Test coverage |
| Deciders | `triage` | Event screening, feedback writing |
| Planners | `specifier` | Task decomposition |
| Mergers | `consolidator` | Branch consolidation |

## Language Policy
- **Scaffold Content**: English
- **Role Content**: User-defined
- **CLI Messages**: English
- **Code Comments**: English
