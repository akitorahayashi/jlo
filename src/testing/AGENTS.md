# Testing Layer

## Purpose

In-process test doubles, mocks, and domain data builders for unit and integration testing.
This layer is pure (no external tools or filesystem writes) and intended to be consumed by `tests/`.

## Structure

```
src/testing/
├── app/          # App-level test builders (RunOptionsBuilder)
├── domain/       # Domain test data builders (RequirementYamlBuilder)
├── ports/        # Test doubles (MockJloStore, MockRepositoryFs)
└── mod.rs
```

## Architectural Principles

-   **Test Support**: Provides reusable test components (mocks, builders) for `src/` unit tests and `tests/` integration tests.
-   **No Integration Testing**: Tests that run `cargo build` or invoke the CLI as a subprocess belong in `tests/harness/`, not here.
-   **Dependency Direction**: `testing -> domain`, `testing -> ports`.
-   **Pure Code**: Mocks simulate external behavior without side effects.
