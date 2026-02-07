# jlo

CLI tool for deploying `.jules/` workspace scaffolding and orchestrating scheduled LLM agent execution.

## Architecture

| Component | Responsibility |
|-----------|----------------|
| **jlo** | `.jules/` scaffold installation, agent orchestration, prompt assembly |
| **GitHub Actions** | Workflow triggers: cron schedules, manual dispatch, merge control |
| **Jules API** | Execution: code analysis, artifact generation, branch/PR creation |

## Quick Start

```bash
cargo install --path .
cd your-project
jlo init
```

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jlo init [scaffold]` | `i` | Create `.jules/` workspace with setup directory |
| `jlo init workflows (--remote | --self-hosted)` | `i w` | Install workflow kit into `.github/` |
| `jlo update [--prompt-preview]` | `u` | Advance .jlo/ control-plane version pin |
| `jlo deinit` | | Remove jlo-managed branch and workflow kit files |
| `jlo template [-l layer] [-n name] [-w workstream]` | `tp` | Apply a template (workstream or role) |
| `jlo run <layer>` | `r` | Execute agents for specified layer |
| `jlo workflow doctor [--workstream <name>]` | | Validate workspace for workflow use |
| `jlo workflow matrix <cmd>` | | Generate GitHub Actions matrices |
| `jlo workflow run <layer> [--matrix-json <json>] [--mock]` | | Run layer with JSON output |
| `jlo workflow workstreams inspect <workstream>` | | Inspect workstream state for automation |
| `jlo workflow workstreams clean issue <issue_file>` | | Remove a processed issue and its source events |
| `jlo workflow workstreams publish-proposals <workstream>` | | Publish innovator proposals as GitHub issues |
| `jlo doctor [--fix] [--strict] [--workstream <name>]` | | Validate `.jules/` structure and content |
| `jlo setup gen [path]` | `s g` | Generate `install.sh` script and `env.toml` |
| `jlo setup list` | `s ls` | List available components |

### Template Command

`jlo template` opens an interactive wizard to apply a workstream template or create an observer/decider role. When creating roles non-interactively, pass an explicit `--workstream` to avoid defaulting to an unintended workstream.

### Run Command

Execute Jules agents for a specific layer. You can use `r` as an alias for `run`, and short aliases for layers: `o` (observers), `d` (deciders), `p` (planners), `i` (implementers), `x` (innovators) (e.g., `jlo r o ...`).

```bash
jlo run observers --workstream generic --scheduled            # Run scheduled observer roles
jlo run deciders --workstream generic --scheduled             # Run scheduled decider roles
jlo run observers --workstream generic --role <role>          # Run specific role (manual)
jlo run observers --workstream generic --role <role1> --role <role2> # Run specific roles (manual)
jlo run observers --workstream generic --scheduled --prompt-preview   # Show prompts without executing
jlo run observers --workstream generic --scheduled --branch custom # Override starting branch
```

**Single-Role Layers** (Planners, Implementers) require an issue file:

```bash
# Run planner for a specific issue
jlo run planners .jules/workstreams/generic/issues/<label>/auth-inconsistency.yml

# Run implementer for a specific issue
jlo run implementers .jules/workstreams/generic/issues/<label>/auth-inconsistency.yml
```

Single-role layers are issue-driven and do not support the `--role` flag.

**Mock Mode**: Validate workflow orchestration without calling Jules API:

```bash
jlo run narrator --mock             # Mock narrator execution
jlo run observers --mock            # Mock observer execution
jlo run deciders --mock             # Mock decider execution
jlo run innovators --mock           # Mock innovator execution (toggle idea.yml)
```

Mock mode creates real branches and PRs with synthetic commit content, enabling E2E workflow validation in CI. The mock tag is auto-generated from `JULES_MOCK_TAG` env var or a timestamp.

**Flags**:
- `-w, --workstream <name>`: Target workstream (required for observers/deciders)
- `--scheduled`: Use roles from `scheduled.toml`
- `-r, --role <name>`: Run specific role(s) (manual mode only)
- `--prompt-preview`: Show assembled prompts without API calls
- `--mock`: Use mock execution (creates branches/PRs without Jules API)
- `--branch <name>`: Override the default starting branch
- `<path>`: Local issue file (required for planners and implementers)

**Configuration**: Execution settings are configured in `.jules/config.toml`:

```toml
[run]
default_branch = "main"

[jules]
# api_url = "https://jules.googleapis.com/v1alpha/sessions"
# timeout_secs = 30
# max_retries = 3
```

**Environment**: Set the API key environment variable referenced by the workflows for authentication.

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

### Deinit Command

`jlo deinit` removes the local `jules` branch and deletes workflow kit files from `.github/` on the current branch.
The command refuses to run while the current branch is `jules` or `jules-test-*`.
GitHub secrets (such as `JULES_API_KEY` and `JULES_API_SECRET`) remain configured and require manual removal.

### Other Examples

```bash
jlo init                                    # Initialize workspace
jlo init workflows --remote                 # Install workflow kit (GitHub-hosted)
jlo init workflows --self-hosted            # Install workflow kit (self-hosted runners)
jlo template                                # Open interactive template wizard
jlo template -l observers -n security -w generic

# Setup compiler
jlo setup list                              # List available components
jlo setup list --detail just                # Show component details
jlo setup gen                               # Generate install script
```

## GitHub Actions Integration

Install the workflow kit with `jlo init workflows` to populate the Jules orchestration files in `.github/`.

Workflows use `jlo run` for agent execution and `jlo workflow` commands for orchestration.

Workflow kit layout:

- `.github/workflows/jules-*.yml`
- `.github/actions/` (Jules composite actions)

**Configuration Variables**:

| Variable | Purpose | Default |
|----------|---------|---------|
| `JULES_PAUSED` | Skip scheduled runs when set to `true` | (unset) |
| `JULES_TARGET_BRANCH` | Target branch for implementer output | `main` |

**Schedule Preservation**: When reinstalling with `jlo init workflows`, the existing `on.schedule` block in `jules-workflows.yml` is preserved.

**Branch Strategy**:

| Branch Pattern | Agent Type | Base Branch | Merge Strategy |
|----------------|------------|-------------|----------------|
| `jules` | N/A | `main` | Synced from main |
| `jules-observer-*` | Observers | `jules` | Auto-merged |
| `jules-decider-*` | Deciders | `jules` | Auto-merged |
| `jules-planner-*` | Planners | `jules` | Auto-merged |
| `jules-implementer-*` | Implementers | `main` | Human review |
| `jules-innovator-*` | Innovators | `jules` | Auto-merged |

**Flow**:
1. **Sync**: `jules` branch syncs from `main` periodically
2. **Analysis**: Observers create event files under `.jules/workstreams/<workstream>/events/`
3. **Triage**: Deciders link and consolidate events into issue files
4. **Expansion**: Planners expand issues that require deep analysis
5. **Implementation**: Implementers are dispatched by workflow policy or manual dispatch with a local issue file
6. **Innovation**: Innovators generate ideas and proposals, published as GitHub issues

**Pause/Resume**: Set the repository pause variable referenced by the workflows to skip scheduled runs.

## Documentation

- **Workflow details**: `.jules/README.md` (created by `jlo init`)
- **Agent contracts**: `.jules/JULES.md` (created by `jlo init`)
- **Development guide**: `AGENTS.md`

## Development

```bash
cargo build                                                    # Build
cargo fmt                                                      # Format
cargo clippy --all-targets --all-features -- -D warnings       # Lint
cargo test --all-targets --all-features                        # Test
```
