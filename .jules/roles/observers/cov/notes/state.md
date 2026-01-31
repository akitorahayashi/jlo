# Coverage State Analysis

## Current Status
- **Tooling:** Missing. No coverage metrics are collected.
- **Unit Tests:**
  - Good coverage in `src/domain` (e.g., `setup.rs`, `run_config.rs`) and `src/services` (e.g., `generator.rs`).
  - **Critical Gap:** `src/app/commands/` is largely devoid of unit tests (except `setup/generate.rs`).
- **Integration Tests:**
  - `tests/` directory contains `assert_cmd` tests.
  - These provide broad "happy path" coverage but likely miss edge cases in command logic.

## Risk Assessment
- **High Risk:** Regression in CLI command logic (git interactions, file handling) due to lack of granular tests.
- **Blind Spot:** We do not know the actual coverage percentage.

## Recommendations
1. Install `tarpaulin` or `grcov` and add a CI step.
2. Refactor `src/app/commands` to use dependency injection (Traits) for `git` and `filesystem` operations to enable unit testing.
