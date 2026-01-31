# Test Architecture & Quality Assessment

## Overview
The project uses a standard Rust testing structure with unit tests in `src/` and integration tests in `tests/`.

## Integration Tests (`tests/`)
- **Harness**: `TestContext` (`tests/common/mod.rs`) provides an isolated environment (temp dir, git repo).
- **Issue**: `TestContext` modifies global process state:
    - Sets `HOME` environment variable via `env::set_var`.
    - Changes current working directory via `env::set_current_dir` in `with_work_dir` and `Drop`.
- **Consequence**: All CLI tests in `tests/cli_commands.rs` use `#[serial]`, preventing parallel execution and slowing down feedback loops.

## Unit Tests (`src/`)
- **Coverage**:
    - `FilesystemWorkspaceStore`: Good isolation using `tempfile`.
    - `Generator`: Pure logic tests, good coverage.
    - `Resolver`: Algorithmic tests, good edge case coverage (cycles, diamonds).
    - `HttpJulesClient`: Uses `mockito` for network isolation.
- **Gaps**:
    - **Template Validation**: Tests for `EmbeddedRoleTemplateStore` verify string containment but do not validate that embedded templates are valid YAML.
    - **Error Modeling**: `AppError` is imprecise, using `ConfigError(String)` for network, git, and CLI errors, leading to "stringly typed" assertions (e.g., matching "429" in error messages).

## Recommendations
1. Refactor `TestContext` to avoid global state mutation. Pass explicit home directory and CWD to `assert_cmd::Command` instead of relying on process-level state.
2. Remove `#[serial]` from CLI tests to enable parallel execution.
3. Introduce structured variants to `AppError` (e.g., `NetworkError`, `GitError`) to improve diagnosability and remove string matching in logic/tests.
4. Add YAML validation to template tests to ensure embedded assets are syntactically correct.
