# Structural Analysis State

**Last Updated:** 2026-02-01

## Current Findings

### Dependency Structure
- **Violation:** `src/app` depends on `src/services` directly. It should only depend on `src/ports`.
- **Direction:** `app -> ports <- services` is the target. Currently `app -> services`.
- **Event:** `kd92la` (App layer bypasses ports to access services directly)

### Public Surface
- **Leak:** `src/lib.rs` exports the `app` module, which exposes internal command logic and structure.
- **Event:** `m9d2ka` (Internal app module is publicly exported)

### Cohesion
- **Services:** `src/services` is a flat list of unrelated implementations.
- **Event:** `j8s7d1` (Services directory lacks cohesion)

## Exclusions
(None)
