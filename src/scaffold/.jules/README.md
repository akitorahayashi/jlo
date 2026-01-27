# .jules/

The `.jules/` directory is a structured workspace for scheduled agents and human execution.
It captures **observations as events** and **actionable work as issues/tasks**.

This file is human-oriented. Agents must also read `.jules/JULES.md` for the formal contract.

## Overview

This workspace implements a **4-layer architecture**:
- **Observers**: Read source, update notes, emit events (taxonomy, data_arch, qa)
- **Deciders**: Screen events, emit issues, delete events (triage)
- **Planners**: Read issues, emit tasks, delete issues (specifier)
- **Implementers**: Read tasks, write code, delete tasks (executor)

## Directory Structure

```
.jules/
├── README.md           # This file (jo-managed)
├── JULES.md            # Agent contract (jo-managed)
├── .jo-version         # Version marker (jo-managed)
│
├── roles/              # [Agent Layer] 4-tier role organization
│   ├── observers/      # Observation layer
│   │   ├── taxonomy/   # Naming consistency specialist
│   │   │   ├── prompt.yml
│   │   │   ├── role.yml
│   │   │   └── notes/
│   │   ├── data_arch/  # Data model specialist
│   │   │   ├── prompt.yml
│   │   │   ├── role.yml
│   │   │   └── notes/
│   │   └── qa/         # Quality assurance specialist
│   │       ├── prompt.yml
│   │       ├── role.yml
│   │       └── notes/
│   │
│   ├── deciders/       # Decision layer
│   │   └── triage/     # Event screening, issue creation
│   │       ├── prompt.yml
│   │       └── role.yml
│   │
│   ├── planners/       # Planning layer
│   │   └── specifier/  # Issue decomposition into tasks
│   │       ├── prompt.yml
│   │       └── role.yml
│   │
│   └── implementers/   # Implementation layer
│       └── executor/   # Code implementation
│           ├── prompt.yml
│           └── role.yml
│
├── events/             # [Inbox] Normalized observations (YAML)
│   ├── bugs/
│   ├── docs/
│   ├── refacts/
│   ├── tests/
│   └── updates/
│
├── issues/             # [Transit] Actionable tasks (Markdown, flat)
│   └── *.md
│
└── tasks/              # [Outbox] Executable tasks (Markdown, flat)
    └── *.md
```

## Workflow

### 1. Observer Agents (Scheduled)

Each observer agent:
1. Reads `JULES.md` and `.jules/JULES.md`
2. Updates `notes/` with current understanding (declarative state)
3. Writes normalized `events/**/*.yml` when observations are issue-worthy

Observers do **not** write `issues/` or `tasks/`.

### 2. Decider Agent (Scheduled)

The triage agent:
1. Reads all `events/**/*.yml` and existing `issues/*.md`
2. Critically validates and merges related observations
3. Creates actionable issues (Markdown with YAML frontmatter)
4. Deletes processed events (accepted or rejected)
5. Updates observer `role.yml` to reduce recurring noise

Only deciders write `issues/`.

### 3. Planner Agent (On-Demand)

The specifier agent:
1. Reads an issue from `issues/`
2. Analyzes impact and decomposes into tasks
3. Creates `tasks/*.md` with verification plans
4. Deletes the processed issue

### 4. Implementer Agent (On-Demand)

The executor agent:
1. Reads a task from `tasks/`
2. Implements code changes
3. Runs verification
4. Deletes the processed task

## Agent Roles by Layer

| Layer | Role | Responsibility |
|-------|------|----------------|
| Observers | taxonomy | Naming conventions, terminology consistency |
| Observers | data_arch | Data models, data flow efficiency |
| Observers | qa | Test coverage, test quality |
| Deciders | triage | Event screening, issue creation |
| Planners | specifier | Issue analysis, task decomposition |
| Implementers | executor | Code implementation, verification |

## CLI Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jo init` | `i` | Create `.jules/` with 4-layer architecture |
| `jo assign <role> [paths...]` | `a` | Generate prompt and copy to clipboard |
| `jo template [-l layer] [-n name]` | `tp` | Create a new role from layer template |
