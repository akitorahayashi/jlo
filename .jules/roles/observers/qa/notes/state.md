# QA State

## Coverage Gaps
- `src/services/jules_api.rs`: Critical logic (retry, error handling) is untested.
- `src/services/clipboard_arboard.rs`: Untested (minor).
- `src/domain/setup.rs`: Domain logic mixing with DTOs, untested (covered by issue `2026-01-29_issue_setup_domain_refactor`).

## Quality Issues
- `tests/common/mod.rs`: Coupling to global state (covered by issue `2026-01-29_issue_decouple_global_state`).
- `tests/cli_commands.rs`: Low diagnostic specificity (covered by issue `2026-01-29_issue_test_specificity`).
