# DevOps State Analysis

## Overview
The project uses GitHub Actions for CI/CD. The pipeline is functional but exhibits significant maturity gaps regarding reproducibility, artifact management, and testing coverage.

## Key Findings

### 1. Toolchain Drift
- **Observation:** CI runs on `stable` (via `.github/actions/setup`), while Release runs on `1.90.0` (via `release.yml`). `rust-toolchain.toml` declares `1.90.0` but is overridden in CI.
- **Risk:** Code may pass CI but fail release builds or behave differently in production.
- **Status:** Resolved. `setup/action.yml` now defaults to `1.90.0`.

### 2. Artifact Management
- **Observation:** Binaries are rebuilt from source during the release process for macOS and ARM targets. Only Linux x64 is promoted.
- **Risk:** The released binary for non-Linux-x64 targets is not the exact byte-sequence that was tested (if tested at all).
- **Status:** Event `partial-artifact-promotion`.

### 3. Testing Gaps
- **Observation:** Installer scripts in `src/assets/catalog/` are only verified via a manual `workflow_dispatch` trigger.
- **Risk:** Regressions in installers may go undetected until a user reports them or a manual test is run.
- **Status:** Resolved. `verify-installers.yml` now runs on `pull_request`.

### 4. Supply Chain & Security
- **Observation:** Dependencies are locked (`Cargo.lock`).
- **Observation:** `cross` is used for cross-compilation with locked installation.
- **Observation:** No evidence of artifact signing or SBOM generation in `release.yml`.

### 5. Pipeline Efficiency
- **Observation:** Monolithic CI workflow (`ci-workflows.yml`) runs on every push without path filtering.
- **Observation:** `verify-installers.yml` runs on every PR without path filtering, executing expensive setup steps for unrelated changes.
- **Status:** Event `inefficient-installer-verification`.

### 6. Platform Coverage
- **Observation:** CI (`build.yml`) only runs on `ubuntu-latest`. Release supports macOS and ARM.
- **Risk:** Cross-compilation failures or platform-specific bugs are not detected until release.
- **Status:** Event `missing-cross-platform-ci`.

## Recommendations
1. **Unify Toolchain:** Update `.github/actions/setup` to respect `rust-toolchain.toml` or explicitly pin the version to match `release.yml`. (Done)
2. **Promote Artifacts:** Modify `build.yml` to upload artifacts and `release.yml` to download them.
3. **Automate Installer Tests:** Add a `pull_request` trigger to `verify-installers.yml` with path filtering for `src/assets/catalog/`. (Trigger added, filtering pending)
