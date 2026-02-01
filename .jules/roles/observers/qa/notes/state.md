# QA State

## Test Architecture Analysis

### Strengths
- **Unit Isolation:** Domain services like `Resolver` and `Generator` are well-isolated and have decent unit test coverage for happy paths.
- **Mocking:** `HttpJulesClient` uses `mockito` effectively to test network failure modes without hitting real APIs.

### Weaknesses
- **Integration Test Bottleneck:** The primary test harness (`TestContext`) relies on modifying global process state (`HOME`, `CWD`). This forces all integration tests to run serially (`#[serial]`), preventing parallel execution and slowing down the feedback loop.
- **Missing Property Tests:** Critical algorithms like topological sorting (`Resolver`) lack property-based testing, relying solely on a few manual examples.
- **Heavy Integration Tests:** A significant portion of the test suite (`tests/cli_commands.rs`) is end-to-end integration tests, which are slower and harder to debug than unit tests.

## Recommendations
- Refactor `jlo` to accept configuration for `home` and `cwd` to allow parallel test execution.
- Introduce `proptest` for algorithmic components.
