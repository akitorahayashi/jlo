# GitHub Actions Workflow Context

See [root AGENTS.md](../AGENTS.md) for design principles.
See [CONTROL_PLANE_OWNERSHIP.md](../docs/CONTROL_PLANE_OWNERSHIP.md) for the `.jlo/` vs `.jules/` ownership model and projection rules.

## Branch Topology

| Branch | Contents | Owner |
|--------|----------|-------|
| `JLO_TARGET_BRANCH` | `.jlo/` (user intent overlay), `.github/` (workflow kit) | User + `jlo init` |
| `JULES_WORKER_BRANCH` | `.jules/` (runtime scaffold, materialized by bootstrap) | Workflow automation |

The `JULES_WORKER_BRANCH` branch is **never edited directly** by users. Its `.jules/` directory is assembled by the bootstrap job from two sources:
1. **Embedded scaffold** — `jlo workflow bootstrap` writes the base structure from the `jlo` binary's embedded assets.
2. **Intent overlay** — `jlo workflow bootstrap` overlays `.jlo/` inputs that are present on the worker branch (created from `JLO_TARGET_BRANCH`).

## Branch Strategy

| Agent Type | Starting Branch | Output Branch | Auto-merge |
|------------|-----------------|---------------|------------|
| Narrator | `JULES_WORKER_BRANCH` | `jules-narrator-*` | ✅ (if `.jules/` only) |
| Observer | `JULES_WORKER_BRANCH` | `jules-observer-*` | ✅ (if `.jules/` only) |
| Decider | `JULES_WORKER_BRANCH` | `jules-decider-*` | ✅ (if `.jules/` only) |
| Planner | `JULES_WORKER_BRANCH` | `jules-planner-*` | ✅ (if `.jules/` only) |
| Implementer | `JLO_TARGET_BRANCH` | `jules-implementer-*` | ❌ (human review) |
| Innovator | `JULES_WORKER_BRANCH` | `jules-innovator-*` | ✅ (if `.jules/` only) |

## Workflow Files

Jules workflows are installed via `jlo init --remote` (or `--self-hosted`) and follow these patterns:

- `.github/workflows/jules-*.yml`
- `.github/actions/` (Jules composite actions)

Non-Jules CI workflows remain in `.github/workflows/` alongside the kit.

The workflow kit is generated from `src/assets/workflows/.github/`. Edit that source directory, not `.github/`, and re-run `jlo init` to apply changes.

The `jules-*.yml` files under `.github/` are jlo’s dogfooding artifacts. Direct edits are not recommended; workflow changes are made in `src/assets/workflows/.github/` and the related rendering pipeline. See `src/assets/workflows/AGENTS.md` for the template pipeline summary.

## Composite Actions

Jules composite actions live under `.github/actions/` and are installed with the workflow kit.

## Orchestration Commands

Workflow orchestration uses `jlo workflow` commands:

- `jlo workflow matrix roles --layer <layer>` → Generate role matrix
- `jlo workflow matrix pending` → Generate decider matrix
- `jlo workflow matrix routing` → Generate planner/implementer routing
- `jlo workflow run <layer>` → Execute layer with JSON output
- `jlo workflow inspect` → Inspect exchange state as JSON
- `jlo workflow clean-issue <issue_file>` → Remove processed issue and source events
- `jlo workflow publish-proposals` → Publish innovator proposals as GitHub issues
- `jlo workflow pr comment-summary-request <pr_number>` → Post/update summary-request comment
- `jlo workflow pr sync-category-label <pr_number>` → Sync implementer category label from branch
- `jlo workflow pr enable-automerge <pr_number>` → Enable auto-merge (policy gates in code)
- `jlo workflow pr process <pr_number>` → Run all PR event commands in order
- `jlo workflow issue label-innovator <issue_number> <persona>` → Apply innovator labels

## Workflow Execution Flow

The primary orchestration workflow in `.github/workflows/jules-*.yml` orchestrates the layers in sequence:

1. **Narrator** → Produces `.jules/changes/latest.yml`
2. **Schedule Check** → Validates schedule conditions
3. **Innovator Execution (creation phase)** → `--phase creation` (parallel with observers)
4. **Observer Execution** → Sequential execution (max-parallel=1)
5. **Innovator Execution (refinement phase)** → `--phase refinement` (after observers + creation)
6. **Proposal Publication** → Published as GitHub issues (validates perspective.yml)
7. **Decider Matrix Generation** → Identifies pending events
8. **Decider Execution** → Sequential execution (max-parallel=1)
9. **Routing Matrix Generation** → Identifies issues for planner/implementer
10. **Planner Execution** → Sequential execution for deep analysis
11. **Implementer Execution** → Sequential execution for code changes

## Required Configuration

Repository variables and secrets referenced by `.github/workflows/jules-*.yml`:

| Name | Type | Purpose | Default |
|------|------|----------|----------|
| `JULES_API_KEY` | Secret | API key for Jules service | (required) |
| `JULES_LINKED_GH_TOKEN` | Secret | GitHub token with PR comment access for summary requests | (required for summary-request workflow) |
| `JLO_PAUSED` | Variable | Set to `true` to skip scheduled runs | `false` |

Branch values (`target_branch`, `worker_branch`) are rendered at build time from `.jlo/config.toml` and baked into the workflow YAML. They are not read from repository variables at runtime.

## Workflow Timing Configuration

Workflow timing is rendered from `.jlo/config.toml` and baked into the workflow kit at install time. The authoritative keys are:

- `[workflow].runner_mode` (`remote` or `self-hosted`)
- `[workflow].cron` (array of cron strings)
- `[workflow].wait_minutes_default` (number)

Reinstalling the workflow kit overwrites the schedule and wait defaults with the values in `.jlo/config.toml`. Existing workflow YAML is never used as a configuration source.

## Mock Mode Validation

The `validate-workflow-kit.yml` workflow tests the workflow kit without Jules API:

1. **build** → Compile jlo
2. **validate-scaffold** → Test `jlo init --remote` (scaffold + workflows)
3. **mock-e2e** → Validate `jlo run <layer> --dry-run` for all layers
4. **validate-workflow-template** → Verify rendered workflow contains mock support

Mock mode (`--mock`) creates real branches/PRs with synthetic content. Mock tag is auto-generated from `JULES_MOCK_TAG` env var. The kit scripts pass `JLO_RUN_FLAGS` to jlo commands, enabling mock flags via environment variable.

Triggers:
- Pull requests modifying `src/assets/workflows/**`, `src/app/commands/run/**`, or `src/domain/mock_config.rs`
- Manual dispatch

## Repository Requirements

- The `JULES_WORKER_BRANCH` branch is created and maintained by workflow automation (bootstrap job)
- The `JLO_TARGET_BRANCH` branch contains `.jlo/` (user intent overlay) and `.github/` (workflow kit)
- Branch protection on `JULES_WORKER_BRANCH` with required status checks and auto-merge enabled
- Bot account used by workflows has write access
- Auto-review tools configured for on-demand review only

## Relationship to REPRODUCTION_GUIDE.md

[REPRODUCTION_GUIDE.md](REPRODUCTION_GUIDE.md) contains setup instructions for reproducing the Jules workflow in other repositories. This file (AGENTS.md) focuses on development knowledge for modifying workflows.
