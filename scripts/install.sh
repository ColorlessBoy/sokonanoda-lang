#!/bin/sh
# sokonanoda installer for END USERS (zero Rust/cargo).
#
# Downloads the version-pinned `sokonanoda` CLI tarball from GitHub Releases.
# There is nothing to compile: the tarball already contains a runnable binary.
#
# Hard rules (REQUIREMENTS.md §2.9, AGENTS.md):
#   * end users / code agents never need Rust or cargo;
#   * release assets are pinned to an explicit tag — NEVER `releases/latest`
#     (the CLI, the LSP and the VSIX must all be the same version).
#
# Usage:
#   sh install.sh --version vX.Y.Z
#   SOKONANODA_VERSION=vX.Y.Z sh install.sh
#   curl -fsSL <release>/install.sh | SOKONANODA_VERSION=vX.Y.Z sh
#
# Environment:
#   SOKONANODA_VERSION   release tag, with or without a leading `v` (required)
#   SOKONANODA_HOME      install root (default: $HOME/.local/share/sokonanoda)
#   SOKONANODA_BASE_URL  download base (override for mirrors/tests)
#
# See docs/design/onboarding.md §5 and REQUIREMENTS.md §2.9.

set -eu

REPO="${SOKONANODA_REPO:-ColorlessBoy/sokonanoda-lang}"
VERSION="${SOKONANODA_VERSION:-}"
HOME_DIR="${SOKONANODA_HOME:-${HOME:-/root}/.local/share/sokonanoda}"

usage() {
    cat <<'EOF'
Install the sokonanoda CLI from a version-pinned GitHub Release (no Rust).

Usage:
  sh install.sh --version vX.Y.Z
  SOKONANODA_VERSION=vX.Y.Z sh install.sh

Options:
  --version <tag>   release tag to install (or set SOKONANODA_VERSION)
  -h, --help        show this help

Environment:
  SOKONANODA_VERSION   release tag, with or without a leading `v` (required)
  SOKONANODA_HOME      install root (default: $HOME/.local/share/sokonanoda)
  SOKONANODA_BASE_URL  download base (override for mirrors/tests)

The tag must be an existing release (it matches [workspace.package].version in
Cargo.toml and the git tag). Never use `releases/latest`: the CLI, the LSP and
the VS Code extension must all be the same version.
EOF
}

err() {
    echo "install.sh: error: $*" >&2
    exit 1
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --version)
            [ "$#" -ge 2 ] || err "--version needs a value"
            VERSION="$2"
            shift 2
            ;;
        --version=*)
            VERSION="${1#*=}"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            err "unknown argument: $1 (try --help)"
            ;;
    esac
done

[ -n "$VERSION" ] || err "a release version is required (assets are version-pinned).
Set SOKONANODA_VERSION or pass --version, e.g.:
  SOKONANODA_VERSION=vX.Y.Z sh install.sh
Take the tag from the GitHub Release page; never use \`releases/latest\`."

case "$VERSION" in
    v*) TAG="$VERSION" ;;
    *)  TAG="v$VERSION" ;;
esac

# Map OS + CPU (and libc on Linux) to the Rust target triple used in the
# Release asset names, matching the 8-platform matrix in
# .github/workflows/release.yml / docs/RELEASE.md:
#   aarch64-apple-darwin          x86_64-apple-darwin
#   x86_64-unknown-linux-gnu      aarch64-unknown-linux-gnu
#   x86_64-unknown-linux-musl     aarch64-unknown-linux-musl
#   x86_64-pc-windows-msvc        aarch64-pc-windows-msvc
detect_target() {
    os="$(uname -s 2>/dev/null || echo unknown)"
    arch="$(uname -m 2>/dev/null || echo unknown)"

    case "$arch" in
        arm64|aarch64) cpu=aarch64 ;;
        x86_64|amd64)  cpu=x86_64 ;;
        *) err "unsupported CPU architecture: $arch.
Builds are published for arm64/aarch64 and x86_64 only." ;;
    esac

    case "$os" in
        Darwin)
            echo "${cpu}-apple-darwin"
            ;;
        Linux)
            libc=
            if [ -f /etc/alpine-release ]; then
                libc=musl
            elif command -v ldd >/dev/null 2>&1; then
                if ldd --version 2>&1 | grep -qi musl; then
                    libc=musl
                fi
            fi
            [ -n "$libc" ] || libc=gnu
            echo "${cpu}-unknown-linux-${libc}"
            ;;
        MINGW*|MSYS*|CYGWIN*|Windows*)
            err "Windows is not supported by this POSIX installer.
Install the platform VSIX from the Marketplace, or download
sokonanoda-cli-${cpu}-pc-windows-msvc.tar.gz manually from:
  https://github.com/${REPO}/releases/tag/${TAG}" ;;
        *)
            err "unsupported OS: $os (macOS and Linux only)" ;;
    esac
}

fetch() {
    url="$1"
    dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$dest" "$url"
    else
        err "need curl or wget to download the release asset"
    fi
}

TARGET="$(detect_target)"
BASE_URL="${SOKONANODA_BASE_URL:-https://github.com/${REPO}/releases/download/${TAG}}"
ASSET="sokonanoda-cli-${TARGET}.tar.gz"
URL="${BASE_URL}/${ASSET}"
BIN_DIR="${HOME_DIR}/bin"

if command -v mktemp >/dev/null 2>&1; then
    TMP="$(mktemp -d "${TMPDIR:-/tmp}/sokonanoda-install.XXXXXX")"
else
    TMP="${TMPDIR:-/tmp}/sokonanoda-install.$$"
    mkdir -p "$TMP"
fi
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

echo "sokonanoda installer"
echo "  version : ${TAG}"
echo "  target  : ${TARGET}"
echo "  asset   : ${URL}"

fetch "$URL" "$TMP/$ASSET" \
    || err "download failed. Check that ${TAG} exists and ships ${ASSET}:
  https://github.com/${REPO}/releases/tag/${TAG}
If you are behind a proxy, export HTTPS_PROXY and retry."

mkdir -p "$BIN_DIR"
tar xzf "$TMP/$ASSET" -C "$BIN_DIR" \
    || err "failed to extract ${ASSET}"

[ -f "$BIN_DIR/sokonanoda" ] \
    || err "archive did not contain a \`sokonanoda\` binary"
chmod 755 "$BIN_DIR/sokonanoda"

echo
echo "installed: $BIN_DIR/sokonanoda"
if command -v "$BIN_DIR/sokonanoda" >/dev/null 2>&1; then
    "$BIN_DIR/sokonanoda" --version 2>/dev/null || true
fi

case ":${PATH}:" in
    *":${BIN_DIR}:"*) ;;
    *)
        echo
        echo "Add it to your PATH (then restart your shell):"
        echo "  export PATH=\"${BIN_DIR}:\$PATH\""
        echo
        echo "Only needed once — append that line to your shell profile"
        echo "(~/.profile, ~/.zshrc, ...)."
        ;;
esac

echo
echo "Next:  sokonanoda doctor"
