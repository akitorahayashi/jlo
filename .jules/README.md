# .jules/

The `.jules/` directory is a structured workspace for scheduled agents and human execution.
It captures **observations as events** and **actionable work as issues/tasks**.

This file is human-oriented. Agents must read `.jules/JULES.md` for the formal contract.

## Document Hierarchy

| Document | Audience | Contains |
|----------|----------|----------|
| `AGENTS.md` | All LLM tools | Repository conventions (exposed to Codex, Claude, etc.) |
| `.jules/JULES.md` | Jules agents | Jules-specific contracts and schemas |
| `.jules/README.md` | Humans | This guide |

**Rule**: Jules-internal details stay in `.jules/`. `AGENTS.md` remains tool-agnostic.

## 4-Layer Architecture

```
Observer → Decider → Planner → Implementer
(events)   (issues)   (tasks)   (code)
```

| Layer | Role(s) | Transformation |
|-------|---------|----------------|
| Observer | taxonomy, data_arch, qa, consistency | Source → Events (domain-specialized observations) |
| Decider | triage | Events → Issues (validation + consolidation) |
| Planner | specifier | Issues → Tasks (decomposition into steps) |
| Implementer | executor | Tasks → Code (execution) |

**Configuration Language**: All YAML files are written in English for optimal LLM processing.

## Directory Structure

```
.jules/
├── README.md           # This file (jo-managed)
├── JULES.md            # Agent contract (jo-managed)
├── .jo-version         # Version marker (jo-managed)
│
├── roles/              # Role definitions
│   ├── observers/
│   │   ├── contracts.yml    # Shared observer contract
│   │   ├── taxonomy/
│   │   │   ├── prompt.yml  # Static: execution parameters
│   │   │   ├── role.yml    # Dynamic: evolving focus
│   │   │   ├── notes/      # Declarative state
│   │   │   └── feedbacks/  # Decider feedback
│   │   ├── data_arch/
│   │   ├── consistency/
│   │   └── qa/
│   │
│   ├── deciders/
│   │   ├── contracts.yml    # Shared decider contract
│   │   └── triage/
│   │       └── prompt.yml
│   │
│   ├── planners/
│   │   ├── contracts.yml    # Shared planner contract
│   │   └── specifier/
│   │       └── prompt.yml
│   │
│   └── implementers/
│       ├── contracts.yml    # Shared implementer contract
│       └── executor/
│           └── prompt.yml
│
└── exchange/           # Transient data flow
    ├── events/         # [Inbox] Raw observations
    │   ├── bugs/
    │   ├── docs/
    │   ├── refacts/
    │   ├── tests/
    │   └── updates/
    │
    ├── issues/         # [Transit] Consolidated problems
    │   └── *.md
    │
    └── tasks/          # [Outbox] Executable tasks
        └── *.md
```

## Configuration Files

### contracts.yml
Layer-level shared constraints and workflows. All roles in the layer reference this file.

### prompt.yml
Execution parameters and references to contracts.yml. Static, scheduled with agent.

### role.yml
Specialized focus that evolves through feedback loop. Only observers have this (stateful layer).

## Workflow

### 1. Observer Agents (Scheduled)

Each observer:
1. Reads contracts.yml (layer behavior)
2. Reads role.yml (specialized focus)
3. Reads feedbacks/, abstracts patterns, updates role.yml
4. Reads notes/ for current state
5. Updates notes/ declaratively
6. Writes exchange/events/**/*.yml when observations warrant

**Stateful**: Maintains `notes/` and receives feedback via `feedbacks/`.

### 2. Decider Agent (Scheduled)

Triage agent:
1. Reads contracts.yml (layer behavior)
2. Reads all exchange/events/**/*.yml and existing exchange/issues/*.md
3. Validates observations (do they exist in codebase?)
4. Merges related events sharing root cause
5. Creates consolidated issues in exchange/issues/
6. Writes feedback for recurring rejections
7. Deletes processed events

**Decider answers**: "Is this real? Should these events merge into one issue?"

### 3. Planner Agent (On-Demand)

Specifier agent:
1. Reads contracts.yml (layer behavior)
2. Reads target issue from exchange/issues/
3. Analyzes impact
4. Decomposes into executable tasks in exchange/tasks/
5. Deletes processed issue

**Planner answers**: "What steps solve this issue?"

### 4. Implementer Agent (On-Demand)

Executor agent:
1. Reads contracts.yml (layer behavior)
2. Reads target task from exchange/tasks/
3. Implements code changes
4. Runs verification
5. Deletes processed task

## Feedback Loop

```
Observer creates events in exchange/events/
       ↓
Decider validates, may reject
       ↓ (rejection)
Decider writes feedbacks/{date}_{desc}.yml
       ↓
Observer reads feedbacks/, updates role.yml
       ↓
Observer avoids similar observations
```

Feedback files are preserved for audit (never deleted).
