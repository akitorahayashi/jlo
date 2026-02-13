# This file is auto-generated. Do not edit manually.
# Workflow Scaffold Template Pipeline

## Scope
This document describes how files under `src/assets/github/` are transformed into the installed workflow scaffold under `.github/`.

## Authoritative Sources
- Source assets (templates and static files): `src/assets/github/**`
- Workflow scaffold loader: `src/adapters/assets/workflow_scaffold_assets/mod.rs`
- Asset collection: `src/adapters/assets/workflow_scaffold_assets/asset_collect.rs`
- Template engine (MiniJinja): `src/adapters/assets/workflow_scaffold_assets/template_engine.rs`
- Render plan (partials exclusion): `src/adapters/assets/workflow_scaffold_assets/render_plan.rs`
- Install/write to disk: `src/app/commands/init.rs`

## Development Workflow

**DO NOT EDIT files in `.github/` directly.** They are auto-generated artifacts.

1. Edit the source templates in `src/assets/github/`.
2. Regenerate the workflow files:
   ```bash
   # For hosted runners
   jlo workflow generate remote

   # For self-hosted runners
   jlo workflow generate self-hosted
   ```

## Transformation Rules
- Every file under `src/assets/github/` is loaded by `include_dir!` in `WorkflowScaffoldAssets`.
- Files with `.j2` are treated as templates; all other files are copied verbatim.
- Template output paths strip the `.j2` suffix.
- Output paths are prefixed with `.github/`.
- Templates under `workflows/**/components/` or `workflows/**/macros/` are partials and are not emitted as files.

## Template Rendering
- `build_template_environment` registers every `.j2` template by its relative path.
- `gha_expr` and `gha_raw` functions emit GitHub Actions expressions (e.g., `${{ ... }}`).
- Rendering context contains:
  - `runner`: `ubuntu-latest` when `runner_mode` is `remote`; otherwise the config value is used directly as the `runs-on` label (e.g. `self-hosted`, `my-mac-mini`, `[self-hosted, macOS, arm64]`).
  - `target_branch`: rendered from `.jlo/config.toml` (`run.jlo_target_branch`).
  - `worker_branch`: rendered from `.jlo/config.toml` (`run.jules_worker_branch`).
  - `workflow_schedule_crons`: cron list from `.jlo/config.toml` (`workflow.cron`).
  - `workflow_wait_minutes_default`: wait default from `.jlo/config.toml` (`workflow.wait_minutes_default`).

## Installed Output Examples
- `src/assets/github/workflows/jules-scheduled-workflows.yml.j2`
  → `.github/workflows/jules-scheduled-workflows.yml`
- `src/assets/github/workflows/jules-run-only-innovators.yml.j2`
  → `.github/workflows/jules-run-only-innovators.yml`
- `src/assets/github/workflows/jules-automerge.yml.j2`
  → `.github/workflows/jules-automerge.yml` (push-scoped trigger for Jules auto-merge branch families)
- `src/assets/github/actions/install-jlo/action.yml`
  → `.github/actions/install-jlo/action.yml`

## Installation Notes
- `install_workflow_scaffold` writes the rendered scaffold to disk, overwriting jlo-managed outputs deterministically.
- Action directories are detected from rendered paths by `collect_action_dirs` and are cleaned before re-installation.
