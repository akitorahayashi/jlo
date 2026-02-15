# ==============================================================================
# Install just - command runner
# ==============================================================================

if command -v just >/dev/null 2>&1; then
  installed_version="$(just --version | awk '{print $2}')"
  requested_version="${JUST_VERSION#v}"
  if [[ -z "${JUST_VERSION:-}" || "$installed_version" == "$requested_version" ]]; then
    echo "just already installed: $(just --version)"
    exit 0
  fi
fi

mkdir -p "$HOME/.local/bin"
export PATH="$HOME/.local/bin:$PATH"

if [[ -n "${JUST_VERSION:-}" ]]; then
  if ! command -v tar >/dev/null 2>&1; then
    echo "tar is required for version-pinned just installation." >&2
    exit 1
  fi

  version="${JUST_VERSION#v}"
  arch="$(uname -m)"
  case "$arch" in
    x86_64) target="x86_64-unknown-linux-musl" ;;
    aarch64|arm64) target="aarch64-unknown-linux-musl" ;;
    *)
      echo "Unsupported architecture for JUST_VERSION install: $arch" >&2
      exit 1
      ;;
  esac

  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' EXIT
  archive="$tmp_dir/just.tgz"
  curl --proto '=https' --tlsv1.2 -fsSL \
    "https://github.com/casey/just/releases/download/${version}/just-${version}-${target}.tar.gz" \
    -o "$archive"
  tar -xzf "$archive" -C "$tmp_dir"
  install -m 0755 "$tmp_dir/just" "$HOME/.local/bin/just"
else
  curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh \
    | bash -s -- --to "$HOME/.local/bin"
fi

just --version
