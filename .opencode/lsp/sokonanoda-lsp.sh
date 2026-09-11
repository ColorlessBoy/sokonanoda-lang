#!/usr/bin/env bash
# Non-opencode LSP launcher shim: resolve the `sokonanoda` binary and exec its
# `lsp` subcommand. (opencode itself wires the native binary via the plugin;
# this is only for harnesses that need a command entrypoint.)
#
# Resolution: SOKONANODA_BIN → repo build (release|debug) → cache/PATH.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
for candidate in \
  "${SOKONANODA_BIN:-}" \
  "$root/target/release/sokonanoda" \
  "$root/target/debug/sokonanoda"
do
  if [ -n "$candidate" ] && [ -x "$candidate" ]; then
    exec "$candidate" lsp
  fi
done
if command -v sokonanoda >/dev/null 2>&1; then
  exec sokonanoda lsp
fi
echo "sokonanoda-lsp: 找不到 sokonanoda 二进制（先跑 \`sokonanoda setup\` 或 cargo build）" >&2
exit 3
