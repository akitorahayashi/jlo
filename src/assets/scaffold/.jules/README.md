# .jules/

The `.jules/` directory is a structured workspace for scheduled agents and human execution.
It captures **observations as events** and **actionable work as issues/tasks**.

This file is human-oriented. Agents must also read `.jules/JULES.md` for the formal contract.

## Overview

This workspace implements a **4-layer architecture**:
- **Observers**: Read source, update notes, emit events (taxonomy, data_arch, qa)
- **Deciders**: Screen events, emit issues, provide feedback (triage)
- **Planners**: Read issues, emit tasks, delete issues (specifier)
- **Implementers**: Read tasks, write code, delete tasks (executor)

**Configuration Language**: All YAML configuration files (role.yml, prompt.yml, archetypes) are written in English for optimal LLM processing.

## Directory Structure

```
.jules/
├── README.md           # This file (jo-managed)
├── JULES.md            # Agent contract (jo-managed)
├── .jo-version         # Version marker (jo-managed)
│
├── roles/              # [Agent Layer] 4-tier role organization
│   ├── observers/      # Observation layer (stateful)
│   │   ├── taxonomy/   # Naming consistency specialist
│   │   │   ├── prompt.yml  # Execution parameters only
│   │   │   ├── role.yml    # Specialized focus
│   │   │   ├── notes/      # Declarative state
│   │   │   └── feedbacks/  # Decider rejection feedback
│   │   ├── data_arch/  # Data model specialist
│   │   │   ├── prompt.yml
│   │   │   ├── role.yml
│   │   │   ├── notes/
│   │   │   └── feedbacks/
│   │   ├── consistency/ # Documentation & implementation alignment
│   │   │   ├── prompt.yml
│   │   │   ├── role.yml
│   │   │   ├── notes/
│   │   │   └── feedbacks/
│   │   └── qa/         # Quality assurance specialist
│   │       ├── prompt.yml
│   │       ├── role.yml
│   │       ├── notes/
│   │       └── feedbacks/
│   │
│   ├── deciders/       # Decision layer (stateless)
│   │   └── triage/     # Event screening, issue creation
│   │       └── prompt.yml  # No role.yml (behavior in archetype)
│   │
│   ├── planners/       # Planning layer (stateless)
│   │   └── specifier/  # Issue decomposition into tasks
│   │       └── prompt.yml  # No role.yml (behavior in archetype)
│   │
│   └── implementers/   # Implementation layer (stateless)
│       └── executor/   # Code implementation
│           └── prompt.yml  # No role.yml (behavior in archetype)
│
├── archetypes/         # [Templates] Layer archetypes and policies
│   ├── layers/
│   │   ├── observer.yml      # Complete observer behavior
│   │   ├── decider.yml       # Complete decider behavior
│   │   ├── planner.yml       # Complete planner behavior
│   │   └── implementer.yml   # Complete implementer behavior
│   └── policy.yml
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
1. Reads `JULES.md`, `.jules/JULES.md`, and `.jules/archetypes/layers/observer.yml`
2. Reads all feedback files in `feedbacks/`, abstracts patterns, updates `role.yml`
3. Reads own `role.yml` for specialized focus
4. Reads `notes/` for current understanding
5. Updates `notes/` declaratively (describe "what is", not "what was done")
6. Writes normalized `events/**/*.yml` when observations warrant issues

**Stateful**: Observers maintain persistent `notes/` and receive feedback via `feedbacks/`.

Observers do **not** write `issues/` or `tasks/`.

### 2. Decider Agent (Scheduled)

The triage agent:
1. Reads `JULES.md`, `.jules/JULES.md`, and `.jules/archetypes/layers/decider.yml`
2. Reads all `events/**/*.yml` and existing `issues/*.md`
3. Critically validates observations (checks if they actually exist in codebase)
4. Merges related observations that share root cause
5. Creates actionable issues (Markdown with YAML frontmatter)
6. **Writes feedback**: When rejecting recurring patterns, creates `feedbacks/<date>_<description>.yml` in observer's directory
7. Deletes processed events (both accepted and rejected)

**Stateless**: Decider behavior is entirely defined in `.jules/archetypes/layers/decider.yml`.

Only deciders write `issues/` and `feedbacks/`.

### 3. Planner Agent (On-Demand)

The specifier agent:
1. Reads `JULES.md`, `.jules/JULES.md`, and `.jules/archetypes/layers/planner.yml`
2. Reads target issue specified in `prompt.yml`
3. Analyzes impact comprehensively (code, tests, documentation)
4. Decomposes into concrete, executable tasks
5. Creates `tasks/*.md` with verification plans
6. Deletes the processed issue

**Stateless**: Planner behavior is entirely defined in `.jules/archetypes/layers/planner.yml`.

### 4. Implementer Agent (On-Demand)

The executor agent:
1. Reads `JULES.md`, `.jules/JULES.md`, and `.jules/archetypes/layers/implementer.yml`
2. Reads target task specified in `prompt.yml`
3. Implements code changes following project conventions
4. Runs verification (or reliable alternative if environment constraints exist)
5. Deletes the processed task

**Stateless**: Implementer behavior is entirely defined in `.jules/archetypes/layers/implementer.yml`.

## Configuration Hierarchy

The configuration follows a **single source of truth** hierarchy:

```
JULES.md (contract, schemas)
  └── archetypes/layers/*.yml (layer default behavior)
       └── roles/observers/*/role.yml (specialized focus, only for observers)
            └── prompt.yml (execution-time parameters only)
```

- **JULES.md**: Defines contracts, schemas, and workflows
- **Archetypes**: Define complete behavior for each layer (observer, decider, planner, implementer)
- **role.yml**: Only exists for observers (stateful roles); defines specialized analytical focus
- **prompt.yml**: The scheduled entry point. It directs the agent to read `role.yml` and other resources; it does not contain role logic itself.

## Feedback Loop

The feedback mechanism enables continuous improvement:

1. **Observer** creates events based on observations
2. **Decider** reviews events and may reject some due to recurring patterns
3. **Decider** writes feedback files to `.jules/roles/observers/<role>/feedbacks/`
4. **Observer** reads feedback files on next execution, abstracts patterns
5. **Observer** updates its own `role.yml` to refine focus and prevent noise

Feedback files are preserved for audit (never deleted). This self-improvement loop reduces false positives over time.

## Agent Roles by Layer

| Layer | Role | Responsibility | Statefulness |
|-------|------|----------------|--------------|
| Observers | taxonomy | Naming conventions, terminology consistency | Stateful (notes, feedbacks) |
| Observers | data_arch | Data models, data flow efficiency | Stateful (notes, feedbacks) |
| Observers | qa | Test coverage, test quality | Stateful (notes, feedbacks) |
| Observers | consistency | Implementation vs Doc alignment | Stateful (notes, feedbacks) |
| Deciders | triage | Event screening, issue creation, feedback writing | Stateless |
| Planners | specifier | Issue analysis, task decomposition | Stateless |
| Implementers | executor | Code implementation, verification | Stateless |

## CLI Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `jo init` | `i` | Create `.jules/` with 4-layer architecture |
| `jo assign <role> [paths...]` | `a` | Generate prompt and copy to clipboard |
| `jo template [-l layer] [-n name]` | `tp` | Create a new role from layer template |
