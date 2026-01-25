# .jules Workspace

The `.jules/` directory is repository-local organizational memory for scheduled agents and humans.
It preserves direction, decisions, and role sessions so new runs regain context quickly.

## Navigation

- `org/` holds source-of-truth direction.
- `decisions/` stores decision records by date.
- `roles/` contains per-role sessions and notes.
- `exchange/` hosts inter-role messages and threads.
- `synthesis/` stores periodic summaries.
- `state/` stores machine-readable state.
- `.jo/` stores jo-managed policy and templates.

## Ownership

- `.jo/` is jo-managed and may be overwritten by `jo update`.
- Everything else is human or agent output and is not overwritten by jo.
