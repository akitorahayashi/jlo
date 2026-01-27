# .jules/ Agent Contract

This document defines the operational contract for agents working in this repository.
All scheduled agents must read this file before acting.

## 4-Layer Architecture

### Layer 1: Observers
Roles: `taxonomy`, `data_arch`, `qa`

Observers are specialized analytical lenses. They:
- Read `JULES.md` and `.jules/JULES.md` (complete contract and behavioral rules)
- Read their own `.jules/roles/observers/<role>/role.yml` for specialized focus
- Read `notes/` and `feedbacks/` directories
- **Initialization**: Read all feedback files in `feedbacks/`, abstract patterns, and update `role.yml` declaratively to reduce recurring noise
- Update `notes/` declaratively (describe "what is", not "what was done")
- Write normalized event files under `.jules/events/<category>/` when observations warrant issues

Observers do **not** write `issues/` or `tasks/`.

### Layer 2: Deciders
Roles: `triage`

Deciders screen and validate observations. They:
- Read `JULES.md` and `.jules/JULES.md` (complete contract and behavioral rules)
- Read all `.jules/events/**/*.yml` and existing `.jules/issues/*.md`
- Validate observations critically (check if they actually exist in the codebase)
- Merge related events that share root cause or converge to same task
- Create actionable issues in `.jules/issues/`
- Delete processed events (both accepted and rejected)
- **Feedback Writing**: When rejecting observations due to recurring patterns, create feedback files in `.jules/roles/observers/<role>/feedbacks/`

Feedback file format:
- Filename: `YYYY-MM-DD_<brief_description>.yml`
- Content:
  ```yaml
  pattern: <characteristic of repeatedly rejected observations>
  reason: <why this should not be raised>
  created_at: <date>
  ```

Only deciders write `issues/` and `feedbacks/`.

### Layer 3: Planners
Roles: `specifier`

Planners decompose issues into tasks. They:
- Read `JULES.md` and `.jules/JULES.md` (complete contract and behavioral rules)
- Read target issue from `.jules/issues/<issue>.md` (path specified in `prompt.yml`)
- Analyze impact comprehensively (code, tests, documentation)
- Write concrete, executable tasks to `.jules/tasks/*.md` with verification plans
- Delete processed issue after task creation
- **Single-issue processing**: Handle one issue per execution to avoid context pollution

Planners do **not** write code, `events/`, or `notes/`.

### Layer 4: Implementers
Roles: `executor`

Implementers execute tasks. They:
- Read `JULES.md` and `.jules/JULES.md` (complete contract and behavioral rules)
- Read target task from `.jules/tasks/<task>.md` (path specified in `prompt.yml`)
- Implement code, tests, and documentation following project conventions
- Run verification plan specified in task (or reliable alternative if environment constraints exist)
- Delete completed task after successful verification
- **Single-task processing**: Handle one task per execution to avoid context pollution

Implementers do **not** write `events/`, `issues/`, or `notes/`. Work output is code changes only, not report files.

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

- Background and rationale
- Concrete change list (files/modules when possible)
- Acceptance criteria

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

- **Purpose**: Why this task exists
- **Change Targets**: Specific files/modules to modify with paths
- **Verification Plan**: Command to run and expected result

## Feedback Loop

The feedback mechanism enables continuous improvement:

1. **Observer** creates events based on observations
2. **Decider** reviews events and may reject some due to recurring patterns
3. **Decider** writes feedback files to `.jules/roles/observers/<role>/feedbacks/`
4. **Observer** reads feedback files on next execution, abstracts patterns
5. **Observer** updates its own `role.yml` to refine focus and prevent noise
6. Feedback files are preserved for audit (not deleted)

This self-improvement loop reduces recurring false positives over time.

## Deletion Policy

- Processed events are deleted after triage (accepted or rejected)
- Processed issues are deleted after planning
- Processed tasks are deleted after implementation
- Feedback files are **never** deleted (preserved for audit)
