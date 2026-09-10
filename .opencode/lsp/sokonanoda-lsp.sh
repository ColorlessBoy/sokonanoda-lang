#!/usr/bin/env bash
# opencode LSP launcher: a thin shim over `scripts/soko.sh lsp`.
#
# opencode spawns the configured command without a shell and with cwd set to
# the directory it was opened in (possibly a repo subdirectory), so this shim
# locates the repo from its own path and delegates the whole resolution chain
# (explicit env → repo build → VS Code extension bundle → cache → version-pinned
# download → cargo build) to the single environment entrypoint.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exec bash "$root/scripts/soko.sh" lsp
