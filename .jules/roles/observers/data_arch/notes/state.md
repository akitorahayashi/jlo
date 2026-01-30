# Data Architecture State

## Overview
The current architecture exhibits significant coupling between Domain and Infrastructure layers, along with primitive obsession in critical identifiers.

## Layer Analysis

### Domain (`src/domain/`)
*   **Status**: Compromised
*   **Issues**:
    *   **Infrastructure Leakage**: `src/domain/setup.rs` entities (`EnvSpec`, `SetupConfig`) derive `serde::Deserialize` and use `#[serde(default)]`, coupling domain logic to serialization formats.
    *   **Weak Typing**: `RunSettings` uses raw `String` for branch names. `EnvSpec` uses raw `String` for variable names without validation.
    *   **Validation Gaps**: `EnvSpec` lacks constructor enforcement, allowing invalid environment variable names.

### Services (`src/services/`)
*   **Status**: Mixed Boundaries
*   **Issues**:
    *   **Layer Violation**: Domain Services (`Generator`, `Resolver`) are co-located with Infrastructure Adapters (`ArboardClipboard`, `HttpJulesClient`, `FilesystemWorkspaceStore`).
    *   **Implicit Dependencies**: `HttpJulesClient` depends on global process state (`std::env::var`).

### Ports (`src/ports/`)
*   **Status**: Weak Typing
*   **Issues**:
    *   **Primitive Obsession**: `DiscoveredRole` uses `String` for `id` instead of the validated `RoleId` value object.

## Principles Compliance

| Principle | Status | Observation |
| :--- | :--- | :--- |
| **Normalized Independence** | ⚠️ Partial | DTO separation exists in `run_config.rs` but is missing in `setup.rs`. |
| **Isomorphic Representation** | ❌ Failed | Valid `RoleId`s are demoted to `String`s in Ports; `EnvSpec` permits invalid states. |
| **Boundary Sovereignty** | ❌ Failed | Domain models depend on `serde`; Services mix adapters and logic. |
| **Temporal Monotonicity** | ⚠️ At Risk | Mutable state in `ArboardClipboard` adapter. |
