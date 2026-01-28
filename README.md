# jlo

CLI tool for deploying `.jules/` workspace scaffolding for scheduled LLM agent execution.

## Quick Start

```bash
cargo install --path .
cd your-project
jlo init
jlo assign taxonomy src/
```

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jlo init` | `i` | Create `.jules/` workspace structure |
| `jlo assign <role> [paths...]` | `a` | Copy role prompt to clipboard |
| `jlo template [-l layer] [-n name]` | `tp` | Create new role from template |
| `jlo prune -d <days>` | `prn` | Delete old jules/* branches |
| `jlo setup init [path]` | `s init` | Initialize `.jules/setup/` workspace |
| `jlo setup gen [path]` | `s gen` | Generate `install.sh` and `env.toml` |
| `jlo setup list` | `s ls` | List available components |

### Examples

```bash
jlo init                                    # Initialize workspace
jlo assign taxonomy src/                    # Assign role with paths
jlo template -l observers -n security       # Create custom role
jlo prune -d 7                              # Clean up old branches
jlo prune --dry-run -d 7                    # Preview deletions

# Setup compiler
jlo setup init                              # Initialize setup workspace
jlo setup list                              # List available components
jlo setup list --detail just                # Show component details
# Edit .jules/setup/tools.yml to select tools
jlo setup gen                               # Generate install script
.jules/setup/install.sh                     # Run installation
```

## Built-in Roles

| Layer | Role | Responsibility |
|-------|------|----------------|
| Observers | `taxonomy` | Naming conventions |
| Observers | `data_arch` | Data models |
| Observers | `qa` | Test quality |
| Deciders | `triage` | Event screening |
| Planners | `specifier` | Task decomposition |
| Mergers | `consolidator` | Branch consolidation |

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
