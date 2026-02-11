---
name: create-jlo-innovator
description: Create or review `.jlo/roles/innovators/<role>/role.yml` with strong strategic-lens quality, explicit evidence contract, and proposal quality bar. Use when asked to design a new innovator persona, refine innovator role definitions, or validate innovator role.yml quality.
---

# Create JLO Innovator

## Core Objective

Produce an innovator `role.yml` that defines a reusable strategic introduction lens, not an observer-like analysis lens.

## Output Contract

Target file:
- `.jlo/roles/innovators/<role>/role.yml`

Required shape:

```yaml
role: <role_id>
layer: innovators
profile:
  focus: <string>
  analysis_points: <non-empty sequence>
  first_principles: <non-empty sequence>
  guiding_questions: <non-empty sequence>
  anti_patterns: <non-empty sequence>
  evidence_expectations: <non-empty sequence>
  proposal_quality_bar: <non-empty sequence>
```

## Design Workflow

1. Define a role name that is domain-specific and non-ambiguous.
2. Set `focus` as a strategic direction of intervention.
3. Write `analysis_points` as recurring leverage classes, not local fix categories.
4. Write `first_principles` as introduction-level decision logic.
5. Write `guiding_questions` to discriminate proposal quality.
6. Write `evidence_expectations` as explicit claim-proof standards.
7. Write `proposal_quality_bar` as publication threshold criteria.
8. Verify strict separation from observer duties.

## Boundary Rules

- Innovator proposes mechanisms that change outcomes, boundaries, or decision quality.
- Innovator does not collapse into quality auditing, issue triage, or patch-level refactoring.
- Innovator optimizes for introduction impact, not output volume.

## Anti-Pattern Checks

- The role only rephrases observer findings.
- The role is defined by one tool preference instead of outcome class.
- The role proposes only local refactoring with no new mechanism.
- The role lacks evidence contract for proposal claims.
- The role lacks clear proposal readiness criteria.

## Review Mode

When reviewing an existing innovator role:
1. Validate required fields and layer value.
2. Flag any observer-duty overlap.
3. Flag weak abstraction (local patch class wording).
4. Return specific rewrite suggestions for `focus`, `analysis_points`, and `proposal_quality_bar`.
