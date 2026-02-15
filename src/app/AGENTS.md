# Application Layer

## Purpose

Orchestration, command workflows, and policy decisions that connect `domain` logic with `ports` interfaces.
The application layer acts as the entry point for CLI commands and manages dependencies.

## Structure

```
src/app/
├── cli/              # Argument parsing (clap)
├── commands/         # Use case implementation
├── config/           # App-level configuration
├── api.rs            # Public API surface
├── context.rs        # AppContext (Dependency Injection)
└── mod.rs
```

## Architectural Principles

-   **Dependency Direction**: `app -> domain`, `app -> ports`.
-   **Wiring**: Wires concrete `adapters` to `ports` at runtime (Dependency Injection via `AppContext`).
-   **No Domain Logic**: `app` delegates parsing, validation, and business rules to `domain`.
-   **No I/O Implementation**: `app` consumes I/O only through `ports` traits, never directly via `adapters` or `std::fs`/`std::net`.
-   **Command Scope**: Each command implementation (in `commands/`) orchestrates a specific user intent.

## CLI Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jlo init (--remote \| --self-hosted)` | `i` | Create `.jlo/` control plane and install workflow scaffold |
| `jlo update [--prompt-preview \| --cli]` | `u` | Advance control-plane version pin or update jlo CLI binary |
| `jlo create <layer> <name>` | `c` | Create a custom role under `.jlo/` |
| `jlo add [layer] [roles...]` | `a` | Add a built-in role under `.jlo/` |
| `jlo run narrator [--prompt-preview] [--branch <branch>] [--mock]` | `r n` | Run narrator (produces changes feed) |
| `jlo run observers --role <role> [--prompt-preview] [--branch <branch>] [--mock]` | `r o` | Run observer agents |
| `jlo run decider [--prompt-preview] [--branch <branch>] [--mock]` | `r d` | Run decider agents |
| `jlo run planner <requirement> [--prompt-preview] [--branch <branch>] [--mock]` | `r p` | Run planner (requirement-driven) |
| `jlo run implementer <requirement> [--prompt-preview] [--branch <branch>] [--mock]` | `r i` | Run implementer (requirement-driven) |
| `jlo run integrator [--prompt-preview] [--branch <branch>]` | `r g` | Run integrator (merges implementer branches) |
| `jlo run innovators --role <role> --task <task_name> [--prompt-preview] [--branch <branch>] [--mock]` | `r x` | Run innovator agents |
| `jlo doctor [--strict]` | | Validate .jules/ structure and content |
| `jlo workflow doctor` | `wf` | Validate runtime repository for workflow use |
| `jlo workflow run <layer> [--mock]` | | Run layer and return wait-gating metadata |
| `jlo workflow exchange inspect` | | Inspect exchange state |
| `jlo workflow exchange publish-proposals` | | Publish innovator proposals as GitHub issues |
| `jlo workflow exchange clean requirement <file>` | | Remove a processed requirement and its source events |
| `jlo workflow exchange clean mock --mock-tag <tag> [--pr-numbers-json <json>] [--branches-json <json>]` | | Cleanup mock artifacts |
| `jlo workflow gh process pr <all\|metadata\|automerge> <pr_number> [--retry-attempts <n>] [--retry-delay-seconds <n>] [--fail-on-error]` | | Run PR process pipeline |
| `jlo workflow gh process issue label-innovator <issue_number> <persona>` | | Apply innovator labels to a proposal issue |
| `jlo setup gen [path]` | `s g` | Generate `.jlo/setup/install.sh`, `vars.toml`, and `secrets.toml` |
| `jlo setup list [--detail <component>]` | `s ls` | List available components |
| `jlo deinit` | | Remove all jlo-managed assets (`.jlo/`, branch, workflows) |

## Layer Architecture

| Layer | Type | Invocation | Config |
|-------|------|------------|--------|
| Narrator | Single-role | `jlo run narrator` | None (git-based) |
| Observers | Multi-role | `jlo workflow run observers` | `.jlo/config.toml` (`[observers].roles`) |
| Decider | Single-role | `jlo run decider` | None |
| Planner | Single-role | `jlo run planner <path>` | None (requirement path) |
| Implementer | Single-role | `jlo run implementer <path>` | None (requirement path) |
| Integrator | Single-role | `jlo run integrator` | None (manual, on-demand) |
| Innovators | Multi-role | `jlo workflow run innovators` | `.jlo/config.toml` (`[innovators].roles`) |

**Single-role layers**: Narrator, Decider, Planner, Implementer have a fixed role with a `<layer>_prompt.j2` template in the layer directory. Template creation not supported.

**Multi-role layers**: Observers and Innovators support multiple configurable roles listed in `.jlo/config.toml` (`[observers].roles`, `[innovators].roles`). Custom roles can have `role.yml` under `.jlo/roles/`; built-ins are resolved from embedded assets when no custom role file exists.

## Mock Mode

Mock mode (`--mock`) enables E2E workflow validation without Jules API calls. Mock tag is auto-generated from `JULES_MOCK_TAG` env var or a timestamp.

Mock execution creates real git branches and GitHub PRs with synthetic commit content.

Key files:
- `src/domain/mock_config.rs`: `MockConfig` and `MockOutput` types
- `src/app/commands/run/mock/`: Mock execution implementation per layer

## Setup Compiler

The setup compiler generates dependency-aware installation scripts for development tools.

### Component Catalog Structure

```
src/assets/setup/<component>/
  meta.toml      # name, summary, dependencies, env specs
  install.sh     # Installation script
```

### meta.toml Schema

```toml
name = "component-name"       # Optional; defaults to directory name
summary = "Short description"
dependencies = ["other-comp"] # Optional

[vars]
ENV_VAR = { description = "What this variable does", default = "optional-default" }

[secrets]
SECRET_VAR = { description = "Secret used by runtime authentication" }
```

### Services

| Module | Responsibility |
|--------|----------------|
| **setup_component_catalog_embedded** | Loads setup components from embedded assets |
| **DependencyGraph** | Topological sort with cycle detection |
| **artifact_generator** | Produces install.sh and merges vars.toml + secrets.toml |

### Environment Contract

Catalog installers assume the Jules environment baseline (Python 3.12+, Node.js 22+, common dev tools). The CI `verify-installers` workflow provisions that baseline in minimal containers.
