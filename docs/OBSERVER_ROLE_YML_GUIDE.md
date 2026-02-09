# Observer `role.yml` Design Guide

## Purpose

This document defines the current design standard for creating a new observer persona.
The scope is limited to `role.yml` design quality.

## Layer Character

Observer roles represent analytical lenses over the current repository state.
They identify evidence-backed signals and boundaries, not implementation plans.
Their output value comes from precision, reproducibility, and consistency of judgment.

## Required Schema

A valid observer role file is located at:

- `.jlo/roles/observers/roles/<role>/role.yml`

Current schema shape:

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

Repository quality baseline fields:

- `profile.first_principles`
- `profile.guiding_questions`
- `profile.anti_patterns`
- `profile.evidence_expectations`

## Abstraction Standard

Observer abstraction stays at the level of recurring signal classes, not one-off fixes.

`focus` describes the stable analytical boundary of the role.
`analysis_points` represent durable signal categories that remain valid across revisions.
`first_principles` encode why the role judges signals as defects or risks.
`guiding_questions` define repeatable reasoning behavior.
`evidence_expectations` define proof format before a claim is accepted.

## Boundary Quality

A strong observer role has a narrow lens and low overlap with existing observers. 
On the other hand, even if they work every day, they do not produce the same output. Each time they work, they make critical and creative proposals based on their own analytical points, which move the project forward.
It avoids generic ownership language and avoids role definitions that can absorb every problem.
The role name stays domain-specific and non-ambiguous.

## Anti-Patterns

- A role that describes coding solutions instead of analytical judgment.
- A role defined by one tool, one file, or one temporary incident.
- A role with only stylistic preferences and no evidence contract.
- A role that duplicates an existing observer lens with renamed vocabulary.
- A role whose `analysis_points` are repository-specific checklists instead of reusable signal classes.
