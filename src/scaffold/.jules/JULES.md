# .jules/ Agent Contract

This document defines the operational contract for agents working in this repository.
All scheduled agents must read this file before acting.

## 4-Layer Architecture

### Layer 1: Observers
Roles: `taxonomy`, `data_arch`, `qa`

Observers are specialized analytical lenses. They:
- Read `JULES.md` and `.jules/JULES.md`
- Read their own `.jules/roles/observers/<role>/role.yml` and `notes/`
- Update `notes/` declaratively
- Write normalized event files under `.jules/events/`

Observers do **not** write `issues/` or `tasks/`.

### Layer 2: Deciders
Roles: `triage`

Deciders screen and validate observations. They:
- Read all `.jules/events/**/*.yml` and existing `.jules/issues/*.md`
- Validate observations, merge related events
- Create actionable issues in `.jules/issues/`
- Delete processed events (accepted or rejected)
- Update observer `role.yml` when rejections indicate recurring noise

Only deciders write `issues/`.

### Layer 3: Planners
Roles: `specifier`

Planners decompose issues into tasks. They:
- Read `.jules/issues/*.md`
- Analyze impact and create concrete tasks
- Write `.jules/tasks/*.md` with verification plans
- Delete processed issues

Planners do **not** write code or `events/`.

### Layer 4: Implementers
Roles: `executor`

Implementers execute tasks. They:
- Read `.jules/tasks/*.md`
- Implement code, tests, documentation
- Run verification
- Delete processed tasks

Implementers do **not** write `events/`, `issues/`, or `notes/`.

## Event Recording (YAML)

Events are normalized observations. They are not plans.
Do not include downstream test/doc scope in events.

### Categories

Events must be written under `.jules/events/<category>/` where `<category>` is one of:

- `bugs`
- `docs`
- `refacts`
- `tests`
- `updates`

### Filename

`YYYY-MM-DD_HHMMSS_<category>_<author_role>_<id>.yml`

`<id>` is a short local identifier (8–12 lowercase hex is sufficient).

### Required Keys (schema v1)

- `schema_version: 1`
- `id: <string>` (matches filename id)
- `created_at: <YYYY-MM-DD or RFC3339>`
- `author_role: <string>`
- `category: <bugs|docs|refacts|tests|updates>`
- `title: <string>`
- `statement: <string>` (concise observation claim)
- `evidence:` (list of evidence items)
- `confidence: <low|medium|high>`

Evidence item shape:

- `path: <string>` (repo-relative path)
- `loc: <string>` (line or symbol reference)
- `note: <string>` (why this supports the statement)

Optional keys:

- `tags: [<string>, ...]`
- `related: [<event_id>, ...]`

## Issues (Markdown + Frontmatter)

Issues are flat files under `.jules/issues/`.
They are actionable tasks derived from one or more events.

### Frontmatter (schema v1)

Required keys:

- `id: <string>`
- `category: <bugs|docs|refacts|tests|updates>`
- `title: <string>`
- `priority: <low|medium|high>`
- `sources: [<event_id>, ...]`
- `status: <open|blocked|done>`

### Body Expectations

Issues must be executable:

- background and rationale
- concrete change list (files/modules when possible)
- acceptance criteria

## Tasks (Markdown + Frontmatter)

Tasks are flat files under `.jules/tasks/`.
They are executable work items derived from issues.

### Frontmatter

Required keys:

- `id: <string>`
- `parent_issue_id: <string>`
- `title: <string>`
- `status: <open|done>`

### Body Expectations

Tasks must include:

- 目的 (Purpose)
- 変更対象 (Change targets with file paths)
- 検証計画 (Verification plan with command and expected result)

## Role Feedback Updates

If an event is rejected, deciders update the originating observer's `role.yml`.
Feedback should be appended under a dedicated `feedback` section to reduce recurring noise.

## Deletion Policy

- Processed events are deleted after triage (accepted or rejected)
- Processed issues are deleted after planning
- Processed tasks are deleted after implementation
