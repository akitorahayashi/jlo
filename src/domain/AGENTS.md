# Domain Layer

## Purpose

Core business logic, types, and path semantics.
This layer is pure, with no dependencies on `adapters` or `app`.

## Structure

```
src/domain/
├── config/           # Configuration models
├── exchange/         # Exchange (events/requirements) models
├── layers/           # Layer taxonomy, prompt assembly, and execution semantics
├── roles/            # Role taxonomy
├── schedule/         # Schedule models
├── setup/            # Setup component logic
├── jlo_paths.rs      # .jlo path semantics
├── jules_paths.rs    # .jules path semantics
├── error.rs          # AppError & Result types
└── mod.rs
```

## Architectural Principles

-   Purity: Depends only on `std` and pure utility crates (`serde`).
-   No I/O: No file system access, network calls, or tool execution.
-   Ownership: Sole owner of `.jlo` and `.jules` path logic (parsing, validation).
-   Dependency Direction: `domain -> ports` (interfaces only), `domain -> std`.
-   Type Safety: All domain concepts are strongly typed (enums, structs).

`src/domain/layers/execute/` owns shared run execution semantics (`RunResult`, `JulesClientFactory`, requirement-path validation, starting-branch resolution, and shared layer policy predicates).
