# Issues Index

This registry tracks active issues in this workstream.
It serves as the central source of truth for the **Decider** to deduplicate observations.

## High Priority
> Critical blockers and major defects in [`high/`](./high/).

| Issue | Summary |
| :--- | :--- |
| [Critical Missing Unit Tests in Core Modules](./high/qa_missing_coverage.yml) | Core components and adapters lack essential unit tests, leaving them vulnerable to regressions. |

## Medium Priority
> Standard bugs and improvements in [`medium/`](./medium/).

| Issue | Summary |
| :--- | :--- |
| [Domain Model Impurity and Coupling](./medium/arch_domain_purity.yml) | Domain entities lack validation and use primitive types in ports, violating Isomorphic Representation. |
| [Service Layer Mixed with Infrastructure Adapters](./medium/arch_service_boundaries.yml) | The `src/services/` directory incorrectly mixes Domain Services with Infrastructure Adapters, violating architectural boundaries. |
| [Inconsistent and Outdated Documentation](./medium/documentation_consistency.yml) | README.md is out of sync with the codebase (missing commands, wrong workflows). |

## Low Priority
> Minor tweaks and housekeeping in [`low/`](./low/).

| Issue | Summary |
| :--- | :--- |
| [Naming Inconsistencies and Missing Templates](./low/consistency_naming.yml) | The codebase has inconsistent layer naming (singular vs plural), filename mismatches, and missing templates for single-role layers. |

<!--
Instructions for Decider:
1. Populate each section with issues from `high/`, `medium/`, and `low/` directories.
2. Format as `| [Title](./path/to/issue.yml) | Summary content |`.
3. Keep this index in sync with the file system.
-->
