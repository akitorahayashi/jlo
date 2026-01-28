#!/usr/bin/env bash
set -euo pipefail
# Install uv - Python package installer

if command -v uv >/dev/null 2>&1; then
  echo "uv already installed: $(uv --version)"
  exit 0
fi

curl -LsSf https://astral.sh/uv/install.sh | sh

# Add uv to the PATH for this script's execution
export UV_HOME="${UV_HOME:-$HOME/.uv}"
export PATH="$UV_HOME/bin:$PATH"

uv --version
