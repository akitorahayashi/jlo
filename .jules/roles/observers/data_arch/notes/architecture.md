# Data Architecture Analysis

## Domain Models

### Setup Domain
- **Component**: Represents an installable tool. Contains metadata (name, summary) and behavior (script content, dependencies).
  - *Pattern*: Rich Domain Model (contains data and behavior definition).
  - *Source*: Loaded from `meta.toml` and `install.sh` via `EmbeddedCatalog`.
- **EnvSpec**: Value object defining environment variable requirements. Shared between `Component` and `ComponentMeta`.
- **ComponentMeta**: Data Transfer Object (DTO) for deserializing `meta.toml`. Maps 1:1 to `Component` fields except for `script_content`.

### Workspace Domain
- **Layer**: Enum representing the three architectural layers (Observers, Deciders, Planners).
  - *Characteristics*: Hardcoded behavior, strictly typed.
- **RoleId**: Value object with validation rules for role identifiers.
  - *Validation*: Alphanumeric + dashes/underscores, no path traversal.
- **DiscoveredRole**: Lightweight reference containing `Layer` and `RoleId`.

## Data Flow

### Setup Workflow
1.  **Catalog Loading**: `EmbeddedCatalog` reads assets -> `ComponentMeta` (DTO) -> `Component` (Domain).
2.  **Resolution**: `Resolver` takes `Vec<String>` (names) -> uses `ComponentCatalog` -> returns `Vec<Component>` (Topologically Sorted).
3.  **Generation**: `Generator` takes `Vec<Component>` -> produces `String` (Bash script) and `String` (TOML config).

### Workspace Workflow
1.  **Discovery**: `FilesystemWorkspaceStore` scans directories -> validates paths -> produces `Vec<DiscoveredRole>`.

## Architectural Patterns

- **Hexagonal Architecture**:
    - `src/domain`: Core entities (`Component`, `Layer`).
    - `src/ports`: Interfaces (`ComponentCatalog`, `WorkspaceStore`).
    - `src/services`: Implementations (`EmbeddedCatalog`, `FilesystemWorkspaceStore`).
- **Separation of Concerns**:
    - `Resolver` handles logic (ordering).
    - `Generator` handles output formatting.
    - `Catalog` handles data access.

## Observations

- **Efficiency**: Component loading is eager (all at once), which is efficient for small catalogs.
- **Consistency**: `EnvSpec` is reused, preventing definition drift.
- **Coupling**: `Generator` is coupled to `Component` structure, which is expected.
