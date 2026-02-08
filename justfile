set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

REMOTE_RENDER_DIR := ".tmp/workflow-kit-render/remote"
SELF_HOSTED_RENDER_DIR := ".tmp/workflow-kit-render/self-hosted"

setup:
    aqua i

render-remote:
    cargo run -- workflow render remote --output {{ REMOTE_RENDER_DIR }} --overwrite

render-self-hosted:
    cargo run -- workflow render self-hosted --output {{ SELF_HOSTED_RENDER_DIR }} --overwrite

alint:
    just render-remote
    just render-self-hosted
    aqua exec -- actionlint {{ REMOTE_RENDER_DIR }}/.github/workflows/*.yml
    aqua exec -- actionlint {{ SELF_HOSTED_RENDER_DIR }}/.github/workflows/*.yml
