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
| **Role** | `layers/<layer>/role.yml` | `jlo create role <layer> <name>` | Creates `.jlo/roles/<layer>/roles/<name>/role.yml` |
| **Workstream** | `workstreams/<name>/` | `jlo create workstream <name>` | Creates `.jlo/workstreams/<name>/scheduled.toml` |

## Role Templates

Role templates are supported only for **multi-role layers** (Observers, Deciders, and Innovators).

- **Observers**: `layers/observers/role.yml`
- **Deciders**: `layers/deciders/role.yml`
- **Innovators**: `layers/innovators/role.yml`

Single-role layers (Narrator, Planners, Implementers) have a fixed role with `contracts.yml` in the layer directory and do not support template creation.

### Application

```bash
# Create a new observer role
jlo create role observers taxonomy

# Create a new decider role
jlo create role deciders triage
```

### Result

Creates `.jlo/roles/<layer>/roles/<name>/role.yml` populated from the template.

## Workstream Templates

Workstream templates define the directory structure for events and issues.

### Available Templates

| Template | Purpose |
|----------|---------|
| `exchange` | General-purpose workstream |
| `workstations` | Development environment workstream |

### Application

```bash
# Create a workstream
jlo create workstream exchange
```

### Result

Creates `.jlo/workstreams/<name>/scheduled.toml` seeded from the template.

## Relationship to jlo create Command

See [src/AGENTS.md](../../../src/AGENTS.md) for CLI command details.

The `jlo create` command:
1. Reads templates from `src/assets/templates/`
2. Validates the target location in `.jlo/`
3. Populates the template file
4. Reports success or error
