# Taxonomy Role State

## Analysis Log
- **2026-01-30**: Full scan of codebase.
  - Confirmed architecture violation: `src/services/` contains adapters (`ArboardClipboard`, `HttpJulesClient`, `FilesystemWorkspaceStore`, `EmbeddedCatalog`, `EmbeddedRoleTemplateStore`). `src/adapters/` does not exist.
  - Confirmed naming inconsistencies:
    - Port `jules_client.rs` vs Adapter `jules_api.rs`.
    - Port `component_catalog.rs` vs Adapter `catalog.rs`.
    - Port `clipboard_writer.rs` vs Adapter `clipboard_arboard.rs`.
  - Verified consistency:
    - Layer naming in `src/assets/templates/layers/` matches `src/domain/layer.rs` (plural directories, singular types).
    - `embedded_role_template_store.rs` matches struct name.

## Current Focus
- Reporting architecture violations and naming inconsistencies.

## Known Inconsistencies
- `src/services/` contains adapters (should be in `src/adapters/`).
- `src/services/jules_api.rs` (should align with `jules_client`).
- `src/services/catalog.rs` (should align with `component_catalog`).
