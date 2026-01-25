# rs-cli-tmpl Development Overview

## Project Summary
`rs-cli-tmpl` is a reference template for building Rust-based command line tools with a clean, layered architecture. It demonstrates how to separate concerns across the CLI interface, application commands, pure business logic, and I/O abstractions, providing a well-tested foundation for new projects. The template includes sample commands (`add`, `list`, and `delete`) that can be replaced or extended with custom domain logic.

## Tech Stack
- **Language**: Rust
- **CLI Parsing**: `clap`
- **Development Dependencies**:
  - `assert_cmd`
  - `assert_fs`
  - `predicates`
  - `serial_test`
  - `tempfile`

## Coding Standards
- **Formatter**: `rustfmt` is used for code formatting. Key rules include a maximum line width of 100 characters, crate-level import granularity, and grouping imports by standard, external, and crate modules.
- **Linter**: `clippy` is used for linting, with a strict policy of treating all warnings as errors (`-D warnings`).

## Naming Conventions
- **Structs and Enums**: `PascalCase` (e.g., `Cli`, `Commands`)
- **Functions and Variables**: `snake_case` (e.g., `run_tests`, `test_context`)
- **Modules**: `snake_case` (e.g., `cli_commands.rs`)

## Key Commands
- **Build (Debug)**: `cargo build`
- **Build (Release)**: `cargo build --release`
- **Format Check**: `cargo fmt --check`
- **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`
- **Test**: `cargo test --all-targets --all-features`

## Testing Strategy
- **Unit Tests**: Located within the `src/` directory alongside the code they test, covering helper utilities and filesystem boundaries.
- **Command Logic Tests**: Found in `src/commands/`, utilizing mock storage (`src/commands/test_support.rs` with `#[cfg(test)]`) to ensure business logic is tested in isolation via the `Execute` trait.
- **Integration Tests**: Housed in the `tests/` directory, these tests cover the public library API and CLI user flows from an external perspective. Separate crates for API (`tests/commands_api.rs`) and CLI workflows (`tests/cli_commands.rs`, `tests/cli_flow.rs`), with shared fixtures in `tests/common/mod.rs`.
- **CI**: GitHub Actions automatically runs build, linting, and test workflows, as defined in `.github/workflows/`.

## Architectural Highlights
- **Two-tier structure**: `src/main.rs` handles CLI parsing, `src/lib.rs` exposes public APIs and the `default_storage()` helper, and `src/commands/` keeps business rules testable.
- **I/O abstraction**: `src/storage.rs` defines a `Storage` trait and a `FilesystemStorage` implementation rooted at `~/.config/rs-cli-tmpl`, making it easy to swap storage backends.
- **Configuration management**: `src/config.rs` provides a `Config` struct for externalized configuration, enabling custom storage paths for testing.
- **Storage Layout**: Items are stored under `~/.config/rs-cli-tmpl/<id>/item.txt`.
