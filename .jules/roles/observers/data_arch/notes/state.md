# Data Architecture State

## Project Structure Overview
The project follows a Hexagonal Architecture (Ports and Adapters) pattern, with `domain`, `ports`, `services`, and `app` modules.

- **Domain (`src/domain/`)**: Contains core business logic and types.
- **Ports (`src/ports/`)**: Defines interfaces for external dependencies (e.g., `ComponentCatalog`).
- **Services (`src/services/`)**: Implements application logic and orchestrates domain objects.

## Data Patterns Observed

### Strong Typing vs. Primitive Obsession
- **Positive**: `RoleId` (`src/domain/role_id.rs`) is a strong Value Object that enforces invariants on creation.
- **Positive**: `Component` (`src/domain/component.rs`) uses `ComponentId` for identification and dependencies, ensuring type safety.
- **Negative**: Error handling relies heavily on `AppError::ConfigError(String)`, leading to "stringly typed" logic in retry mechanisms (`HttpJulesClient`).

### Boundary Sovereignty
- **Positive**: `RunConfig` (`src/domain/run_config.rs`) uses a private `dto` module to handle serialization, keeping the domain type clean.
- **Negative**: Service layer (`EmbeddedComponentCatalog`) imports DTOs (`ComponentMeta`) directly from the App layer (`src/app/config.rs`), violating architectural boundaries.

### Data Efficiency
- **Negative**: The `Resolver` service (`src/services/dependency_resolver.rs`) performs inefficient cloning of heavy objects during dependency resolution.

### Cohesion
- **Negative**: `ScaffoldManifest` (`src/services/scaffold_manifest.rs`) mixes domain logic with low-level hashing and file path business rules.

## Active Observations
1. **Inefficient Dependency Resolution**: `Resolver` clones `Component` structs (Reported as Issue).
2. **Service-App Layer Violation**: Services import App DTOs.
3. **Low Cohesion in ScaffoldManifest**: Mixing hashing/paths with domain.
4. **Stringly Typed Error Handling**: `AppError` and retry logic rely on strings.

## Future Focus
- Monitor for further leakage of DTOs into the domain.
- Identify opportunities to introduce Value Objects for other primitives (e.g., file paths, checksums).
- Evaluate data flow in `Generator` service.
