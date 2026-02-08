# Workflow Kit Template Pipeline

## Scope
This document describes how files under `src/assets/workflows/.github/` are transformed into the installed workflow kit under `.github/`.

## Authoritative Sources
- Source assets (templates and static files): `src/assets/workflows/.github/**`
- Workflow kit loader: `src/adapters/assets/workflow_kit_assets/mod.rs`
- Asset collection: `src/adapters/assets/workflow_kit_assets/asset_collect.rs`
- Template engine (MiniJinja): `src/adapters/assets/workflow_kit_assets/template_engine.rs`
- Render plan (partials exclusion): `src/adapters/assets/workflow_kit_assets/render_plan.rs`
- Install/write to disk: `src/app/commands/init_workflows.rs`

## Transformation Rules
- Every file under `src/assets/workflows/.github/` is loaded by `include_dir!` in `WorkflowKitAssets`.
- Files with `.j2` are treated as templates; all other files are copied verbatim.
- Template output paths strip the `.j2` suffix.
- Output paths are prefixed with `.github/`.
- Templates under `workflows/**/components/` or `workflows/**/macros/` are partials and are not emitted as files.

## Template Rendering
- `build_template_environment` registers every `.j2` template by its relative path.
- `gha_expr` and `gha_raw` functions emit GitHub Actions expressions (e.g., `${{ ... }}`).
- Rendering context contains:
  - `runner`: `ubuntu-latest` for `WorkflowRunnerMode::Remote`, `self-hosted` for `WorkflowRunnerMode::SelfHosted`.

## Installed Output Examples
- `src/assets/workflows/.github/workflows/jules-workflows.yml.j2`
  → `.github/workflows/jules-workflows.yml`
- `src/assets/workflows/.github/workflows/jules-sync.yml.j2`
  → `.github/workflows/jules-sync.yml`
- `src/assets/workflows/.github/actions/install-jlo/action.yml`
  → `.github/actions/install-jlo/action.yml`

## Installation Notes
- `execute_workflows` writes the rendered kit to disk and preserves the `on.schedule` block (and `wait_minutes` default) in `.github/workflows/jules-workflows.yml` when overwriting.
- Action directories are detected from rendered paths by `collect_action_dirs` and are cleaned before re-installation.
