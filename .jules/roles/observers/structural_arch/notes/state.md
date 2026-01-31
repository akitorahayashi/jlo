# Structural Architecture State

## Current Analysis
- **Domain Layer**: Compromised purity. `RunConfig` and `Setup` entities depend on `toml` and `serde`.
- **Services Layer**: Low cohesion. Mixes Domain Services (`Resolver`) with Infrastructure Adapters (`HttpJulesClient`, `FilesystemWorkspaceStore`).
- **Adapters Layer**: Missing. Infrastructure code resides in `services/`.
- **Ports Layer**: Well-defined in `src/ports/`.

## Recommendations
- Extract serialization logic from Domain entities.
- Split `src/services/` into `src/services/` (domain services) and `src/adapters/` (infrastructure implementations).
