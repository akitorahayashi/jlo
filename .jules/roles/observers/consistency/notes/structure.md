# Project Structure Analysis

## CLI Structure
Based on `src/main.rs`, the CLI supports the following commands:
- `init` (i): Create workspace.
- `update` (u): Update workspace.
- `template` (tp): Create role from template.
- `workstream` (w): Manage workstreams (`new`, `list`).
- `setup` (s): Manage components (`gen`, `list`).
- `run` (r): Execute agents (`observers`, `deciders`, `planners`, `implementers`).

## Documentation Status
- `README.md` covers `init`, `update`, `template`, `run`, `setup`.
- `README.md` **misses** `workstream`.

## Asset Inventory
- `src/assets/templates/workflows/` contains only `jules.yml`.
- `README.md` references `jules-workflows.yml`, `jules-automerge.yml`, `sync-jules.yml`, `jules-e2e-pipeline.yml`, which are missing.

## Recent Findings (2026-01-30)
- **Issue Index Desync**: `issues/index.md` lists files that do not exist in `high/`, `medium/`, or `low/` directories.
- **Inconsistent Command Implementation**: `workstream` command is implemented in `src/lib.rs` instead of `src/app/commands/`, breaking the module structure pattern.
