# Observer Role Guide

The **Observers** layer analyzes repository state and emits event artifacts for Decider.

## Inputs

- Repository codebase state
- Optional narrator change summary: `.jules/exchange/changes.yml`
- Workstation perspective: `.jules/workstations/<role>/perspective.yml`
- Role contract: `.jlo/roles/observers/<role>/role.yml`

Observers do not participate in innovator idea/comment bridging.

## Outputs

- Event files in:
  - `.jules/exchange/events/pending/*.yml`
- Updated workstation perspective:
  - `.jules/workstations/<role>/perspective.yml`

## Execution

```bash
jlo run observer <role_name>
```

Example:

```bash
jlo run observer taxonomy
```

## Event Schema

Event schema is defined by:

- `.jules/layers/observers/schemas/event.yml`

Typical required fields include:

- `id`
- `created_at`
- `author_role`
- `confidence`
- `title`
- `statement`
- `evidence`
