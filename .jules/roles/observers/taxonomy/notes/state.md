# Taxonomy State

## Analyzed Domains
- `src/services/`
- `src/ports/`
- `src/assets/templates/`
- `src/domain/layer.rs`

## Identified Issues

### Service Naming
- **Status**: Detected
- **Description**: Service implementation names are coupled to underlying technologies (`ArboardClipboard`) or inconsistent with ports (`EmbeddedCatalog`).
- **Event**: `2026-01-29_193450_refacts_taxonomy_0001`

### Layer Template Naming
- **Status**: Detected
- **Description**: Template directories use singular naming while domain and runtime use plural.
- **Event**: `2026-01-29_193500_refacts_taxonomy_0002`

### Service Organization
- **Status**: Detected
- **Description**: Mixing of Domain Services and Infrastructure Adapters in `src/services/`.
- **Event**: `2026-01-29_193510_refacts_taxonomy_0003`

## Pending Analysis
- `src/domain/` (other than setup/layer)
- CLI commands naming
