# Taxonomy State

## Naming Patterns

### Service Layer (`src/services/`)
- **Structs**: Consistently use **Prefix** strategy (e.g., `EmbeddedRoleTemplateStore`, `EmbeddedComponentCatalog`, `HttpJulesClient`).
- **Filenames**: Inconsistent mix of strategies.
  - `embedded_role_template_store.rs` (Prefix match)
  - `component_catalog_embedded.rs` (Suffix match)
  - `jules_client_http.rs` (Suffix match)
- **Generic Names**: `Resolver` and `Generator` are too broad for their specific domain functions.

### Domain Layer (`src/domain/`)
- Generally consistent.
- `SetupConfig` vs `install.sh` artifact.

### CLI
- "Scaffold" vs "Template": Reasonably consistent distinction (Base vs Additive).
- "Setup" command -> `install.sh` generation.
- "Workstream" command: Only `inspect` is available, limiting "workstream management" vocabulary.

## Identified Issues
- `tx0001`: Service filename inconsistency.
- `tx0002`: Vague "Managed Defaults" terminology.
- `tx0003`: Setup vs Install ambiguity.
- `vague-service-names` (x9k2m4): Generic service names `Resolver` and `Generator`.

## Recommendations
- Standardize service filenames to either match the struct name (Prefix) or use a strict `[interface]_[impl]` (Suffix) pattern. Given `embedded_role_template_store.rs` is the outlier in filenames (prefix) but matches the struct name (prefix), while others use suffix filenames but prefix structs, there is a mismatch.
- Rename `ManagedDefaultsManifest` to `ScaffoldManifest` to reflect its role as a state file.
- Rename `Resolver` to `ComponentResolver` or `DependencyResolver`.
- Rename `Generator` to `SetupGenerator` or `InstallScriptGenerator`.
