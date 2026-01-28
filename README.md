# jlo

CLI tool for deploying `.jules/` workspace scaffolding for scheduled LLM agent execution.

## Architecture

| Component | Responsibility |
|-----------|----------------|
| **jlo** | `.jules/` scaffold installation, versioning, prompt asset management |
| **GitHub Actions** | Orchestration: cron triggers, matrix execution, PR creation, merge control |
| **jules-invoke** | Session creation: prompt delivery, starting_branch specification |
| **Jules (VM)** | Execution: code analysis, artifact generation, branch/PR creation |

## Quick Start

```bash
cargo install --path .
cd your-project
jlo init
```

Then copy `.github/workflows/jules-*.yml` and `.github/actions/read-role-prompt/` from this repository as reference implementations.

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jlo init` | `i` | Create `.jules/` workspace with setup directory |
| `jlo template [-l layer] [-n name]` | `tp` | Create new role from template |
| `jlo setup gen [path]` | `s g` | Generate `install.sh` and `env.toml` |
| `jlo setup list` | `s ls` | List available components |

### Examples

```bash
jlo init                                    # Initialize workspace (includes setup)
jlo template -l observers -n security       # Create custom role

# Setup compiler
jlo setup list                              # List available components
jlo setup list --detail just                # Show component details
# Edit .jules/setup/tools.yml to select tools
jlo setup gen                               # Generate install script
.jules/setup/install.sh                     # Run installation
```

## GitHub Actions Workflows

This repository includes reference workflow implementations in `.github/`:

| File | Type | Purpose |
|------|------|---------|
| `workflows/jules-workflows.yml` | Orchestrator | Coordinates all agent execution |
| `workflows/jules-automerge.yml` | Automation | Auto-merge jules/* branches |
| `workflows/run-observer.yml` | Reusable | Execute single observer |
| `workflows/run-decider.yml` | Reusable | Execute decider |
| `workflows/run-planner.yml` | Reusable | Execute single planner |
| `workflows/run-implementer.yml` | Reusable | Execute single implementer |
| `actions/read-role-prompt/` | Action | Read prompt.yml from role |

**Branch Strategy**:
- `jules/*` branches (observers, deciders, planners): Auto-merged after CI passes
- `impl/*` branches (implementers): Require human review

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


