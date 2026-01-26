# .jules/

The `.jules/` directory maintains organizational memory for this repository, enabling a PM/Worker agent workflow.

## Overview

This workspace implements a **distributed agent organization** where:
- **Worker Agents** analyze code from specialized perspectives and maintain persistent notes
- **PM Agent** reviews proposals and creates actionable issues
- **Humans** execute approved issues

## Directory Structure

```
.jules/
├── README.md           # This file (jo-managed)
├── .jo-version         # Version marker (jo-managed)
│
├── roles/              # [Worker Layer] Agent workspaces
│   ├── taxonomy/       # Naming consistency specialist
│   │   ├── role.yml    # Role definition and behavior
│   │   └── notes/      # Persistent declarative memory
│   ├── data_arch/      # Data model specialist
│   │   ├── role.yml
│   │   └── notes/
│   ├── qa/             # Quality assurance specialist
│   │   ├── role.yml
│   │   └── notes/
│   └── pm/             # Project Manager (gatekeeper)
│       ├── role.yml
│       └── policy.md   # Decision criteria
│
├── reports/            # [Inbox] Proposals from Workers
│   └── YYYY-MM-DD_<role>_<title>.md
│
└── issues/             # [Outbox] Approved actionable tasks
    ├── bugs/           # Bug fixes (+tests, +docs)
    ├── refacts/        # Refactoring (+tests, +docs)
    ├── updates/        # New features (+tests, +docs)
    ├── tests/          # Test-only changes
    └── docs/           # Documentation-only changes
```

## Workflow

### 1. Worker Agents (Scheduled)

Each worker agent:
1. Reads source code and their `notes/` directory
2. Updates `notes/` with current understanding (declarative state)
3. Creates proposals in `reports/` when improvements are identified

**Workers do NOT write to `issues/`.**

### 2. PM Agent (Scheduled)

The PM agent:
1. Reads all proposals from `reports/`
2. Reviews against `policy.md` criteria
3. Screens out inappropriate proposals
4. Converts approved proposals to `issues/<category>/*.md`

**Only the PM writes to `issues/`.**

### 3. Human Execution

Humans:
1. Review issues in `issues/`
2. Select issues to implement
3. Execute or delegate to coding agents
4. Archive completed issues

## Agent Roles

| Role | Type | Responsibility |
|------|------|----------------|
| taxonomy | Worker | Naming conventions, terminology consistency |
| data_arch | Worker | Data models, data flow efficiency |
| qa | Worker | Test coverage, test quality |
| pm | Manager | Proposal review, issue creation |

## Memory Model

Workers maintain **declarative memory** in `notes/`:
- Describe "what is" not "what was done"
- Update state on each run
- Enable context continuity across executions

## Issue Categories

| Category | Description |
|----------|-------------|
| bugs | Bug fixes (includes related tests/docs) |
| refacts | Refactoring without feature changes |
| updates | New features or library updates |
| tests | Test-only changes |
| docs | Documentation-only changes |

## Managed Files

`jo update` manages only:
- `.jules/README.md`
- `.jules/.jo-version`

All other files are user-owned and never modified by jo.
