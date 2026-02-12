# Workflow Branch Impact Map

This document is an operational index for branch-contract changes in workflow templates.

## Scope

Branch contract covered by this map:
- `JLO_TARGET_BRANCH`: authoritative branch for `.jlo/`, `.github/actions`, and implementer output target.
- `JULES_WORKER_BRANCH`: runtime branch where `.jules/` is materialized and workflow layers execute.

## Invariants

- Branch selection is explicit; no implicit dependency on `github.event.repository.default_branch`.
- `.jlo/.jlo-version` is read from `JLO_TARGET_BRANCH`.
- Runtime execution operates on `JULES_WORKER_BRANCH`.
- Missing required branch variables fail fast with explicit errors.

## Failure Signature Index

| Symptom | Probable contract break | First files to inspect |
|---|---|---|
| `Can't find 'action.yml' ... .github/actions/install-jlo` | Action resolution path missing in checkout | `src/assets/github/workflows/jules-scheduled-workflows.yml.j2`, `src/assets/github/actions/install-jlo/action.yml` |
| `.jlo/.jlo-version is missing or empty` | Wrong source branch for version pin lookup | `src/assets/github/actions/install-jlo/action.yml` |
| Worker branch sync does not converge | Branch sync path mis-specified | `src/assets/github/workflows/jules-scheduled-workflows.yml.j2` |
| Implementer metadata not applied | Implementer trigger/matching drifted | `src/assets/github/workflows/jules-scheduled-workflows.yml.j2`, `src/app/commands/workflow/gh/pr/process.rs` |
| Auto-merge process fails unexpectedly | PR processing mode/retry/policy drifted | `src/assets/github/workflows/jules-scheduled-workflows.yml.j2`, `src/app/commands/workflow/gh/pr/process.rs`, `src/app/commands/workflow/gh/pr/events/enable_automerge.rs` |

## Change-to-Impact Matrix

| Change type | Workflow templates impacted | Composite actions impacted | Tests/doc impacted |
|---|---|---|---|
| Rename/add/remove branch variables | `jules-scheduled-workflows.yml.j2`, `jules-mock-cleanup.yml.j2`, `jules-workflows/components/*.yml.j2`, `jules-workflows/macros/*.j2` | `install-jlo/action.yml` | `tests/workflow.rs`, `.github/AGENTS.md`, `docs/CONTROL_PLANE_OWNERSHIP.md`, this file |
| Change source of `.jlo/.jlo-version` | `jules-scheduled-workflows.yml.j2` | `install-jlo/action.yml` | `tests/workflow.rs`, branch-contract docs |
| Change worker bootstrap/sync strategy | `jules-scheduled-workflows.yml.j2`, `jules-workflows/components/bootstrap.yml.j2` | none | `tests/workflow.rs`, ownership docs |
| Change implementer target routing | `jules-scheduled-workflows.yml.j2` | `install-jlo/action.yml` | `tests/workflow.rs`, `.github/AGENTS.md` |

## Fast Investigation Path

1. Identify resolved branch values in the failing job.
2. Confirm active checkout branch and local action availability.
3. Verify `.jlo/.jlo-version` exists on `JLO_TARGET_BRANCH`.
4. Verify worker sync/creation path executed (not skipped).
5. Diff rendered workflow output from current templates.

## No-Fallback Review Checklist

- No use of `github.event.repository.default_branch` in workflow templates.
- No hardcoded `main` fallback for branch resolution.
- No silent branch substitution on lookup failure.
- All required branch variables validated before branch-dependent steps.

## Minimum Verification Set

```bash
cargo test --test workflow
cargo test workflow_templates_parse_with_serde_yaml
cargo test workflow_templates_pass_yaml_lint_remote
```

Optional grep guard:

```bash
rg -n "default_branch|\.jlo-control" src/assets/github -S
```
