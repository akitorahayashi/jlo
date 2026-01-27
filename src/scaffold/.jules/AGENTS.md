# .jules/ Agent Contract

This document defines the operational contract for agents working in this repository.
All scheduled agents must read this file before acting.

## Roles and Responsibilities

### Workers

Workers are specialized lenses (taxonomy, data_arch, qa).
They:

- read `AGENTS.md` and `.jules/AGENTS.md`
- read their own `.jules/roles/<role>/role.yml` and `notes/`
- update `notes/` declaratively
- write normalized event files under `.jules/events/`

Workers do **not** write `issues/`.

### Triage

`triage` is the only role that writes `issues/`.
It:

- reads all `.jules/events/**/*.yml` and existing `.jules/issues/*.md`
- validates observations, merges related events, and creates actionable issues
- deletes processed events (accepted or rejected)
- updates worker `role.yml` when rejections indicate recurring noise

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

## Role Feedback Updates

If an event is rejected, `triage` updates the originating worker's `role.yml`.
Feedback should be appended under a dedicated `feedback` section to reduce recurring noise.

## Deletion Policy

Processed events are deleted after triage (accepted or rejected).
