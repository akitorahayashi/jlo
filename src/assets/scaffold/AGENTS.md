# .jules/ Scaffold Design

See [root AGENTS.md](../../../../AGENTS.md) for design principles.

## Directory Structure

```
.jules/
├── JULES.md              # Agent contract (formal rules)
├── README.md             # Human guide (informal)
├── changes/
│   └── latest.yml        # Narrator output (bounded changes summary)
├── roles/
│   ├── narrators/
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
| `change.yml` | `.jules/roles/narrators/schemas/` | Changes summary structure |
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
narrators -> observers -> deciders -> [planners] -> implementers
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
