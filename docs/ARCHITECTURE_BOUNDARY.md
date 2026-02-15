# Architecture Boundary Model

## Purpose

This document defines the canonical architecture vocabulary and dependency boundaries for `src/`.
It is the source of truth for structural decisions used by implementation and migration tasks.

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

`jlo` and `jules` structure semantics are owned by capability-focused modules under `src/domain/`:

- `src/domain/workstations/` for top-level `.jlo/.jules` constants and managed manifests.
- `src/domain/config/` and `src/domain/schedule/` for control-plane config/schedule paths and models.
- `src/domain/roles/` for `.jlo/roles/...` path semantics and role identifiers.
- `src/domain/layers/` for `.jules/layers/...` path semantics and prompt assembly.
- `src/domain/exchange/` for `.jules/exchange/...` path semantics.

Adapters do not own `jlo`/`jules` layout semantics.
Adapters implement I/O behavior only.

## Module Responsibilities

- `domain`: parsing, validation, identifiers, and path semantics. No process execution, environment probing, filesystem mutation, or network/tool calls.
- `ports`: contract types and traits required by application use cases.
- `adapters`: concrete implementations for git, GitHub, local repository I/O, HTTP clients, and embedded catalogs.
- `app`: command workflows, orchestration, and policy decisions that combine domain logic and ports.

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
