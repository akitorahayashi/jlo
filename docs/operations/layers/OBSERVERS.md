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

## Resources
- Schema: `.jules/layers/observers/schemas/event.yml`
- Tasks:
  - emit_events.yml: Logic for repo state analysis and event emission.
