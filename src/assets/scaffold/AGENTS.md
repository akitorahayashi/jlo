# Scaffold Design

## Critical Design Principles
Ref: [root AGENTS.md](../../AGENTS.md).

## Concepts
- Layer: Pipeline stage (e.g., Decider).
- Role: Agent execution identity.
  - Single-Role: Layer = one execution role (Narrator, Decider, Planner, Implementer).
  - Multi-Role: Multiple roles (Observers, Innovators).

## Directory Structure
```
.jlo/ (Config & Instances)
├── config.toml
├── roles/<layer>/role.yml
└── setup/ (tools.yml, vars.toml, secrets.toml, install.sh)

.jules/ (System Definition)
├── schemas/
│   ├── narrator/changes.yml
│   ├── observers/event.yml
│   ├── decider/requirements.yml
│   └── innovators/proposal.yml
├── exchange/
│   ├── changes.yml
│   ├── events/{pending,decided}/
│   ├── requirements/
│   └── proposals/
└── workstations/<role>/perspective.yml
```

## Prompt Assembly
Jinja2 templates (`<layer>_prompt.j2`) define the context window.
Prompt assets (contracts, tasks, templates) are embedded in the binary from `src/assets/prompt-assemble/`.
References use the `prompt-assemble://` scheme to load from the embedded catalog.
- Modular: Inject content via `include_*` functions.
- Context-Aware: Use variables (e.g., `role`) for dynamic configuration.
- DRY: Reference definitive artifacts directly.
- Direct Injection: Inject file content directly; do not instruct agents to "read file X".

## Artifacts

### Schemas (`.jules/schemas/<layer>/`)
| Schema | Layer | Purpose |
|--------|-------|---------|
| `changes.yml` | Narrator | Diff summary |
| `event.yml` | Observers | Issue findings |
| `requirements.yml` | Decider | Requirements |
| `proposal.yml` | Innovators | Improvement proposals |
| `perspective.yml` | Obs/Inn | Memory state |

### Exchange (`.jules/exchange/`)
1. Narrator: `changes.yml` (Git summary).
2. Observers: `events/pending/*.yml` (Issues).
3. Decider: `requirements/*.yml` (Triage).
4. Planner: Expands requirements.
5. Implementer: Code changes (PRs).
6. Innovators: `proposals/*.yml` (Ideas).

## Flow
`narrator` -> `observers` -> `decider` -> `planner`? -> `implementer`

`innovators` (independent)
