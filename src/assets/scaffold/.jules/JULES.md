# .jules/ Agent Contract

This document defines the operational contract for agents working in this repository.
All scheduled agents must read this file before acting.

## Document Hierarchy

| Document | Audience | Purpose |
|----------|----------|---------|
| `AGENTS.md` | All LLM tools (Codex, Claude, etc.) | Repository-wide conventions |
| `.jules/JULES.md` | Jules agents only | Jules-specific workflow contract |
| `.jules/README.md` | Humans | Human-readable workflow guide |

**JULES.md is internal to Jules**. Do not expose Jules-specific workflows in AGENTS.md.

## File Semantics

### prompt.yml vs role.yml vs contracts.yml

| File | Lifecycle | Purpose |
|------|-----------|---------|
| `contracts.yml` | **Static** (layer-level) | Shared constraints and schemas for all roles in a layer |
| `prompt.yml` | **Static** (scheduled) | Execution parameters and references to contracts.yml |
| `role.yml` | **Dynamic** (evolves) | Specialized focus that updates based on feedback (observers only) |
| `*.yml` (templates) | **Static** (layer-level) | Copyable templates for artifacts (event.yml, issue.yml, feedback.yml, task.yml) |

- `contracts.yml` exists at layer level (e.g., `roles/observers/contracts.yml`)
- `prompt.yml` references the layer's contracts.yml for behavioral instructions
- `role.yml` exists only for observers and evolves through feedback loop

## Role Flow

```
Observer -> Decider -> Planner -> (GitHub Issue)
(events)    (issues)   (tasks)    (implementation)
```

Parallel observer branches are consolidated by Merger roles.

Each role type has a **distinct transformation responsibility**:

| Role Type | Input | Output | Key Distinction |
|-----------|-------|--------|-----------------|
| Observer | source code | events | Domain-specialized observations |
| Decider | events | issues | **Validation and consolidation** (Is this real? Should events merge?) |
| Planner | issues | tasks | **Decomposition** (What steps are needed to solve this?) |
| Merger | observer branches | consolidated branch | **Branch consolidation** (Merge parallel observer work) |

**Implementation is invoked via GitHub Issues with `jules` label.**

**Detailed role behaviors and schemas are defined in each layer's contracts.yml file.**

## Branch Naming Convention

All agents must create branches using this format:

```
jules/<layer>-<role>-<YYYYMMDD>-<HHMM>-<short_id>
```

Examples:
- `jules/observer-taxonomy-20260128-1345-a1b2`
- `jules/decider-triage-20260128-1400-c3d4`
- `jules/merger-consolidator-20260128-1415-e5f6`

This convention enables:
- Automated pruning via `jlo prune`
- Age-based filtering
- Role identification

## window_hours Behavior

Deciders and Planners use `window_hours` parameter to filter input files.

- **Default**: 24 hours
- **Behavior**: Files older than `window_hours` from execution time are ignored
- **Filename format**: Files must contain timestamp (e.g., `YYYY-MM-DD_HHMMSS_*.yml`)

This prevents re-processing of old events/issues without requiring cursor files.

## Workspace Structure

```
.jules/
+-- roles/
|   +-- observers/
|   |   +-- contracts.yml    # Shared observer contract
|   |   +-- event.yml        # Event template
|   |   +-- <role>/
|   +-- deciders/
|   |   +-- contracts.yml    # Shared decider contract
|   |   +-- issue.yml        # Issue template
|   |   +-- feedback.yml     # Feedback template
|   |   +-- <role>/
|   +-- planners/
|   |   +-- contracts.yml    # Shared planner contract
|   |   +-- task.yml         # Task template
|   |   +-- <role>/
|   +-- mergers/
|       +-- contracts.yml    # Shared merger contract
|       +-- <role>/
+-- exchange/
    +-- events/    # Inbox: raw observations from observers
    +-- issues/    # Transit: consolidated problems from deciders
    +-- tasks/     # Outbox: executable work from planners
```

All files in `exchange/` are transient and deleted after processing.

## Feedback Loop

- Observer creates events in `exchange/events/`
- Decider reviews events, writes feedback to observer's `feedbacks/` directory if rejecting recurring patterns
- Observer reads feedback at next execution, updates role.yml to reduce noise

## Deletion Policy

- Processed events: deleted after triage (accepted or rejected)
- Processed issues: deleted after planning
- Processed tasks: deleted after implementation via GitHub Issue
- Feedback files: **never deleted** (preserved for audit)
