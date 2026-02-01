# Taxonomy State

## Naming Patterns

### Service Layer (`src/services/`)
- **Structs**: Consistently use **Prefix** strategy (e.g., `EmbeddedRoleTemplateStore`, `EmbeddedComponentCatalog`, `HttpJulesClient`).
- **Filenames**: Inconsistent mix of strategies.
  - `embedded_role_template_store.rs` (Prefix match)
  - `component_catalog_embedded.rs` (Suffix match)
  - `jules_client_http.rs` (Suffix match)
- **Generic Names**: `Resolver` and `Generator` are too broad for their specific domain functions.
- **Asset/Template Fragmentation**: Logic for accessing static assets is split between `RoleTemplateStore` (Port), `scaffold_assets.rs` (Service functions), and `workstream_template_assets.rs` (Service functions).

### Domain Layer (`src/domain/`)
- `SetupConfig` vs `install.sh` artifact.
- `SetupConfig` struct vs `tools.yml` file.
- `src/domain/setup.rs` contains `Component`, `EnvSpec`, `ComponentMeta`, which are distinct entities from the setup *process*.

### CLI
- "Scaffold" vs "Template": Reasonably consistent distinction (Base vs Additive) in CLI, but internal implementation is muddled.
- "Setup" command -> `install.sh` generation.
- "Workstream" command: Only `inspect` is available.

### Documentation
- `AGENTS.md` lists `arboard` (Clipboard) but it is not a dependency.
- `AGENTS.md` is missing `reqwest`, `serde_json`, `url` in Tech Stack.
- `README.md` is missing `--adopt-managed` flag for `jlo update`.

## Identified Issues
- `tx0001`: Service filename inconsistency.
- `tx0002`: Vague "Managed Defaults" terminology.
- `tx0003`: Setup vs Install ambiguity.
- `vague-service-names` (x9k2m4): Generic service names `Resolver` and `Generator`.
- `fav001`: Fragmented Asset and Template Vocabulary (`RoleTemplateStore` vs `scaffold_assets`).
- `mcd001`: Misplaced Component Domain Entities (in `setup.rs`).
- `cnm001`: Config Name Mismatch (`SetupConfig` vs `tools.yml`).
- `ghost-dependency-reference`: `arboard` in AGENTS.md.
- `undocumented-dependencies`: Missing deps in AGENTS.md.
- `undocumented-cli-flag-adopt-managed`: Missing `--adopt-managed` in README.

## Recommendations
- Standardize service filenames to either match the struct name (Prefix) or use a strict `[interface]_[impl]` (Suffix) pattern.
- Rename `ManagedDefaultsManifest` to `ScaffoldManifest` to reflect its role as a state file.
- Rename `Resolver` to `ComponentResolver` or `DependencyResolver`.
- Rename `Generator` to `SetupGenerator` or `InstallScriptGenerator`.
- Unify asset/template access under a single `AssetStore` or `TemplateStore` port, replacing disjointed service functions.
- Move `Component` and related structs to `src/domain/component.rs` or `src/domain/catalog.rs`.
- Align `SetupConfig` and `tools.yml` (e.g., rename struct to `ToolsConfig` or file to `setup.yml`).
- Update `AGENTS.md` to reflect actual dependencies.
- Update `README.md` to document `--adopt-managed` flag.
