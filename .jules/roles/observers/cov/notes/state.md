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
  - **Missing:** `src/services/workstream_template_assets.rs` lacks unit tests.

## Coverage Gaps
- **Workstream Creation:** `src/services/workstream_template_assets.rs` is critical for creating new workstreams but has no unit tests. It relies on embedded assets that are not validated for content or structure (only presence is checked implicitly by compilation, but logic to collect them is untested).
- **Asset Validation:** While `scaffold_assets.rs` checks for non-empty files, it does not validate that the content is syntactically correct (e.g., valid YAML).

## Recommendations
1. **Backfill Unit Tests:** Add unit tests for `src/services/workstream_template_assets.rs` to ensure templates are correctly loaded and parsed.
2. **Enhance Asset Validation:** Expand tests to validate the syntax of embedded assets (YAML/TOML parsing) to catch broken templates before release.
