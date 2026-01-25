# Run Output Conventions

## Session Files

Session files capture the output of a single run.

### Naming Convention

```
roles/<role_id>/sessions/YYYY-MM-DD/HHMMSS_<slug>.md
```

- `YYYY-MM-DD` is the session date (ISO).
- `HHMMSS` is the timestamp (UTC recommended).
- `<slug>` is a short description (lowercase, hyphens or underscores).

### Session Structure

```markdown
# Session: <slug>

**Role:** <role_id>
**Date:** YYYY-MM-DD HH:MM:SS UTC
**Trigger:** <what initiated this run>

## Context

<What was read and understood before starting>

## Analysis

<Reasoning and observations>

## Outputs

<Concrete deliverables or recommendations>

## Next Steps

<Suggested follow-up actions>
```

## Exchange Messages

Messages to another role are stored under:

```
exchange/inbox/<role_id>/YYYY-MM-DD_HHMMSS_<slug>.md
```
