# Ports Layer

## Purpose

Interface boundaries (traits) defining capabilities required by the application use cases.
These interfaces decouple the application from implementation details (adapters).

## Structure

```
src/ports/
├── git.rs                    # Git operations (run)
├── github.rs                 # GitHub operations (issues, prs)
├── jlo_store.rs              # .jlo/ control plane access
├── jules_client.rs           # Jules API access
├── jules_store.rs            # .jules/ runtime access
├── repository_filesystem.rs  # Abstract filesystem access
├── role_template_store.rs    # Role template access
└── setup_component_catalog.rs # Setup component access
```

## Architectural Principles

-   Interface Only: Contains traits (`trait JloStore`, `trait Git`) and their request/response types.
-   Abstraction: Decouples `app` (what) from `adapters` (how).
-   Testing Support: Provides mockable boundaries (`MockJloStore` in `src/testing/ports`).
-   No Implementation: Implementations reside in `src/adapters/`.
