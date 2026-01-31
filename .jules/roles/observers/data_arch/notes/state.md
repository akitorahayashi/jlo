# Data Architecture State

## Current Observations
- **Domain Coupling**: The domain layer (`src/domain`) is currently coupled to serialization logic (serde, toml), specifically in `setup.rs` and `run_config.rs`.
- **Type Safety**: Ports (`src/ports`) sometimes use primitive types (`String`) where domain types (`RoleId`) exist, as seen in `DiscoveredRole`.
- **Boundaries**: Services (`src/services`) sometimes have implicit dependencies on environment variables (e.g., `HttpJulesClient`), rather than receiving configuration explicitly.
