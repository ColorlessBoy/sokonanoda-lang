#!/usr/bin/env bash
# **U1 片的端到端正例**（IA-4 §2.9 第一刀 · 开关 `SOKO_UNIVERSE_METAVAR`）——
# **省前导隐式实参 + 结果再收一个实参 + 宇宙层级由解得出** 的那条声明必须判绿 ✓。
#
# ── 形状（一句话）──────────────────────────────────────────────────────────
#   `axiom Show {u} : {α : Sort u} → α → Nat → Nat`
#   `def t1 : Nat := Show 0 5`
#     · `{α : Sort u}` 是**前导隐式** ⇒ 省掉不写 ✓（`α` 由 `0 : Nat` 解出 ✓）；
#     · 结果 `Nat → Nat` **再收一个实参** ✓（`5` 落到结果上 ✓ —— G-85 那一档 ✓）；
#     · **宇宙参数 `u` 没写** ✗ —— 修前一律按 `0` 建常量 ✗ ⇒ 内核
#       `类型不匹配：期望 Sort(0)，实际是 Sort(1)` ✗（`Nat : Sort 1` ✓）。
#   `u` 从哪来：`α` 被解成 `Nat` ✓ ⇒ `Nat : Sort 1` ✓ ⇒ `u := 1` ✓
#   （即"**解出来的隐式项参数自己的类型**"这一路 —— 本轮实测：缺它 ⇒ 接线在跑也解不出 ✗）。
#
# ── 判据（四条，缺一不算成立）────────────────────────────────────────────
#   ① **开关关**：`t1` 必须判红 ✓（缺口面 —— 基线行为 ✓）；
#   ② **开关开**：`t1` 必须判绿 ✓（正例成立 ✓）；
#   ③ **反面**：把层级**写死成错的**（`Show.{0} 0 5`）在**开关开**时仍必须判红 ✓
#      —— 证明"解出 `u := 1`"不是"什么都放行" ✗（**不是"看着像过"的断言** ✓）；
#   ④ **对照**：`Show.{1} 0 5`（层级写对）**两态都绿** ✓ —— 显式写法优先级不变 ✓。
#
# ── 退出码约定（⚠ **与 `docs/gaps/README.md` 相反**，按本棒的用户指示 ✓）──────
#   0 = **正例成立**（①②③④ 全过 ✓）    1 = 判据不成立 ✗    2 = 环境不满足
#   （本件**不进台账**：它不是一个"缺口"，是 U1 切片在**开关**下的验收 ✓ ——
#    台账里 G-63 仍 `open` ✓：`Quot.lift` 那一族还没吃到本片 ✓。）
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "U1: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cat > "$WORK/u1.sokonanoda" <<'EOF'
axiom Show {u} : {α : Sort u} → α → Nat → Nat

def t1 : Nat := Show 0 5
def t2 : Nat := Show.{1} 0 5
def bad : Nat := Show.{0} 0 5
EOF

# 跑一态，打印 `checked/failed` 与判红的行号；把结果写进 $1 文件
run_state() {
  local out="$1" mode="$2"
  SOKO_UNIVERSE_METAVAR="$mode" "$BIN" query check --file "$WORK/u1.sokonanoda" \
    | python3 -c '
import json, sys
d = json.load(sys.stdin)["data"]
failed = sorted(f.get("start_line") or 0 for f in (d.get("failed") or []))
print(json.dumps({"checked": d["counts"]["decl_checked"], "failed_lines": failed}))
' > "$out"
}

run_state "$WORK/off.json" ""
run_state "$WORK/on.json" "1"

python3 - "$WORK/off.json" "$WORK/on.json" <<'PY'
import json, sys

off = json.load(open(sys.argv[1]))
on = json.load(open(sys.argv[2]))
# 探针文件的行号（见上面 heredoc）：t1=3 · t2=4 · bad=5
T1, T2, BAD = 3, 4, 5
bad = []

print(f"  开关关：checked={off['checked']} 判红行={off['failed_lines']}")
print(f"  开关开：checked={on['checked']} 判红行={on['failed_lines']}")

if off["failed_lines"] != [T1, BAD]:
    bad.append(f"① 开关关时必须**只有** t1 与 bad 判红（缺口面 ✓），实际 {off['failed_lines']}")
if on["failed_lines"] != [BAD]:
    bad.append(f"② 开关开时必须**只有** bad 判红（t1 转绿 ✓、对照 t2 绿 ✓），实际 {on['failed_lines']}")
if on["checked"] != 3:
    bad.append(f"② 开关开时应有 3 条判过（Show/t1/t2 ✓），实际 {on['checked']}")
if off["checked"] != 2:
    bad.append(f"① 开关关时应有 2 条判过（Show/t2 ✓），实际 {off['checked']}")
if T2 in on["failed_lines"] or T2 in off["failed_lines"]:
    bad.append("④ 对照 `Show.{1} 0 5` 两态都必须绿 ✗")
if BAD not in on["failed_lines"]:
    bad.append("③ 反面 `Show.{0} 0 5`（层级写死成错的）在开关开时必须仍判红 ✗ —— 否则是'什么都放行' ✗")

if bad:
    print("✗ 判据不成立：", file=sys.stderr)
    for b in bad:
        print("  -", b, file=sys.stderr)
    sys.exit(1)
print("✓ 四条判据全过：开关关 t1 红 ✓ / 开关开 t1 绿（`u := 1` ✓）· 对照两态绿 ✓ · 反面仍红 ✓")
PY
