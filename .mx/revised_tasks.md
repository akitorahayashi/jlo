# Revised Task List: Actions-Driven Workflow Integration

## Critical Analysis

### Original Plan Adjustments

1. **`implementer.yml` asset creation**: The original plan proposes creating `src/assets/prompts/implementer.yml`. However, this conflicts with jlo's design principle that "jlo is scaffold + prompt/config management only" and implementation is "invoked via GitHub Issues with `jules` label." The implementer workflow (`.mx/sample.md`) is a repository-local workflow prompt, not a jlo-distributed asset. **Decision: Skip implementer.yml creation.** Implementers read tasks from `exchange/tasks/` and follow repository-specific instructions.

2. **Scaffold documentation updates**: The scaffold's README.md and JULES.md already clarify that "Execution + git + PR operations are out of scope" and implementation is via GitHub Issues. No changes needed there.

3. **CLI alias in help output**: The `--help` output test checks for `[aliases: a]`. This test must be removed since `assign` is being deleted.

---

## Implementation Tasks

### Phase 1: Remove `assign` Command

- [ ] **1.1** Delete `src/app/commands/assign.rs`
- [ ] **1.2** Remove `pub mod assign;` from `src/app/commands/mod.rs`
- [ ] **1.3** Remove `Assign` variant from `Commands` enum in `src/main.rs`
- [ ] **1.4** Remove `Commands::Assign` match arm in `src/main.rs`
- [ ] **1.5** Remove `assign()` function from `src/lib.rs` (lines 34-74)
- [ ] **1.6** Remove `use` of `ArboardClipboard` in `src/lib.rs` (no longer needed)

### Phase 2: Update Tests

- [ ] **2.1** Remove `assign_fails_without_workspace` and `assign_fails_for_unknown_role` from `tests/cli_commands.rs`
- [ ] **2.2** Update `help_lists_visible_aliases` to remove `[aliases: a]` check
- [ ] **2.3** Remove `assign_without_workspace_fails` from `tests/commands_core.rs`

### Phase 3: Update Documentation

- [ ] **3.1** Update `README.md`:
  - Remove `assign` from Quick Start example
  - Remove `assign` row from Commands table
  - Remove `assign` from Examples section
- [ ] **3.2** Update `AGENTS.md`:
  - Update Project Structure (remove `assign` from comments)
  - Remove `assign` from CLI Commands section

### Phase 4: Verify

- [ ] **4.1** Run `cargo build`
- [ ] **4.2** Run `cargo fmt --check`
- [ ] **4.3** Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] **4.4** Run `cargo test --all-targets --all-features`
- [ ] **4.5** Verify `jlo assign` returns unknown command error
