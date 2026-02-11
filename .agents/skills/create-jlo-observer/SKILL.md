---
name: create-jlo-observer
description: Create or review `.jlo/roles/observers/<role>/role.yml` with a narrow analytical lens, reusable signal classes, and evidence-backed judgment quality. Use when asked to design a new observer persona, refine observer role definitions, or validate observer role.yml quality.
---

# Create JLO Observer

## Core Objective

Produce an observer `role.yml` that defines a precise analytical lens over repository state and yields reproducible evidence-backed judgment.

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
  first_principles: <sequence>
  guiding_questions: <sequence>
  anti_patterns: <sequence>
  evidence_expectations: <sequence>
```

Validator-critical fields:
- `role`
- `layer` (must be `observers`)
- `profile.focus`
- `profile.analysis_points`

## Design Workflow

1. Define a domain-specific role name with low overlap against existing observers.
2. Set `focus` as a stable analytical boundary.
3. Write `analysis_points` as reusable signal classes, not one-off incidents.
4. Write `first_principles` as repeatable judgment logic.
5. Write `guiding_questions` to enforce consistent reasoning behavior.
6. Write `evidence_expectations` as required proof format before accepting claims.
7. Validate the role as analytical, not solution-authoring.

## Boundary Rules

- Observer identifies signals and boundaries from current repository state.
- Observer does not produce implementation plans as core role definition.
- Observer role avoids generic ownership language that can absorb every problem.

## Anti-Pattern Checks

- The role describes coding solutions instead of analytical judgment.
- The role is defined by one tool, one file, or one temporary incident.
- The role has stylistic preferences but no evidence contract.
- The role duplicates an existing observer lens with renamed wording.
- The role uses repository-specific checklists instead of reusable signal classes.

## Review Mode

When reviewing an existing observer role:
1. Validate required fields and layer value.
2. Flag broad or ambiguous `focus`.
3. Flag non-reusable `analysis_points`.
4. Return concrete rewrites for `focus`, `analysis_points`, and `evidence_expectations`.
