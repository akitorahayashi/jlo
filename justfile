set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

REMOTE_GEN_DIR := ".tmp/workflow-scaffold-generate/remote"
SELF_HOSTED_GEN_DIR := ".tmp/workflow-scaffold-generate/self-hosted"

setup:
    aqua i

generate-remote:
    cargo run -- workflow generate remote --output-dir {{ REMOTE_GEN_DIR }}

generate-self-hosted:
    cargo run -- workflow generate self-hosted --output-dir {{ SELF_HOSTED_GEN_DIR }}

alint:
    just generate-remote
    just generate-self-hosted
    aqua exec -- actionlint {{ REMOTE_GEN_DIR }}/.github/workflows/*.yml
    aqua exec -- actionlint {{ SELF_HOSTED_GEN_DIR }}/.github/workflows/*.yml
