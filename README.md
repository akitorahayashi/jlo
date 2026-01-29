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
| `jlo template [-l layer] [-n name]` | `tp` | Create new role from template |
| `jlo run <layer>` | `r` | Execute agents for specified layer |
| `jlo setup gen [path]` | `s g` | Generate `install.sh` and `env.toml` |
| `jlo setup list` | `s ls` | List available components |

### Run Command

Execute Jules agents for a specific layer:

```bash
jlo run observers                      # Run all observer roles
jlo run deciders --role triage         # Run specific role
jlo run planners --dry-run             # Show prompts without executing
jlo run observers --branch custom      # Override starting branch
```

**Implementer Invocation** (requires issue file):

```bash
jlo run implementers --issue .jules/exchange/issues/auth_inconsistency.yml
```

Implementers require a local issue file path. The issue content is embedded in the prompt.

**Flags**:
- `--role <name>`: Run specific role(s) instead of all configured
- `--dry-run`: Show assembled prompts without API calls
- `--branch <name>`: Override the default starting branch
- `--issue <path>`: Local issue file (required for implementers)

**Configuration**: Agent roles are configured in `.jules/config.toml`:

```toml
[agents]
observers = ["taxonomy", "data_arch", "qa", "consistency"]
deciders = ["triage"]
planners = ["specifier"]
implementers = ["executor"]

[run]
default_branch = "main"

[jules]
# api_url = "https://api.jules.ai/v1/sessions"
# timeout_secs = 30
# max_retries = 3
```

**Environment**: Set `JULES_API_KEY` for API authentication.

### Other Examples

```bash
jlo init                                    # Initialize workspace
jlo template -l observers -n security       # Create custom role

# Setup compiler
jlo setup list                              # List available components
jlo setup list --detail just                # Show component details
jlo setup gen                               # Generate install script
```

## GitHub Actions Integration

The simplified workflow uses `jlo run` for all agent execution.

| File | Purpose |
|------|---------|
| `jules-workflows.yml` | Agent execution (scheduled + manual dispatch) |
| `jules-automerge.yml` | Auto-merge jules-* branches (optional) |
| `sync-jules.yml` | Sync main → jules branch (optional) |
| `jules-pipeline.yml` | Orchestrate deciders/planners after observer merge |

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
2. **Analysis**: Observers create event files in `.jules/exchange/events/`
3. **Triage**: Deciders consolidate events into issue files
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
