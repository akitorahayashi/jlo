# QA Role State - Test Architecture Analysis

## Overview
The repository utilizes a standard Rust testing structure with:
- **Unit Tests**: Co-located in `src/` modules (e.g., `src/services/dependency_resolver.rs`).
- **Integration Tests**: Located in `tests/`, primarily testing CLI behavior via `assert_cmd`.

## Key Observations

### 1. Global State Mutation & Serial Execution
Integration tests in `tests/` rely on a shared `TestContext` harness (`tests/common/mod.rs`) that modifies the global process state:
- `env::set_var("HOME", ...)`
- `env::set_current_dir(...)`

This design forces all integration tests to use the `#[serial]` attribute, preventing parallel execution. As the test suite grows, this will become a significant bottleneck for feedback speed.

### 2. Missing Property-Based Testing
The `DependencyResolver` service implements complex graph algorithms (topological sort, cycle detection). Current tests are limited to manual example-based unit tests. There is a lack of property-based testing (e.g., `proptest`) to verify algorithmic invariants across generated inputs, leaving the system vulnerable to edge-case regressions.

### 3. Test Isolation
Unit tests in `src/` appear to be well-isolated and deterministic. However, the heavy reliance on "black-box" CLI integration tests for verification means that many logic paths are tested indirectly via the file system, which is slower and harder to diagnose than direct unit tests.

## Active Events
- `global-state-mutation-in-tests.yml`: Tracks the integration test architecture issue.
- `missing-property-tests-resolver.yml`: Tracks the need for better algorithmic verification.
