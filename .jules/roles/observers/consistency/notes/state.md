# Consistency State

## Analysis: 2026-02-01

Identified 4 consistency issues between documentation and implementation:

1.  **Undocumented Feature**: The `jlo update` command supports an `--adopt-managed` flag which is not documented in the `README.md`.
2.  **Misleading CLI Help**: The `jlo template` command help text lists single-role layers (planners, implementers) as valid options, but the implementation explicitly blocks them.
3.  **Inconsistent Configuration**: The default API URL is `https://jules.googleapis.com/v1alpha/sessions` in the code, but the `README.md` implies it is `https://api.jules.ai/v1/sessions`.
4.  **Outdated Documentation**: `AGENTS.md` lists removed dependencies (`arboard`) and omits new ones (`reqwest`, `serde_json`, `url`).

Events created for these findings in `.jules/workstreams/generic/events/pending/`.
