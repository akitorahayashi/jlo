# .jules Workspace Contract

## Purpose

The `.jules/` directory is repository-local organizational memory and a workflow contract for
scheduled agents and humans. It preserves direction, decisions, and per-role session outputs
so each scheduled run starts fresh while regaining context by reading `.jules/`.

## Invariants

- Session outputs are immutable; each run creates a new file.
- Scheduled tasks are read-only for product code; outputs stay under `.jules/`.
- Source-of-truth documents in `org/` prevent drift.
- Roles are stable decision functions rather than domain-specific job titles.
- `.jo/` is jo-managed and may be overwritten by `jo update`.

## Ownership

| Path | Owner | Notes |
|------|-------|-------|
| `.jules/.jo/` | jo | Overwritten by `jo update` |
| `.jules/.jo-version` | jo | Version marker |
| `.jules/org/` | Human | Source-of-truth documents |
| `.jules/decisions/` | Human/Agent | Decision records |
| `.jules/roles/` | Human/Agent | Role outputs |
| `.jules/exchange/` | Agent | Inter-role communication |
| `.jules/synthesis/` | Agent | Periodic synthesis |
| `.jules/state/` | Agent | Machine-readable state |
