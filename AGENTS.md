# .jules/ Scaffold Design

See [root AGENTS.md](../../../../AGENTS.md) for design principles.

## Directory Structure

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

## Document Hierarchy

### 3. Workflow-Driven Execution
Agent execution is orchestrated by GitHub Actions using `jlo run`. The CLI delegates to Jules API; workflows control scheduling, branching, and merge policies.

**Rule**: Jules-internal details stay in `.jules/`. Execution/orchestration belongs in `.github/`.

## Prompt Hierarchy

See [root AGENTS.md](../../../../AGENTS.md#2-prompt-hierarchy-no-duplication) for the contract structure.

| File | Scope | Content |
|------|-------|---------|
| `prompt.yml` | Role | Entry point. Lists all contracts to follow. |
| `role.yml` | Role | Specialized focus (observers/deciders only). |
| `contracts.yml` | Layer | Workflow, inputs, outputs, constraints shared within layer. |
| `JULES.md` | Global | Rules applying to ALL layers (branch naming, system boundaries). |

## Schema Files

Schemas define the structure for artifacts produced by agents.

| Schema | Location | Purpose |
|--------|----------|---------|
| `change.yml` | `.jules/roles/narrator/schemas/` | Changes summary structure |
| `event.yml` | `.jules/roles/observers/schemas/` | Observer event structure |
| `issue.yml` | `.jules/roles/deciders/schemas/` | Issue structure |

**Rule**: Agents copy the schema and fill its fields. Never invent structure.

## Workstream Model

Workstreams isolate events and issues so that decider rules do not mix across unrelated operational areas.

- Observers and deciders declare their destination workstream in `prompt.yml` via `workstream: <name>`.
- If the workstream directory is missing, execution fails fast.
- Planners and implementers do not declare a workstream; the issue file path is authoritative.

### Workstream Directories

| Directory | Purpose |
|-----------|---------|
| `.jules/workstreams/<workstream>/events/<state>/` | Observer outputs, Decider inputs |
| `.jules/workstreams/<workstream>/issues/<label>/` | Decider/Planner outputs, Implementer inputs |

## Data Flow

The pipeline is file-based and uses local issues as the handoff point:

```
narrator -> observers -> deciders -> [planners] -> implementers
(changes)   (events)    (issues)    (expand)      (code changes)
```

1. **Narrator** runs first, producing `.jules/changes/latest.yml` for observer context.
2. **Observers** emit events to workstream event directories.
3. **Deciders** read events, emit issues, and link related events via `source_events`.
4. **Planners** expand issues with `requires_deep_analysis: true`.
5. **Implementers** execute approved tasks and create PRs with code changes.

## Setup Compiler

See [src/AGENTS.md](../../../src/AGENTS.md#setup-compiler) for implementation details.

The setup directory contains:
- `tools.yml`: User-selected components
- `env.toml`: Generated environment variables (gitignored)
- `install.sh`: Generated installation script (dependency-sorted)
- `.gitignore`: Excludes `env.toml`
