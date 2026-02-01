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
| [Decouple CLI Logic from Binary Entry Point](./refacts/decouple-cli-from-main.yml) | Move CLI structs and enums to a dedicated adapter module. |
| [Refactor and clean up Setup domain model](./refacts/domain-setup-improvements.yml) | Split setup.rs, separate concerns, and use strict types. |
| [Fix Dependency Inversion Violations in App Layer](./refacts/fix-dependency-inversion.yml) | Refactor commands to depend on traits defined in ports. |
| [Improve Error Handling and Remove Primitive Obsession](./refacts/improve-error-handling.yml) | Introduce structured error variants in AppError. |
| [Optimize Dependency Resolver Cloning](./refacts/optimize-resolver.yml) | Refactor Resolver to use references or lightweight metadata. |
| [Rename ManagedDefaults to reflect purpose](./refacts/rename-managed-defaults.yml) | Rename ManagedDefaultsManifest to ScaffoldManifest. |
| [Align Setup Command and Artifact Name](./refacts/rename-setup-artifact.yml) | Rename setup command or install.sh artifact for consistency. |
| [Restrict Public API Surface in lib.rs](./refacts/restrict-public-api.yml) | Reduce public exports in lib.rs to protect internal layers. |
| [Restructure Services and Clarify Terminology](./refacts/restructure-services.yml) | Clean up services directory, rename generic services, clarify scaffold/template. |

## Bugs
> Defect reports and fixes in [`bugs/`](./bugs/).

| Issue | Summary |
| :--- | :--- |
| [Fix Inconsistency in Template Command Support for Single Role Layers](./bugs/fix-template-command-inconsistency.yml) | Address mismatch between help text and implementation for single-role templates. |
| [Stop Rebuilding Artifacts in Release Workflow](./bugs/release-workflow-rebuild.yml) | Use verified build artifacts in release workflow instead of recompiling. |

## Tests
> Test coverage and infrastructure changes in [`tests/`](./tests/).

| Issue | Summary |
| :--- | :--- |
| [Add Property-Based Tests for Resolver](./tests/add-resolver-property-tests.yml) | Add proptest to verify resolver correctness. |
| [Automate Installer Verification](./tests/automate-installer-verification.yml) | Trigger installer verification on PRs automatically. |
| [Enable Code Coverage Collection](./tests/enable-coverage-collection.yml) | Add coverage tools to CI pipeline. |
| [Fix Global State Modification in Test Harness](./tests/fix-test-harness-isolation.yml) | Refactor TestContext to allow parallel execution. |
| [Pin CI Toolchain Version](./tests/pin-ci-toolchain.yml) | Ensure CI uses the same pinned toolchain as release. |

## Docs
> Documentation updates in [`docs/`](./docs/).

| Issue | Summary |
| :--- | :--- |
| [Fix discrepancies between README.md and implementation](./docs/fix-readme-discrepancies.yml) | Align README.md with code regarding API URLs and CLI flags. |
| [Update AGENTS.md to reflect current dependencies and project structure](./docs/update-agents-md.yml) | Sync AGENTS.md with actual Cargo.toml dependencies and remove obsolete clipboard references. |

<!--
Instructions for Decider:
1. Populate each section with issues from `feats/`, `refacts/`, `bugs/`, `tests/`, and `docs/` directories.
2. Format as `| [Title](./path/to/issue.yml) | Summary content |`.
3. Keep this index in sync with the file system.
-->
