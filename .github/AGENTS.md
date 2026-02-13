# GitHub Actions Workflow Context

See [root AGENTS.md](../AGENTS.md) for design principles.
See [CONTROL_PLANE_OWNERSHIP.md](../docs/CONTROL_PLANE_OWNERSHIP.md) for the `.jlo/` vs `.jules/` ownership model and projection rules.

## Branch Topology

| Branch | Contents | Owner |
|--------|----------|-------|
| `JLO_TARGET_BRANCH` | `.jlo/` (user intent overlay), `.github/` (workflow kit) | User + `jlo init` |
| `JULES_WORKER_BRANCH` | `.jules/` (runtime scaffold, materialized by bootstrap) | Workflow automation |

The `JULES_WORKER_BRANCH` branch is never edited directly by users.
Its `.jules/` directory is assembled by `jlo workflow bootstrap` from embedded scaffold assets and `.jlo/` intent overlay.

## Branch Strategy

| Agent Type | Starting Branch | Output Branch | Auto-merge |
|------------|-----------------|---------------|------------|
| Narrator | `JULES_WORKER_BRANCH` | `jules-narrator-*` | ✅ (if `.jules/` only) |
| Observer | `JULES_WORKER_BRANCH` | `jules-observer-*` | ✅ (if `.jules/` only) |
| Decider | `JULES_WORKER_BRANCH` | `jules-decider-*` | ✅ (if `.jules/` only) |
| Planner | `JULES_WORKER_BRANCH` | `jules-planner-*` | ✅ (if `.jules/` only) |
| Implementer | `JLO_TARGET_BRANCH` | `jules-implementer-*` | ❌ (human review) |
| Integrator | `JLO_TARGET_BRANCH` | `jules-integrator-*` | ❌ (human review) |
| Innovator | `JULES_WORKER_BRANCH` | `jules-innovator-*` | ✅ (if `.jules/` only) |

## Workflow Files

Jules workflow kit files are installed by `jlo init --remote` (or `--self-hosted`).
Runtime orchestration is centralized in:

- `.github/workflows/jules-scheduled-workflows.yml`
- `.github/workflows/jules-integrator.yml` (manual dispatch only)

Local composite actions are installed under `.github/actions/`.

The source of truth is `src/assets/github/`; generated files under `.github/` are installation artifacts.

## Orchestration Commands

Workflow orchestration delegates to `jlo workflow` commands:

- `jlo workflow run <layer>`
- `jlo workflow workspace inspect`
- `jlo workflow workspace publish-proposals`
- `jlo workflow workspace clean requirement <file>`
- `jlo workflow workspace clean mock --mock-tag <tag>`
- `jlo workflow gh pr comment-summary-request <pr_number>`
- `jlo workflow gh pr sync-category-label <pr_number>`
- `jlo workflow gh pr enable-automerge <pr_number>`
- `jlo workflow gh pr process <pr_number> [--mode all|metadata|automerge] [--retry-attempts N] [--retry-delay-seconds N] [--fail-on-error]`
- `jlo workflow gh issue label-innovator <issue_number> <persona>`

## Workflow Execution Flow

`jules-scheduled-workflows.yml` contains the consolidated trigger paths:

1. Schedule/dispatch/call orchestration path for layer execution
2. Implementer-branch push path for PR metadata synchronization
3. Worker-branch pull_request path for doctor validation and auto-merge enablement

Layer orchestration sequence remains narrator → schedule check → innovators/observers → decider → planner → implementer.

Integrator is a manual-only workflow (`workflow_dispatch`). It discovers all remote `jules-implementer-*` branches, retrieves their PR discussions via `gh`, and merges them into a single integration branch targeting `JLO_TARGET_BRANCH`.

## Required Configuration

Repository secrets/variables referenced by the workflow kit:

| Name | Type | Purpose | Default |
|------|------|----------|----------|
| `JULES_API_KEY` | Secret | API key for Jules service | (required) |
| `JLO_BOT_TOKEN` | Secret | GitHub PAT for checkout, push, and merge operations | (required) |
| `JULES_LINKED_GH_PAT` | Secret | GitHub token for implementer PR metadata processing | (required) |
| `JLO_PAUSED` | Variable | Set `true` to skip scheduled runs | `false` |

Integrator does not require runner-side `JULES_LINKED_GH_PAT`; `gh` runtime authentication is provided inside the Jules VM.

Branch values (`target_branch`, `worker_branch`) are rendered from `.jlo/config.toml` into workflow YAML at generation time.

## Workflow Timing Configuration

Workflow timing keys in `.jlo/config.toml`:

- `[workflow].runner_mode`
- `[workflow].cron`
- `[workflow].wait_minutes_default`

Reinstalling the workflow kit overwrites rendered schedule/wait values with `.jlo/config.toml` values.

## Repository Requirements

- `JULES_WORKER_BRANCH` is created and maintained by workflow automation.
- `JLO_TARGET_BRANCH` contains `.jlo/` and `.github/`.
- Branch protection on `JULES_WORKER_BRANCH` requires status checks and allows auto-merge.
- Workflow bot identities have required repository permissions.

## Relationship to REPRODUCTION_GUIDE.md

[REPRODUCTION_GUIDE.md](../docs/REPRODUCTION_GUIDE.md) provides reproduction setup guidance.
This file focuses on workflow development context.
