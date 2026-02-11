# Scaffold Design

## Critical Design Principles

### 1. Prompt Hierarchy (No Duplication)
Prompts are constructed by layer-specific `<layer>_prompt.j2` templates, which render prompt sections via explicit include helpers. Each layer has a single prompt template that references contracts, role definitions, and exchange data.

```jinja
{{ section("Layer Contracts", include_required(".jules/roles/<layer>/contracts.yml")) }}
{{ section("Role", include_required(".jlo/roles/<layer>/" ~ role ~ "/role.yml")) }}
{{ section("Change Summary", include_optional(".jules/exchange/changes.yml")) }}
```

**Rule**: Never duplicate content across levels. Each level refines the constraints of the previous one.

### 2. Workflow-Driven Execution
Agent execution is orchestrated by GitHub Actions using `jlo run`. The CLI delegates to Jules API; workflows control scheduling, branching, and merge policies.

## Directory Structure

```
.jlo/ (Configuration & Instance)
├── config.toml           # Workspace configuration
├── scheduled.toml        # Scheduled tasks
├── roles/                # Role instance configurations
│   ├── <layer>/
│   │   ├── role.yml      # Role-specific configuration
│   │   └── .gitkeep
│   └── .gitkeep
└── setup/
    ├── tools.yml         # Tool selection
    ├── env.toml          # Environment variables (generated/merged)
    ├── install.sh        # Installation script (generated)
    └── .gitignore        # Ignores env.toml

.jules/ (System Definition)
├── JULES.md              # Agent contract (formal rules)
├── README.md             # Human guide (informal)
├── github-labels.json    # GitHub labels definition
├── roles/
│   ├── narrator/
│   │   ├── narrator_prompt.j2       # Prompt construction rules
│   │   ├── contracts.yml            # Layer contract
│   │   ├── tasks/                   # Action units
│   │   │   ├── bootstrap_summary.yml
│   │   │   └── overwrite_summary.yml
│   │   └── schemas/
│   │       └── changes.yml
│   ├── observers/
│   │   ├── observers_prompt.j2 # Prompt construction rules
│   │   ├── contracts.yml      # Layer contract
│   │   ├── tasks/             # Action units
│   │   └── schemas/
│   │       ├── event.yml
│   │       └── perspective.yml
│   ├── decider/
│   │   ├── decider_prompt.j2 # Prompt construction rules
│   │   ├── contracts.yml      # Layer contract
│   │   ├── tasks/             # Action units
│   │   └── schemas/
│   │       └── issue.yml
│   ├── planner/
│   │   ├── planner_prompt.j2 # Prompt construction rules
│   │   ├── contracts.yml      # Layer contract
│   │   └── tasks/             # Action units
│   ├── implementer/
│   │   ├── implementer_prompt.j2 # Prompt construction rules
│   │   ├── contracts.yml      # Layer contract
│   │   └── tasks/             # Action units
│   └── innovators/
│       ├── innovators_prompt.j2      # Prompt construction (uses {{phase}})
│       ├── contracts.yml             # Layer contract
│       ├── tasks/
│       │   ├── create_idea.yml       # Creation phase task
│       │   └── refine_proposal.yml   # Refinement phase task
│       └── schemas/
│           ├── perspective.yml
│           ├── idea.yml
│           ├── proposal.yml
│           └── comment.yml
├── exchange/
│   ├── events/
│   │   ├── pending/
│   │   │   └── .gitkeep
│   │   └── decided/
│   │       └── .gitkeep
│   ├── issues/
│   │   ├── <label>/
│   │   │   └── .gitkeep
│   │   └── .gitkeep
│   └── innovators/
│       └── <persona>/
│           ├── perspective.yml
│           ├── idea.yml       # Temporary (creation phase)
│           ├── proposal.yml   # Temporary (refinement output)
│           └── comments/
│               └── .gitkeep
└── workstations/
    └── <role>/
        └── perspective.yml
```

## Document Hierarchy

| Document | Audience | Contains |
|----------|----------|----------|
| `JULES.md` | Jules agents | Formal contracts and schemas |
| `README.md` | Humans | Informal guide |

**Rule**: Jules-internal definitions stay in `.jules/`. User configuration stays in `.jlo/`. Execution/orchestration belongs in `.github/`.

## Prompt Hierarchy

See "Critical Design Principles" above for the contract structure.

| File | Scope | Content |
|------|-------|---------|
| `<layer>_prompt.j2` | Layer | Prompt template that assembles contracts, tasks, and includes. |
| `role.yml` | Role | Specialized focus (observers/innovators). |
| `contracts.yml` | Layer | Universal constraints shared within layer. |
| `tasks/<task-id>.yml` | Layer | Independent action units with local limits and output expectations. |
| `JULES.md` | Global | Rules applying to ALL layers (branch naming, system boundaries). |

## Schema Files

Schemas define the structure for artifacts produced by agents.

| Schema | Location | Purpose |
|--------|----------|---------|
| `changes.yml` | `.jules/roles/narrator/schemas/` | Changes summary structure |
| `event.yml` | `.jules/roles/observers/schemas/` | Observer event structure |
| `perspective.yml` | `.jules/roles/observers/schemas/` | Observer perspective structure |
| `issue.yml` | `.jules/roles/decider/schemas/` | Issue structure |
| `perspective.yml` | `.jules/roles/innovators/schemas/` | Innovator persona memory |
| `idea.yml` | `.jules/roles/innovators/schemas/` | Idea draft structure |
| `proposal.yml` | `.jules/roles/innovators/schemas/` | Finalized proposal structure |
| `comment.yml` | `.jules/roles/innovators/schemas/` | Observer feedback on ideas |

**Rule**: Agents copy the schema and fill its fields. Never invent structure.

## Exchange Model

Jules uses a flat exchange model for handing off events and requirements between layers. The exchange is located in `.jules/exchange/`.

### Exchange Directories

| Directory | Purpose |
|-----------|---------|
| `.jules/exchange/changes.yml` | Narrator output (bounded changes summary) |
| `.jules/exchange/events/<state>/` | Observer outputs |
| `.jules/exchange/requirements/` | Decider outputs, Planner/Implementer inputs |
| `.jules/exchange/innovators/<persona>/` | Innovator perspectives, ideas, proposals, comments |
| `.jules/workstations/<role>/` | Role perspectives (memory) |

## Data Flow

The pipeline is file-based and uses local requirements as the handoff point:

```
narrator -> observers -> decider -> [planner] -> implementer
(changes)   (events)    (requirements) (expand)  (code changes)

innovators (independent cycle)
perspective -> idea -> comments -> proposal
```

1. **Narrator** runs first, producing `.jules/exchange/changes.yml` as a secondary hint for observer triage.
2. **Observers** emit events to exchange event directories.
3. **Decider** reads events, emits requirements, and links related events via `source_events`.
4. **Planner** expands requirements with `requires_deep_analysis: true`.
5. **Implementer** executes approved tasks and creates PRs with code changes.
6. **Innovators** run independently: each persona maintains a `perspective.yml`, drafts `idea.yml`, receives `comments/` from other personas, and produces `proposal.yml`.
