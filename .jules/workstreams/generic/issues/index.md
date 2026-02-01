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
| [Standardize Service Naming and Structure](./refacts/standardize-service-naming-and-structure.yml) | Inconsistent service naming, filename strategies, and misplaced domain entities create confusion. |
| [Clarify Domain Terminology](./refacts/clarify-domain-terminology.yml) | Ambiguous terms (Setup/Install, Scaffold/Template/Managed Defaults) and config mismatches need rationalization. |
| [Improve Error Modeling](./refacts/improve-error-modeling.yml) | AppError relies on a generic ConfigError(String) variant for distinct failure modes. |

## Bugs
> Defect reports and fixes in [`bugs/`](./bugs/).

| Issue | Summary |
| :--- | :--- |
| [Fix Silent Scaffold Loading Failure](./bugs/fix-silent-scaffold-loading-failure.yml) | EmbeddedRoleTemplateStore silently ignores non-UTF8 files. |
| [Fix CLI Template Command Inconsistency](./bugs/fix-cli-template-command-inconsistency.yml) | CLI help text lists single-role layers for template command, but implementation forbids them. |

## Tests
> Test coverage and infrastructure changes in [`tests/`](./tests/).

| Issue | Summary |
| :--- | :--- |
| [Improve Test Harness Isolation](./tests/improve-test-harness-isolation.yml) | Test harness modifies global process state, forcing serial execution and reducing reliability. |
| [Strengthen Resolver Testing](./tests/strengthen-resolver-testing.yml) | The dependency resolver lacks property-based tests to ensure correctness under complex graphs. |
| [Improve Template Validation](./tests/improve-template-validation.yml) | Tests for role templates use string matching instead of YAML parsing, risking invalid syntax. |

## Docs
> Documentation updates in [`docs/`](./docs/).

| Issue | Summary |
| :--- | :--- |
| [Inaccurate and Incomplete Documentation](./docs/inaccurate-and-incomplete-documentation.yml) | AGENTS.md and README.md contain stale, incomplete, or conflicting information regarding API URLs, dependencies, and command structure. |
| [Undocumented CLI Options](./docs/undocumented-cli-options.yml) | The jlo update command supports an --adopt-managed flag that is not documented in README.md. |

<!--
Instructions for Decider:
1. Populate each section with issues from `feats/`, `refacts/`, `bugs/`, `tests/`, and `docs/` directories.
2. Format as `| [Title](./path/to/issue.yml) | Summary content |`.
3. Keep this index in sync with the file system.
-->
