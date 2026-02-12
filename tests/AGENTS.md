# Integration Tests

This directory contains integration and end-to-end tests for the `jlo` CLI and workflow orchestration.

## Purpose

- **Behavior Verification**: Ensure the CLI behaves as expected from the user's perspective.
- **Workflow Simulation**: Verify that complex workflows (init -> bootstrap -> run) execute correctly.
- **Artifact Validation**: Check that generated files (scaffolds, install scripts) are correct.

## Structure

| File | Purpose |
|------|---------|
| `cli_commands.rs` | Tests specific CLI command parsing, arguments, and simple execution. |
| `cli_flow.rs` | Tests full workflow lifecycles (e.g., `init` then `bootstrap`). |
| `workflow_scaffold.rs` | Verifies the content and structure of generated workflow files. |
| `mock_mode.rs` | Tests the mock execution mode (`--mock`). |
| `bootstrap.rs` | Tests specific to the bootstrap process. |
| `api_coverage.rs` | Tests covering API surface area. |
| `commands_core.rs` | Core command logic tests. |
| `common/` | Shared test utilities and `TestContext` setup. |

## Patterns

### Isolation via `TestContext`

All tests should use `common::TestContext` to create an isolated temporary directory.

```rust
use common::TestContext;

#[test]
fn my_feature_works() {
    let ctx = TestContext::new();
    // ctx.work_dir() is a temp dir
    // ctx.cli() returns a command builder for the current binary
}
```

### Assertions

Use `assert_cmd` and `predicates` for robust assertions on stdout/stderr and exit codes.

```rust
ctx.cli()
    .args(["my", "command"])
    .assert()
    .success()
    .stdout(predicate::str::contains("Success message"));
```

## Running Tests

Run all integration tests:
```bash
cargo test --tests
```

Run a specific test file:
```bash
cargo test --test cli_commands
```
