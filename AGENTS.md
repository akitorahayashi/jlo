# jlo Development Overview

## Architecture

| Component | Responsibility |
|-----------|----------------|
| **jlo** | Scaffold installation, versioning, prompt asset management |
| **GitHub Actions** | Orchestration: cron triggers, matrix execution, auto-merge control |
| **jules-invoke** | Session creation: prompt delivery, starting_branch specification |
| **Jules (VM)** | Execution: code analysis, artifact generation, branch/PR creation |

## Critical Design Principles

### 1. Assets are Static Files, Never Hardcoded in Rust
All scaffold files, configurations, and prompts must exist as real files within `src/assets/`.
**Never** embed file contents (like `DEFAULT_CONFIG_TOML`, `tools.yml`, or default `.gitignore`) as string constants in Rust source code.
- **Why**: Keeps the scaffold structure visible and maintainable without digging into implementation details.
- **How**: Use `include_dir!` to load the `src/assets/scaffold` directory as the authoritative source of truth.

### 2. Prompt Hierarchy (No Duplication)
```
prompt.yml (entry point, role-specific)
  └─ contracts.yml (layer-shared workflow)
       └─ JULES.md (global constraints only)
```

| File | Scope | Content |
|------|-------|---------|
| `prompt.yml` | Role | Entry point. Lists contracts to follow. |
| `role.yml` | Role | Specialized focus (observers only). |
| `contracts.yml` | Layer | Workflow, inputs, outputs, constraints shared within layer. |
| `JULES.md` | Global | Rules applying to ALL layers (branch naming, system boundaries). |

**Rule**: Never duplicate content across levels. Each level references the next.

### 3. Workflow-Driven Execution
Agent execution is orchestrated by GitHub Actions, not jlo. The `.github/workflows/jules-workflows.yml` coordinates all agent invocations via reusable workflows.

## Project Summary
`jlo` is a CLI tool that deploys and manages `.jules/` workspace scaffolding for scheduled LLM agent execution. Specialized agents are organized by their operational responsibilities: Observers analyze code, Deciders screen events and produce issues, Planners expand issues requiring deep analysis, and Implementers are triggered manually from GitHub Issues.

## Branch Strategy

| Agent Type | Starting Branch | Output Branch | Auto-merge |
|------------|-----------------|---------------|------------|
| Observer | `jules` | `jules-observer-*` | ✅ (if `.jules/` only) |
| Decider | `jules` | `jules-decider-*` | ✅ (if `.jules/` only) |
| Planner | `jules` | `jules-planner-*` | ✅ (if `.jules/` only) |
| Implementer | `main` | `jules-implementer-*` | ❌ (human review) |

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

## Key Commands
- **Format**: `cargo fmt`, `cargo fmt --check`
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
│   └── commands/      # init, template, setup
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
- `jlo init` (alias: `i`): Create `.jules/` structure with setup directory
- `jlo update [--dry-run] [--workflows]` (alias: `u`): Update workspace to current jlo version
- `jlo template [-l layer] [-n name]` (alias: `tp`): Create custom role (observers, deciders only)
- `jlo run observers [--role <name>]`: Run observer agents
- `jlo run deciders [--role <name>]`: Run decider agents
- `jlo run planners <path>`: Run planner (single-role, issue-driven)
- `jlo run implementers <path>`: Run implementer (single-role, issue-driven)
- `jlo setup gen [path]` (alias: `s g`): Generate `install.sh` and `env.toml`
- `jlo setup list` (alias: `s ls`): List available components
- `jlo setup list --detail <component>`: Show component details

## Layer Architecture

| Layer | Type | Invocation | Config |
|-------|------|------------|--------|
| Observers | Multi-role | `jlo run observers` | `config.toml` |
| Deciders | Multi-role | `jlo run deciders` | `config.toml` |
| Planners | Single-role | `jlo run planners --issue <path>` | None |
| Implementers | Single-role | `jlo run implementers --issue <path>` | None |

**Single-role layers**: Planners and Implementers have a fixed role with `prompt.yml` directly in the layer directory. They are issue-driven and require the `--issue` flag. Template creation is not supported.

**Multi-role layers**: Observers and Deciders support multiple configurable roles listed in `config.toml`. Each role has its own subdirectory with `prompt.yml`.

## Setup Compiler

The setup compiler generates dependency-aware installation scripts for development tools.

### Workspace Structure
```
.jules/
  workstreams/
    <workstream>/
      events/        # Observer outputs
      issues/        # Decider outputs
        index.md
        high/
        medium/
        low/
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
