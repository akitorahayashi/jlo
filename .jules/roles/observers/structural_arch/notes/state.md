# Structural Analysis State

**Last Updated:** 2026-02-01

## Current Findings

### Dependency Structure
- **Violation:** `src/app` depends on `src/services` directly. It should only depend on `src/ports`.
- **Direction:** `app -> ports <- services` is the target. Currently `app -> services`.

### Public Surface
- **Leak:** `src/lib.rs` exports internal modules (`services`, `domain`) making the API surface massive and hard to evolve.

### Cohesion
- **Services:** `src/services` is a flat list of unrelated implementations. `managed_defaults.rs` is a utility grab-bag.
- **Domain:** `src/domain/setup.rs` mixes domain entities (`Component`) with process configuration, and the filename doesn't match the content.

### Findability
- **Ambiguity:** `setup.rs` hides `Component`.

## Exclusions
(None)
