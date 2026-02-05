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
│   ├── workflows/     # Workflow kit assets
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
| **Scaffold** | The immutable `.jules/` directory structure managed by `jlo init` and `jlo update`. |
| **Template** | Blueprints for creating new roles or workstreams, applied via `jlo template`. |
| **Workflow kit** | `.github/` automation assets installed by `jlo init workflows`. |
| **Component** | Development tools managed by `jlo setup`, defined in `src/assets/catalog/`. |

## CLI Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jlo init [scaffold]` | `i` | Create `.jules/` structure with setup directory |
| `jlo init workflows (--remote \| --self-hosted)` | `i w` | Install workflow kit into `.github/` |
| `jlo update [--prompt-preview] [--adopt-managed]` | `u` | Update workspace to current jlo version |
| `jlo template [-l layer] [-n name] [-w workstream]` | `tp` | Apply a template (workstream or role) |
| `jlo run narrator [--prompt-preview] [--mock]` | `r n` | Run narrator (produces changes feed) |
| `jlo run observers --workstream <name> [--role <name> \| --scheduled] [--mock]` | `r o` | Run observer agents |
| `jlo run deciders --workstream <name> [--role <name> \| --scheduled] [--mock]` | `r d` | Run decider agents |
| `jlo run planners <path> [--mock]` | `r p` | Run planner (issue-driven) |
| `jlo run implementers <path> [--mock]` | `r i` | Run implementer (issue-driven) |
| `jlo schedule export --scope <scope>` | | Export schedule data (scope: `workstreams` or `roles`) |
| `jlo workstreams inspect --workstream <name> [--format json\|yaml]` | | Inspect workstream state |
| `jlo setup gen [path]` | `s g` | Generate `install.sh` and `env.toml` |
| `jlo setup list` | `s ls` | List available components |
| `jlo setup list --detail <component>` | | Show component details |

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
cargo test --test workflow_kit    # Workflow kit & YAML linting
cargo test --test mock_mode       # Mock execution flow
```

## Layer Architecture

| Layer | Type | Invocation | Config |
|-------|------|------------|--------|
| Narrator | Single-role | `jlo run narrator` | None (git-based) |
| Observers | Multi-role | `jlo run observers --workstream <name>` | `workstreams/<workstream>/scheduled.toml` |
| Deciders | Multi-role | `jlo run deciders --workstream <name>` | `workstreams/<workstream>/scheduled.toml` |
| Planners | Single-role | `jlo run planners <path>` | None (issue path) |
| Implementers | Single-role | `jlo run implementers <path>` | None (issue path) |

**Single-role layers**: Narrator, Planners, Implementers have a fixed role with `prompt.yml` in the layer directory. Template creation not supported.

**Multi-role layers**: Observers and Deciders support multiple configurable roles listed in `workstreams/<workstream>/scheduled.toml`. Each role has its own subdirectory with `prompt.yml`.

## Mock Mode

Mock mode (`--mock`) enables E2E workflow validation without Jules API calls. Scope is auto-generated from `JULES_MOCK_SCOPE` env var or a timestamp.

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
