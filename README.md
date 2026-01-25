# rs-cli-tmpl

`rs-cli-tmpl` is a reference template for building Rust-based command line tools with a clean,
layered architecture. It demonstrates how to separate concerns across the CLI interface,
application commands, pure business logic, and I/O abstractions so new projects can start from a
well-tested foundation.

## Architectural Highlights

- **Two-tier structure** &mdash; `src/main.rs` handles CLI parsing, `src/lib.rs` exposes public 
  command APIs, and `src/commands/` keeps business rules testable via the `Execute` trait.
- **I/O abstraction** &mdash; `src/storage.rs` defines a `Storage` trait and a `FilesystemStorage`
  implementation rooted at `~/.config/rs-cli-tmpl`, making it easy to swap storage backends.
- **Configuration management** &mdash; `src/config.rs` provides a `Config` struct for externalized
  configuration, enabling easy testing with custom storage paths.
- **Robust testing strategy** &mdash; unit tests live next to their modules, `src/commands/test_support.rs`
  offers a `MockStorage` for command logic tests (with `#[cfg(test)]`), and the `tests/` directory 
  provides integration suites for both the library API and the CLI binary.

The template ships with minimal sample commands (`add`, `list`, and `delete`) that show how to
thread dependencies through each layer. Replace or extend them with your own domain logic while
reusing the same structure.

## Storage Layout

The template stores items under `~/.config/rs-cli-tmpl/<id>/item.txt`. For example, after running `rs-cli-tmpl add my-item --content '...'`:

```text
~/.config/rs-cli-tmpl/
  my-item/
    item.txt
```

## Quick Start

```bash
cargo install --path .
# or
cargo build --release
```

The optimized binary will be created at `target/release/rs-cli-tmpl`.

## Usage

```bash
rs-cli-tmpl --version    # Show version information
rs-cli-tmpl add <id>     # Add an item
rs-cli-tmpl list         # List items
rs-cli-tmpl delete <id>  # Delete an item
```

## Development Commands

- `cargo build` &mdash; build a debug binary.
- `cargo build --release` &mdash; build the optimized release binary.
- `cargo fmt` &mdash; format code using rustfmt.
- `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings` &mdash; format check and lint with clippy.
- `cargo test --all-targets --all-features` &mdash; run all tests.
- `cargo fetch --locked` &mdash; pre-fetch dependencies.

## Testing Culture

- **Unit Tests**: Live alongside their modules inside `src/`, covering helper utilities and
  filesystem boundaries.
- **Command Logic Tests**: Use the mock storage in `src/commands/test_support.rs` (conditionally
  compiled with `#[cfg(test)]`) to exercise command implementations without touching the filesystem.
- **Integration Tests**: Located in the `tests/` directory. Separate crates cover the public
  library API (`tests/commands_api.rs`) and CLI workflows (`tests/cli_commands.rs`,
  `tests/cli_flow.rs`). Shared fixtures live in `tests/common/mod.rs`.

## Project Structure

```
rs-cli-tmpl/
├── src/
│   ├── main.rs           # CLI parsing (clap)
│   ├── lib.rs            # Public API + default_storage() helper
│   ├── config.rs         # Config struct for externalized configuration
│   ├── error.rs          # AppError definitions
│   ├── storage.rs        # Storage trait + FilesystemStorage
│   └── commands/         # Command implementations
│       ├── mod.rs        # Execute trait
│       ├── add_item.rs
│       ├── list_items.rs
│       ├── delete_item.rs
│       └── test_support.rs  # MockStorage (#[cfg(test)])
└── tests/
    ├── common/           # Shared test fixtures
    └── ...
```

## Adapting the Template

1. Replace the sample commands in `src/commands/` with your own business logic.
2. Extend `src/lib.rs` to wire new dependencies and expose public APIs.
3. Update the CLI definitions in `src/main.rs` to match your command surface.
4. Refresh the integration tests and documentation to describe the new behavior.

Happy hacking!
