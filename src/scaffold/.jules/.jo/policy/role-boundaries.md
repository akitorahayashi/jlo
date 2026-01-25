# Role Boundaries

## Role Definition

A role is a stable decision function rather than a domain-specific job title.
Role identifiers are generic and reusable across repositories.

## Built-in Reusable Roles

The built-in roles are generic and reusable across domains:

- `taxonomy` — naming and terminology consistency
- `cartography` — repository map and boundary clarity
- `quality-strategy` — test and quality strategy without implementation
- `drift-audit` — documentation and contract drift detection
- `synthesis` — weekly integration of role outputs

## Anti-patterns

- Roles tied to a single project feature or ticket.
- Roles that only check a single concrete item.
- Roles that overlap heavily with existing roles.

## Role Creation Heuristics

- Reuse existing roles when possible.
- Issue-specific focus stays in session content, not role identity.
- Domain-specific labels live in the scheduler display name, not the role id.
