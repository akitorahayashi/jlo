# QA Role State

## Test Architecture

The repository employs a split testing strategy:
- **Integration Tests**: Located in `tests/`, covering CLI commands and end-to-end flows.
- **Unit Tests**: Co-located in `src/` modules, covering internal logic of services and domain entities.

## Integration Testing Strategy

Integration tests rely on a `TestContext` harness (`tests/common/mod.rs`) that:
- Creates a temporary directory for each test run.
- Initializes a git repository and switches branches (required by `jlo`).
- **Modifies process-global state**: Sets `CWD` and `HOME` environment variables to point to the temporary directory.

**Implication**: All CLI tests using `TestContext` must be run serially (using the `serial_test` crate) to prevent race conditions on global state. This limits test parallelism and feedback speed.

## Unit Testing Strategy

Service-level unit tests are generally well-isolated, with some exceptions:
- **Pure Logic**: `DependencyResolver` has good unit tests for its algorithms but lacks property-based testing.
- **Mixed IO**: `ArtifactGenerator` mixes file reading with logic, requiring tempfile usage in tests.

## Identified Risks

1.  **Flakiness via Isolation Leaks**: The reliance on process-global state modification in `TestContext` is a fragility. If a test panics or fails to cleanup, it could pollute the environment for subsequent tests or the developer's shell (though `TestContext` tries to restore state in `Drop`).
2.  **Algorithm Correctness**: `DependencyResolver`'s topological sort is complex and critical. The lack of property-based tests leaves edge cases potentially uncovered.
3.  **IO Coupling**: Some services interact directly with the filesystem rather than using abstractions or taking content as input, complicating unit tests.
