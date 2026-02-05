# .jules/ Scaffold Design

## Critical Design Principles

### 1. Prompt Hierarchy (No Duplication)
Prompts are constructed as a flat list of contracts in `prompt.yml`.

```yaml
contracts:
  - .jules/JULES.md (global)
  - .jules/roles/<layer>/contracts.yml (layer)
  - .jules/roles/<layer>/<role>/role.yml (role-specific)
```

**Rule**: Never duplicate content across levels. Each level refines the constraints of the previous one.

### 2. Workflow-Driven Execution
Agent execution is orchestrated by GitHub Actions using `jlo run`. The CLI delegates to Jules VM; workflows control scheduling, branching, and merge policies.

## Directory Structure

```
.jules/
├── JULES.md              # Agent contract (formal rules)
├── README.md             # Human guide (informal)
├── config.toml           # Execution configuration (parallelism, API settings)
├── github-labels.json    # Standard issue labels for workstream routing
├── changes/
│   └── latest.yml        # Narrator output (bounded changes summary)
├── roles/
│   ├── narrator/
│   │   ├── prompt.yml    # Entry point
│   │   ├── contracts.yml # Layer contract
│   │   └── schemas/
│   │       └── change.yml
│   ├── observers/
│   │   ├── contracts.yml
│   │   ├── schemas/
│   │   │   └── event.yml
│   │   └── <role>/
│   │       └── role.yml
│   ├── deciders/
│   │   ├── contracts.yml
│   │   ├── schemas/
│   │   │   ├── issue.yml
│   │   └── <role>/
│   │       └── role.yml
│   ├── planners/
│   │   ├── prompt.yml
│   │   └── contracts.yml
│   └── implementers/
│       ├── prompt.yml
│       └── contracts.yml
├── workstreams/
│   └── <workstream>/
│       ├── events/
│       │   └── <state>/
│       │       └── *.yml
│       └── issues/
│           └── <label>/
│               └── *.yml
└── setup/
    ├── tools.yml         # Tool selection
    ├── env.toml          # Environment variables (generated/merged)
    ├── install.sh        # Installation script (generated)
    └── .gitignore        # Ignores env.toml
```

## Document Hierarchy

| Document | Audience | Contains |
|----------|----------|----------|
| `JULES.md` | Jules agents | Formal contracts and schemas |
| `README.md` | Humans | Informal guide |

**Rule**: Jules-internal details stay in `.jules/`. Execution/orchestration belongs in `.github/`.

## Prompt Hierarchy

See "Critical Design Principles" above for the contract structure.

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

The setup compiler generates dependency-aware installation scripts for development tools.

### Component Catalog Structure

```
src/assets/catalog/<component>/
  meta.toml      # name, summary, dependencies, env specs
  install.sh     # Installation script
```

### meta.toml Schema

```toml
name = "component-name"       # Optional; defaults to directory name
summary = "Short description"
dependencies = ["other-comp"] # Optional

[[env]]
name = "ENV_VAR"
description = "What this variable does"
default = "optional-default"  # Optional
```

### Services

| Service | Responsibility |
|---------|----------------|
| CatalogService | Loads components from embedded assets |
| ResolverService | Topological sort with cycle detection |
| GeneratorService | Produces install.sh and merges env.toml |

### Environment Contract

Catalog installers assume the Jules environment baseline (Python 3.12+, Node.js 22+, common dev tools). The CI verify-installers workflow provisions that baseline in minimal containers.

### Setup Directory Contents

The `.jules/setup/` directory contains:

- `tools.yml`: User-selected components
- `env.toml`: Generated environment variables (gitignored)
- `install.sh`: Generated installation script (dependency-sorted)
- `.gitignore`: Excludes `env.toml`
