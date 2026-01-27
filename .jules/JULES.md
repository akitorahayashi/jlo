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

- `contracts.yml` exists at layer level (e.g., `roles/observers/contracts.yml`)
- `prompt.yml` references the layer's contracts.yml for behavioral instructions
- `role.yml` exists only for observers and evolves through feedback loop

## 4-Layer Flow

```
Observer → Decider → Planner → Implementer
(events)   (issues)   (tasks)   (code)
```

Each layer has a **distinct transformation responsibility**:

| Layer | Input | Output | Key Distinction |
|-------|-------|--------|-----------------|
| Observer | source code | events | Domain-specialized observations |
| Decider | events | issues | **Validation and consolidation** (Is this real? Should events merge?) |
| Planner | issues | tasks | **Decomposition** (What steps are needed to solve this?) |
| Implementer | tasks | code | Execution |

**Detailed layer behaviors and schemas are defined in each layer's contracts.yml file.**

## Workspace Structure

```
.jules/
├── roles/
│   ├── observers/
│   │   ├── contracts.yml    # Shared observer contract
│   │   └── <role>/
│   ├── deciders/
│   │   ├── contracts.yml    # Shared decider contract
│   │   └── <role>/
│   ├── planners/
│   │   ├── contracts.yml    # Shared planner contract
│   │   └── <role>/
│   └── implementers/
│       ├── contracts.yml    # Shared implementer contract
│       └── <role>/
└── exchange/
    ├── events/    # Inbox: raw observations from observers
    ├── issues/    # Transit: consolidated problems from deciders
    └── tasks/     # Outbox: executable work from planners
```

All files in `exchange/` are transient and deleted after processing.

## Feedback Loop

- Observer creates events in `exchange/events/`
- Decider reviews events, writes feedback to observer's `feedbacks/` directory if rejecting recurring patterns
- Observer reads feedback at next execution, updates role.yml to reduce noise

## Deletion Policy

- Processed events: deleted after triage (accepted or rejected)
- Processed issues: deleted after planning
- Processed tasks: deleted after implementation
- Feedback files: **never deleted** (preserved for audit)

