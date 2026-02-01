# Taxonomy State

## Identified Patterns

### Service Naming
- Services in `src/services/` generally follow `Type` = `Filename` (snake_case) but with inconsistent ordering (Prefix vs Suffix).
  - Example: `jules_client_http.rs` vs `HttpJulesClient`.
- Structural inconsistency: Some services are domain structs (`DependencyResolver`), others are modules of functions (`scaffold_assets.rs`).
- `Resolver` and `Generator` are overly generic.

### Vocabulary Collisions
- `EmbeddedRoleTemplateStore` serves `scaffold` assets, violating the separation between "Scaffold" (immutable) and "Template" (blueprints).

### CLI vs Domain
- `setup` command maps to `install` behavior.
- `scaffold` vs `template` terminology is overloaded in both CLI (`jlo template`, `jlo update`) and internal services.

### Architecture
- `src/main.rs` is heavy, containing CLI definitions.

## Vocabulary Map
- **Scaffold**: The immutable `.jules/` directory structure and reference assets.
- **Template**: Blueprints for creating new roles/workstreams.
- **Component**: An installable tool managed by `jlo setup`.
