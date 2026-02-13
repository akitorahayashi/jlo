#!/usr/bin/env bash
set -euo pipefail
# Install just - command runner

if command -v just >/dev/null 2>&1; then
  echo "just already installed: $(just --version)"
  exit 0
fi

mkdir -p "$HOME/.local/bin"
export PATH="$HOME/.local/bin:$PATH"

curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh \
  | bash -s -- --to "$HOME/.local/bin"

just --version
