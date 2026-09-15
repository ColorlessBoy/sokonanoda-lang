#!/usr/bin/env sh
# Contributor-only: run the frozen kernel against the external Lean Kernel Arena
# corpus (the real performance/soundness baseline beyond the in-repo canaries).
#
# End users never need this, and it is NOT wired into CI: the corpus is large and
# lives in its own repository. See docs/PERF.md "External baseline".
#
# Usage:
#   LEAN_KERNEL_ARENA=/path/to/lean-kernel-arena scripts/perf-arena.sh
#   scripts/perf-arena.sh /path/to/lean-kernel-arena
set -eu

ROOT="${1:-${LEAN_KERNEL_ARENA:-}}"

if [ -z "$ROOT" ]; then
  cat <<'EOF'
LEAN_KERNEL_ARENA is not set.

The arena corpus is external (https://github.com/leanprover/lean-kernel-arena):

  git clone https://github.com/leanprover/lean-kernel-arena
  cd lean-kernel-arena && uv run lka.py build-test     # writes _build/tests/*.ndjson

Then re-run:

  LEAN_KERNEL_ARENA=/path/to/lean-kernel-arena scripts/perf-arena.sh

See docs/PERF.md "External baseline".
EOF
  exit 0
fi

if [ ! -d "$ROOT/_build/tests" ] && [ ! -d "$ROOT/tests" ]; then
  echo "error: $ROOT does not look like a built arena checkout (no _build/tests or tests)" >&2
  echo "hint: cd $ROOT && uv run lka.py build-test" >&2
  exit 2
fi

echo "arena: $ROOT"
echo "+ LEAN_KERNEL_ARENA=$ROOT cargo test -p sokonanoda --locked --test arena -- --nocapture"
START=$(date +%s)
LEAN_KERNEL_ARENA="$ROOT" cargo test -p sokonanoda --locked --test arena -- --nocapture
END=$(date +%s)
echo "arena run: $((END - START))s"
