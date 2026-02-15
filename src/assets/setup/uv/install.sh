# ==============================================================================
# Install uv - Python package installer
# ==============================================================================

if command -v uv >/dev/null 2>&1; then
  echo "uv already installed: $(uv --version)"
  exit 0
fi

curl -LsSf https://astral.sh/uv/install.sh | sh

# Add uv to the PATH for this script's execution (standard install locations)
export PATH="$HOME/.local/bin:$HOME/.uv/bin:$PATH"

uv --version
