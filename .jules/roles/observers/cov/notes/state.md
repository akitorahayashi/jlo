# Coverage State Analysis

## Current Status
- **Tooling:** Missing. No coverage metrics are collected.
- **Unit Tests:**
  - Exists for `src/domain` and `src/services`, but primarily covers happy-path data transformation and parsing.
  - **Critical Gap:** `src/app/commands/` is largely devoid of unit tests.
- **Integration Tests:**
  - `tests/` uses `assert_cmd` but relies on weak assertions (checking for substring presence rather than semantic correctness).
  - **Critical Gap:** CI/Production execution paths (e.g., `run` in CI mode) are completely untested as the harness forces local/dry-run modes.

## Risk Assessment
- **Critical Risk:** Production code paths (CI execution) are untested.
- **High Risk:** False safety from integration tests that execute code but verify little.
- **High Risk:** Regression in CLI command logic due to lack of granular tests.
- **Blind Spot:** No coverage metrics.

## Recommendations
1. Install `tarpaulin` or `grcov` and add a CI step.
2. Refactor `src/app/commands` to use dependency injection for `git`, `filesystem`, and `JulesClient`.
3. Strengthen integration tests to assert specific side effects (file content, plan details).
4. Introduce a mock/simulator for `JulesClient` to allow testing CI paths in the integration harness.
