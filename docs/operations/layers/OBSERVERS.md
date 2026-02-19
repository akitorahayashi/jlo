# Observers
Agents analyzing repository state and emitting event artifacts.

## Interface
- Input: repo state, `.jules/exchange/changes.yml`, workstation `.jules/workstations/<role>/perspective.yml`, contract `.jlo/roles/observers/<role>/role.yml`.
- Output: `.jules/exchange/events/pending/*.yml`, updated `perspective.yml`.
- Execution: `jlo run observers --role <role_name>`

## Constraints
- Scope: Modifies `pending/` events and workstation `perspective.yml`. Reads entire repo.
- Deduplication: Avoid duplicate findings with open requirements or recent events.
- Memory: Persistent state strictly resides in `perspective.yml`.
- Prioritization memory: `focus_paths` in `perspective.yml` stores repository-relative areas to inspect first on subsequent runs.
- Memory quality: goals remain durable role-owned analytical objectives and do not store run history, event IDs, or ticket IDs.

## Resources
- Schema: `.jules/schemas/observers/event.yml`
- Tasks:
  - prepare_scope.yml: Reads role memory and defines scope/signal filter.
  - emit_events.yml: Emits 0-3 evidence-backed event files from scoped inspection.
  - refresh_perspective.yml: Updates durable role memory for the next run.
