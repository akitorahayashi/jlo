---
name: create-jlo-innovator
description: Create or review `.jlo/roles/innovators/<role>/role.yml` with a strategic intervention lens, explicit evidence standards, and a clear proposal quality bar.
---

# Create JLO Innovator

## Core Objective

Define an innovator `role.yml` that generates high-leverage intervention proposals from repository reality.

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

Validator-critical fields:
- `role`
- `layer` (must be `innovators`)
- `profile.focus`
- `profile.analysis_points`
- `profile.proposal_quality_bar`

## Design Workflow

1. Set `focus` as one stable intervention boundary.
2. Write `analysis_points` as recurring leverage classes, not patch-level fix categories.
3. Write `first_principles` as decision logic for selecting interventions.
4. Write `guiding_questions` that force comparative judgment between alternatives.
5. Write `evidence_expectations` as minimum proof required before accepting strategic claims.
6. Write `proposal_quality_bar` as explicit publish/no-publish criteria.
7. Confirm strict separation from observer duties.

## Boundary Rules

- Do not collapse into observer work (quality auditing, issue triage, patch diagnosis).
- Do not define the role by one tool, one file, or one temporary incident.
- Do not encode layer-level task procedure into role.yml.
- Keep wording narrow enough to reject low-leverage proposal classes.

## Anti-Pattern Checks

- `focus` is broad enough to absorb unrelated domains.
- `analysis_points` are local refactoring categories with no mechanism shift.
- The role is defined by one tool preference instead of intervention outcome class.
- `evidence_expectations` are weak, missing, or unfalsifiable.
- `proposal_quality_bar` cannot reject low-quality proposals.

## Review Mode

When reviewing an existing innovator role, return only:
1. Schema violations.
2. Observer-duty overlap.
3. Weak abstraction in `analysis_points`.
4. Concrete rewrites for `focus`, `analysis_points`, and `proposal_quality_bar`.
