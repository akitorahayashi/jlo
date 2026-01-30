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
