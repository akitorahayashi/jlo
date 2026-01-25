# Run Bootstrap

A scheduled run starts with a minimal context refresh so decisions stay consistent.

## Reading Order

1. `README.md` provides workspace orientation.
2. `org/` provides current direction, constraints, and priorities.
3. The role's `charter.md` describes its decision function.
4. The role's `direction.md` captures role-specific focus.
5. `exchange/inbox/<role_id>/` contains incoming requests.
6. Recent sessions provide continuity.

## Output Placement

Session output is written to:

```
roles/<role_id>/sessions/YYYY-MM-DD/HHMMSS_<slug>.md
```

Product code is not modified during scheduled runs.
`org/` and `decisions/` are treated as human-managed sources of truth.
