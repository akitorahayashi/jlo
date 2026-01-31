# Test Architecture & Quality Assessment

## Overview
The project uses a standard Rust testing structure with unit tests in `src/` and integration tests in `tests/`.

## Integration Tests (`tests/`)
- **Harness**: `TestContext` (`tests/common/mod.rs`) provides an isolated environment (temp dir, git repo).
- **Issue**: `TestContext` modifies global `HOME` environment variable (`env::set_var`), forcing sequential execution.
- **Consequence**: All CLI tests in `tests/cli_commands.rs` use `#[serial]`, preventing parallel execution.

## Unit Tests (`src/`)
- **Coverage**:
    - `FilesystemWorkspaceStore`: Good isolation using `tempfile`.
    - `Generator`: Pure logic tests, good coverage.
    - `Resolver`: Algorithmic tests, good edge case coverage (cycles, diamonds).
    - `HttpJulesClient`: Uses `mockito` for network isolation.
- **Gaps**:
    - `ArboardClipboardWriter` was mentioned in historical memory as untested but not found in codebase (possibly removed).

## Recommendations
1. Refactor `TestContext` to avoid global state mutation. Pass explicit home directory to `assert_cmd::Command` instead of relying on process-level `HOME`.
2. Remove `#[serial]` from CLI tests to enable parallel execution.
