# jlo CLI Development Context

See [root AGENTS.md](../AGENTS.md) for design principles.

## Project Structure

```
src/
├── main.rs            # CLI entry point (clap)
├── lib.rs             # Public API
├── domain/            # Pure types (Layer, RoleId, AppError, setup models)
├── ports/             # Trait boundaries
├── services/          # I/O implementations
├── app/
│   ├── context.rs     # AppContext (DI container)
│   └── commands/      # Command implementations
├── assets/
│   ├── scaffold/      # Embedded .jules/ structure
│   ├── templates/     # Role templates by layer
│   ├── workflows/     # Workflow scaffold assets
│   └── setup/         # Setup component definitions
└── testing/           # Mock implementations
tests/
├── harness/           # Shared fixtures (TestContext, git helpers, config writers)
├── cli.rs             # CLI behavior contracts
├── workflow.rs         # Bootstrap + workflow-kit contracts
├── doctor.rs          # Doctor/schema + mock fixture validity contracts
├── mock.rs            # Mock mode CLI contracts
└── library.rs          # Public library API lifecycle contract
```

## Tech Stack

| Library | Purpose |
|---------|---------|
| `clap` | CLI parsing |
| `serde`, `serde_yaml` | YAML processing |
| `toml` | TOML processing |
| `serde_json` | JSON processing |
| `sha2` | Hashing |
| `include_dir` | Embedded scaffold |
| `dialoguer` | Interactive prompts |
| `chrono` | Date/Time |
| `reqwest` | HTTP client |
| `url` | URL parsing |

## Terminology

| Term | Definition |
|------|------------|
| **Control plane** | The `.jlo/` directory on the control branch (e.g. `main`). Source of truth for all configuration, role definitions, and version pins. |
| **Runtime plane** | The `.jules/` directory on the `jules` branch. Materialized from `.jlo/` by workflow bootstrap; hosts agent exchange artifacts. |
| **Scaffold** | Embedded static files in `src/assets/scaffold/` that seed `.jlo/` on init and are reconciled on update. |
| **Projection** | Deterministic materialization of `.jules/` from `.jlo/` + scaffold assets during workflow bootstrap. See `docs/CONTROL_PLANE_OWNERSHIP.md`. |
| **Exchange** | The flat handoff directory structure under `.jules/exchange/`. |
| **Workflow scaffold** | `.github/` automation assets installed by `jlo init`. |
| **Component** | Development tools managed by `jlo setup`, defined in `src/assets/setup/`. |

## Domain Modules

Core domain logic located in `src/domain/`.

| Module | Purpose |
|--------|---------|
| `configuration` | Global configuration models (`config.toml`, `scheduled.toml`). |
| `identifiers` | Structural identifiers (`RoleId`, `Layer`). |
| `prompt` | Prompt assembly and template rendering models. |
| `workspace` | Filesystem abstraction and path management. |
| `error` | `AppError` and error handling types. |
| `issue` | Requirement parsing and schema validation. |

## CLI Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jlo init (--remote \| --self-hosted)` | `i` | Create `.jlo/` control plane and install workflow scaffold |
| `jlo update [--prompt-preview \| --cli]` | `u` | Advance control-plane version pin or update jlo CLI binary |
| `jlo create <layer> <name>` | `c` | Create a custom role under `.jlo/` |
| `jlo run narrator [--prompt-preview] [--branch <branch>] [--mock]` | `r n` | Run narrator (produces changes feed) |
| `jlo run observers --role <role> [--prompt-preview] [--branch <branch>] [--mock]` | `r o` | Run observer agents |
| `jlo run decider [--prompt-preview] [--branch <branch>] [--mock]` | `r d` | Run decider agents |
| `jlo run planner <requirement> [--prompt-preview] [--branch <branch>] [--mock]` | `r p` | Run planner (requirement-driven) |
| `jlo run implementer <requirement> [--prompt-preview] [--branch <branch>] [--mock]` | `r i` | Run implementer (requirement-driven) |
| `jlo run integrator [--prompt-preview] [--branch <branch>]` | `r g` | Run integrator (merges implementer branches) |
| `jlo run innovators --role <role> --phase <creation\|refinement> [--prompt-preview] [--branch <branch>] [--mock]` | `r x` | Run innovator agents |
| `jlo doctor [--strict]` | | Validate .jules/ structure and content |
| `jlo workflow doctor` | `wf` | Validate workspace for workflow use |
| `jlo workflow run <layer> [--mock]` | | Run layer and return wait-gating metadata |
| `jlo workflow workspace inspect` | | Inspect exchange state |
| `jlo workflow workspace publish-proposals` | | Publish innovator proposals as GitHub issues |
| `jlo workflow workspace clean requirement <file>` | | Remove a processed requirement and its source events |
| `jlo workflow workspace clean mock --mock-tag <tag> [--pr-numbers-json <json>] [--branches-json <json>]` | | Cleanup mock artifacts |
| `jlo workflow gh process pr <all\|metadata\|automerge> <pr_number> [--retry-attempts <n>] [--retry-delay-seconds <n>] [--fail-on-error]` | | Run PR process pipeline |
| `jlo workflow gh process issue label-innovator <issue_number> <persona>` | | Apply innovator labels to a proposal issue |
| `jlo setup gen [path]` | `s g` | Generate `.jlo/setup/install.sh`, `vars.toml`, and `secrets.toml` |
| `jlo setup list [--detail <component>]` | `s ls` | List available components |
| `jlo deinit` | | Remove all jlo-managed assets (`.jlo/`, branch, workflows) |

## Verification Commands

Auto-merge ownership is centralized in the `jules-automerge` workflow (push-scoped trigger on Jules auto-merge branch families). Mock cleanup remains PR-based for branch protection and auditability.

### Full Suite

```bash
cargo check
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

### Partial Testing

Run only relevant tests to save time:

```bash
# By name (substring match)
cargo test <test_name>

# By integration target
cargo test --test cli
cargo test --test workflow
cargo test --test doctor
cargo test --test mock
cargo test --test library
```

## Layer Architecture

| Layer | Type | Invocation | Config |
|-------|------|------------|--------|
| Narrator | Single-role | `jlo run narrator` | None (git-based) |
| Observers | Multi-role | `jlo workflow run observers` | `.jlo/scheduled.toml` |
| Decider | Single-role | `jlo run decider` | None |
| Planner | Single-role | `jlo run planner <path>` | None (requirement path) |
| Implementer | Single-role | `jlo run implementer <path>` | None (requirement path) |
| Integrator | Single-role | `jlo run integrator` | None (manual, on-demand) |
| Innovators | Multi-role | `jlo workflow run innovators` | `.jlo/scheduled.toml` |

**Single-role layers**: Narrator, Decider, Planner, Implementer have a fixed role with a `<layer>_prompt.j2` template in the layer directory. Template creation not supported.

**Multi-role layers**: Observers and Innovators support multiple configurable roles listed in `.jlo/scheduled.toml`. Each role has its own subdirectory with `role.yml`.

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

| Service | Responsibility |
|---------|----------------|
| **CatalogService** | Loads components from embedded assets |
| **ResolverService** | Topological sort with cycle detection |
| **GeneratorService** | Produces install.sh and merges vars.toml + secrets.toml |

### Environment Contract

Catalog installers assume the Jules environment baseline (Python 3.12+, Node.js 22+, common dev tools). The CI `verify-installers` workflow provisions that baseline in minimal containers.
