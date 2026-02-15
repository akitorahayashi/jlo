# Adapters Layer

## Purpose

Implementations of `ports` interfaces for I/O operations (filesystem, network, git, GitHub).
Adapters provide the "how" of interaction with the outside world.

## Structure

```
src/adapters/
├── catalogs/          # Embedded asset access
├── git/               # Git CLI wrapping
├── github/            # GitHub CLI (gh) wrapping
├── jules_client/      # HTTP client for Jules API
├── local_repository/  # Filesystem & Store implementations
├── control_plane_config.rs
└── workflow_installer.rs
```

## Architectural Principles

-   **Dependency Direction**: `adapters -> ports`. Adapters implement traits defined in `ports`.
-   **No Business Logic**: Adapters must **not** contain domain logic (parsing, validation, path semantics). They only execute I/O and map errors.
-   **No Cross-Adapter Dependencies**: Adapters should not depend on other adapters directly. Wiring happens in `app`.
-   **Ownership**: Adapters do **not** own `.jlo` or `.jules` layout semantics. They use paths provided by `domain` or `app`.
