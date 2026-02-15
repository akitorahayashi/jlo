---
name: create-jlo-observer
description: Create or review `.jlo/roles/observers/<role>/role.yml` with a narrow analytical lens, reusable signal classes, and explicit evidence standards.
---

# Create JLO Observer

## Core Objective

Define an observer `role.yml` that produces repeatable analysis quality from repository evidence.

## Output Contract

Target file:
- `.jlo/roles/observers/<role>/role.yml`

Required shape:

```yaml
role: <role_id>
layer: observers
profile:
  focus: <string>
  analysis_points: <non-empty sequence>
  first_principles: <non-empty sequence>
  guiding_questions: <non-empty sequence>
  anti_patterns: <non-empty sequence>
  evidence_expectations: <non-empty sequence>
```

Validator-critical fields:
- `role`
- `layer` (must be `observers`)
- `profile.focus`
- `profile.analysis_points`

## Design Workflow

1. Set `focus` as one stable analytical boundary.
2. Write `analysis_points` as recurring signal classes, not incident examples.
3. Write `first_principles` as judgment logic that can be reused across repositories.
4. Write `guiding_questions` that force falsifiable reasoning.
5. Write `evidence_expectations` as minimum proof required before accepting claims.
6. Confirm the role stays analytical and does not prescribe implementation work.

## Boundary Rules

- Do not define the role by one tool, one file, or one temporary incident.
- Do not encode layer-level task procedure into role.yml.
- Do not add repository-specific input checklists in role.yml.
- Keep wording narrow enough to reject out-of-scope requests.

## Anti-Pattern Checks

- `focus` is broad enough to absorb unrelated domains.
- `analysis_points` are action items or refactoring plans.
- `analysis_points` are path-specific checklists instead of reusable signal classes.
- `evidence_expectations` are missing, weak, or unfalsifiable.
- The role duplicates another observer with renamed wording only.

## Review Mode

When reviewing an existing observer role, return only:
1. Schema violations.
2. Scope ambiguity in `focus`.
3. Non-reusable entries in `analysis_points`.
4. Concrete rewrites for `focus`, `analysis_points`, and `evidence_expectations`.
