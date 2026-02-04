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

Jules workflows are installed via `jlo init workflows` and follow these patterns:

- `.github/workflows/jules-*.yml`
- `.github/actions/` (Jules composite actions)
- `.github/scripts/jules-*.sh`

Non-Jules CI workflows remain in `.github/workflows/` alongside the kit.

## Composite Actions

Jules composite actions live under `.github/actions/` and are installed with the workflow kit.

## Scripts

Jules automation scripts live under `.github/scripts/` and are installed with the workflow kit.

## Workflow Execution Flow

The primary orchestration workflow in `.github/workflows/jules-*.yml` orchestrates the layers in sequence:

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

Repository variables and secrets referenced by `.github/workflows/jules-*.yml`:

| Name | Type | Purpose |
|------|------|---------|
| `JULES_API_KEY` | Secret | API key for Jules service |
| `JULES_PAUSED` | Variable | Set to `true` to skip scheduled runs |
| `JULES_TARGET_BRANCH` | Variable | Target branch for implementer output (default: `main`) |

## Schedule Preservation

When reinstalling the workflow kit with `jlo init workflows --overwrite`, the existing `on.schedule` block in `jules-workflows.yml` is preserved. If the existing file contains invalid YAML, installation fails with an explicit error.

## Repository Requirements

- The `jules` branch exists and contains the `.jules/` scaffold
- Branch protection on `jules` with required status checks and auto-merge enabled
- Bot account used by workflows has write access
- Auto-review tools configured for on-demand review only

## Relationship to REPRODUCTION_GUIDE.md

[REPRODUCTION_GUIDE.md](REPRODUCTION_GUIDE.md) contains setup instructions for reproducing the Jules workflow in other repositories. This file (AGENTS.md) focuses on development knowledge for modifying workflows.
