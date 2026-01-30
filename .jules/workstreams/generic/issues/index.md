# Issues Index

This registry tracks active issues in this workstream.
It serves as the central source of truth for the **Decider** to deduplicate observations.

## High Priority
> Critical blockers and major defects in [`high/`](./high/).

| Issue | Summary |
| :--- | :--- |
| [Critical Missing Unit Tests in ArboardClipboard](./high/qa_missing_coverage.yml) | The ArboardClipboard component lacks any unit tests. |

## Medium Priority
> Standard bugs and improvements in [`medium/`](./medium/).

| Issue | Summary |
| :--- | :--- |
| [Domain Model Impurity and Coupling](./medium/arch_domain_purity.yml) | Domain models are coupled to infrastructure (serde), use primitive types, and lack validation. |
| [Service Layer Mixed with Infrastructure Adapters](./medium/arch_service_boundaries.yml) | The `src/services/` directory incorrectly mixes Domain Services with Infrastructure Adapters. |
| [Inconsistent Command Implementation Pattern](./medium/arch_command_structure.yml) | The `workstream` command logic is misplaced in `src/lib.rs` instead of `src/app/commands/`. |
| [Inconsistent and Outdated Documentation](./medium/documentation_consistency.yml) | Documentation (README.md) is out of sync with codebase capabilities (commands, assets). |

## Low Priority
> Minor tweaks and housekeeping in [`low/`](./low/).

| Issue | Summary |
| :--- | :--- |
| [Naming Inconsistencies](./low/consistency_naming.yml) | Inconsistent naming between Ports and Adapter implementations. |

<!--
Instructions for Decider:
1. Populate each section with issues from `high/`, `medium/`, and `low/` directories.
2. Format as `| [Title](./path/to/issue.yml) | Summary content |`.
3. Keep this index in sync with the file system.
-->
