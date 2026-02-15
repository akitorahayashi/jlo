# Domain Layer

This directory contains the core business logic and type definitions for the application.

## Purpose

- **Pure Logic**: Encapsulate business rules, validation, and data structures.
- **Type Safety**: Define `enum` and `struct` types that represent domain concepts (e.g., `Layer`, `RoleId`, `RequirementHeader`).
- **Interfaces**: Define interactions with the outside world via `crate::ports` traits.

## Dependencies

Strict dependency rules apply to maintain architectural purity:

- **Allowed**:
  - `crate::domain` (sibling modules)
  - `crate::ports` (interfaces/traits)
  - `std` (Standard Library)
  - `serde`, `thiserror`, and other pure utility crates.

- **Forbidden**:
  - `crate::adapters` (Infrastructure implementations)
  - `crate::app` (Application wiring and CLI commands)
  - `crate::assets` (Embedded assets, unless accessed via a domain trait)

## Modules

| Module | Purpose |
|--------|---------|
| `config` | `config.toml` models and parsing (`RunConfig`, `WorkflowGenerateConfig`, mock config). |
| `schedule` | `scheduled.toml` model and validation. |
| `layers` | Layer taxonomy (`Layer`), `.jules/layers` path semantics, and prompt assembly logic. |
| `roles` | Role identifiers, builtin role entries, and `.jlo/roles` path semantics. |
| `exchange` | `.jules/exchange` structure (`events`, `requirements`, `innovators`) and requirement schema. |
| `workstations` | `.jlo/.jules` top-level path semantics and managed manifest model. |
| `setup` | Setup component model, dependency resolution, and artifact generation. |
| `error` | Domain-level error types (`AppError`). |

## Testing

Domain logic should be testable via unit tests.
Currently, coverage is sparse; new logic should include `#[test]` modules within the source files.
