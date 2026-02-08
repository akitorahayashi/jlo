# Workflow Branch Impact Map

This document is an operational index for branch-contract changes in workflow templates.
It is not an ownership spec and not a reproduction guide.
Its purpose is to minimize impact-analysis time when branch behavior changes.

## Scope

Branch contract covered by this map:
- `JLO_TARGET_BRANCH`: authoritative branch for `.jlo/`, `.github/actions`, and implementer output target.
- `JULES_WORKER_BRANCH`: runtime branch where `.jules/` is materialized and workflow layers execute.

Out of scope:
- Layer semantics and prompt/schema contracts.
- Jules API behavior.
- Generic GitHub repository setup.

## Invariants

- Branch selection is explicit; no implicit dependency on `github.event.repository.default_branch`.
- `.jlo/.jlo-version` is read from `JLO_TARGET_BRANCH`.
- Runtime execution operates on `JULES_WORKER_BRANCH`.
- Local composite action resolution is deterministic from the checked-out workspace.
- Missing required branch variables fail fast with explicit errors.

## Failure Signature Index

| Symptom | Probable contract break | First files to inspect |
|---|---|---|
| `Can't find 'action.yml' ... .github/actions/install-jlo` | Action resolution path does not exist in current checkout | `src/assets/workflows/.github/workflows/jules-workflows/components/bootstrap.yml.j2`, `src/assets/workflows/.github/workflows/jules-run-implementer.yml.j2`, `src/assets/workflows/.github/workflows/jules-sync.yml.j2` |
| `.jlo/.jlo-version is missing or empty` | Wrong source branch for version pin lookup | `src/assets/workflows/.github/actions/install-jlo/action.yml` |
| Worker branch bootstrap never converges | Worker creation/sync path mis-specified | `src/assets/workflows/.github/workflows/jules-workflows/components/bootstrap.yml.j2`, `src/assets/workflows/.github/workflows/jules-sync.yml.j2` |
| Implementer writes to unexpected branch | Target-branch variable wiring drifted | `src/assets/workflows/.github/workflows/jules-workflows.yml.j2`, `src/assets/workflows/.github/workflows/jules-run-implementer.yml.j2`, `src/assets/workflows/.github/actions/run-implementer/action.yml` |
| Sync workflow reports success but no effective sync | Skip gating or branch match logic is wrong | `src/assets/workflows/.github/workflows/jules-sync.yml.j2` |

## Change-to-Impact Matrix

| Change type | Workflow templates impacted | Composite actions impacted | Tests/doc impacted |
|---|---|---|---|
| Rename/add/remove branch variables | `jules-workflows.yml.j2`, `jules-sync.yml.j2`, `jules-run-implementer.yml.j2`, `jules-run-planner.yml.j2`, `jules-automerge.yml.j2`, `jules-mock-cleanup.yml.j2`, `jules-workflows/components/*.yml.j2`, `jules-workflows/macros/run_job.j2` | `install-jlo/action.yml`, `run-implementer/action.yml` | `tests/workflow_kit.rs`, `.github/AGENTS.md`, `docs/CONTROL_PLANE_OWNERSHIP.md`, this file |
| Change source of `.jlo/.jlo-version` | Any workflow using install step | `install-jlo/action.yml` | `tests/workflow_kit.rs`, docs branch-contract sections |
| Change worker bootstrap branch creation strategy | `jules-workflows/components/bootstrap.yml.j2`, `jules-sync.yml.j2` | none | `tests/workflow_kit.rs`, ownership docs |
| Change action resolution strategy (`./.github/actions/*`) | All workflows invoking local actions | `run-implementer/action.yml` | `tests/workflow_kit.rs` (string assertions) |
| Change implementer target routing | `jules-workflows.yml.j2`, `jules-run-implementer.yml.j2` | `run-implementer/action.yml` | `tests/workflow_kit.rs`, `.github/AGENTS.md` |

## Fast Investigation Path

When a branch-related workflow failure occurs:
1. Identify which branch variable value was actually resolved in the failing job.
2. Confirm the active checkout branch and whether `.github/actions/*` exists at that commit.
3. Verify `.jlo/.jlo-version` is present on `JLO_TARGET_BRANCH`.
4. Verify sync/creation semantics for `JULES_WORKER_BRANCH` were executed (not skipped).
5. Diff rendered workflow kit output from current templates before modifying runtime scripts.

## No-Fallback Review Checklist

- No use of `github.event.repository.default_branch` in workflow kit templates.
- No hardcoded `main` fallback for branch resolution.
- No silent branch substitution on lookup failure.
- No duplicate projection logic when `jlo workflow bootstrap` already materializes `.jules`.
- All required variables validated before branch-dependent steps run.

## Minimum Verification Set

Run after any branch-contract edit:

```bash
cargo test --test workflow_kit
cargo test workflow_templates_parse_with_serde_yaml
cargo test workflow_templates_pass_yaml_lint_remote
```

Optional focused grep guard:

```bash
rg -n "default_branch|\\.jlo-control" src/assets/workflows/.github -S
```
