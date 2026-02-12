# jlo Design Overview

`jlo` manages the `.jules/` scaffolding for agent configuration and assets, and orchestrates the complete autonomous lifecycle via GitHub Actions.

## Architecture

| Component | Responsibility |
|-----------|----------------|
| **jlo** | Scaffold installation, versioning, prompt asset management |
| **GitHub Actions** | Orchestration: cron triggers, matrix execution, auto-merge control |
| **Jules API** | Execution: code analysis, artifact generation, branch/PR creation |

## Critical Design Principles

### 1. Assets are Static Files, Never Hardcoded in Rust
All scaffold files, workflow kits, configurations, and prompts must exist as real files within `src/assets/`.
**Never** embed file contents (like `DEFAULT_CONFIG_TOML`, `tools.yml`, or default `.gitignore`) as string constants in Rust source code.
- **Why**: Keeps the scaffold structure visible and maintainable without digging into implementation details.
- **How**: Use `include_dir!` to load `src/assets/scaffold` and `src/assets/github` as authoritative sources of truth.

### 2. Scaffold Mapping
The directory `src/assets/scaffold/jules/layers` in the source code maps directly to `.jules/roles` in the deployed environment. This renaming (layers -> roles) occurs during scaffold installation to better reflect the user-facing concept of "roles" while maintaining a "layered" architecture internally.

### 3. Prompt Hierarchy (No Duplication)
Prompts are constructed by layer-specific `<layer>_prompt.j2` templates, which render prompt sections via explicit include helpers. Each layer has a single prompt template that references contracts, role definitions, and exchange data.

```jinja
{{ section("Layer Contracts", include_required(".jules/roles/<layer>/contracts.yml")) }}
{{ section("Role", include_required(".jlo/roles/<layer>/" ~ role ~ "/role.yml")) }}
{{ section("Change Summary", include_optional(".jules/exchange/changes.yml")) }}
```

**Rule**: Never duplicate content across levels. Each level refines the constraints of the previous one.

### 4. Workflow-Driven Execution
Agent execution is orchestrated by GitHub Actions using `jlo run`. The CLI delegates to Jules API; workflows control scheduling, branching, and merge policies.

## Development Context

See [src/AGENTS.md](src/AGENTS.md) for development verification commands and CLI architecture details.

## Documentation Index

### Core Guides
- [src/AGENTS.md](src/AGENTS.md) — Rust CLI development context (SSOT for verification)
- [.github/AGENTS.md](.github/AGENTS.md) — GitHub Actions workflows design
- [src/assets/scaffold/AGENTS.md](src/assets/scaffold/AGENTS.md) — `.jules/` scaffold design
- [src/assets/templates/AGENTS.md](src/assets/templates/AGENTS.md) — Template system

### Operational Guides (docs/)
- [Control Plane Ownership](docs/CONTROL_PLANE_OWNERSHIP.md) — `.jlo/` vs `.jules/` ownership model and projection rules
- [Innovator Role Guide](docs/INNOVATOR_ROLE_YML_GUIDE.md) — Design standard for creating innovator personas
- [Observer Role Guide](docs/OBSERVER_ROLE_YML_GUIDE.md) — Design standard for creating observer personas
- [Reproduction Guide](docs/REPRODUCTION_GUIDE.md) — How to reproduce the Jules workflow in other projects
- [Workflow Branch Impact Map](docs/WORKFLOW_BRANCH_IMPACT_MAP.md) — Operational index for branch-contract changes
- [Workflow Layer Change Map](docs/WORKFLOW_LAYER_CHANGE_MAP.md) — Repository touch points for layer-level changes
