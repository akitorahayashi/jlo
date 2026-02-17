# jlo

CLI tool for managing `.jlo/` control-plane scaffolding and orchestrating scheduled LLM agent execution via GitHub Actions.

## Architecture

| Component | Responsibility |
|-----------|----------------|
| **jlo** | `.jlo/` control-plane management, `.jules/` scaffold installation, prompt assembly |
| **GitHub Actions** | Workflow triggers: cron schedules, bootstrap, merge control |
| **Jules API** | Execution: code analysis, artifact generation, branch/PR creation |

### Branch Topology

| Branch | Purpose |
|--------|---------|
| Control branch (e.g. `main`) | Hosts `.jlo/` intent overlay and `.github/` workflow scaffold |
| `jules` | Hosts materialized `.jules/` runtime state (managed by workflow bootstrap) |

## Quick Start

```bash
cargo install --path .
cd your-project
jlo init --remote
```

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jlo init (--remote \| --self-hosted)` | `i` | Create `.jlo/` control plane and install workflow scaffold |
| `jlo update [--prompt-preview] [--cli]` | `u` | Advance version pin, refresh workflow scaffold, and refresh unchanged defaults. Use `--cli` to update the binary itself. |
| `jlo deinit` | | Remove `.jlo/`, workflow scaffold, and local `jules` branch |
| `jlo create [<layer> <name>]` | `cr` | Create a custom role under `.jlo/` (observers, innovators) |
| `jlo add [<layer> <role>]` | `a, ad` | Install a built-in role under `.jlo/` (observers, innovators) |
| `jlo run <layer> [role]` | `r` | Execute agents for specified layer |
| `jlo doctor [--strict]` | | Validate `.jules/` structure and content |
| `jlo workflow run <layer>` | `wf` | Run layer and return orchestration metadata |
| `jlo workflow exchange inspect` | | Inspect exchange state for automation |
| `jlo workflow exchange publish-proposals` | | Publish innovator proposals as GitHub issues |
| `jlo workflow exchange clean requirement <file>` | | Remove a processed requirement and its source events |
| `jlo workflow exchange clean mock --mock-tag <tag>` | | Clean up mock artifacts |
| `jlo workflow gh process pr <all\|metadata\|automerge> <pr_number>` | | Run PR process pipeline (add `--fail-on-error` to fail on step errors) |
| `jlo workflow gh process issue label-innovator <issue> <persona>` | | Apply innovator labels to a proposal issue |
| `jlo workflow generate <mode> [--output-dir <dir>]` | `g [-o]` | Generate workflow scaffold files to an output directory |
| `jlo setup gen [path]` | `s g` | Generate `.jlo/setup/install.sh`, `.jlo/setup/vars.toml`, and `.jlo/setup/secrets.toml` |
| `jlo setup list` | `s ls` | List available components |

### Create Command

`jlo create` authors new custom roles for multi-role layers. When no arguments are provided, it
prompts for the layer and role name.

```bash
jlo create observers taxonomy     # Create observer role
jlo create innovators researcher  # Create innovator role
```

### Add Command

`jlo add` installs built-in roles from the embedded catalog. When no arguments are provided, it
guides you through layer, category, and role selection.

```bash
jlo add observers pythonista       # Install built-in observer role
jlo add innovators recruiter       # Install built-in innovator role
```

### Run Command

Execute Jules agents for a specific layer. You can use `r` as an alias for `run`, and short aliases for layers: `n` (narrator), `o` (observer), `d` (decider), `p` (planner), `i` (implementer), `x` (innovator).

**Multi-role layers** (Observer, Innovator) require a role argument:

```bash
jlo run observer <role>                    # Run specific observer role
jlo run observer <role> --prompt-preview   # Show prompts without executing
jlo run observer <role> --branch custom    # Override starting branch
jlo run innovator <role> --task create_three_proposals  # Run innovator role with a task
```

**Single-role layers** (Narrator, Decider, Planner, Implementer):

```bash
jlo run narrator                     # Run narrator (no role flag needed)
jlo run decider                      # Run decider (single role)
```

**Requirement-driven layers** (Planner, Implementer) require a requirement file:

```bash
jlo run planner .jules/exchange/requirements/auth-inconsistency.yml
jlo run implementer .jules/exchange/requirements/auth-inconsistency.yml
```

**Mock Mode**: Validate workflow orchestration without calling Jules API:

```bash
jlo run narrator --mock
jlo run observer <role> --mock
jlo run decider --mock
jlo run innovator <role> --mock
```

Mock mode creates real branches and PRs with synthetic commit content, enabling E2E workflow validation in CI. The mock tag is auto-generated from `JULES_MOCK_TAG` env var or a timestamp.

**Flags**:
- `--task <name>`: Innovator task selector (`create_three_proposals`)
- `--prompt-preview`: Show assembled prompts without API calls
- `--mock`: Use mock execution (creates branches/PRs without Jules API)
- `--branch <name>`: Override the default starting branch
- `<path>`: Local requirement file (required for planner and implementer)

**Configuration**: Execution settings are configured in `.jlo/config.toml`:

```toml
[run]
jlo_target_branch = "main"

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
jlo doctor --strict
```

Exit codes:
- `0`: No errors (warnings allowed unless `--strict`)
- `1`: Errors detected
- `2`: Warnings detected with `--strict`

### Deinit Command

`jlo deinit` removes the `.jlo/` control plane, the local `JULES_WORKER_BRANCH`, and workflow scaffold files from `.github/`.
The command refuses to run while the current branch is `JULES_WORKER_BRANCH` or `jules-test-*`.
GitHub secrets (such as `JULES_API_KEY` and `JLO_BOT_TOKEN`) remain configured and require manual removal.

### Other Examples

```bash
jlo init --remote                           # Initialize control plane + workflow scaffold (GitHub-hosted)
jlo init --self-hosted                      # Initialize control plane + workflow scaffold (self-hosted)
jlo create observers security               # Create observer role
jlo create innovators researcher            # Create innovator role
jlo add observers pythonista                # Install built-in observer role

