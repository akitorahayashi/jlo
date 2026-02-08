# Template System

See [root AGENTS.md](../../../../AGENTS.md) for design principles.

## Directory Structure

```
src/assets/templates/
├── layers/
│   ├── observers/
│   │   └── role.yml      # Observer role template
│   └── deciders/
│       └── role.yml      # Decider role template
└── workstreams/
    ├── exchange/
    │   ├── events/       # Event state directories
    │   └── issues/       # Issue label directories
    ├── workstations/
    │   ├── events/
    │   └── issues/
    └── scheduled.toml    # Workstream schedule template
```

## Template Types

| Template Type | Location | Applied By | Result |
|---------------|----------|------------|--------|
| **Role** | `layers/<layer>/role.yml` | `jlo template -l <layer> -n <name>` | Creates `.jules/roles/<layer>/<name>/role.yml` |
| **Workstream** | `workstreams/<name>/` | `jlo template -w <name>` | Creates `.jules/workstreams/<name>/` with event/issue directories |

## Role Templates

Role templates are supported only for **multi-role layers** (Observers, Deciders, and Innovators).

- **Observers**: `layers/observers/role.yml`
- **Deciders**: `layers/deciders/role.yml`
- **Innovators**: `layers/innovators/role.yml`

Single-role layers (Narrator, Planners, Implementers) have a fixed role with `contracts.yml` in the layer directory and do not support template creation.

### Application

```bash
# Create a new observer role
jlo template -l observers -n taxonomy

# Create a new decider role
jlo template -l deciders -n triage
```

### Result

Creates `.jules/roles/<layer>/<name>/role.yml` populated from the template.

## Workstream Templates

Workstream templates define the directory structure for events and issues.

### Available Templates

| Template | Purpose |
|----------|---------|
| `exchange` | General-purpose workstream |
| `workstations` | Development environment workstream |

### Application

```bash
# Apply the exchange workstream template
jlo template -w exchange
```

### Result

Creates:
```
.jules/workstreams/<name>/
  events/
    <state>/          # State directories from template
  issues/
    <label>/          # Label directories from template
```

Also creates `.jules/workstreams/<name>/scheduled.toml` if `scheduled.toml` exists in the template.

## Relationship to jlo template Command

See [src/AGENTS.md](../../../src/AGENTS.md) for CLI command details.

The `jlo template` command:
1. Reads templates from `src/assets/templates/`
2. Validates the target location
3. Copies and populates the template files
4. Reports success or error
