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
- **TOML Processing**: `toml`
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
├── domain/            # Pure types (Layer, RoleId, AppError, setup models)
├── ports/             # Trait boundaries
├── services/          # I/O implementations (catalog, resolver, generator)
├── app/
│   ├── context.rs     # AppContext (DI container)
│   └── commands/      # init, assign, template, prune, setup
├── assets/
│   ├── scaffold/      # Embedded .jules/ structure
│   ├── templates/     # Role templates by layer
│   └── catalog/       # Setup component definitions (meta.toml + install.sh)
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
- `jlo setup init [path]` (alias: `s init`): Initialize `.jules/setup/` workspace
- `jlo setup gen [path]` (alias: `s gen`): Generate `install.sh` and `env.toml`
- `jlo setup list` (alias: `s ls`): List available components
- `jlo setup list --detail <component>`: Show component details

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

## Setup Compiler

The setup compiler generates dependency-aware installation scripts for development tools.

### Workspace Structure
```
.jules/
  setup/
    tools.yml      # Tool selection configuration
    env.toml       # Environment variables (generated/merged)
    install.sh     # Installation script (generated)
    .gitignore     # Ignores env.toml
```

### Component Catalog
Each component is a directory under `src/assets/catalog/<component>/` with:
- `meta.toml`: name, summary, dependencies, env specs
- `install.sh`: Installation script

### meta.toml Schema
```toml
name = "component-name"       # Optional; defaults to directory name
summary = "Short description"
dependencies = ["other-comp"] # Optional

[[env]]
name = "ENV_VAR"
description = "What this variable does"
default = "optional-default"  # Optional
```

### Services
- **CatalogService**: Loads components from embedded assets
- **ResolverService**: Topological sort with cycle detection
- **GeneratorService**: Produces install.sh and merges env.toml

### Environment Contract
Catalog installers assume the Jules environment baseline (Python 3.12+, Node.js 22+, common dev tools). The CI verify-installers workflow provisions that baseline in minimal containers.
