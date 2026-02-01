# Consistency State

## Analysis: 2026-02-01

Previous findings (1-4) have been resolved or verified as consistent.

Identified 3 new consistency issues between documentation and implementation:

1.  **Undocumented Feature**: `AGENTS.md` lists the `jlo update` command but omits the `--adopt-managed` flag, which is present in the implementation and `README.md`.
2.  **Broken Example**: `jlo schedule export` is documented without arguments in both `README.md` and `AGENTS.md`, but the implementation requires `--scope`.
3.  **Broken Example**: `jlo workstreams inspect` is documented without arguments in both `README.md` and `AGENTS.md`, but the implementation requires `--workstream`.

Events created for these findings in `.jules/workstreams/generic/events/pending/`.
