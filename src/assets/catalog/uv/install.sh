#!/usr/bin/env bash
set -euo pipefail
# Install uv - Python package installer

if command -v uv >/dev/null 2>&1; then
  echo "uv already installed: $(uv --version)"
  exit 0
fi

curl -LsSf https://astral.sh/uv/install.sh | sh

# Source the env to make uv available
if [[ -f "$HOME/.local/bin/env" ]]; then
  # shellcheck disable=SC1091
  . "$HOME/.local/bin/env"
fi

uv --version
