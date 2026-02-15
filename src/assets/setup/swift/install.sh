# ==============================================================================
# Install Swift via swiftly
# ==============================================================================

if command -v swift >/dev/null 2>&1; then
  if swift_version_line="$(swift --version 2>/dev/null | head -1)"; then
    echo "swift already installed: $swift_version_line"
    exit 0
  fi
fi

require_command() {
  local cmd="$1"
  local deb_pkg="$2"

  if command -v "$cmd" >/dev/null 2>&1; then
    return 0
  fi

  echo "Missing required command: $cmd" >&2
  if [[ -n "$deb_pkg" && "$(uname -s)" == "Linux" ]] && command -v apt-get >/dev/null 2>&1; then
    echo "On Debian/Ubuntu (as root): apt-get -y install $deb_pkg" >&2
  fi
  exit 1
}

require_command curl curl
require_command tar tar
require_command gpg gnupg

SWIFTLY_HOME_DIR="${SWIFTLY_HOME_DIR:-$HOME/.local/share/swiftly}"
SWIFTLY_BIN_DIR="${SWIFTLY_BIN_DIR:-$SWIFTLY_HOME_DIR/bin}"
SWIFTLY_TOOLCHAINS_DIR="${SWIFTLY_TOOLCHAINS_DIR:-$SWIFTLY_HOME_DIR/toolchains}"
export SWIFTLY_HOME_DIR SWIFTLY_BIN_DIR SWIFTLY_TOOLCHAINS_DIR

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

gnupg_home="$tmp/gnupg"
mkdir -p "$gnupg_home"
chmod 700 "$gnupg_home"
export GNUPGHOME="$gnupg_home"

arch="$(uname -m)"
curl --proto '=https' --tlsv1.2 -fsSL \
  "https://download.swift.org/swiftly/linux/swiftly-${arch}.tar.gz" \
  -o "$tmp/swiftly.tgz"
tar -xzf "$tmp/swiftly.tgz" -C "$tmp"

(
  cd "$tmp"
  "$tmp/swiftly" init --assume-yes --no-modify-profile --quiet-shell-followup
)

env_sh="$SWIFTLY_HOME_DIR/env.sh"
if [[ -f "$env_sh" ]]; then
  # shellcheck disable=SC1090
  . "$env_sh"
fi

export PATH="$SWIFTLY_BIN_DIR:$PATH"
swiftly_cmd="$SWIFTLY_BIN_DIR/swiftly"
if [[ ! -x "$swiftly_cmd" ]]; then
  swiftly_cmd="$tmp/swiftly"
fi

if [[ -n "${SWIFT_VERSION:-}" ]]; then
  "$swiftly_cmd" install "$SWIFT_VERSION"
  (
    cd "$tmp"
    "$swiftly_cmd" use "$SWIFT_VERSION"
  )
fi

if ! command -v swift >/dev/null 2>&1; then
  toolchains_dir="$SWIFTLY_TOOLCHAINS_DIR"
  swift_path=""
  if [[ -x "$toolchains_dir/usr/bin/swift" ]]; then
    swift_path="$toolchains_dir/usr/bin/swift"
  else
    for candidate in "$toolchains_dir"/*/usr/bin/swift "$toolchains_dir"/*/bin/swift; do
      if [[ -x "$candidate" ]]; then
        swift_path="$candidate"
        break
      fi
    done
  fi
  if [[ -z "$swift_path" && -d "$toolchains_dir" ]]; then
    swift_path="$(find "$toolchains_dir" -type f -name swift -perm -111 \
      \( -path "*/usr/bin/swift" -o -path "*/bin/swift" \) -print -quit 2>/dev/null || true)"
  fi
  if [[ -n "$swift_path" ]]; then
    swift_bin_dir="$(dirname "$swift_path")"
    export PATH="$swift_bin_dir:$PATH"
  fi
fi

if ! command -v swift >/dev/null 2>&1; then
  echo "swift not found after swift toolchain install" >&2
  exit 1
fi

set +e
swift_version_output="$(swift --version 2>&1)"
swift_version_status="$?"
set -e
if [[ "$swift_version_status" -ne 0 ]]; then
  echo "$swift_version_output" >&2
  if [[ "$swift_version_output" == *"error while loading shared libraries"* ]] \
    && [[ "$(uname -s)" == "Linux" ]] \
    && command -v apt-get >/dev/null 2>&1; then
    echo "Swift toolchain requires system libraries to run." >&2
    echo "On Debian/Ubuntu (as root): apt-get -y install binutils unzip gnupg2 libc6-dev libcurl4-openssl-dev libedit2 libgcc-13-dev libpython3-dev libstdc++-13-dev libxml2-dev libncurses-dev libz3-dev pkg-config tzdata zlib1g-dev" >&2
  fi
  exit "$swift_version_status"
fi

echo "$swift_version_output"
