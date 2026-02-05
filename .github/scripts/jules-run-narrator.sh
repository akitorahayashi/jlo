#!/usr/bin/env bash
set -euo pipefail

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "::error::Required command not found: $name"
    exit 1
  fi
}

require_command jlo

echo "Running narrator"
# shellcheck disable=SC2086
jlo run narrator ${JLO_RUN_FLAGS:-}
