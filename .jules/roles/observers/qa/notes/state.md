# QA State

## Quality Principles Status

- **Environmental Invariance:** Mostly maintained, but tests depend on global process state (env vars, CWD).
- **Implementation Agnosticism:** Integration tests (CLI) are agnostic, checking behavior via `assert_cmd`. Unit tests in `src/lib.rs` check implementation details.
- **Diagnostic Specificity:** Violated in integration tests where single tests assert many conditions.
- **State Isolation:** Violated. Tests modify global process state (`HOME`, `CWD`), requiring `#[serial]` execution.

## Coverage Summary

- **Unit Tests:** 42 tests covering core logic.
- **Integration Tests:** ~20 tests covering CLI workflows.
- **Coverage Gaps:** Not fully assessed, but core paths seem covered.

## Active Issues

- Coupling of `FilesystemWorkspaceStore` to `std::env::current_dir`.
- Usage of `TestContext` forcing serial execution.
