# Integration Tests

This directory contains integration and end-to-end tests for the `jlo` CLI and its workflow scaffolding.

## Purpose

- **Behavior verification**: validate CLI behavior and exit codes from a user perspective.
- **Scaffold contracts**: validate generated `.jules/` runtime scaffolding and installed `.github/` workflow kit.
- **Schema safety**: validate that doctor catches contract violations and that shipped mock fixtures remain valid.

## Structure

Integration tests are organized as **small, stable targets** (top-level `tests/*.rs`), with detailed contract modules under `tests/<target>/`.

```text
tests/
    harness/                 # Shared fixtures (no tests)
        test_context.rs        # TestContext
        git_repository.rs      # git helpers (commits/remotes)
        jlo_config.rs          # .jlo/config.toml writers
        scheduled_roles.rs     # config schedule readers

    cli.rs
    cli/                     # CLI behavior contracts (by command)

    workflow.rs
    workflow/                 # bootstrap + workflow-kit contracts

    doctor.rs
    doctor/                   # schema failure + mock-fixture validity contracts

    mock.rs
    mock/                     # mock-mode CLI contracts

    library.rs
    library/                  # public API lifecycle contract
```

## Contract Granularity

- Default rule: **one behavior contract per file**.
- Multiple `#[test]` functions in one file are allowed only when they validate the **same contract** (typical target: 1–3 tests, ~250 LOC max).
- Avoid catch-all buckets and unrelated assertions in the same file.

## Shared Harness

All integration tests should use `TestContext` to create an isolated temporary workspace.

```rust
use crate::harness::TestContext;

#[test]
fn my_contract() {
        let ctx = TestContext::new();
        ctx.init_remote_and_bootstrap();
        // ctx.work_dir() is a temp repo root
        // ctx.cli() invokes the compiled jlo binary
}
```

## Running Tests

Run all integration tests:

```bash
cargo test --tests
```

Run by target:

```bash
cargo test --test cli
cargo test --test workflow
cargo test --test doctor
cargo test --test mock
cargo test --test library
```
