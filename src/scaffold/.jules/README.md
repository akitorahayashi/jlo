# .jules/

The `.jules/` directory is a structured workspace for scheduled agents and human execution.
It captures **observations as events** and **actionable work as issues**.

This file is human-oriented. Agents must also read `.jules/JULES.md` for the formal contract.

## Overview

This workspace implements a **worker + triage** workflow:
- **Worker agents** record issue-worthy observations as normalized events
- **Triage agent** critically reviews events, produces issues, and deletes processed events
- **Humans** execute issues

## Directory Structure

```
.jules/
├── README.md           # This file (jo-managed)
├── JULES.md            # Agent contract (jo-managed)
├── .jo-version         # Version marker (jo-managed)
│
├── roles/              # [Agent Layer] Deployed roles
│   ├── taxonomy/
│   │   ├── prompt.yml  # Scheduler prompt template (jo-managed)
│   │   ├── role.yml    # Role definition (agent-owned)
│   │   └── notes/      # Declarative memory (agent-owned)
│   ├── data_arch/
│   │   ├── prompt.yml
│   │   ├── role.yml
│   │   └── notes/
│   ├── qa/
│   │   ├── prompt.yml
│   │   ├── role.yml
│   │   └── notes/
│   └── triage/
│       ├── prompt.yml
│       └── role.yml
│
├── events/             # [Inbox] Normalized observations (YAML)
│   ├── bugs/
│   ├── docs/
│   ├── refacts/
│   ├── tests/
│   └── updates/
│
└── issues/             # [Outbox] Actionable tasks (Markdown, flat)
    └── *.md

└── tasks/              # [Transit] Executable tasks (Markdown, flat)
    └── *.md
```

## Workflow

### 1. Worker Agents (Scheduled)

Each worker agent:
1. Reads `JULES.md` and `.jules/JULES.md`
2. Updates `notes/` with current understanding (declarative state)
3. Writes normalized `events/**/*.yml` when observations are issue-worthy

Workers do **not** write `issues/`.

### 2. Triage Agent (Scheduled)

The triage agent:
1. Reads all `events/**/*.yml` and existing `issues/*.md`
2. Critically validates and merges related observations
3. Creates actionable issues (Markdown with YAML frontmatter)
4. Deletes processed events (accepted or rejected)
5. Updates worker `role.yml` to reduce recurring noise

Only triage writes `issues/`.

### 3. Human Execution

Humans:
1. Review issues in `issues/`
2. Select issues to implement
3. Execute or delegate to coding agents
4. Close issues when complete

### 4. Specifier/Executor (On-Demand)

This pipeline automates execution:
1. **Specifier** converts an issue into granular `tasks/`.
2. **Executor** implements `tasks/` and runs verification.

## Agent Roles

| Role | Type | Responsibility |
|------|------|----------------|
| taxonomy | Worker | Naming conventions, terminology consistency |
| data_arch | Worker | Data models, data flow efficiency |
| qa | Worker | Test coverage, test quality |
| triage | Manager | Event screening, issue creation, role feedback |
| specifier | Architect | Issue analysis, task decomposition |
| executor | Engineer | Implementation, verification, cleanup |

## Managed Files

`jo update` manages only:
- `.jules/README.md`
- `.jules/JULES.md`
- `.jules/.jo-version`
- `.jules/roles/*/prompt.yml`

All other files are agent-owned and never modified by jo.
