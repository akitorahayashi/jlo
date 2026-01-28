# jlo Update Plan: Actions-Driven Workflow Integration

## Overview

This plan transitions `jlo` from a manual prompt-copy tool to a **scaffold + contract management CLI** optimized for GitHub Actions + jules-invoke orchestration.

### Role Boundaries (Post-Update)

| Component | Responsibility |
|-----------|----------------|
| **jlo** | `.jules/` scaffold installation, versioning, migration, prompt asset management |
| **GitHub Actions** | Orchestration: cron triggers, matrix execution, PR creation, merge control |
| **jules-invoke** | Session creation: prompt delivery, starting_branch specification |
| **Jules (VM)** | Execution: code analysis, artifact generation, branch/PR creation |

### Branch Strategy (Recommended)

**Preferred**: Create `jules` branch from `main`, run `jlo init` there. Discussions branch from `jules`, implementation branches from `main`.
**Alternative**: Run `jlo init` directly on `main`. Both approaches function correctly.

---

## Changes

### 1. Remove `assign` Command

**Rationale**: New flow uses Actions + jules-invoke for all invocations. Clipboard-based prompt copying has no role.

**Files to modify**:
- `src/main.rs` - Remove `Assign` variant from CLI enum
- `src/app/commands/mod.rs` - Remove `assign` module export
- `src/app/commands/assign.rs` - Delete file
- `src/lib.rs` - Remove `assign` from public API
- `README.md` - Remove `assign` from command table
- `AGENTS.md` - Remove `assign` from CLI Commands section
- `tests/` - Remove/update tests referencing `assign`

**Verification**: `cargo build`, `cargo test`, confirm `jlo assign` returns "unknown command"

---

### 2. Create Implementer Prompt Assets

**Rationale**: Planners produce `task.yml`. Implementers need structured execution prompts for critical scope review.

**Location**: `src/assets/prompts/implementer.yml`

**Content structure** (based on `.mx/sample.md`):

```yaml
layer: implementer
version: 1

workflow:
  - step: analyze_goal
    instruction: |
      Study the provided task.yml and understand the goal.
      
  - step: critical_scope_review
    instruction: |
      Critically review the scope of edits needed.
      Consider what might be missing in the plan.
      Ensure sufficient editing is contemplated for the goal.
      
  - step: audit_tests
    instruction: |
      Review test structure to identify required additions or updates.
      
  - step: identify_documentation
    instruction: |
      Determine which existing documentation will need updates.
      Follow project documentation culture.
      
  - step: implement
    instruction: |
      Execute all changes defined in the task, including:
      - Code modifications
      - Test additions/updates
      - Documentation updates
      Complete the entire implementation without interruption.
      
  - step: verify
    instruction: |
      Run tests and validate that all parts of the task are complete.

constraints:
  - Backward compatibility is not a primary concern during migration
  - Transition should be simple and free of technical debt
  - Do not stop after planning - execute the full workflow
```

**Files to create**:
- `src/assets/prompts/` directory
- `src/assets/prompts/implementer.yml`

---

### 3. Update Documentation

#### 3.1 Root README.md

**Changes**:
- Remove `assign` command from table
- Add "Branch Strategy" section with recommended vs alternative approaches
- Add reference to Actions integration

#### 3.2 AGENTS.md

**Changes**:
- Remove `assign` from CLI Commands
- Update Project Summary to reflect Actions-driven model
- Add note that prompts are in `src/assets/prompts/`

#### 3.3 `.jules/README.md` (scaffold)

**Changes**:
- Clarify that execution/git/PR operations are orchestration's responsibility
- Keep content focused on artifacts and contracts
- No branch strategy specifics (that's Actions' domain)

#### 3.4 `.jules/JULES.md` (scaffold)

**Changes**:
- No changes needed for branch strategy
- Ensure implementer role flow is documented if missing

---

### 4. Scaffold Structure Updates

#### 4.1 Add Implementer Layer (Optional)

Currently the scaffold has:
- `roles/observers/`
- `roles/deciders/`
- `roles/planners/`

**Question**: Should `roles/implementers/` be added to scaffold?

**Recommendation**: No. Implementers are invoked via GitHub Issues with `jules` label. They read `exchange/tasks/` but don't have persistent role state. The `src/assets/prompts/implementer.yml` serves as their execution prompt, delivered via `jules-invoke`.

---

### 5. Dogfooding: jlo Repository as Reference

**Approach**: The `jlo` repository itself will implement the Actions-driven workflow. Other repositories can reference `jlo/.github/workflows/` as a working example.

**Not included in scaffold**: `.github/workflows/` files are **not** part of `jlo init`. Each project configures its own workflows based on the jlo reference implementation.

---

## Implementation Order

1. **Phase 1: Cleanup**
   - Remove `assign` command and all references
   - Update tests
   - Verify build

2. **Phase 2: Prompt Assets**
   - Create `src/assets/prompts/` directory
   - Add `implementer.yml`
   - (Future: other role prompts if needed)

3. **Phase 3: Documentation**
   - Update README.md, AGENTS.md
   - Update scaffold's README.md and JULES.md

---

## Verification Checklist

- [ ] `cargo build` succeeds
- [ ] `cargo test --all-targets --all-features` passes
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `jlo init` creates valid scaffold
- [ ] `jlo assign` returns error (command removed)
- [ ] Documentation accurately reflects new model

---

## Out of Scope

- `jlo validate` / `jlo lint` commands (future work)
- `jlo render` command (not needed; prompts delivered via jules-invoke)
- Implementer role in scaffold (invoked via GitHub Issues, not scheduled)
