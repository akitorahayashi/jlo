# Workflow Template Consolidation Task

## Objective
Reduce maintenance cost in `src/assets/workflows/.github/workflows/jules-workflows*` by removing repetitive `wait` and `run` component files and generating jobs through reusable Jinja macros + loop-based configuration.

## Current State
- `jules-workflows.yml.j2` is already split into component includes.
- Wait jobs are partially abstracted via:
  - `src/assets/workflows/.github/workflows/jules-workflows/macros/wait_jobs.j2`
  - `src/assets/workflows/.github/workflows/jules-workflows/components/wait-after-*.yml.j2`
- Legacy wait action has been removed:
  - `src/assets/workflows/.github/actions/wait/action.yml` (deleted)
- Workflow renderer supports includes/partials from:
  - `src/adapters/assets/workflow_kit_assets/`

## Required Work
1. Rename macro file for single-responsibility naming clarity.
   - Candidate: `wait_job.j2` or `wait_job_macro.j2` (not plural `jobs`).

2. Remove per-wait component files.
   - Replace `components/wait-after-*.yml.j2` with loop-driven generation in top-level template.
   - Keep job order and dependency readability in `jules-workflows.yml.j2`.

3. Abstract run-family jobs similarly.
   - Target duplicated patterns in:
     - `run-innovators-1`
     - `run-observers`
     - `run-innovators-2`
     - `run-deciders`
     - `run-planners`
     - `run-implementers`
   - Use macro(s) + declarative config list and Jinja loop.
   - Keep job-specific differences explicit in config (needs, if, matrix input key, layer, extra env, sort/unique behavior, log labels).

4. Reduce component file count materially.
   - Keep only components that are genuinely unique (for example: narrator, matrix generation, publish proposals).
   - Delete obsolete per-job component files after migration.

5. Preserve behavior exactly.
   - Do not change workflow semantics, dependencies, entry-point behavior, mock behavior, or wait timing behavior.

## Constraints
- Do not reintroduce `.github/actions/wait`.
- Do not move final output path:
  - `.github/workflows/jules-workflows.yml`
- Keep compatibility with schedule/wait_minutes preservation logic in:
  - `src/app/commands/init_workflows.rs`
- Keep partial templates non-emittable under current render plan.

## Acceptance Criteria
- Rendered workflow is valid YAML.
- Installed workflow does not contain `{% include` or template artifacts.
- Installed workflow does not create `.github/workflows/jules-workflows/components`.
- Installed workflow does not create `.github/actions/wait/action.yml`.
- Existing behavior-focused tests still pass.

## Verification Commands
```bash
cargo fmt
cargo test --test workflow_kit
cargo test workflow_kit_assets
```

## Deliverables
- Updated templates and deleted obsolete template files.
- Any minimal test updates required to lock in the new structure.
- Short summary of which files were removed and how job generation is now configured.
