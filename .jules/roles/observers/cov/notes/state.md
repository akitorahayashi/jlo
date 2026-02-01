# Coverage State

## Overview
The project relies heavily on black-box integration tests (`tests/`) to verify CLI behavior and file system side effects. While these provide good end-to-end confidence, there is no code coverage measurement in place, making it difficult to assess regression risks or identify dead code.

## Testing Strategy
- **Integration Tests:** The `tests/` directory contains comprehensive tests using `assert_cmd` and `predicates` to verify CLI commands (`init`, `template`, `doctor`, etc.). These tests are valuable but slow and coarse-grained.
- **Unit Tests:**
  - **Domain:** `src/domain/setup.rs` has unit tests for component metadata parsing.
  - **Services:** `src/services/resolver.rs` (dependency resolution) and `src/services/generator.rs` (script generation) have unit tests.
  - **Missing:** `src/services/managed_defaults.rs` (integrity/hashing) and asset services (`scaffold_assets.rs`) lack unit tests.

## Coverage Gaps
- **Reporting:** No coverage tools (`tarpaulin`, `grcov`) are configured in CI.
- **Critical Paths:** The logic for verifying workspace integrity (`ManagedDefaultsManifest`) is untested at the unit level, posing a risk for the `jlo doctor` and `jlo update` commands.
- **Asset Integrity:** Embedded assets are not validated by tests, meaning broken templates could ship to production.

## Recommendations
1. **Instrument CI:** Add a coverage step to `run-tests.yml` using `tarpaulin` or `grcov` to establish a baseline.
2. **Backfill Unit Tests:** Prioritize adding tests for `src/services/managed_defaults.rs` to secure the update mechanism.
3. **Asset Validation:** Add tests to verify that embedded assets can be loaded and parsed correctly.
