# Issues Index

This registry tracks active issues in this workstream.
It serves as the central source of truth for the **Decider** to deduplicate observations.

## Feats
> New feature specifications in [`feats/`](./feats/).

| Issue | Summary |
| :--- | :--- |
| _No open issues_ | - |

## Refacts
> Code improvements and technical debt in [`refacts/`](./refacts/).

| Issue | Summary |
| :--- | :--- |
| [Architecture and Layer Boundary Violations](./refacts/architecture-and-boundaries.yml) | App layer bypasses ports, services lack cohesion, and internal app module is public. |
| [Performance and IO Decoupling](./refacts/performance-and-io-decoupling.yml) | ArtifactGenerator and DependencyResolver mix IO with logic and perform inefficient cloning. |
| [Scaffold and Template Logic Refactoring](./refacts/scaffold-and-template-logic.yml) | EmbeddedRoleTemplateStore misnamed, mixes scaffold/template responsibilities; ScaffoldManifest lacks cohesion. |
| [Error Typing and Service Naming Consistency](./refacts/error-typing-and-consistency.yml) | AppError uses stringly-typed variants, and HttpJulesClient violates naming conventions. |

## Bugs
> Defect reports and fixes in [`bugs/`](./bugs/).

| Issue | Summary |
| :--- | :--- |
| [Asset Loading and Validation Robustness](./bugs/asset-loading-robustness.yml) | Asset loading fails silently on non-UTF8 files, and integrity tests are weak. |

## Tests
> Test coverage and infrastructure changes in [`tests/`](./tests/).

| Issue | Summary |
| :--- | :--- |
| [CI/CD Infrastructure Improvements](./tests/ci-cd-infrastructure-improvements.yml) | CI pipeline is monolithic, lacks cross-platform checks, has manual installer verification, and partial artifact promotion. |
| [Test Isolation and Coverage](./tests/test-isolation-and-coverage.yml) | CLI tests leak global state, and DependencyResolver lacks property-based tests. |

## Docs
> Documentation updates in [`docs/`](./docs/).

| Issue | Summary |
| :--- | :--- |
| [Documentation Inconsistencies](./docs/documentation-inconsistencies.yml) | Inaccuracies in AGENTS.md and README.md regarding templates, run aliases, and inspect format. |

<!--
Instructions for Decider:
1. Populate each section with issues from `feats/`, `refacts/`, `bugs/`, `tests/`, and `docs/` directories.
2. Format as `| [Title](./path/to/issue.yml) | Summary content |`.
3. Keep this index in sync with the file system.
-->
