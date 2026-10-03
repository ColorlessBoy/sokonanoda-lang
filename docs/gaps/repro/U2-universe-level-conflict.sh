#!/usr/bin/env bash
# **U2-a 的端到端正例**（设计 `docs/design/metavar-engine.md` §2.9 的第三件事 ✓）：
# **显式 `.{n}` 优先** ✓ —— 写出来的层级就是权威 ✓；但**与解出来的字面冲突**时要报**专用码** ✗，
# 不再一路落到内核报成 `类型不匹配：期望 Sort(1)，实际是 Sort(2)`（看不出"是层级写错了" ✓）。
#
# ── 形状 ──────────────────────────────────────────────────────────────────
#   `axiom Show {u} : {α : Sort u} → α → Nat`
#   `def ok1  : Nat := Show.{1} 0`   ← 写对 ⇒ 两态都绿 ✓
#   `def bad  : Nat := Show.{0} 0`   ← 写错 ⇒ 开关开时报 **`elab-universe-level-conflict`** ✓
#   `def auto : Nat := Show 0`       ← 不写 ⇒ 开关开时由 U1 解出（绿 ✓）
#
# ── 判据（四条，缺一不算成立）─────────────────────────────────────────────
#   ① **开关关**：`bad` 与 `auto` 都判红 ✓（基线行为 —— 前者内核不符 ✗、后者 U1 未开 ✓），
#      且**没有** `elab-universe-level-conflict`（新码不许在关着时出现 ✗）；
#   ② **开关开**：`bad` **只**报新码 ✓、`ok1` 与 `auto` 都绿 ✓（`checked=3 · failed=1` ✓）；
#   ③ **反面（误报守卫）**：写**对**的 `.{1}` 不许被判冲突 ✓（它在两态都必须绿 ✓）；
#   ④ **旧码不许被顶掉**：项层解不出仍报 `elab-implicit-argument-unsolved` ✓（见
#      `crates/cli/tests/notation.rs::an_unsolvable_implicit_argument_reports_its_own_code` ✓）。
#
# ── 退出码约定（与 `docs/gaps/README.md` **相反**，同 `U1-…` / `B1-…` ✓）──────────
#   0 = **正例成立**（①②③ 全过 ✓）    1 = 判据不成立 ✗    2 = 环境不满足
#   （本件**不进台账** ✓：它是 U2 切片在**开关**下的验收 ✓，不是一个缺口 ✓。）
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "U2: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cat > "$WORK/u2a.sokonanoda" <<'EOF'
axiom Show {u} : {α : Sort u} → α → Nat

def ok1 : Nat := Show.{1} 0
def bad : Nat := Show.{0} 0
def auto : Nat := Show 0
EOF

run_state() {
  local out="$1" mode="$2"
  SOKO_UNIVERSE_METAVAR="$mode" "$BIN" query check --file "$WORK/u2a.sokonanoda" \
    | python3 -c '
import json, sys
d = json.load(sys.stdin)["data"]
rows = sorted(((f.get("start_line") or 0), (f.get("code") or "")) for f in (d.get("failed") or []))
print(json.dumps({"checked": d["counts"]["decl_checked"], "failed": rows}))
' > "$out"
}

run_state "$WORK/off.json" ""
run_state "$WORK/on.json" "1"

python3 - "$WORK/off.json" "$WORK/on.json" <<'PY'
import json, sys

off = json.load(open(sys.argv[1]))
on = json.load(open(sys.argv[2]))
# 行号（见 heredoc）：ok1=3 · bad=4 · auto=5
OK1, BAD, AUTO = 3, 4, 5
NEW = "elab-universe-level-conflict"
bad = []

print(f"  开关关：checked={off['checked']} failed={off['failed']}")
print(f"  开关开：checked={on['checked']} failed={on['failed']}")

off_codes = {c for _, c in off["failed"]}
on_lines = {ln for ln, _ in on["failed"]}
on_codes = {c for _, c in on["failed"]}

if NEW in off_codes:
    bad.append("① 开关**关**时不许出现新码（`elab-universe-level-conflict`）✗ —— 那说明检查没挂在开关下 ✗")
if off["checked"] != 2:
    bad.append(f"① 开关关时应判过 2 条（axiom + `ok1` ✓），实际 {off['checked']}")
if on_lines != {BAD} or on["checked"] != 3:
    bad.append(f"② 开关开时**只**该 `bad` 判红（`checked=3` ✓），实际 checked={on['checked']} failed={on['failed']}")
if on_codes != {NEW}:
    bad.append(f"② 开关开时 `bad` 必须报**新码** `{NEW}` ✓，实际 {sorted(on_codes)}")
if OK1 in on_lines or OK1 in {ln for ln, _ in off["failed"]}:
    bad.append("③ 反面：写**对**的 `.{1}` 被判红了 ✗（误报 ⇒ 这条守卫会把正确写法也拦掉 ✗）")
if AUTO in on_lines:
    bad.append("③ `Show 0`（不写层级）在开关开时必须绿 ✓（U1 的解出路径 ✓）")

if bad:
    print("✗ 判据不成立：", file=sys.stderr)
    for b in bad:
        print("  -", b, file=sys.stderr)
    sys.exit(1)
print("✓ 三条判据全过：开关关无新码 ✓ / 开关开 `bad` 只报 `elab-universe-level-conflict` ✓ / 写对的与不写的都绿 ✓")
PY
