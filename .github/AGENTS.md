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
| Innovator | `jules` | `jules-innovator-*` | ✅ (if `.jules/` only) |

## Workflow Files

Jules workflows are installed via `jlo init workflows` and follow these patterns:

- `.github/workflows/jules-*.yml`
- `.github/actions/` (Jules composite actions)

Non-Jules CI workflows remain in `.github/workflows/` alongside the kit.

The workflow kit is generated from `src/assets/workflows/.github/`. Edit that source directory, not `.github/`, and re-run `jlo init workflows` to apply changes.

## Composite Actions

Jules composite actions live under `.github/actions/` and are installed with the workflow kit.

## Orchestration Commands

Workflow orchestration uses `jlo workflow` commands:

- `jlo workflow matrix workstreams` → Generate workstream matrix
- `jlo workflow matrix roles --layer <layer>` → Generate role matrix
- `jlo workflow matrix pending-workstreams` → Generate decider matrix
- `jlo workflow matrix routing` → Generate planner/implementer routing
- `jlo workflow run <layer>` → Execute layer with JSON output
- `jlo workflow workstreams publish-proposals <workstream>` → Publish innovator proposals

## Workflow Execution Flow

The primary orchestration workflow in `.github/workflows/jules-*.yml` orchestrates the layers in sequence:

1. **Narrator** → Produces `.jules/changes/latest.yml`
2. **Doctor Validation** → Validates workspace structure
3. **Workstream Matrix Generation** → Reads workstream schedules
4. **Innovator Execution (first pass)** → Idea creation (parallel with observers)
5. **Observer Execution** → Sequential execution (max-parallel=1)
6. **Innovator Execution (second pass)** → Proposal refinement and cleanup
7. **Proposal Publication** → Published as GitHub issues
8. **Decider Matrix Generation** → Reads workstream schedules
9. **Decider Execution** → Sequential execution (max-parallel=1)
10. **Routing Matrix Generation** → Identifies issues for planner/implementer
11. **Planner Execution** → Sequential execution for deep analysis
12. **Implementer Execution** → Sequential execution for code changes

## Required Configuration

Repository variables and secrets referenced by `.github/workflows/jules-*.yml`:

| Name | Type | Purpose |
|------|------|---------|
| `JULES_API_KEY` | Secret | API key for Jules service |
| `JULES_PAUSED` | Variable | Set to `true` to skip scheduled runs |
| `JULES_TARGET_BRANCH` | Variable | Target branch for implementer output (default: `main`) |

## Schedule Preservation

When reinstalling the workflow kit with `jlo init workflows --overwrite`, the existing `on.schedule` block in `jules-workflows.yml` is preserved. If the existing file contains invalid YAML, installation fails with an explicit error.

## Mock Mode Validation

The `validate-workflow-kit.yml` workflow tests the workflow kit without Jules API:

1. **build** → Compile jlo
2. **validate-scaffold** → Test `jlo init scaffold` and `jlo init workflows`
3. **mock-e2e** → Validate `jlo run <layer> --dry-run` for all layers
4. **validate-workflow-template** → Verify rendered workflow contains mock support

Mock mode (`--mock`) creates real branches/PRs with synthetic content. Mock tag is auto-generated from `JULES_MOCK_TAG` env var. The kit scripts pass `JLO_RUN_FLAGS` to jlo commands, enabling mock flags via environment variable.

Triggers:
- Pull requests modifying `src/assets/workflows/**`, `src/app/commands/run/**`, or `src/domain/mock_config.rs`
- Manual dispatch

## Repository Requirements

- The `jules` branch exists and contains the `.jules/` scaffold
- Branch protection on `jules` with required status checks and auto-merge enabled
- Bot account used by workflows has write access
- Auto-review tools configured for on-demand review only

## Relationship to REPRODUCTION_GUIDE.md

[REPRODUCTION_GUIDE.md](REPRODUCTION_GUIDE.md) contains setup instructions for reproducing the Jules workflow in other repositories. This file (AGENTS.md) focuses on development knowledge for modifying workflows.
