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
- **How**: Use `include_dir!` to load `src/assets/scaffold` and `src/assets/workflows` as authoritative sources of truth.

### 2. Prompt Hierarchy (No Duplication)
Prompts are constructed as a flat list of contracts in `prompt.yml`.

```yaml
contracts:
  - .jules/JULES.md (global)
  - .jules/roles/<layer>/contracts.yml (layer)
  - .jules/roles/<layer>/<role>/role.yml (role-specific)
```

**Rule**: Never duplicate content across levels. Each level refines the constraints of the previous one.

### 3. Workflow-Driven Execution
Agent execution is orchestrated by GitHub Actions using `jlo run`. The CLI delegates to Jules VM; workflows control scheduling, branching, and merge policies.

## Context-Specific Documentation

- [src/AGENTS.md](src/AGENTS.md) — Rust CLI development
- [.github/AGENTS.md](.github/AGENTS.md) — GitHub Actions workflows
- [src/assets/scaffold/AGENTS.md](src/assets/scaffold/AGENTS.md) — `.jules/` scaffold design
- [src/assets/templates/AGENTS.md](src/assets/templates/AGENTS.md) — Template system
