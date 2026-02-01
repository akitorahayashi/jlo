# Test Architecture & Quality Assessment

## Overview
The project uses a standard Rust testing structure with unit tests in `src/` and integration tests in `tests/`. While basic coverage exists, the test suite suffers from global state dependencies, weak assertions, and "happy path" bias in unit tests.

## Integration Tests (`tests/`)
- **Harness**: `TestContext` (`tests/common/mod.rs`) provides an isolated environment (temp dir, git repo).
- **Global State Issue**: `TestContext` modifies global process state (`env::set_var("HOME")`, `env::set_current_dir`).
    - **Consequence**: All CLI tests in `tests/cli_commands.rs` use `#[serial]`, preventing parallel execution.
- **Untested CI Paths**: Tests force execution down "local" paths (by unsetting `GITHUB_ACTIONS`), leaving critical CI-specific logic (e.g., in `run/single_role.rs`) completely untested.
- **Weak Assertions**: Tests often assert on partial string matches (e.g., "Would dispatch workflow") without validating the correctness of the structured data or side effects (e.g., *which* workflow, *what* parameters).

## Unit Tests (`src/`)
- **Coverage**:
    - `FilesystemWorkspaceStore`: Good isolation using `tempfile`.
    - `Generator`: Pure logic tests, good coverage.
    - `Resolver`: Algorithmic tests cover standard edge cases (cycles, diamonds) but lack property-based rigour.
    - `HttpJulesClient`: Uses `mockito` for network isolation.
- **Gaps**:
    - **Silent Failures**: `EmbeddedRoleTemplateStore` logic silently ignores non-UTF8 files, making asset loading brittle and hard to debug.
    - **Template Validation**: Tests for `EmbeddedRoleTemplateStore` verify string containment but do not validate that embedded templates are valid YAML.
    - **Error Modeling**: `AppError` is imprecise, using `ConfigError(String)` for network, git, and CLI errors, leading to "stringly typed" assertions.
    - **Untested Command Logic**: Complex logic in `src/app/commands/` (git operations, path validation) is tightly coupled to IO and system commands, making it untestable at the unit level.

## Recommendations
1. **Refactor TestContext**: Remove global state mutation. Pass explicit `HOME` and `CWD` to `assert_cmd::Command`.
2. **Parallelize Tests**: Remove `#[serial]` once the harness is thread-safe.
3. **Property-Based Testing**: Introduce `proptest` for `Resolver` to verify graph algorithms against generated random DAGs.
4. **Structured Errors**: Introduce specific `AppError` variants (`NetworkError`, `GitError`) to enable typed error handling and testing.
5. **Strict Asset Loading**: Modify `EmbeddedRoleTemplateStore` to return errors for invalid files instead of silently ignoring them.
6. **Mocking for Commands**: Refactor CLI commands to use traits for IO and System interaction to allow unit testing of orchestration logic.
