# GitHub Actions Workflow Context

See [root AGENTS.md](../AGENTS.md) for design principles.

## Branch Strategy

| Agent Type | Starting Branch | Output Branch | Auto-merge |
|------------|-----------------|---------------|------------|
| Narrator | `jules` | `jules-narrator-*` | ✅ (if `.jules/` only) |
| Observer | `jules` | `jules-observer-*` | ✅ (if `.jules/` only) |
| Decider | `jules` | `jules-decider-*` | ✅ (if `.jules/` only) |
| Planner | `jules` | `jules-planner-*` | ✅ (if `.jules/` only) |
| Implementer | `main` | `jules-implementer-*` | ❌ (human review) |

## Workflow Files

| File | Purpose |
|------|---------|
| `jules-workflows.yml` | Main orchestration (scheduled execution, matrix-based sequential execution) |
| `jules-run-planner.yml` | Manual Planner execution (workflow dispatch) |
| `jules-run-implementer.yml` | Manual Implementer execution (workflow dispatch) |
| `jules-automerge.yml` | Auto-merge Observer/Decider/Planner PRs |
| `jules-implementer-review.yml` | Auto-post review comments on Implementer PRs |
| `jules-sync.yml` | Sync main branch to jules branch |
| `build.yml` | Build verification |
| `run-tests.yml` | Test execution |
| `run-linters.yml` | Linting |
| `verify-installers.yml` | Setup component verification |
| `ci-workflows.yml` | CI orchestration |
| `release.yml` | Release automation |

## Composite Actions

| Directory | Purpose |
|-----------|---------|
| `install-jlo/` | Install jlo CLI |
| `configure-git/` | Configure git user |
| `run-implementer/` | Run implementer and cleanup issue/events |

## Scripts

| File | Purpose |
|------|---------|
| `jules-generate-workstream-matrix.sh` | Generate workstream matrix output |
| `jules-generate-decider-matrix.sh` | Generate decider matrix output |
| `jules-generate-routing-matrices.sh` | Generate planner/implementer matrix outputs |
| `jules-delete-processed-issue-and-events.sh` | Delete processed issue and source events |

## Workflow Execution Flow

`jules-workflows.yml` orchestrates the layers in sequence:

1. **Narrator** → Produces `.jules/changes/latest.yml`
2. **Doctor Validation** → Validates workspace structure
3. **Observer Matrix Generation** → Reads workstream schedules
4. **Observer Execution** → Sequential execution (max-parallel=1)
5. **Decider Matrix Generation** → Reads workstream schedules
6. **Decider Execution** → Sequential execution (max-parallel=1)
7. **Routing Matrix Generation** → Identifies issues for planner/implementer
8. **Planner Execution** → Sequential execution for deep analysis
9. **Implementer Execution** → Sequential execution for code changes

## Required Configuration

| Setting | Type | Purpose |
|---------|------|---------|
| `vars.JLO_VERSION` | Variable | jlo CLI version (defaults to `latest`) |
| `secrets.JULES_API_KEY` | Secret | Agent execution authentication |
| `secrets.JLO_BOT_TOKEN` | Secret | Auto-merge and write operations |
| `vars.JULES_PAUSED` | Variable | Pause scheduled runs |

## Repository Requirements

- The `jules` branch exists and contains the `.jules/` scaffold
- Branch protection on `jules` with required status checks and auto-merge enabled
- Bot account associated with `JLO_BOT_TOKEN` has write access
- Auto-review tools configured for on-demand review only

## Relationship to REPRODUCTION_GUIDE.md

[REPRODUCTION_GUIDE.md](REPRODUCTION_GUIDE.md) contains setup instructions for reproducing the Jules workflow in other repositories. This file (AGENTS.md) focuses on development knowledge for modifying workflows.
