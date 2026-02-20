# Template System

See [root AGENTS.md](../../AGENTS.md) for design principles.

## Directory Structure

```
src/assets/templates/
└── layers/
    ├── observers/
    │   └── role.yml      # Observer role template
    └── innovators/
        └── role.yml      # Innovator role template
```

## Template Types

| Template Type | Location | Applied By | Result |
|---------------|----------|------------|--------|
| Role | `layers/<layer>/role.yml` | `jlo role create <layer> <name>` | Creates `.jlo/roles/<layer>/<name>/role.yml` |

## Role Templates

Role templates are supported only for multi-role layers (Observers and Innovators).

- Observers: `layers/observers/role.yml`
- Innovators: `layers/innovators/role.yml`

Single-role layers (Narrator, Decider, Planner, Implementer) have a fixed role with `contracts.yml` in the layer directory and do not support template creation.

### Application

```bash
# Create a new observer role
jlo role create observers taxonomy
```

### Result

Creates `.jlo/roles/<layer>/<name>/role.yml` populated from the template.

## Relationship to jlo role create Command

See [src/AGENTS.md](../../src/AGENTS.md) for CLI command details.

The `jlo role create` command:
1. Reads templates from `src/assets/templates/`
2. Validates the target location in `.jlo/`
3. Populates the template file
4. Reports success or error
