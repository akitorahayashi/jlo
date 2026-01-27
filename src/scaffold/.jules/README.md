# .jules/

The `.jules/` directory is a structured workspace for scheduled agents and human execution.
It captures **observations as events** and **actionable work as issues**.

This file is human-oriented. Agents must also read `.jules/AGENTS.md` for the formal contract.

## Overview

This workspace implements a **worker + triage** workflow:
- **Worker agents** record issue-worthy observations as normalized events
- **Triage agent** critically reviews events, produces issues, and deletes processed events
- **Humans** execute issues

## Directory Structure

```
.jules/
├── README.md           # This file (jo-managed)
├── AGENTS.md           # Agent contract (jo-managed)
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
```

## Workflow

### 1. Worker Agents (Scheduled)

Each worker agent:
1. Reads `AGENTS.md` and `.jules/AGENTS.md`
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

## Agent Roles

| Role | Type | Responsibility |
|------|------|----------------|
| taxonomy | Worker | Naming conventions, terminology consistency |
| data_arch | Worker | Data models, data flow efficiency |
| qa | Worker | Test coverage, test quality |
| triage | Manager | Event screening, issue creation, role feedback |

## Managed Files

`jo update` manages only:
- `.jules/README.md`
- `.jules/AGENTS.md`
- `.jules/.jo-version`
- `.jules/roles/*/prompt.yml`

All other files are agent-owned and never modified by jo.
