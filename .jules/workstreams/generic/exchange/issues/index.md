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
| [Excessive allocation in template rendering](./refacts/excessive-allocation-in-template-rendering.yml) | The render_template function re-initializes Minijinja Environment for every call. |
| [Inefficient git output parsing](./refacts/inefficient-git-output-parsing.yml) | Git output parsing logic allocates intermediate vectors and strings for every line. |
| [Public leak of internal app module](./refacts/public-leak-of-internal-app-module.yml) | The internal `app` module is publicly exposed via `pub mod app;` in `src/lib.rs`. |
| [Weak error model in Narrator command](./refacts/weak-error-model-in-narrator-command.yml) | The Narrator command implementation uses a generic stringified error model. |

## Bugs
> Defect reports and fixes in [`bugs/`](./bugs/).

| Issue | Summary |
| :--- | :--- |
| _No open issues_ | - |

## Tests
> Test coverage and infrastructure changes in [`tests/`](./tests/).

| Issue | Summary |
| :--- | :--- |
| _No open issues_ | - |

## Docs
> Documentation updates in [`docs/`](./docs/).

| Issue | Summary |
| :--- | :--- |
| _No open issues_ | - |

<!--
Instructions for Decider:
1. Populate each section with issues from `feats/`, `refacts/`, `bugs/`, `tests/`, and `docs/` directories.
2. Format as `| [Title](./path/to/issue.yml) | Summary content |`.
3. Keep this index in sync with the file system.
-->
