# Taxonomy Role State

## Analysis Log
- **2026-01-30**: Initial full scan of codebase.
  - Analyzed `src/` structure.
  - Checked naming conventions for Layers, Services, and Domain models.
  - Found inconsistencies in layer naming (plural vs singular).
  - Found structural mixing of Services and Adapters.
  - Found filename/struct naming mismatches.
- **2026-01-30**: Detailed scan and verification.
  - Confirmed `src/assets/templates/layers/` uses **plural** names (contradicting previous note).
  - Confirmed `src/services/` mixes Domain Services and Adapters.
  - Confirmed `src/ports/` and `src/services/` file naming mismatch.
  - Identified `workstream` command inconsistency in `src/lib.rs`.

## Current Focus
- Resolving naming inconsistencies and separating adapters from services.

## Known Inconsistencies
- `src/services/` contains both Domain Services (`resolver.rs`, `generator.rs`) and Adapters (`clipboard_arboard.rs`, `jules_api.rs`).
- `src/ports/` filenames do not match `src/services/` adapter filenames (e.g., `jules_client` vs `jules_api`).
- `workstream` command implemented in `src/lib.rs`, others in `src/app/commands/`.
- `src/services/role_template_service.rs` does not exist; likely `embedded_role_template_store.rs`.
