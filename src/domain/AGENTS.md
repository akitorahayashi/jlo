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
  - `crate::services` (Service implementations)
  - `crate::assets` (Embedded assets, unless accessed via a domain trait)

## Modules

| Module | Purpose |
|--------|---------|
| `configuration` | Global configuration models (`config.toml`, `scheduled.toml`). |
| `identifiers` | Strongly-typed IDs (`RoleId`, `ComponentId`). |
| `prompt_assembly` | Logic for assembling prompt contexts for different layers. |
| `workspace` | Logical paths and layer structures. |
| `error` | Domain-level error types (`AppError`). |
| `requirement` | Requirement file parsing and schema validation. |
| `component_graph` | Component dependency resolution logic. |
| `setup_artifacts` | Setup script generation models. |
| `builtin_role` | Definitions for built-in roles. |

## Testing

Domain logic should be testable via unit tests.
Currently, coverage is sparse; new logic should include `#[test]` modules within the source files.
