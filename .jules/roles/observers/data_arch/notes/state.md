# Data Architecture State

## Project Structure Overview
The project follows a Hexagonal Architecture (Ports and Adapters) pattern, with `domain`, `ports`, `services`, and `app` modules.

- **Domain (`src/domain/`)**: Contains core business logic and types.
- **Ports (`src/ports/`)**: Defines interfaces for external dependencies (e.g., `ComponentCatalog`).
- **Services (`src/services/`)**: Implements application logic and orchestrates domain objects.

## Data Patterns Observed

### Strong Typing vs. Primitive Obsession
- **Positive**: `RoleId` (`src/domain/role_id.rs`) is a strong Value Object that enforces invariants on creation.
- **Negative**: `Component` (`src/domain/setup.rs`) relies on `String` for names and dependencies, lacking type safety and centralization of validation.

### Boundary Sovereignty
- **Positive**: `RunConfig` (`src/domain/run_config.rs`) uses a private `dto` module to handle serialization, keeping the domain type clean.
- **Negative**: `Setup` (`src/domain/setup.rs`) mixes domain entities (`Component`) with DTOs (`ComponentMeta`) and configuration models (`SetupConfig`), leaking persistence details into the domain.

### Data Efficiency
- The `Resolver` service (`src/services/resolver.rs`) performs inefficient cloning of heavy objects during dependency resolution, rather than operating on lightweight references or IDs.

## Active Observations
1. **Inefficient Dependency Resolution**: `Resolver` clones `Component` structs.
2. **Primitive Obsession in Component**: `Component` uses `String` instead of `ComponentId`.
3. **Leaky Domain Models in Setup**: `ComponentMeta` and `SetupConfig` pollute the domain layer.

## Future Focus
- Monitor for further leakage of DTOs into the domain.
- Identify opportunities to introduce Value Objects for other primitives (e.g., file paths, checksums).
- Evaluate data flow in `Generator` service.
