# Consistency Observer State

## Structural Understanding

### Scaffolding vs Templates
- `src/assets/scaffold/.jules/roles/` contains the source of truth for all roles (including Planners and Implementers).
- `src/assets/templates/layers/` only contains templates for multi-role layers (Observers, Deciders).
- Planners and Implementers are single-role layers and rely on the fixed scaffold files; they do not support template-based creation of new roles.

### Removed Functionality
- Clipboard functionality (`arboard`) has been completely removed from the codebase, despite references in `AGENTS.md` and `README.md`.

### Naming Conventions
- Service implementations in `src/services/` have drifted from `AGENTS.md` documentation (e.g., `role_template_service` -> `embedded_role_template_store.rs`).

### Documentation Completeness
- `AGENTS.md` Project Structure section is significantly incomplete, missing entire directories (`src/app/commands`, `src/domain`) and newer modules.
- `README.md` Command Reference is missing newer flags (e.g., `--adopt-managed` for `update`).

## Active Observations
- `doc-api-url-mismatch` (q1mwwx): README vs Code API URL.
- `cli-template-inconsistency` (odyrzk): CLI Help vs Implementation for `template` command.
- `agents-md-structure-inconsistency` (djpmsr): `AGENTS.md` and `README.md` vs Filesystem structure.
- `incomplete-structure-docs` (inc0mp): `AGENTS.md` missing modules.
- `undocumented-cli-flags` (und0c1): `README.md` missing flags.
