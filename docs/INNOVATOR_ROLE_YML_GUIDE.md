# Innovator `role.yml` Design Guide

## Purpose

This document defines the current design standard for creating a new innovator persona.
The scope is limited to `role.yml` design quality.

## Layer Character

Innovator roles represent introduction-oriented strategic lenses.
They exist to propose mechanisms that change outcomes, boundaries, or decision quality.
They are not static-analysis cleanup roles.

## Required Schema

A valid innovator role file is located at:

- `.jlo/roles/innovators/<role>/role.yml`

Current schema shape:

```yaml
role: <role_id>
layer: innovators
profile:
  bias_focus: <string>
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
- all `profile.*` fields listed above

## Abstraction Standard

Innovator abstraction stays at leverage classes, not local patch classes.

`bias_focus` defines the direction of strategic preference.
`analysis_points` define recurring leverage opportunities.
`first_principles` encode introduction-level decision logic.
`guiding_questions` define proposal discrimination criteria.
`proposal_quality_bar` defines publication threshold for proposal quality.

A strong innovator role remains technology-agnostic enough to transfer across repositories while still expressing a clear intervention style.

## Boundary Quality

A strong innovator role has explicit separation from observer duties. Innovators propose strategic mechanisms that introduce new outcomes, boundaries, or decision quality, whereas observers focus on identifying evidence-backed signals and boundaries rather than proposing new mechanisms.
The role does not collapse into quality auditing or issue triage.
The role does not optimize for output volume; it optimizes for introduction impact.

## Anti-Patterns

- Roles that only repackage observer findings into new wording.
- Roles defined by single-tool preference instead of outcome class.
- Roles that propose local refactoring without introducing a new mechanism.
- Roles with no explicit evidence contract for proposal claims.
- Roles with no quality bar for deciding proposal readiness.