# Setup compiler
jlo setup list                              # List available components
jlo setup list --detail just                # Show component details
jlo setup gen                               # Generate install.sh + vars.toml + secrets.toml
```

## GitHub Actions Integration

`jlo init --remote` (or `--self-hosted`) installs the Jules orchestration files in `.github/`.

Workflows use `jlo workflow bootstrap` to materialize `.jules/` on `JULES_WORKER_BRANCH`, then `jlo workflow run` for agent execution.

Workflow scaffold layout:

- `.github/workflows/jules-*.yml`
- `.github/actions/` (Jules composite actions)

**Configuration Variables**:

| Variable | Purpose | Default |
|----------|---------|---------|
| `JLO_PAUSED` | Skip scheduled runs when set to `true` | `false` |
| `JLO_TARGET_BRANCH` | Control branch for `.jlo/` and implementer output | `main` |
| `JULES_WORKER_BRANCH` | Runtime branch for `.jules/` execution | `jules` |

Workflow expressions read these values from GitHub Actions variables (`vars.*`), so define them as repository variables (for example, `vars.JLO_PAUSED`).

**Workflow Timing**: Schedule cron entries and the default wait minutes are rendered from `.jlo/config.toml` (`[workflow]`) at install time. Reinstalling the kit overwrites existing schedule and wait defaults with the config values.

**Branch Strategy**:

| Branch Pattern | Agent Type | Base Branch | Merge Strategy |
|----------------|------------|-------------|----------------|
| `JULES_WORKER_BRANCH` | N/A | `JLO_TARGET_BRANCH` | Synced from target |
| `jules-observer-*` | Observers | `JULES_WORKER_BRANCH` | Auto-merged |
| `jules-decider-*` | Decider | `JULES_WORKER_BRANCH` | Auto-merged |
| `jules-planner-*` | Planner | `JULES_WORKER_BRANCH` | Auto-merged |
| `jules-implementer-*` | Implementer | `JLO_TARGET_BRANCH` | Human review |
| `jules-innovator-*` | Innovators | `JULES_WORKER_BRANCH` | Auto-merged |
| `jules-mock-cleanup-*` | Mock cleanup | `JULES_WORKER_BRANCH` | Auto-merged |

Auto-merge authority is centralized in `.github/workflows/jules-automerge.yml`, triggered by push on the Jules auto-merge branch families.
Cleanup keeps a PR-based merge path for branch-protection compatibility and auditable history.

**Flow**:
1. **Sync**: `JULES_WORKER_BRANCH` syncs from `JLO_TARGET_BRANCH` periodically
2. **Analysis**: Observers create event files under `.jules/exchange/events/`
3. **Triage**: Decider links and consolidates events into requirements
4. **Expansion**: Planner expands requirements that require deep analysis
5. **Implementation**: Implementer implements solutions for requirements, either automatically via workflow or manually with a specified requirement file
6. **Innovation**: Innovators generate ideas and proposals, published as GitHub issues

**Pause/Resume**: Set the repository pause variable referenced by the workflows to skip scheduled runs.

## Documentation

- **Control plane ownership**: `docs/CONTROL_PLANE_OWNERSHIP.md`
- **Workflow details**: `.jules/README.md` (materialized by bootstrap)
- **Agent contracts**: `.jules/JULES.md` (materialized by bootstrap)
- **Development guide**: `AGENTS.md`

## Development

```bash
cargo build                                                    # Build
cargo fmt                                                      # Format
cargo clippy --all-targets --all-features -- -D warnings       # Lint
cargo test --all-targets --all-features                        # Test
```

### Workflow Linting (actionlint)

Workflow scaffold generation and linting are deterministic and run against generated output under `.tmp/`.
This catches workflow expression-context errors (for example, invalid `vars`/`inputs` usage) before changes are pushed.

```bash
just setup
just alint
```

The `alint` recipe generates both runner modes and runs `actionlint` via `aqua exec`.
