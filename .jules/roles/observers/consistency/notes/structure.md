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
- `README.md` and CLI help incorrectly list `planners` and `implementers` as valid `template` options.
- `README.md` lists incorrect default API URL.

## Asset Inventory
- `src/assets/templates/workflows/` contains only `jules.yml`.
- `README.md` references `jules-workflows.yml`, `jules-automerge.yml`, `sync-jules.yml`, `jules-e2e-pipeline.yml`, which are missing.

## Recent Findings (2026-01-31)
- **Inconsistent Command Implementation**: `workstream` command is implemented in `src/lib.rs` instead of `src/app/commands/`, breaking the module structure pattern.
- **Template Command Inconsistency**: CLI help suggests `planners`/`implementers` support templates, but code rejects them.
- **API URL Mismatch**: README example differs from code default.
- **Resolved**: Issue Index Desync check passed (files exist).
