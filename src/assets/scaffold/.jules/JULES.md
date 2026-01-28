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

## Workspace Data Flow

The pipeline is file-based:

`events -> issues -> tasks -> code changes`

Exchange directories:

- Events (Observer output, Decider input): `.jules/exchange/events/<category>/*.yml`
- Issues (Decider output, Planner input): `.jules/exchange/issues/*.yml`
- Tasks (Planner output, Implementer input): `.jules/exchange/tasks/*.yml`

Categories are the directory names under `.jules/exchange/events/`.

## File Rules

- YAML only (`.yml`) and English only.
- Artifacts are created by copying the corresponding template and filling its fields:
  - Events: `.jules/roles/observers/event.yml`
  - Issues: `.jules/roles/deciders/issue.yml`
  - Feedback: `.jules/roles/deciders/feedback.yml`
  - Tasks: `.jules/roles/planners/task.yml`

## Git And Branch Rules

The runner provides `starting_branch`. Agents do not change it.

Branch names:

- Observers: `jules-observer-<id>`
- Deciders: `jules-decider-<id>`
- Planners: `jules-planner-<id>`
- Implementers: `jules-implementer-<task_id>-<short_description>`

`<id>` is 4 alphanumeric characters unless the layer contract specifies otherwise.

## Safety Boundaries

- Observers, Deciders, and Planners modify only `.jules/`.
- Implementers modify only what the task specifies, run the verification command, then delete the
  processed task file.

## Forbidden By Default

- `.github/workflows/` is not modified unless explicitly required by the issue/task.
