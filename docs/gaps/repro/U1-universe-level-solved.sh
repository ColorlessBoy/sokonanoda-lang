#!/usr/bin/env bash
# **U1 片的端到端正例**（IA-4 §2.9 第一刀）——
# **省前导隐式实参 + 结果再收一个实参 + 宇宙层级由解得出** 的那条声明必须判绿 ✓。
#
# ── 形状（一句话）──────────────────────────────────────────────────────────
#   `axiom Show {u} : {α : Sort u} → α → Nat → Nat`
#   `def t1 : Nat := Show 0 5`
#     · `{α : Sort u}` 是**前导隐式** ⇒ 省掉不写 ✓（`α` 由 `0 : Nat` 解出 ✓）；
#     · 结果 `Nat → Nat` **再收一个实参** ✓（`5` 落到结果上 ✓ —— G-85 那一档 ✓）；
#     · **宇宙参数 `u` 没写** ✓ —— 修前一律按 `0` 建常量 ✗ ⇒ 内核
#       `类型不匹配：期望 Sort(0)，实际是 Sort(1)` ✗（`Nat : Sort 1` ✓）。
#   `u` 从哪来：`α` 被解成 `Nat` ✓ ⇒ `Nat : Sort 1` ✓ ⇒ `u := 1` ✓
#   （即"**解出来的隐式项参数自己的类型**"这一路 ✓）。
#
# ── 判据（四条，缺一不算成立）────────────────────────────────────────────
#   ① **开关关**：`t1` 判绿 ✓（`u` 由 G-30 的**恒解层**给出 —— 对齐 Lean
#      `mkFreshLevelMVars` + `levelMVarToParam`）、**只有**写死错的 `bad` 判红 ✓；
#   ② **开关开**：与 ① **逐项相同** ✓ —— `SOKO_UNIVERSE_METAVAR` 已于
#      2026-10-05（G-30 收口）**作废**（`universe_metavar_enabled()` 恒 false：
#      恒解已覆盖它，再开会造出**冲突的第二套解** ✗）⇒ 两态必须一字不差；
#   ③ **反面**：把层级**写死成错的**（`Show.{0} 0 5`）**两态都仍须判红** ✓
#      —— 证明"解出 `u := 1`"不是"什么都放行" ✗（**不是"看着像过"的断言** ✓）；
#   ④ **对照**：`Show.{1} 0 5`（层级写对）**两态都绿** ✓ —— 显式写法优先级不变 ✓。
#
# ── 退出码约定（⚠ **与 `docs/gaps/README.md` 相反**，按本棒的用户指示 ✓）──────
#   0 = **正例成立**（①②③④ 全过 ✓）    1 = 判据不成立 ✗    2 = 环境不满足
#   （本件**不进台账**：它不是一个"缺口"，是 U1 切片的验收 ✓ ——
#    台账里 G-63 仍 `open` ✓：`Quot.lift` 那一族还没吃到本片 ✓。）
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  # **优先 `scripts/soko`**（harness 中立的启动器，AGENTS.md 的推荐形态）：
  # 受管环境里 `CARGO_TARGET_DIR` 指向别处 ⇒ `target/release/sokonanoda`
  # **永远不存在** ⇒ 这批复现件一律 exit 2，而 `gap.py check`（`soko gate` 与 CI
  # 的一步）把 exit 2 记成"环境异常" ⇒ **门禁在受管环境下必然判红** ✗
  # （2026-10-12 独立复核实测：23 个脚本硬编码路径 · 22 条判环境异常）。
  # ⚠ **先探测它真的能答**：`scripts/soko` 没有可信构建时 **exit 3 拒跑** ——
  # 若不探测就交给它，退出码会掉进"grep 不到复现标记 ⇒ 已修"的**假绿**路径 ✗
  # （同族隐患：干净缓存 + 无构建时 lang 探针会假报 FIXED）。探测不过就往下走旧路径 ✓。
  if [ -x scripts/soko ] && scripts/soko version >/dev/null 2>&1; then BIN=scripts/soko
  elif [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
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
# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE`）：本件原来把
# `query check` 的输出**直接 json.load** —— 零观测（零输出 / 拒跑）时 Python 抛栈
# ⇒ 退出码 1（本件约定 = 判据不成立）⇒ **空观测被读成判据结论** ✗。⇒ 这里**只**把
# "解析不出来"这一种情形（= 零观测 ✓）记成哨兵 `{"checked": null, ...}`，交给下面的
# 判据打印 exit 2；**其余异常一概不吞** ✗（不许把所有异常都读成 exit 2 ✓）。
try:
    d = json.load(sys.stdin)["data"]
except Exception:
    print(json.dumps({"checked": None, "failed_lines": None}))
    sys.exit(0)
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

# ⭐ **零观测 ⇒ exit 2**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE`）：两态里任何一档
# 没拿到 `query check` 的 JSON（`checked is None` = 上面记的哨兵 ✓）⇒ 这次判卷
# **什么都没观测到** ⇒ 不是判据结论 ✗。
for tag, r in (("开关关", off), ("开关开", on)):
    if r.get("checked") is None:
        print(f"U1：{tag} 观测不到 query check 的答案（零输出 / 拒跑）⇒ 环境/形状异常（exit 2），不是判据结论 ✗",
              file=sys.stderr)
        print("      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 夹具不对",
              file=sys.stderr)
        sys.exit(2)
print(f"  开关关：checked={off['checked']} 判红行={off['failed_lines']}")
print(f"  开关开：checked={on['checked']} 判红行={on['failed_lines']}")

if off["failed_lines"] != [BAD]:
    bad.append(f"① 开关关时必须**只有** `bad` 判红（`t1` 已由恒解层转绿 ✓），实际 {off['failed_lines']}")
if off["checked"] != 3:
    bad.append(f"① 开关关时应有 3 条判过（Show/t1/t2 ✓），实际 {off['checked']}")
if on != off:
    bad.append(f"② 开关已作废 ⇒ 两态必须**逐项相同**：关={off} 开={on}")
if T2 in on["failed_lines"] or T2 in off["failed_lines"]:
    bad.append("④ 对照 `Show.{1} 0 5` 两态都必须绿 ✗")
if BAD not in on["failed_lines"] or BAD not in off["failed_lines"]:
    bad.append("③ 反面 `Show.{0} 0 5`（层级写死成错的）两态都必须仍判红 ✗ —— 否则是'什么都放行' ✗")
if T1 in off["failed_lines"]:
    bad.append("① `Show 0 5`（缺省前导隐式 + 结果再收实参 + 层级由解得出）必须判绿 ✗")

if bad:
    print("✗ 判据不成立：", file=sys.stderr)
    for b in bad:
        print("  -", b, file=sys.stderr)
    sys.exit(1)
print("✓ 四条判据全过：`t1` 两态绿（恒解层 `u := 1` ✓）· 开关已作废两态逐项相同 ✓ · 对照两态绿 ✓ · 反面仍红 ✓")
PY
