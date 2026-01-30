# QA State

## Coverage Analysis
- **Services**:
    - `ArboardClipboard`: 0% coverage (Untested).
    - `HttpJulesClient`: Low coverage. Retry logic and session creation untested.
    - `FilesystemWorkspaceStore`: Partial coverage. Discovery methods untested.
- **Domain**:
    - `Setup`: 0% coverage (Untested).
    - `RunConfig`: Untested (Primitive Obsession noted).

## Quality Analysis
- **Integration Tests**:
    - `TestContext` relies on global state modification (CWD, HOME), forcing serial execution.
    - `tests/commands_api.rs` tests library internals directly, bypassing CLI boundaries.

## Architecture
- **Test Isolation**: Compromised by global state usage in `TestContext` and `FilesystemWorkspaceStore`.
