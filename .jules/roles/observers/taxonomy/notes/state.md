# Taxonomy State

## Identified Patterns

### Service Naming
- Services in `src/services/` generally follow `Type` = `Filename` (snake_case) but with inconsistent ordering (Prefix vs Suffix).
  - Example: `jules_client_http.rs` vs `HttpJulesClient` (Confirmed: struct uses Prefix, file uses Suffix).
- Structural inconsistency: Some services are domain structs (`DependencyResolver`), others are modules of pure functions (`scaffold_assets.rs`).
- `Resolver` and `Generator` are overly generic (referring to potential base names, actual names are `DependencyResolver` and `ArtifactGenerator`).

### Vocabulary Collisions
- `EmbeddedRoleTemplateStore` serves `scaffold` assets, violating the separation between "Scaffold" (immutable) and "Template" (blueprints).
- `scaffold_assets.rs` and `EmbeddedRoleTemplateStore` both access `src/assets/scaffold`, creating ambiguity about which service owns the scaffold domain.

### CLI vs Domain
- `setup` command maps to `install` behavior (generates `install.sh`).
- `scaffold` vs `template` terminology is overloaded in both CLI (`jlo template`, `jlo update`) and internal services.

### Architecture
- `src/main.rs` is a thin wrapper (Fixed).

## Vocabulary Map
- **Scaffold**: The immutable `.jules/` directory structure and reference assets.
- **Template**: Blueprints for creating new roles/workstreams.
- **Component**: An installable tool managed by `jlo setup`.
