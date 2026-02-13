#!/usr/bin/env bash
set -euo pipefail
# Install GitHub CLI

if command -v gh >/dev/null 2>&1; then
  echo "gh already installed: $(gh --version | head -1)"
  exit 0
fi

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "Unsupported platform for automated gh install: $(uname -s). Install gh manually." >&2
  exit 1
fi

if ! command -v apt-get >/dev/null 2>&1; then
  echo "apt-get is required to install gh automatically. Install gh manually for this environment." >&2
  exit 1
fi

if [[ "$(id -u)" -eq 0 ]]; then
  apt-get update
  apt-get install -y --no-install-recommends gh
elif command -v sudo >/dev/null 2>&1; then
  sudo apt-get update
  sudo apt-get install -y --no-install-recommends gh
else
  echo "Root or sudo is required to install gh with apt-get. Install gh manually." >&2
  exit 1
fi

gh --version
