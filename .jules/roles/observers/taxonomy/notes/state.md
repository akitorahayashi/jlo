# Taxonomy Role State

## Analysis Log
- **2026-01-30**: Initial full scan of codebase.
  - Analyzed `src/` structure.
  - Checked naming conventions for Layers, Services, and Domain models.
  - Found inconsistencies in layer naming (plural vs singular).
  - Found structural mixing of Services and Adapters.
  - Found filename/struct naming mismatches.

## Current Focus
- Resolving naming inconsistencies and separating adapters from services.

## Known Inconsistencies
- `src/assets/templates/layers/` uses singular names.
- `src/services/` contains adapters.
- `src/services/role_template_service.rs` naming.
