# Structural Analysis State

**Last Updated:** 2026-02-01

## Current Findings

### Dependency Structure
- **Violation:** `src/app` depends on `src/services` directly. It should only depend on `src/ports`.
- **Direction:** `app -> ports <- services` is the target. Currently `app -> services`.

### Public Surface
- **Leak:** `src/lib.rs` exports the `app` module, which exposes internal command logic and structure.

### Cohesion
- **Services:** `src/services` is a flat list of unrelated implementations.

## Exclusions
(None)
