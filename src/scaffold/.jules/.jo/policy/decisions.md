# Decision Records

## Purpose

The `decisions/` directory preserves significant decisions with rationale.

## Structure

```text
decisions/
  YYYY/
    YYYY-MM-DD_<slug>.md
```

## Scope

Decision records capture architectural choices, policy changes, scope decisions,
and trade-off resolutions that affect future work.

## Decision Record Format

```markdown
# Decision: <title>

**Date:** YYYY-MM-DD
**Status:** proposed | accepted | deprecated | superseded
**Deciders:** <who made this decision>

## Context

<What prompted this decision>

## Decision

<The decision made>

## Rationale

<Why this decision was made>

## Consequences

<Expected outcomes, both positive and negative>

## Alternatives Considered

<Other options and why they were not chosen>
```
