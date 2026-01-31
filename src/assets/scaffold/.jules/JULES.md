# Jules Contract

This file defines the binding rules for Jules agents operating in this repository.

## Authority

- This file is authoritative for global rules and shared conventions.
- Each layer contract is authoritative for layer-specific workflows and schemas:
  - `.jules/roles/observers/contracts.yml`
  - `.jules/roles/deciders/contracts.yml`
  - `.jules/roles/planners/contracts.yml`
  - `.jules/roles/implementers/contracts.yml`

If a required contract file is missing or conflicts with another contract, execution stops and the
conflict is reported.

## Required Read Order

1. The role's `prompt.yml` (already provided as the run prompt)
2. `.jules/JULES.md`
3. The layer `contracts.yml`
4. Role-specific inputs required by the layer contract

## Workstream Model

Workstreams isolate events and issues so that decider rules do not mix across unrelated operational areas.

- Observers and deciders declare their destination workstream in `prompt.yml` via `workstream: <name>`.
- If the workstream directory is missing, execution fails fast.
- Planners and implementers do not declare a workstream; the issue file path is authoritative.

Workstream directories:

- Events (Observer output, Decider input):
  - `.jules/workstreams/<workstream>/events/pending/*.yml` (pending)
  - `.jules/workstreams/<workstream>/events/decided/*.yml` (processed)
- Issues (Decider/Planner output, Implementer input): `.jules/workstreams/<workstream>/issues/<priority>/*.yml`

## Workspace Data Flow

The pipeline is file-based and terminates at local issues:

`events -> issues`

After decider output:
- Issues with `requires_deep_analysis: false` are ready for implementation.
- Issues with `requires_deep_analysis: true` trigger deep analysis by planners.
- Implementers are invoked manually via `workflow_dispatch` with a local issue file.

## Issue Identity and Deduplication

- Issue filenames use stable ids, not dates (e.g. `auth_inconsistency.yml`).
- Observers check open issues before emitting events to avoid duplicates.
- Deciders link related events to issues (populating `source_events` in the issue).
- Events are preserved in the workstream until the issue is resolved by an Implementer.
- Issues have `status: open|closed` to track lifecycle.

## Deep Analysis

When an issue requires deep analysis:
- `requires_deep_analysis: true` must have a non-empty `deep_analysis_reason` field.
- Planners expand the issue and set `requires_deep_analysis: false`.
- The original rationale is preserved and expanded with findings.

## File Rules

- YAML only (`.yml`) and English only.
- Artifacts are created by copying the corresponding template and filling its fields:
  - Events: `.jules/roles/observers/event.yml`
  - Issues: `.jules/roles/deciders/issue.yml`
  - Feedback: `.jules/roles/deciders/feedback.yml`

## Git And Branch Rules

The runner provides `starting_branch`. Agents do not change it.

Branch names:

- Observers: `jules-observer-<id>`
- Deciders: `jules-decider-<id>`
- Planners: `jules-planner-<id>`
- Implementers: `jules-implementer-<id>-<short_description>`

`<id>` is 4 alphanumeric characters unless the layer contract specifies otherwise.

## Safety Boundaries

- Observers, Deciders, and Planners modify only `.jules/`.
- Implementers modify only what the issue specifies, run the verification command, then
  create a pull request for human review.

## Forbidden By Default

- `.github/workflows/` is not modified unless explicitly required by the issue.
