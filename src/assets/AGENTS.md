# Assets Layer

## Purpose

Embedded static resources for project scaffolding, templates, and setup.
Accessed at runtime via `adapters/catalogs` (using `include_dir!`).

## Structure

```
src/assets/
├── github/           # Workflow templates
├── mock/             # Test mock data
├── roles/            # Built-in role definitions
├── scaffold/         # The .jules/ directory structure
├── setup/            # Setup component definitions
├── summary-requests/ # Example summary requests
└── templates/        # Role templates
```

## Architectural Principles

-   Static Resources: Everything here is embedded into the binary.
-   Usage: Access is mediated by `adapters/catalogs` (e.g., `scaffold_assets.rs`).
-   Content Ownership:
    -   `scaffold`: Defines the exact structure of the `.jules/` directory.
    -   `github`: Workflow templates (source of truth for `.github/` artifacts).
    -   `setup`: Component definitions for `jlo setup`.
