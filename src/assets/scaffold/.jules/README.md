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

## Role Flow

```
Observer -> Decider -> Planner -> (GitHub Issue)
(events)    (issues)   (tasks)    (implementation)
```

Parallel observer branches are consolidated by Merger roles.

| Role Type | Role(s) | Transformation |
|-----------|---------|----------------|
| Observer | taxonomy, data_arch, qa, consistency | Source -> Events (domain-specialized observations) |
| Decider | triage | Events -> Issues (validation + consolidation) |
| Planner | specifier | Issues -> Tasks (decomposition into steps) |
| Merger | consolidator | Branches -> Unified branch (parallel work consolidation) |

**Implementation**: Invoked via GitHub Issues with `jules` label.

**Configuration Language**: All YAML files are written in English for optimal LLM processing.

## Directory Structure

```
.jules/
+-- README.md           # This file (jlo-managed)
+-- JULES.md            # Agent contract (jlo-managed)
+-- .jlo-version        # Version marker (jlo-managed)
|
+-- roles/              # Role definitions
|   +-- observers/
|   |   +-- contracts.yml    # Shared observer contract
|   |   +-- event.yml        # Event template
|   |   +-- taxonomy/
|   |   |   +-- prompt.yml   # Static: execution parameters
|   |   |   +-- role.yml     # Dynamic: evolving focus
|   |   |   +-- notes/       # Declarative state
|   |   |   +-- feedbacks/   # Decider feedback
|   |   +-- data_arch/
|   |   +-- consistency/
|   |   +-- qa/
|   |
|   +-- deciders/
|   |   +-- contracts.yml    # Shared decider contract
|   |   +-- issue.yml        # Issue template
|   |   +-- feedback.yml     # Feedback template
|   |   +-- triage/
|   |       +-- prompt.yml
|   |
|   +-- planners/
|   |   +-- contracts.yml    # Shared planner contract
|   |   +-- task.yml         # Task template
|   |   +-- specifier/
|   |       +-- prompt.yml
|   |
|   +-- mergers/
|       +-- contracts.yml    # Shared merger contract
|       +-- consolidator/
|           +-- prompt.yml
|
+-- exchange/           # Transient data flow
    +-- events/         # [Inbox] Raw observations
    |   +-- bugs/
    |   +-- docs/
    |   +-- refacts/
    |   +-- tests/
    |   +-- updates/
    |   +-- issues/         # [Transit] Consolidated problems
    |   +-- *.yml
    |
    +-- tasks/          # [Outbox] Executable tasks
        +-- *.yml
```

## Configuration Files

### contracts.yml
Layer-level shared constraints and workflows. All roles in the layer reference this file.

### prompt.yml
Execution parameters and references to contracts.yml. Static, scheduled with agent.
Includes `window_hours` parameter for Deciders, Planners, and Mergers.

### role.yml
Specialized focus that evolves through feedback loop. Only observers have this (stateful layer).

### Templates (*.yml)
Copyable templates (event.yml, issue.yml, feedback.yml) defining the structure of artifacts.
Agents `cp` these files and fill them out.

## Workflow

### 1. Observer Agents (Scheduled)

Each observer:
1. Reads contracts.yml (layer behavior)
2. Reads role.yml (specialized focus)
3. Reads feedbacks/, abstracts patterns, updates role.yml
4. Reads notes/ for current state
5. Updates notes/ declaratively
6. Writes exchange/events/**/*.yml when observations warrant
7. Creates branch: `jules/observer-<role>-<timestamp>-<id>`

**Stateful**: Maintains `notes/` and receives feedback via `feedbacks/`.

### 2. Decider Agent (Scheduled)

Triage agent:
1. Reads contracts.yml (layer behavior)
2. Reads all exchange/events/**/*.yml within window_hours
3. Validates observations (do they exist in codebase?)
4. Merges related events sharing root cause
5. Creates consolidated issues in exchange/issues/
6. Writes feedback for recurring rejections
7. Deletes processed events

**Decider answers**: "Is this real? Should these events merge into one issue?"

### 3. Planner Agent (On-Demand)

Specifier agent:
1. Reads contracts.yml (layer behavior)
2. Reads target issue from exchange/issues/ within window_hours
3. Analyzes impact
4. Decomposes into executable tasks in exchange/tasks/
5. Deletes processed issue

**Planner answers**: "What steps solve this issue?"

### 4. Merger Agent (Scheduled)

Consolidator agent:
1. Reads contracts.yml (layer behavior)
2. Lists branches matching jules/observer-* within window_hours
3. Analyzes changes from each branch
4. Resolves conflicts between parallel observers
5. Creates consolidated branch: `jules/merger-<role>-<timestamp>-<id>`

**Merger answers**: "How do parallel observer changes combine?"

### 5. Implementation (Via GitHub Issue)

Implementation is invoked by creating a GitHub Issue with `jules` label.
The issue contains tasks from exchange/tasks/.

## Feedback Loop

```
Observer creates events in exchange/events/
       |
       v
Decider validates, may reject
       |
       v (rejection)
Decider writes feedbacks/{date}_{desc}.yml
       |
       v
Observer reads feedbacks/, updates role.yml
       |
       v
Observer avoids similar observations
```

Feedback files are preserved for audit (never deleted).

## Branch Naming Convention

All agents must create branches using this format:

```
jules/<layer>-<role>-<YYYYMMDD>-<HHMM>-<short_id>
```

Examples:
- `jules/observer-taxonomy-20260128-1345-a1b2`
- `jules/decider-triage-20260128-1400-c3d4`
- `jules/merger-consolidator-20260128-1415-e5f6`

Old branches can be cleaned up with `jlo prune -d <days>`.
