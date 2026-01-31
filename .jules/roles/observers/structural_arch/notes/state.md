# Structural Architecture State

## Current Analysis
- **Domain Layer**: Compromised purity. `RunConfig` and `Setup` entities depend on `toml` and `serde`.
- **Services Layer**: Low cohesion. Mixes Domain Services (`Resolver`) with Infrastructure Adapters (`HttpJulesClient`, `FilesystemWorkspaceStore`).
- **Adapters Layer**: Missing. Infrastructure code resides in `services/`.
- **Ports Layer**: Well-defined in `src/ports/`.
- **Application Layer**: Violates dependency rules. Commands (`init`, `doctor`) directly import concrete services, bypassing ports.
- **Entry Points**:
    - `src/lib.rs`: Excessive public surface. Exports all internal modules (`app`, `services`, `ports`), breaking encapsulation.
    - `src/main.rs`: Contains untestable CLI mapping logic that belongs in an adapter.

## Recommendations
- Extract serialization logic from Domain entities.
- Split `src/services/` into `src/services/` (domain services) and `src/adapters/` (infrastructure implementations).
- Introduce `src/adapters/cli` to encapsulate CLI argument mapping and result formatting, leaving `main.rs` as a thin bootstrap.
- Restrict visibility in `src/lib.rs` to only export the necessary public API (e.g., `app` facade), hiding `services` and `ports` implementations.
- Refactor `src/app/commands` to depend only on `ports`, removing direct dependencies on `services`.
