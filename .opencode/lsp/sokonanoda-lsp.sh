#!/usr/bin/env bash
# opencode LSP launcher for `.sokonanoda` files.
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
#   3. VS Code extension bundle                   (~/.vscode*/extensions/
#      sokonanoda-lang.sokonanoda-*/bin/<target>/sokonanoda-lsp) — the same
#      binary the Marketplace extension ships, so no build is needed
#   4. the extension's download cache             (~/.local/share/sokonanoda/bin)
#   5. cargo build                                (fresh clone, last resort)
#
# stdout must stay pure LSP — every diagnostic message goes to stderr.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$script_dir/../.." && pwd)"

bin_name="sokonanoda-lsp"
[ "${OS:-}" = "Windows_NT" ] && bin_name="sokonanoda-lsp.exe"

if [ -n "${SOKONANODA_LSP_BIN:-}" ] && [ -x "$SOKONANODA_LSP_BIN" ]; then
  exec "$SOKONANODA_LSP_BIN"
fi

# 1) Local development build (newest of release/debug).
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

# 2) VS Code (and common forks) bundled server, newest installed version.
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

# 3) The extension's version-pinned download cache (universal fallback path).
cache="${HOME:-}/.local/share/sokonanoda/bin/$bin_name"
if [ -x "$cache" ]; then
  exec "$cache"
fi

# 4) Nothing installed: build once (fresh clone), preferring cargo if present.
if command -v cargo >/dev/null 2>&1; then
  cargo build --quiet --manifest-path "$root/Cargo.toml" -p sokonanoda-lsp >&2
  exec "$root/target/debug/$bin_name"
fi

echo "sokonanoda-lsp: 找不到语言服务器二进制。" >&2
echo "任选其一：安装 sokonanoda VS Code 扩展；或在本仓库运行" >&2
echo "  cargo build --release -p sokonanoda-lsp" >&2
exit 1
