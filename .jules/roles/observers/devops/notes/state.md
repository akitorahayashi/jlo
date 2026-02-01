# DevOps State Analysis

## Overview
The project uses GitHub Actions for CI/CD. The pipeline is functional but exhibits significant maturity gaps regarding reproducibility, artifact management, and testing coverage.

## Key Findings

### 1. Toolchain Drift
- **Observation:** CI runs on `stable` (via `.github/actions/setup`), while Release runs on `1.90.0` (via `release.yml`). `rust-toolchain.toml` declares `1.90.0` but is overridden in CI.
- **Risk:** Code may pass CI but fail release builds or behave differently in production.
- **Status:** Pending Event `dft001`.

### 2. Artifact Management
- **Observation:** Binaries are rebuilt from source during the release process. There is no "build once, promote everywhere" strategy. Intermediate artifacts from CI are discarded.
- **Risk:** The released binary is not the exact byte-sequence that was tested.
- **Status:** Pending Event `rel001`.

### 3. Testing Gaps
- **Observation:** Installer scripts in `src/assets/catalog/` are only verified via a manual `workflow_dispatch` trigger.
- **Risk:** Regressions in installers may go undetected until a user reports them or a manual test is run.
- **Status:** Pending Event `ins001`.

### 4. Supply Chain & Security
- **Observation:** Dependencies are locked (`Cargo.lock`).
- **Observation:** `cross` is used for cross-compilation with locked installation.
- **Observation:** No evidence of artifact signing or SBOM generation in `release.yml`.

### 5. Pipeline Efficiency
- **Observation:** Monolithic CI workflow (`ci-workflows.yml`) runs on every push without path filtering.
- **Note:** Acceptable for current scale but likely to become a bottleneck.

## Recommendations
1. **Unify Toolchain:** Update `.github/actions/setup` to respect `rust-toolchain.toml` or explicitly pin the version to match `release.yml`.
2. **Promote Artifacts:** Modify `build.yml` to upload artifacts and `release.yml` to download them.
3. **Automate Installer Tests:** Add a `pull_request` trigger to `verify-installers.yml` with path filtering for `src/assets/catalog/`.
