# Consistency Observer State

## Structural Understanding

### Scaffolding vs Templates
- `src/assets/scaffold/.jules/roles/` contains the source of truth for all roles (including Planners and Implementers).
- `src/assets/templates/layers/` only contains templates for multi-role layers (Observers, Deciders).
- Planners and Implementers are single-role layers and rely on the fixed scaffold files; they do not support template-based creation of new roles.

### Removed Functionality
- Clipboard functionality (`arboard`) has been completely removed from the codebase, despite references in `AGENTS.md`.

### Naming Conventions
- Service implementations in `src/services/` have drifted from `AGENTS.md` documentation (e.g., `role_template_service` -> `embedded_role_template_store.rs`).

## Active Observations
- `doc-api-url-mismatch` (x7k2m9): README vs Code API URL.
- `outdated-tech-stack` (b4n1p8): AGENTS.md vs Cargo.toml (arboard, deps).
- `undocumented-update-flag` (q9l3v2): `--adopt-managed` missing from README.
- `incomplete-command-docs` (m5j8h4): AGENTS.md missing commands.
- `scaffold-template-ambiguity` (z2c6x1): AGENTS.md confusing scaffold/templates.
- `cli-template-inconsistency` (w8n4k7): CLI Help vs Implementation for `template` command.
