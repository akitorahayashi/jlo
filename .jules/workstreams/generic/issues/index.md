# Issues Index

This registry tracks active issues in this workstream.
It serves as the central source of truth for the **Decider** to deduplicate observations.

## Feats
> New feature specifications in [`feats/`](./feats/).

| Issue | Summary |
| :--- | :--- |
| [Enable Cross-Platform CI](./feats/enable-cross-platform-ci.yml) | Update CI/CD pipelines to build and test on macOS and ARM targets, and promote these artifacts in the release workflow. |

## Refacts
> Code improvements and technical debt in [`refacts/`](./refacts/).

| Issue | Summary |
| :--- | :--- |
| [Optimize Dependency Resolution Performance](./refacts/optimize-dependency-resolution.yml) | Improve the performance of `Resolver::resolve` by reducing unnecessary cloning. |
| [Refactor Error Handling](./refacts/refactor-error-handling.yml) | Refactor `AppError` to replace stringly-typed variants like `ConfigError(String)` with structured error types. |
| [Fix Inconsistent Service Naming and Structure](./refacts/fix-service-naming-structure.yml) | Standardize service naming and structure in `src/services/` to match filenames and architectural patterns. |
| [Optimize Installer Verification](./refacts/optimize-installer-verification.yml) | Add path filtering to the `verify-installers.yml` workflow to prevent it from running on unrelated changes. |
| [Refactor Scaffold Manifest Service](./refacts/refactor-scaffold-manifest.yml) | Improve cohesion in `src/services/scaffold_manifest.rs` by separating domain logic from hashing utilities. |
| [Fix Service Layer Violation](./refacts/fix-service-layer-violation.yml) | Remove the dependency of `EmbeddedComponentCatalog` service on `crate::app::config::ComponentMeta`. |

## Bugs
> Defect reports and fixes in [`bugs/`](./bugs/).

| Issue | Summary |
| :--- | :--- |
| [Fix Silent Asset Loading Failures](./bugs/fix-silent-asset-failures.yml) | Ensure `EmbeddedRoleTemplateStore` and `workstream_template_assets` handle non-UTF8 files correctly. |

## Tests
> Test coverage and infrastructure changes in [`tests/`](./tests/).

| Issue | Summary |
| :--- | :--- |
| [Test Workstream Assets](./tests/test-workstream-assets.yml) | Add unit tests for `src/services/workstream_template_assets.rs` to ensure workstream templates are served correctly. |

## Docs
> Documentation updates in [`docs/`](./docs/).

| Issue | Summary |
| :--- | :--- |
| [Document Update Flag](./docs/document-update-flag.yml) | Update `AGENTS.md` to include the `--adopt-managed` flag for the `jlo update` command. |
| [Fix CLI Docs Inconsistencies](./docs/fix-cli-docs-inconsistencies.yml) | Update documentation for `jlo schedule export` and `jlo workstreams inspect` to reflect mandatory arguments. |

<!--
Instructions for Decider:
1. Populate each section with issues from `feats/`, `refacts/`, `bugs/`, `tests/`, and `docs/` directories.
2. Format as `| [Title](./path/to/issue.yml) | Summary content |`.
3. Keep this index in sync with the file system.
-->
