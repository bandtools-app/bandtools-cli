#!/bin/sh
set -eu

REPO="${BT_INSTALL_REPO:-bandtools-app/bandtools-cli}"
BINARY="${BT_INSTALL_BINARY:-bt}"
INSTALL_DIR="${BT_INSTALL_DIR:-}"
VERSION="${BT_INSTALL_VERSION:-latest}"

log() {
  printf '%s\n' "$*" >&2
}

fail() {
  log "bt installer: $*"
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin)
      case "$arch" in
        arm64 | aarch64) printf '%s\n' "aarch64-apple-darwin" ;;
        x86_64 | amd64) printf '%s\n' "x86_64-apple-darwin" ;;
        *) fail "unsupported macOS architecture: $arch" ;;
      esac
      ;;
    Linux)
      case "$arch" in
        arm64 | aarch64) printf '%s\n' "aarch64-unknown-linux-gnu" ;;
        x86_64 | amd64) printf '%s\n' "x86_64-unknown-linux-gnu" ;;
        *) fail "unsupported Linux architecture: $arch" ;;
      esac
      ;;
    *)
      fail "unsupported operating system: $os"
      ;;
  esac
}

default_install_dir() {
  if [ -w /usr/local/bin ]; then
    printf '%s\n' "/usr/local/bin"
  else
    printf '%s\n' "$HOME/.local/bin"
  fi
}

download() {
  url="$1"
  destination="$2"

  if command -v curl >/dev/null 2>&1; then
    curl --fail --location --silent --show-error "$url" --output "$destination"
  elif command -v wget >/dev/null 2>&1; then
    wget --quiet --output-document="$destination" "$url"
  else
    fail "required command not found: curl or wget"
  fi
}

download_optional() {
  url="$1"
  destination="$2"

  if command -v curl >/dev/null 2>&1; then
    curl --fail --location --silent --show-error "$url" --output "$destination"
  elif command -v wget >/dev/null 2>&1; then
    wget --quiet --output-document="$destination" "$url"
  else
    return 1
  fi
}

verify_checksum() {
  archive="$1"
  checksums="$2"
  asset_name="$3"

  expected="$(awk -v name="$asset_name" '$2 == name { print $1 }' "$checksums")"
  [ -n "$expected" ] || fail "checksum not found for $asset_name"

  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$archive" | awk '{ print $1 }')"
  elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$archive" | awk '{ print $1 }')"
  else
    log "Skipping checksum verification; sha256sum or shasum is required."
    return
  fi

  [ "$expected" = "$actual" ] || fail "checksum verification failed for $asset_name"
}

need tar
need uname
need mktemp

target="$(detect_target)"
install_dir="${INSTALL_DIR:-$(default_install_dir)}"
asset="${BINARY}-${target}.tar.gz"

if [ "$VERSION" = "latest" ]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
  checksum_url="https://github.com/${REPO}/releases/latest/download/checksums.txt"
else
  url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
  checksum_url="https://github.com/${REPO}/releases/download/${VERSION}/checksums.txt"
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT HUP INT TERM

archive="$tmpdir/$asset"
download "$url" "$archive"

checksums="$tmpdir/checksums.txt"
if download_optional "$checksum_url" "$checksums"; then
  verify_checksum "$archive" "$checksums" "$asset"
else
  log "Skipping checksum verification; checksums.txt was not found."
fi

mkdir -p "$install_dir"
tar -xzf "$archive" -C "$tmpdir"

if [ ! -f "$tmpdir/$BINARY" ]; then
  fail "release asset did not contain $BINARY"
fi

chmod 755 "$tmpdir/$BINARY"
mv "$tmpdir/$BINARY" "$install_dir/$BINARY"

log "Installed $BINARY to $install_dir/$BINARY"
if ! command -v "$BINARY" >/dev/null 2>&1; then
  log "Add $install_dir to your PATH to run $BINARY from any shell."
fi
