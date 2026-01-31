# Observer Role Index

This index exists to make role selection fast and to clarify boundaries between similar roles.

Scheduling is configured per workstream in `workstreams/<workstream>/scheduled.toml`.

## Repository Structure And Architecture

- [`structural_arch`](./structural_arch/role.yml): Repository/module placement, dependency direction, boundaries, and unidirectional flow.
- [`data_arch`](./data_arch/role.yml): Data models, invariants, validation, and data flow (SSOT, conversions, schema evolution).

## Project Language And Documentation

- [`taxonomy`](./taxonomy/role.yml): Vocabulary and naming system (concept boundaries, canonical terms, collisions, rename strategy).
- [`consistency`](./consistency/role.yml): Truth drift between docs and implementation (broken examples, stale docs, contradictions).

## Testing

- [`qa`](./qa/role.yml): Test structure and quality (boundaries, determinism, diagnosability, feedback-speed design).
- [`cov`](./cov/role.yml): Coverage as a risk signal (critical-path weighting, diff coverage, meaningful exclusions).

## Delivery And Operations

- [`devops`](./devops/role.yml): CI/CD design (reproducibility, fast/slow paths, supply chain, artifact promotion, rollback/traceability).
- [`observability`](./observability/role.yml): Runtime diagnosability (questions, correlation keys, causality, cost/PII constraints).

## UI And Product Experience

- [`ui_designer`](./ui_designer/role.yml): Intent clarity, hierarchy, interaction contract, cognitive load, robustness.

## Language Specialists (Bring When The Repo Matches)

- [`rustacean`](./rustacean/role.yml): Rust ownership/borrowing, error models, lifetimes, concurrency, trait design.
- [`swifter`](./swifter/role.yml): Swift types/state modeling, ownership/lifetimes, concurrency isolation, static abstractions.
- [`typescripter`](./typescripter/role.yml): TypeScript boundary validation, unions, failure semantics, module hygiene.
- [`pythonista`](./pythonista/role.yml): Python boundaries/types, exception design, I/O separation, performance, imports/config/deps.
- [`gopher`](./gopher/role.yml): Go errors, context propagation, goroutine lifecycle, small interfaces, zero-value safety.
