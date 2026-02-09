# .jules/ Scaffold Design

## Critical Design Principles

### 1. Prompt Hierarchy (No Duplication)
Prompts are constructed by `prompt_assembly.j2`, which renders prompt sections via explicit include helpers. Each layer has a single `prompt_assembly.j2` that references contracts, role definitions, and exchange data.

```jinja
{{ section("Role", include_required(".jules/roles/<layer>/roles/" ~ role ~ "/role.yml")) }}
{{ section("Layer Contracts", include_required(".jules/roles/<layer>/contracts.yml")) }}
{{ section("Change Summary", include_optional(".jules/changes/latest.yml")) }}
```

**Rule**: Never duplicate content across levels. Each level refines the constraints of the previous one.

### 2. Workflow-Driven Execution
Agent execution is orchestrated by GitHub Actions using `jlo run`. The CLI delegates to Jules API; workflows control scheduling, branching, and merge policies.

## Directory Structure

```
.jules/
├── JULES.md              # Agent contract (formal rules)
├── README.md             # Human guide (informal)
├── config.toml           # Workspace configuration
├── github-labels.json    # GitHub labels definition
├── changes/
│   ├── latest.yml        # Narrator output (bounded changes summary)
│   └── .gitkeep          # Ensures directory exists in git
├── roles/
│   ├── narrator/
│   │   ├── prompt_assembly.j2 # Prompt construction rules
│   │   ├── contracts.yml # Layer contract
│   │   └── schemas/
│   │       └── change.yml
│   ├── observers/
│   │   ├── prompt_assembly.j2 # Prompt construction rules
│   │   ├── contracts.yml # Layer contract
│   │   ├── schemas/
│   │   │   ├── event.yml
│   │   │   └── perspective.yml
│   │   └── roles/
│   │       ├── <role>/
│   │       │   └── role.yml
│   │       └── .gitkeep
│   ├── deciders/
│   │   ├── prompt_assembly.j2 # Prompt construction rules
│   │   ├── contracts.yml # Layer contract
│   │   ├── schemas/
│   │   │   └── issue.yml
│   │   └── roles/
│   │       ├── <role>/
│   │       │   └── role.yml
│   │       └── .gitkeep
│   ├── planners/
│   │   ├── prompt_assembly.j2 # Prompt construction rules
│   │   └── contracts.yml
│   ├── implementers/
│   │   ├── prompt_assembly.j2 # Prompt construction rules
│   │   └── contracts.yml
│   └── innovators/
│       ├── prompt_assembly.j2      # Prompt construction (uses {{phase}})
│       ├── contracts_creation.yml   # Creation phase contract
│       ├── contracts_refinement.yml # Refinement phase contract
│       ├── schemas/
│       │   ├── perspective.yml
│       │   ├── idea.yml
│       │   ├── proposal.yml
│       │   └── comment.yml
│       └── roles/
│           ├── <persona>/
│           │   └── role.yml
│           └── .gitkeep
├── workstreams/
│   └── <workstream>/
│       ├── exchange/
│       │   ├── events/
│       │   │   ├── pending/
│       │   │   │   └── .gitkeep
│       │   │   └── decided/
│       │   │   │   └── .gitkeep
│       │   ├── issues/
│       │   │   ├── <label>/
│       │   │   │   └── .gitkeep
│       │   │   └── .gitkeep
│       │   └── innovators/
│       │       └── <persona>/
│       │           ├── perspective.yml
│       │           ├── idea.yml       # Temporary (creation phase)
│       │           ├── proposal.yml   # Temporary (refinement output)
│       │           └── comments/
│       │               └── .gitkeep
│       └── workstations/
│           └── <role>/
│               └── perspective.yml
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
| `prompt_assembly.j2` | Layer | Prompt template that assembles contracts and includes. |
| `role.yml` | Role | Specialized focus (observers/deciders/innovators). |
| `contracts.yml` | Layer | Workflow, inputs, outputs, constraints shared within layer. |
| `contracts_<phase>.yml` | Phase | Phase-specific contracts (innovators only: creation, refinement). |
| `JULES.md` | Global | Rules applying to ALL layers (branch naming, system boundaries). |

## Schema Files

Schemas define the structure for artifacts produced by agents.

| Schema | Location | Purpose |
|--------|----------|---------|
| `change.yml` | `.jules/roles/narrator/schemas/` | Changes summary structure |
| `event.yml` | `.jules/roles/observers/schemas/` | Observer event structure |
| `perspective.yml` | `.jules/roles/observers/schemas/` | Observer perspective structure |
| `issue.yml` | `.jules/roles/deciders/schemas/` | Issue structure |
| `perspective.yml` | `.jules/roles/innovators/schemas/` | Innovator persona memory |
| `idea.yml` | `.jules/roles/innovators/schemas/` | Idea draft structure |
| `proposal.yml` | `.jules/roles/innovators/schemas/` | Finalized proposal structure |
| `comment.yml` | `.jules/roles/innovators/schemas/` | Observer feedback on ideas |

**Rule**: Agents copy the schema and fill its fields. Never invent structure.

## Workstream Model

Workstreams isolate events and issues so that decider rules do not mix across unrelated operational areas.

- Observers and deciders declare their destination workstream via the `workstream` runtime context variable in `prompt_assembly.j2`.
- If the workstream directory is missing, execution fails fast.
- Planners and implementers do not declare a workstream; the issue file path is authoritative.

### Workstream Directories

| Directory | Purpose |
|-----------|---------|
| `.jules/workstreams/<workstream>/exchange/events/<state>/` | Observer outputs, Decider inputs |
| `.jules/workstreams/<workstream>/exchange/issues/<label>/` | Decider/Planner outputs, Implementer inputs |
| `.jules/workstreams/<workstream>/exchange/innovators/<persona>/` | Innovator perspectives, ideas, proposals, comments |
| `.jules/workstreams/<workstream>/workstations/<role>/` | Role perspectives (memory) |

## Data Flow

The pipeline is file-based and uses local issues as the handoff point:

```
narrator -> observers -> deciders -> [planners] -> implementers
(changes)   (events)    (issues)    (expand)      (code changes)

innovators (independent cycle)
perspective -> idea -> comments -> proposal
```

1. **Narrator** runs first, producing `.jules/changes/latest.yml` for observer context.
2. **Observers** emit events to workstream event directories.
3. **Deciders** read events, emit issues, and link related events via `source_events`.
4. **Planners** expand issues with `requires_deep_analysis: true`.
5. **Implementers** execute approved tasks and create PRs with code changes.
6. **Innovators** run independently: each persona maintains a `perspective.yml`, drafts `idea.yml`, receives `comments/` from other personas, and produces `proposal.yml`.
