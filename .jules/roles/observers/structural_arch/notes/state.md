# Structural Architecture State

**Last Updated:** 2026-02-01

## Executive Summary
The project follows a hexagonal-like architecture on the surface (ports, adapters, domain), but strictly violates it in practice. The most critical issues are dependency inversion violations where the core application layer depends directly on concrete service implementations, and a lack of clear separation between the CLI entry point and the application logic.

## Key Findings

### Dependency Structure
- **Violation:** `src/app/commands` imports from `src/services` directly.
- **Goal:** `app` should only depend on `ports`. `services` should implement `ports` and be injected.

### Public Surface
- **Violation:** `src/lib.rs` exports internal modules (`services`, `app`), exposing implementation details.
- **Goal:** Only `ports` and necessary domain types should be public.

### Cohesion
- **Violation:** `src/services` is a mixed bag of Adapters and Domain Services.
- **Goal:** Split `services` into `adapters` (implementations) and `domain/services` (logic).

### CLI Coupling
- **Violation:** `src/main.rs` contains CLI definitions.
- **Goal:** Move CLI logic to an adapter layer.

## Active Events
- `depv01`: Dependency Violation: App Layer Imports Services Directly
- `publ01`: Public Surface Leakage
- `clic01`: CLI Logic Coupled to Binary
- `mixs01`: Mixed Responsibilities in Services Directory
