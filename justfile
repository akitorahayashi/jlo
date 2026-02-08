set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

setup:
    aqua i

render-remote:
    cargo run -- workflow render remote --output .tmp/workflow-kit-render/remote --overwrite

render-self-hosted:
    cargo run -- workflow render self-hosted --output .tmp/workflow-kit-render/self-hosted --overwrite

alint:
    just render-remote
    just render-self-hosted
    aqua exec -- actionlint .tmp/workflow-kit-render/remote/.github/workflows/*.yml
    aqua exec -- actionlint .tmp/workflow-kit-render/self-hosted/.github/workflows/*.yml
