# Test Coverage State

## Overview
The project maintains a high level of test coverage across both core services and CLI integration points. Tests are organized into unit tests (within `src/`) and integration tests (within `tests/`).

## Core Services (Unit Tests)
- **Catalog Service** (`src/services/catalog.rs`):
  - Validates loading of embedded components.
  - Verifies component retrieval and listing.
  - Ensures correct parsing of metadata.
- **Resolver Service** (`src/services/resolver.rs`):
  - Comprehensive coverage of dependency resolution (Kahn's algorithm).
  - Validates cycle detection, diamond dependencies, and missing dependencies.
  - Checks deduplication of requests.
- **Generator Service** (`src/services/generator.rs`):
  - Verifies generation of `install.sh` scripts with headers and sections.
  - Validates `env.toml` merging logic, ensuring preservation of user values and addition of new defaults.
- **Role Template Service** (`src/services/role_template_service.rs`):
  - Checks generation of YAML structures for roles and prompts.
  - Verifies presence of scaffold files.

## CLI Commands (Integration Tests)
- **Init Command** (`tests/cli_commands.rs`):
  - Verifies creation of `.jules/` directory structure.
  - Checks for idempotency/failure if workspace exists.
  - Validates setup directory initialization.
- **Template Command** (`tests/cli_commands.rs`):
  - Verifies role creation in valid layers.
  - Checks error handling for invalid layers or existing roles.
  - Validates generation of role-specific files (role.yml, notes/).
- **Setup Command** (`tests/cli_commands.rs`):
  - Verifies `setup gen` produces executable scripts.
  - Verifies `setup list` outputs available components.
  - Checks error handling for uninitialized setup.

## Gaps & Notes
- **Interactive Prompts**: Interactive paths (e.g., `template` without args) rely on `dialoguer` and are not covered by automated tests. Integration tests bypass prompts using CLI arguments.
- **Main Entrypoint**: `src/main.rs` is minimal wiring and tested via integration tests.
