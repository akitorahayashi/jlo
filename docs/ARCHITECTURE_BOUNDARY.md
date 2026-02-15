# Architecture Boundary Model

## Purpose

This document defines the canonical architecture vocabulary, dependency boundaries, and technical quality policies for `src/`.
It is the source of truth for structural decisions used by implementation and migration tasks.

## Quality & Compatibility Policy

1. **Zero Technical Debt Carry-over**: Achieve the target architecture with no leftover legacy structures.
2. **Phase-Locked Removal**: Any replaced structure must be removed in the same phase that introduces its replacement.
3. **No Hidden Fallbacks**: Fallback behavior is opt-in, explicit, and observable; implicit fallback is prohibited.
4. **No Migration Shims**: Temporary migration shims, alias modules, and compatibility adapters are prohibited in Final State code.
5. **Clean Architecture Search**: Completion of a phase is verified when search for legacy module/path references returns zero matches.

## Canonical Vocabulary

The codebase uses the following words as primary identifiers:

- `repository_root`: repository base path.
- `jlo`: control-branch intent files under `.jlo/`.
- `jules`: worker-branch runtime files under `.jules/`.
- `domain`: pure business rules and structure semantics.
- `ports`: boundary contracts used by application use cases (`Git`, `GitHub`, `RepositoryFilesystem`, `JloStore`, `JulesStore`).
- `adapters`: external I/O implementations of ports.
- `app`: command/use-case orchestration.
- `testing`: in-process test doubles and test-only builders.

`workspace` is not a primary code identifier for new modules, traits, or adapter types.

## Boundary Contracts

The architecture follows this dependency direction:

- `app -> domain`
- `app -> ports`
- `app -> adapters` (wiring only; behavior is consumed through `ports` traits)
- `adapters -> ports`
- `testing -> domain | ports | app (test-only)`

The following dependencies are prohibited:

- `domain -> adapters`
- `domain -> testing`
- `adapters -> adapters` (direct cross-adapter coupling)
- `ports -> adapters`

## jlo/jules Ownership

`jlo` and `jules` structure semantics are owned by capability-focused modules under `src/domain/`. Ownership is segmented by business capability.

### Domain Capability Partition

| Capability | Ownership Description |
|------------|-----------------------|
| `setup/` | Component models, dependency resolution, artifact generation, and setup config. |
| `prompt_assembly/` | Prompt contracts, loading semantics, and composition errors. |
| `roles/` | Role identity, roster semantics, and level invariants. |
| `events/` | Event schema, state model, parsing, and validation. |
| `requirements/` | Requirement schema, parse/validation, and requirement-event linkage. |
| `ideas/` | Innovator idea/proposal domain models and lifecycle semantics. |
| `workstations/` | State schema, perspective constraints, and pure-policy lifecycle (Ensure/Prune). |
| `layers/` | Layer identity, metadata, and contract semantics. |
| `config/` | `config.toml` domain model, parser, and pure validation policies. |
| `schedule/` | `scheduled.toml` domain model, parser, and pure validation policies. |

### Domain Error Model

1. Each capability directory defines a local `error.rs` for internal domain errors.
2. `src/domain/error.rs` is the integration error type (`AppError`) that composes capability errors via `From`.
3. Capability code returns local error types and maps to `AppError` at domain/app boundaries.

### Dependency Rules

1. Capability modules MUST NOT import from `app` or `adapters`.
2. Cross-capability imports must be explicit and minimal.
3. Common utilities used by only one capability must remain encapsulated in that capability.

## Module Responsibilities

- `domain`: parsing, validation, identifiers, and path semantics. No process execution, environment probing, filesystem mutation, or network/tool calls.
- `ports`: contract types and traits required by application use cases.
- `adapters`: concrete implementations for git, GitHub, local repository I/O, HTTP clients, and embedded catalogs.
- `app`: command workflows, orchestration, and policy decisions.

### App Input & Configuration Boundaries

To prevent configuration sprawl and implicit I/O in the orchestration layer:

1. **Input Boundaries**: Each command family (e.g., `run`, `workflow run`) owns a local `input.rs` for repository/environment loading.
2. **Normalization Layer**: `src/app/config/` is the authoritative normalization layer. It performs filesystem/environment reads but delegates all parsing and validation to domain capability modules.
3. **No Direct Domain I/O**: Application logic reads data through designated input boundaries or normalization layers, never through direct domain-level file I/O.

## Testing Model

Testing is split by execution mode:

- `src/testing/`: unit-level in-process test support.
- `tests/harness/`: integration-level black-box harness using real filesystem/process boundaries.

`src/testing/` is organized by boundary ownership:

- `src/testing/ports/`: port test doubles.
- `src/testing/app/`: use-case test builders/helpers.
- `src/testing/domain/`: domain test data builders/helpers.

`tests/harness/` remains independent from `src/testing/` and is not treated as a duplicate.
Each serves a distinct test layer.

## Structural Invariants

- One responsibility has one owner module.
- Duplicate loaders/parsers across `domain`, `app`, and `adapters` are not allowed.
- New filesystem layout logic for `.jlo/` or `.jules/` is added only in the corresponding
  `src/domain/{workstations,config,schedule,roles,layers,exchange}/` owner module.
- Migration steps may be incremental, but each step preserves these boundaries.
