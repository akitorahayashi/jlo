# Observers
Agents analyzing repository state and emitting event artifacts.

## Interface
- Input: repo state, optional `.jules/exchange/changes.yml`, contract `.jlo/roles/observers/<role>/role.yml`.
- Output: `.jules/exchange/events/pending/*.yml`.
- Execution: `jlo run observers --role <role_name>`

## Constraints
- Scope: Modifies `pending/` events only. Reads entire repo.
- Deduplication: Avoid duplicate findings with open requirements or recent events.
- Role policy: `role.yml` `profile` defines analysis lens and `constraint` defines prohibited or required boundaries.

## Resources
- Schema: `.jules/schemas/observers/event.yml`
- Tasks:
  - emit_events.yml: Emits 0-3 evidence-backed event files from repository inspection.
