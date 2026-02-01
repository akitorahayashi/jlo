# Coverage State

## Overview
The project uses a mix of black-box integration tests (`tests/`) and unit tests. CI is now instrumented with `tarpaulin` to measure code coverage.

## Testing Strategy
- **Integration Tests:** The `tests/` directory contains comprehensive tests using `assert_cmd` and `predicates` to verify CLI commands (`init`, `template`, `doctor`, etc.).
- **Unit Tests:**
  - **Domain:** `src/domain/setup.rs` has unit tests.
  - **Services:**
    - `scaffold_manifest.rs` (formerly `managed_defaults`) has unit tests for integrity logic.
    - `scaffold_assets.rs` has integrity tests to ensure embedded assets are non-empty.
    - `dependency_resolver.rs` and `artifact_generator.rs` have unit tests.
    - `workstream_template_assets.rs` has unit tests verifying file loading and basic content checks.

## Coverage Gaps
- **Asset Validation:** Both `scaffold_assets.rs` and `workstream_template_assets.rs` verify that embedded assets exist and are not empty, but they do not exhaustively validate the syntax (YAML/TOML) or schema compliance of the embedded files. This creates a risk of shipping broken templates.

## Recommendations
1. **Enhance Asset Validation:** Expand tests to validate the syntax of embedded assets (YAML/TOML parsing) to catch broken templates before release.
