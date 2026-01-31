# jlo

CLI tool for deploying `.jules/` workspace scaffolding and orchestrating scheduled LLM agent execution.

## Architecture

| Component | Responsibility |
|-----------|----------------|
| **jlo** | `.jules/` scaffold installation, agent orchestration, prompt assembly |
| **GitHub Actions** | Workflow triggers: cron schedules, manual dispatch, merge control |
| **Jules (VM)** | Execution: code analysis, artifact generation, branch/PR creation |

## Quick Start

```bash
cargo install --path .
cd your-project
jlo init
```

Copy the sample workflow from `src/assets/templates/workflows/jules.yml` to your repository's `.github/workflows/`.

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jlo init` | `i` | Create `.jules/` workspace with setup directory |
| `jlo update [--dry-run] [--workflows]` | `u` | Update workspace to current jlo version |
| `jlo template [-l layer] [-n name] [-w workstream]` | `tp` | Apply a template (workstream or role) |
| `jlo run <layer>` | `r` | Execute agents for specified layer |
| `jlo schedule export` | | Export schedule data for automation |
| `jlo workstreams inspect` | | Inspect workstream state for automation |
| `jlo doctor [--fix] [--strict] [--workstream <name>]` | | Validate `.jules/` structure and content |
| `jlo setup gen [path]` | `s g` | Generate `install.sh` and `env.toml` |
| `jlo setup list` | `s ls` | List available components |

### Template Command

`jlo template` opens an interactive wizard to apply a workstream template or create an observer/decider role. When creating roles non-interactively, pass an explicit `--workstream` to avoid defaulting to an unintended workstream.

### Run Command

Execute Jules agents for a specific layer:

```bash
jlo run observers                      # Run all observer roles
jlo run deciders --role triage_generic # Run specific role
jlo run observers --dry-run            # Show prompts without executing
jlo run observers --branch custom      # Override starting branch
```

**Single-Role Layers** (Planners, Implementers) require an issue file:

```bash
# Run planner for a specific issue
jlo run planners .jules/workstreams/generic/issues/<label>/auth_inconsistency.yml

# Run implementer for a specific issue
jlo run implementers .jules/workstreams/generic/issues/<label>/auth_inconsistency.yml
```

Single-role layers are issue-driven and do not support the `--role` flag.

**Flags**:
- `--role <name>`: Run specific role(s) instead of all configured (multi-role layers only)
- `--dry-run`: Show assembled prompts without API calls
- `--branch <name>`: Override the default starting branch
- `<path>`: Local issue file (required for planners and implementers)

**Configuration**: Agent roles are configured in `.jules/config.toml`:

```toml
[agents]
# Multi-role layers: list roles to run
observers = ["taxonomy", "data_arch", "qa", "consistency"]
deciders = ["triage_generic"]
# Single-role layers (planners, implementers) are issue-driven
# and do not require configuration here.

[run]
default_branch = "main"

[jules]
# api_url = "https://api.jules.ai/v1/sessions"
# timeout_secs = 30
# max_retries = 3
```

**Environment**: Set `JULES_API_KEY` for API authentication.

### Doctor Command

Validate the `.jules/` workspace after agent execution:

```bash
jlo doctor
jlo doctor --workstream generic
jlo doctor --strict
jlo doctor --fix
```

Exit codes:
- `0`: No errors (warnings allowed unless `--strict`)
- `1`: Errors detected
- `2`: Warnings detected with `--strict`

### Other Examples

```bash
jlo init                                    # Initialize workspace
jlo template                                # Open interactive template wizard
jlo template -l observers -n security -w generic

# Setup compiler
jlo setup list                              # List available components
jlo setup list --detail just                # Show component details
jlo setup gen                               # Generate install script
```

## GitHub Actions Integration

The simplified workflow uses `jlo run` for all agent execution.

| File | Purpose |
|------|---------|
| `jules.yml` | Agent execution (scheduled + manual dispatch) |

**Branch Strategy**:

| Branch Pattern | Agent Type | Base Branch | Merge Strategy |
|----------------|------------|-------------|----------------|
| `jules` | N/A | `main` | Synced from main |
| `jules-observer-*` | Observers | `jules` | Auto-merged |
| `jules-decider-*` | Deciders | `jules` | Auto-merged |
| `jules-planner-*` | Planners | `jules` | Auto-merged |
| `jules-implementer-*` | Implementers | `main` | Human review |

**Flow**:
1. **Sync**: `jules` branch syncs from `main` periodically
2. **Analysis**: Observers create event files in `.jules/workstreams/<workstream>/events/`
3. **Triage**: Deciders link and consolidate events into issue files
4. **Expansion**: Planners expand issues requiring deep analysis
5. **Implementation**: Implementers are triggered manually with a local issue file

**Pause/Resume**: Set repository variable `JULES_PAUSED=true` to skip scheduled runs.

## Documentation

- **Workflow details**: `.jules/README.md` (created by `jlo init`)
- **Agent contracts**: `.jules/JULES.md` (created by `jlo init`)
- **Development guide**: `AGENTS.md`

## Development

```bash
cargo build                                                    # Build
cargo fmt --check && cargo clippy -- -D warnings               # Lint
cargo test --all-targets --all-features                        # Test
```
