#!/usr/bin/env bash
# opencode LSP launcher for `.sokonanoda` files.
#
# Code agents are decoupled from the VS Code extension: this launcher gets the
# server from the sources below, in order, without requiring the extension,
# `cargo`, or a manual setup step.
#
# Why a launcher instead of `cargo run --quiet -p sokonanoda-lsp`:
#   - opencode spawns the configured command directly (no shell) with
#     cwd = the directory opencode was opened in (which may be a repo
#     subdirectory), and marks the server broken for the session when the
#     spawn fails or the handshake stalls;
#   - `cargo` may be missing from a GUI-launched opencode PATH, and a first
#     run that has to compile can stall the LSP handshake.
#
# Resolution order (first executable wins):
#   1. $SOKONANODA_LSP_BIN                        (explicit override)
#   2. repo target/{release,debug}                (local development build)
#   3. VS Code extension bundle, if installed     (zero-network reuse)
#   4. the download cache                         (~/.local/share/sokonanoda/bin)
#   5. version-pinned GitHub Release tarball      (v<repo version>, never latest)
#   6. cargo build                                (fresh clone, last resort)
#
# Set SOKONANODA_LSP_OFFLINE=1 to forbid the network step (step 5).
# stdout must stay pure LSP — every diagnostic message goes to stderr.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$script_dir/../.." && pwd)"

bin_name="sokonanoda-lsp"
[ "${OS:-}" = "Windows_NT" ] && bin_name="sokonanoda-lsp.exe"
cache_dir="${HOME:-}/.local/share/sokonanoda/bin"
cache="$cache_dir/$bin_name"

# 1) Explicit override.
if [ -n "${SOKONANODA_LSP_BIN:-}" ] && [ -x "$SOKONANODA_LSP_BIN" ]; then
  exec "$SOKONANODA_LSP_BIN"
fi

# 2) Local development build (newest of release/debug).
best=""
for candidate in "$root/target/release/$bin_name" "$root/target/debug/$bin_name"; do
  [ -x "$candidate" ] || continue
  if [ -z "$best" ] || [ "$candidate" -nt "$best" ]; then
    best="$candidate"
  fi
done
if [ -n "$best" ]; then
  exec "$best"
fi

# Host target (used by the extension lookup and the release asset name).
case "$(uname -s)" in
  Darwin)
    if [ "$(uname -m)" = "arm64" ]; then target="darwin-arm64"; else target="darwin-x64"; fi
    ;;
  Linux)
    if [ -f /etc/alpine-release ]; then
      if [ "$(uname -m)" = "aarch64" ]; then target="alpine-arm64"; else target="alpine-x64"; fi
    else
      if [ "$(uname -m)" = "aarch64" ]; then target="linux-arm64"; else target="linux-x64"; fi
    fi
    ;;
  *)
    if [ "$(uname -m)" = "aarch64" ]; then target="win32-arm64"; else target="win32-x64"; fi
    ;;
esac

rust_target="$target"
case "$target" in
  linux-x64)   rust_target="x86_64-unknown-linux-gnu" ;;
  linux-arm64) rust_target="aarch64-unknown-linux-gnu" ;;
  alpine-x64)  rust_target="x86_64-unknown-linux-musl" ;;
  alpine-arm64) rust_target="aarch64-unknown-linux-musl" ;;
  darwin-arm64) rust_target="aarch64-apple-darwin" ;;
  darwin-x64)  rust_target="x86_64-apple-darwin" ;;
  win32-x64)   rust_target="x86_64-pc-windows-msvc" ;;
  win32-arm64) rust_target="aarch64-pc-windows-msvc" ;;
esac

# 3) VS Code (and common forks) bundled server, newest installed version.
best=""
for ext_root in \
  "$HOME/.vscode/extensions" \
  "$HOME/.vscode-insiders/extensions" \
  "$HOME/.vscode-oss/extensions" \
  "$HOME/.cursor/extensions" \
  "$HOME/.windsurf/extensions" \
  "$HOME/.vscode-server/extensions"
do
  [ -d "$ext_root" ] || continue
  for candidate in "$ext_root"/sokonanoda-lang.sokonanoda-*/bin/"$target"/"$bin_name"; do
    [ -x "$candidate" ] || continue
    if [ -z "$best" ] || [ "$candidate" -nt "$best" ]; then
      best="$candidate"
    fi
  done
done
if [ -n "$best" ]; then
  exec "$best"
fi

# 4) Version-pinned download cache (shared with the VS Code extension).
if [ -x "$cache" ]; then
  exec "$cache"
fi

# 5) Version-pinned GitHub Release download (agents are decoupled from the
#    extension; the repo version decides the tag — never `latest`).
if [ -z "${SOKONANODA_LSP_OFFLINE:-}" ] && command -v curl >/dev/null 2>&1; then
  version="$(grep -m1 '^version' "$root/Cargo.toml" 2>/dev/null | cut -d'"' -f2 || true)"
  if [ -n "$version" ]; then
    url="https://github.com/ColorlessBoy/sokonanoda-lang/releases/download/v${version}/sokonanoda-lsp-${rust_target}.tar.gz"
    mkdir -p "$cache_dir"
    if curl -fsSL "$url" | tar xz -C "$cache_dir" 2>/dev/null && [ -x "$cache" ]; then
      exec "$cache"
    fi
    echo "sokonanoda-lsp: 下载 ${url} 失败，尝试本地构建。" >&2
  fi
fi

# 6) Nothing installed: build once (fresh clone).
if command -v cargo >/dev/null 2>&1; then
  cargo build --quiet --manifest-path "$root/Cargo.toml" -p sokonanoda-lsp >&2
  exec "$root/target/debug/$bin_name"
fi

echo "sokonanoda-lsp: 找不到语言服务器二进制，也无法下载（离线或网络失败）。" >&2
echo "任选其一：联网重试；在本仓库运行 cargo build --release -p sokonanoda-lsp；" >&2
echo "或用 SOKONANODA_LSP_BIN 指向已有的二进制。" >&2
exit 1
