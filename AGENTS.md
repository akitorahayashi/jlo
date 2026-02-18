# jlo Design Overview

`jlo` manages the `.jules/` scaffolding for agent configuration and assets, and orchestrates the complete autonomous lifecycle via GitHub Actions.

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
The directory `src/assets/scaffold/jules/schemas` in the source code maps directly to `.jules/schemas` in the deployed environment.
Prompt-assembly assets (contracts, tasks, templates) live in `src/assets/prompt-assemble/` and are embedded into the binary via `include_dir!`; they are never deployed to `.jules/`.

### 3. Worker Branch Merge Policy
`JULES_WORKER_BRANCH` is assumed to enforce GitHub Branch protection with `Require a pull request before merging`.

Two merge lanes are intentionally distinct:
- Jules API lane: Layer PRs use `jlo workflow gh pr enable-automerge` (via `--auto`) to delegate merge timing to GitHub asynchronously.
- Programmatic maintenance lane: `jlo workflow gh push worker-branch` waits for status checks in-process and performs an immediate merge without `--auto`.

`doctor` remains workflow orchestration responsibility.
Programmatic commands do not embed a mandatory internal `doctor` execution; workflows run `jlo workflow doctor` as a separate step after command execution.

### 4. Generated Workflow Files Are Not Manually Edited
Generated workflow files under `.github/workflows/` are projection artifacts from templates in `src/assets/github/workflows/`.
Manual edits to generated files are not part of the maintained state; changes are applied in templates and then regenerated through `jlo workflow generate`.

### 5. Branch Context Terminology Is Explicit
Automation and documentation distinguish only two branch contexts: `target branch` (`JLO_TARGET_BRANCH`) and `worker branch` (`JULES_WORKER_BRANCH`).
Workflow logic, command surfaces, and design descriptions avoid hardcoded branch-name terms such as `main`, `jules`, or `default branch` as normative identifiers.

## Development Context

See [src/AGENTS.md](src/AGENTS.md) for development verification commands and CLI architecture details.

## Documentation Index

### Core Guides
- [src/AGENTS.md](src/AGENTS.md) — Rust CLI development context (SSOT for verification)
- [.github/AGENTS.md](.github/AGENTS.md) — GitHub Actions workflows design
- [src/assets/scaffold/AGENTS.md](src/assets/scaffold/AGENTS.md) — `.jules/` scaffold design
- [src/assets/templates/AGENTS.md](src/assets/templates/AGENTS.md) — Template system

### Architectural Guides
- [Control Plane Ownership](docs/architecture/CONTROL_PLANE_OWNERSHIP.md) — `.jlo/` vs `.jules/` ownership model and projection rules
- [Prompt Assembly Policy](docs/architecture/PROMPT_ASSEMBLY.md) — Policy for `contracts.yml` + `tasks/*.yml` assembly

### Operational Guides
- [Reproduction Guide](docs/operations/REPRODUCTION_GUIDE.md) — How to reproduce the Jules workflow in other projects
- [Workflow Branch Impact Map](docs/operations/WORKFLOW_BRANCH_IMPACT_MAP.md) — Operational index for branch-contract changes
- [Workflow Layer Change Map](docs/operations/WORKFLOW_LAYER_CHANGE_MAP.md) — Repository touch points for layer-level changes
