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
- Jules API lane: Layer PRs use `jlo workflow process pr automerge` to delegate merge timing to GitHub asynchronously.
- Programmatic maintenance lane: `jlo workflow push worker-branch` waits for status checks in-process and performs an immediate merge without `--auto`.

`doctor` remains workflow orchestration responsibility.
Programmatic commands do not embed a mandatory internal `doctor` execution; workflows run `jlo workflow doctor` as a separate step after command execution.

### 4. Generated Workflow Files Are Not Manually Edited
Generated workflow files under `.github/workflows/` are projection artifacts from templates in `src/assets/github/workflows/`.
Manual edits to generated files are not part of the maintained state; changes are applied in templates and then regenerated through `jlo workflow generate`.

### 5. Branch Context Terminology Is Explicit
Automation and documentation distinguish only two branch contexts: `target branch` (`JLO_TARGET_BRANCH`) and `worker branch` (`JULES_WORKER_BRANCH`).
Workflow logic, command surfaces, and design descriptions avoid hardcoded branch-name terms such as `main`, `jules`, or `default branch` as normative identifiers.

## Documentation

- [Documentation Index](docs/README.md): The central index for all architectural decisions, operational guides, and design principles.
- [Development Context](src/AGENTS.md): Specific instructions for Rust CLI development and verification.
