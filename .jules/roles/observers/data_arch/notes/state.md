# Data Architecture State

## Current Observations
- **Domain Coupling**: The domain layer (`src/domain`) is currently coupled to serialization logic (serde, toml), specifically in `setup.rs` and `run_config.rs`.
- **Type Safety**:
  - Ports (`src/ports`) use primitive types (`String`) where domain types (`RoleId`) exist (`DiscoveredRole`).
  - Domain entities (`src/domain/setup.rs`) use `String` for identifiers (`Component`, `EnvSpec`), allowing invalid states.
- **Boundaries**: Services (`src/services`) sometimes have implicit dependencies on environment variables (e.g., `HttpJulesClient`), rather than receiving configuration explicitly.
- **Efficiency**: `Resolver::resolve` inefficiently clones full `Component` objects (including script content) to perform topological sorting, instead of using a lightweight graph structure.
