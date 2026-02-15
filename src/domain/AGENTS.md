# Domain Layer

## Purpose

Core business logic, types, and path semantics.
This layer is pure, with no dependencies on `adapters` or `app`.

## Structure

```
src/domain/
├── config/           # Configuration models
├── exchange/         # Exchange (events/requirements) models
├── layers/           # Layer taxonomy & Prompt assembly
├── roles/            # Role taxonomy
├── schedule/         # Schedule models
├── setup/            # Setup component logic
├── workstations/     # .jlo/.jules path semantics
├── error.rs          # AppError & Result types
└── mod.rs
```

## Architectural Principles

-   **Purity**: Depends only on `std` and pure utility crates (`serde`).
-   **No I/O**: No file system access, network calls, or tool execution.
-   **Ownership**: Sole owner of `.jlo` and `.jules` path logic (parsing, validation).
-   **Dependency Direction**: `domain -> ports` (interfaces only), `domain -> std`.
-   **Type Safety**: All domain concepts are strongly typed (enums, structs).
