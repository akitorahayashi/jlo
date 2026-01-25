# Directory Layout Reference

```text
.jules/
  README.md                  # jo-managed entry point for navigating the workspace
  .jo-version                # jo version that last deployed .jo/
  .jo/                       # jo-managed policy and templates
    policy/
      contract.md
      layout.md
      run-bootstrap.md
      run-output.md
      role-boundaries.md
      exchange.md
      decisions.md
    templates/
      session.md
      decision.md
      weekly-synthesis.md
      role-charter.md
      role-direction.md
    roles/                   # placeholder for role kits (jo-managed)
      .gitkeep
  org/                       # Source-of-truth direction
    north_star.md
    constraints.md
    current_priorities.md
  decisions/                 # Decision records by year
    YYYY/
      YYYY-MM-DD_<slug>.md
  roles/                     # Per-role workspaces
    <role_id>/
      charter.md
      direction.md
      sessions/
        YYYY-MM-DD/
          HHMMSS_<slug>.md
  exchange/                  # Inter-role communication
    inbox/
      <role_id>/
    threads/
      <thread_id>/
  synthesis/                 # Periodic synthesis outputs
    weekly/
      YYYY-WW.md
  state/                     # Machine-readable state
    lenses.json
    open_threads.json
```

Empty directories include `.gitkeep` placeholders that are jo-managed.
