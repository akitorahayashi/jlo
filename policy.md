# E2E Validation Policy

## Scope
E2E validation covers two orthogonal behaviors: exchange artifact flow within `.jules/` and GitHub mechanics that create or merge repository state.

## Coverage Rules
Pipeline-only validation is classified as artifact-level validation. Full E2E validation combines a `jlo` CLI dry run for prompt assembly with GitHub mechanics that exercise the auto-merge path and issue lifecycle. Mechanics validation is complete only when it exercises the same merge path used by the auto-merge workflow.

## Sources of Truth
Event categories are sourced from `.jules/exchange/events` on the `jules` branch. E2E fixture templates live in `.github/assets/jules-e2e` and remain aligned with the `.jules/roles` templates. Auto-merge eligibility and branch/role constraints are defined in `.github/workflows/jules-automerge.yml` and `.jules/JULES.md`. CLI inputs are sourced from `.jules/config.toml` and workflow dispatch inputs.

## Execution Constraints
Workflows that mutate repository state are manually dispatched and use a bot token with write permissions. Runs that rely on `.jules/` execute on the `jules` branch where the scaffold exists. Artifacts used for mock flows are stored under `.jules-mock` in the workflow workspace.

## Evidence
Successful E2E runs do not retain artifacts or persistent GitHub objects. PRs are auto-merged and deleted, issues are created and closed, and mock artifacts are not retained. Failures stop the workflow and leave the in-flight PR or issue open for inspection.
