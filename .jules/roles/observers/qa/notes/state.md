# QA State Analysis

## Coverage Gaps
- `src/services/clipboard_arboard.rs`: Zero coverage (confirmed).

## Verified Coverage
The following core modules have verified unit tests:
- `src/domain/setup.rs`: `mod tests` exists and passes.
- `src/services/jules_api.rs`: `mod tests` exists and passes.
- `src/services/workspace_filesystem.rs`: `mod tests` exists and passes.

## Registry State
- **Consistency Issue**: The issue `high/qa_missing_coverage.yml` (and others) are listed in `index.md` but missing from the file system.
- **Accuracy Issue**: The claims in the ghost issue `qa_missing_coverage.yml` regarding missing tests in `setup.rs`, `jules_api.rs`, and `workspace_filesystem.rs` are factually incorrect.

## Architecture
- Confirmed `src/services/` contains infrastructure adapters (`clipboard_arboard.rs`, `jules_api.rs`, `workspace_filesystem.rs`), supporting `medium/arch_service_boundaries.yml` (which is also missing from file system).
