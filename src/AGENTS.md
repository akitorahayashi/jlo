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
│   └── catalog/       # Setup component definitions
└── testing/           # Mock implementations
tests/
├── common/            # Shared test fixtures
├── cli_commands.rs    # CLI tests
├── cli_flow.rs        # Workflow tests
└── commands_api.rs    # Library API tests
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
| **Component** | Development tools managed by `jlo setup`, defined in `src/assets/catalog/`. |

## Domain Modules

Core domain logic located in `src/domain/`.

| Module | Purpose |
|--------|---------|
| `configuration` | Global configuration models (`config.toml`, `scheduled.toml`). |
| `identifiers` | Structural identifiers (`RoleId`, `Layer`). |
| `prompt` | Prompt assembly and template rendering models. |
| `workspace` | Filesystem abstraction and path management. |
| `error` | `AppError` and error handling types. |
| `issue` | Issue parsing and schema validation. |

## CLI Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jlo init (--remote \| --self-hosted)` | `i` | Create `.jlo/` control plane and install workflow scaffold |
| `jlo update [--prompt-preview]` | `u` | Advance `.jlo/` control-plane version pin |
| `jlo create <layer> <name>` | `c` | Create a custom role under `.jlo/` |
| `jlo run narrator [--prompt-preview] [--branch <branch>] [--mock]` | `r n` | Run narrator (produces changes feed) |
| `jlo run observers --role <role> [--prompt-preview] [--branch <branch>] [--mock]` | `r o` | Run observer agents |
| `jlo run deciders [--prompt-preview] [--branch <branch>] [--mock]` | `r d` | Run decider agents |
| `jlo run planners <issue> [--prompt-preview] [--branch <branch>] [--mock]` | `r p` | Run planner (issue-driven) |
| `jlo run implementers <issue> [--prompt-preview] [--branch <branch>] [--mock]` | `r i` | Run implementer (issue-driven) |
| `jlo run innovators --role <role> --phase <creation\|refinement> [--prompt-preview] [--branch <branch>] [--mock]` | `r x` | Run innovator agents |
| `jlo doctor [--strict]` | | Validate .jules/ structure and content |
| `jlo workflow doctor` | `wf` | Validate workspace for workflow use |
| `jlo workflow matrix pending` | | Generate pending execution matrix for GitHub Actions |
| `jlo workflow matrix routing --routing-labels <csv>` | | Generate routing matrix for issues |
| `jlo workflow run <layer> [--mock]` | | Run layer and return wait-gating metadata |
| `jlo workflow cleanup mock --mock-tag <tag> [--pr-numbers-json <json>] [--branches-json <json>]` | | Cleanup mock artifacts |
| `jlo workflow pr comment-summary-request <pr_number>` | | Post or update summary-request comment on a Jules PR |
| `jlo workflow pr sync-category-label <pr_number>` | | Sync implementer category label from branch to PR |
| `jlo workflow pr enable-automerge <pr_number>` | | Enable auto-merge on an eligible PR |
| `jlo workflow pr process <pr_number>` | | Run all event-level PR commands in order |
| `jlo workflow issue label-innovator <issue_number> <persona>` | | Apply innovator labels to a proposal issue |
| `jlo workflow inspect` | | Inspect exchange state |
| `jlo workflow clean-issue <issue_file>` | | Remove a processed issue and its source events |
| `jlo workflow publish-proposals` | | Publish innovator proposals as GitHub issues |
| `jlo setup gen [path]` | `s g` | Generate `install.sh` and `env.toml` |
| `jlo setup list [--detail <component>]` | `s ls` | List available components |
| `jlo deinit` | | Remove all jlo-managed assets (`.jlo/`, branch, workflows) |

## Verification Commands

### Full Suite

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

### Partial Testing

Run only relevant tests to save time:

```bash
# By name (substring match)
cargo test <test_name>

# By integration test file
cargo test --test cli_commands    # CLI parsing & commands
cargo test --test workflow_scaffold    # Workflow scaffold & YAML linting
cargo test --test mock_mode       # Mock execution flow
```

## Layer Architecture

| Layer | Type | Invocation | Config |
|-------|------|------------|--------|
| Narrator | Single-role | `jlo run narrator` | None (git-based) |
| Observers | Multi-role | `jlo workflow run observers` | `.jlo/scheduled.toml` |
| Deciders | Single-role | `jlo run deciders` | None |
| Planners | Single-role | `jlo run planners <path>` | None (issue path) |
| Implementers | Single-role | `jlo run implementers <path>` | None (issue path) |
| Innovators | Multi-role | `jlo workflow run innovators` | `.jlo/scheduled.toml` |

**Single-role layers**: Narrator, Deciders, Planners, Implementers have a fixed role with `prompt.yml` in the layer directory. Template creation not supported.

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
src/assets/catalog/<component>/
  meta.toml      # name, summary, dependencies, env specs
  install.sh     # Installation script
```

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

| Service | Responsibility |
|---------|----------------|
| **CatalogService** | Loads components from embedded assets |
| **ResolverService** | Topological sort with cycle detection |
| **GeneratorService** | Produces install.sh and merges env.toml |

### Environment Contract

Catalog installers assume the Jules environment baseline (Python 3.12+, Node.js 22+, common dev tools). The CI `verify-installers` workflow provisions that baseline in minimal containers.
