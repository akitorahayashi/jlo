# jlo CLI Development Context

See [root AGENTS.md](../AGENTS.md) for design principles.

## Architecture & Context

| Layer | Path | Responsibility |
|-------|------|----------------|
| [Adapters](adapters/AGENTS.md) | `src/adapters/` | I/O Implementations (Git, GitHub, FS, HTTP) |
| [Application](app/AGENTS.md) | `src/app/` | Command Orchestration, Config, & Wiring |
| [Assets](assets/AGENTS.md) | `src/assets/` | Embedded Static Resources (Scaffold, Templates) |
| [Domain](domain/AGENTS.md) | `src/domain/` | Pure Business Logic, Types, & Path Semantics |
| [Ports](ports/AGENTS.md) | `src/ports/` | Interface Boundaries (Traits) |
| [Testing](testing/AGENTS.md) | `src/testing/` | In-process Test Doubles & Builders |
| Integration | `tests/` | Black-box CLI & Workflow Contracts |

## Core Concepts

-   Control Plane (`.jlo/`): Source of truth on Target Branch. Config, role definitions, version pins.
-   Runtime Plane (`.jules/`): Materialized agent environment on Worker Branch.
-   Projection: `.jlo/` + `assets/scaffold/` $\to$ `.jules/` (Deterministic materialization via `jlo init`/`update`).
-   Exchange: `.jules/exchange/` (Flat handoff for Events, Requirements, Proposals).

## Development Cycle

Verification
```bash
cargo check && cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Targeted Testing
```bash
cargo test <test_name>                        # By substring
cargo test --test {cli,workflow,doctor,mock}  # By contract suite
```
