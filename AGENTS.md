# jlo Design Overview

`jlo` manages the `.jules/` scaffolding for agent configuration and assets, and orchestrates the complete autonomous lifecycle via GitHub Actions.

## Architecture

| Component | Responsibility |
|-----------|----------------|
| jlo | Scaffold installation, versioning, prompt asset management |
| GitHub Actions | Orchestration: cron triggers, matrix execution, auto-merge control |
| Jules API | Execution: code analysis, artifact generation, branch/PR creation |

## Critical Design Principles

### 1. Assets are Static Files, Never Hardcoded in Rust
All scaffold files, workflow kits, configurations, and prompts must exist as real files within `src/assets/`.
Never embed file contents (like `DEFAULT_CONFIG_TOML`, `tools.yml`, or default `.gitignore`) as string constants in Rust source code.
- Why: Keeps the scaffold structure visible and maintainable without digging into implementation details.
- How: Use `include_dir!` to load `src/assets/scaffold` and `src/assets/github` as authoritative sources of truth.

### 2. Scaffold Mapping
The directory `src/assets/scaffold/jules/layers` in the source code maps directly to `.jules/layers` in the deployed environment. This structure aligns the internal source organization with the deployed "layered" pipeline concept.

### 3. Worker Branch Merge Policy
`JULES_WORKER_BRANCH` is assumed to enforce GitHub Branch protection with `Require a pull request before merging`.

Two merge lanes are intentionally distinct:
- Jules API lane: Jules-created layer PRs use the existing Jules PR processing and auto-merge policy.
- Programmatic maintenance lane: worker-branch runtime maintenance updates are merged through `jlo workflow gh push worker-branch` (PR create + merge path), not by direct push.

`doctor` remains workflow orchestration responsibility.
Programmatic commands do not embed a mandatory internal `doctor` execution; workflows run `jlo workflow doctor` as a separate step after command execution.

## Development Context

See [src/AGENTS.md](src/AGENTS.md) for development verification commands and CLI architecture details.

## Documentation Index

### Core Guides
- [src/AGENTS.md](src/AGENTS.md) — Rust CLI development context (SSOT for verification)
- [.github/AGENTS.md](.github/AGENTS.md) — GitHub Actions workflows design
- [src/assets/scaffold/AGENTS.md](src/assets/scaffold/AGENTS.md) — `.jules/` scaffold design
- [src/assets/templates/AGENTS.md](src/assets/templates/AGENTS.md) — Template system

### Documentation Index
- [Documentation Index](docs/README.md) — Central index for all documentation.

### Architectural Guides
- [Control Plane Ownership](docs/architecture/CONTROL_PLANE_OWNERSHIP.md) — `.jlo/` vs `.jules/` ownership model and projection rules
- [Prompt Assembly Policy](docs/architecture/PROMPT_ASSEMBLY.md) — Policy for `contracts.yml` + `tasks/*.yml` assembly

### Operational Guides
- [Reproduction Guide](docs/operations/REPRODUCTION_GUIDE.md) — How to reproduce the Jules workflow in other projects
- [Workflow Branch Impact Map](docs/operations/WORKFLOW_BRANCH_IMPACT_MAP.md) — Operational index for branch-contract changes
- [Workflow Layer Change Map](docs/operations/WORKFLOW_LAYER_CHANGE_MAP.md) — Repository touch points for layer-level changes
