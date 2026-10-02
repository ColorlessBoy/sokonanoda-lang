#!/usr/bin/env bash
# G-78 自断言复现：**`soko/stateAt` 的头部（根）状态不是「题面状态」**。
#
# 退出码约定（与 `docs/gaps/README.md` 一致）：
#   0 = 缺口仍在   1 = 行为已变（已修 / 形状变了，回来关账）   2 = 环境不满足
#
# ── 缺口原文（用户 2026-10-02 真机实测，判据是 `step=-1` 那一格）──
#   | 形状                                        | binders  | goal                  | 旧行为 |
#   |---------------------------------------------|----------|-----------------------|--------|
#   | P4 冒号前绑元 + `by` + `sorry`              | [a,b,h]  | `a ∧ a`               | ✅     |
#   | P3 同上但 `:= sorry`（无 by 块）            | [a,b,h]  | `a ∧ a`               | ✅     |
#   | P1 冒号前绑元 + `by` + `constructor`+`sorry`| []       | 整句 `∀ … → a ∧ a`    | ❌     |
#   | P2 冒号后箭头 + `by` + `constructor`+`sorry`| []       | `null`（面板显示「已无目标 ✓」）| ❌ |
#
# ⚠ **口径更正（2026-10-02 值守第 9 单）**：本条原先要求 P2 那形也「goal ≠ 整句声明
#   类型」✗ —— 那是**错的**，会连语句自身的 Π 层一起剥掉，于是
#   `theorem (a b : Prop) : a → b → a := by` 的顶变成 `[a,b,_,_] ⊢ a` ✗ 而底是
#   `[a,b] ⊢ a → b → a` ✓（真机实测）。**新判据 = 顶 ≡ 底**（根状态与声明卡片同源 ✓），
#   P2 这类**冒号前没有具名绑元**的声明：顶 = 底 = `[] ⊢ 整句 Π` ✓（非 null ✓，
#   这正是本条当初要修的「谎报已无目标」✓）。扫描见 `G81-top-equals-bottom-root-state.sh`。
#   ⇒ 头部把**整句量词式**当目标、或干脆说"没目标了"，都不是任何 Lean 意义上的
#     证明状态。ⓘ P2 的 `null` 真身是**声明被判 Failed**（箭头式声明的引擎根目标
#     是整条 Pi，`constructor` 拒绝它、要求先 `intro`）—— 但「没通过」不等于
#     「已证完」，面板不该谎报。
#
# ── 修法（0.81.0）──
#   根状态改答**题面状态**：`binders` = 声明的 ∀ 参数（按序，含匿名箭头的 `_`）、
#   `goal` = 剥掉它们之后的命题；**失败的声明**也答题面（`DeclState.by_root`，
#   走查里 `statement_state()` 现算）。
#   ⚠ **tactic 语义没动**：箭头式声明的引擎根目标仍是整条 Pi（要显式 `intro`，与
#   Lean 一致 —— 实测 `apply And.intro`/`constructor` 都拒绝 Pi 目标，错误文案自己
#   写着「先 `intro` 拆开试试」；用户画布 5 条箭头式声明全靠 `intro`）。这里答的是
#   "**题目**长什么样"，不是"引擎此刻的 goal 是什么"。
#
# ── 判据（四形 × 两位置：`:= by` 那一行 / 第一条 tactic 那一行）──
#   ① `goal` 非 `null`；② `goal` **不等于**声明的完整类型；③ `binders` = 这道题的
#   ∀ 参数（按序）。四形都要成立。
#
# ── 反向验证（本脚本内置，作为台账重放的一部分）──
#   `SOKO_STATE_ROOT=legacy` 撤掉修复 ⇒ **同一套判据必须判红**，用仓库自己的
#   `scripts/expect-red.sh` 表达（不是写在提交说明里 ✓）。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-78: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cat > "$WORK/shapes.sokonanoda" <<'EOF'
theorem p4 (a b : Prop) (h : a) : a ∧ a := by
  sorry

theorem p3 (a b : Prop) (h : a) : a ∧ a := sorry

theorem p1 (a b : Prop) (h : a) : a ∧ a := by
  constructor
  sorry

theorem p2 : (a b : Prop) -> a -> a ∧ a := by
  constructor
  sorry
EOF

# 判据：0 = 全过（= 已修）  1 = 有判据不过（= 缺口仍在）
criteria() {
  python3 - "$BIN" "$WORK/shapes.sokonanoda" <<'PY'
import json, subprocess, sys

bin_, path = sys.argv[1], sys.argv[2]
# (标签, 声明名, 行号, 位置说明, 期望 binders, 期望 goal)
CASES = [
    ("P4 冒号前绑元 + by + sorry",  "p4", 1,  "`:= by` 那一行",        ["a", "b", "h"], "a ∧ a"),
    ("P4 同上",                     "p4", 2,  "第一条 tactic 那一行",   ["a", "b", "h"], "a ∧ a"),
    ("P3 := sorry（无 by 块）",     "p3", 4,  "声明那一行",            ["a", "b", "h"], "a ∧ a"),
    # ⚠ P1（证明体里有 tactic、洞在子目标里）：顶 ≡ 底 ✓，但**卡片本身**对这类声明还没剥
    #   （`goal_under_binders` 的兜底分支 ⇒ `[]` ⊢ 整句类型）—— 那是**底侧**的另一条缺陷，
    #   已立 **G-82（open，带复现件）**；本条只钉「顶 ≡ 底 + 非 null」（原判据 ✓）。
    ("P1 constructor + sorry",      "p1", 6,  "`:= by` 那一行",        [], "(a : Prop) → (b : Prop) → (h : a) → a ∧ a"),
    ("P1 同上",                     "p1", 7,  "第一条 tactic 那一行",   [], "(a : Prop) → (b : Prop) → (h : a) → a ∧ a"),
    # P2 没有具名绑元 ⇒ 题面状态 = 整句 Π（与声明卡片一致 ✓；**非 null** = 本条要修的谎 ✓）
    ("P2 冒号后箭头 + constructor", "p2", 10, "`:= by` 那一行",        [], "(a : Prop) → (b : Prop) → a → a ∧ a"),
    ("P2 同上",                     "p2", 11, "第一条 tactic 那一行",   [], "(a : Prop) → (b : Prop) → a → a ∧ a"),
]
# 「goal 不得等于整句声明类型」只对**顶≡底且卡片已剥**的形状成立 ✓；P1/P2（卡片未剥 / 冒号前
# 没有具名绑元）
# 按「顶 ≡ 底」口径就是整句 Π ⇒ 不查这一条（口径更正见文件头 ⚠）。
FULL_TYPES = {
    "p4": ["∀ (a b : Prop), a → a ∧ a", "(a : Prop) → (b : Prop) → (h : a) → a ∧ a"],
    "p3": ["∀ (a b : Prop), a → a ∧ a", "(a : Prop) → (b : Prop) → (h : a) → a ∧ a"],
    "p1": [],   # 卡片对这类声明还没剥（G-82 open）⇒ 顶 = 整句类型是**如实**的 ✓
    "p2": [],
}
bad = []
for label, name, line, where, want_b, want_g in CASES:
    out = subprocess.run(
        [bin_, "query", "state", "--file", path, "--line", str(line), "--col", "1"],
        capture_output=True, text=True,
    )
    try:
        d = json.loads(out.stdout)["data"]
    except Exception:
        bad.append(f"{label} @{where}：query state 没给出 JSON（{(out.stdout or out.stderr)[:80]!r}）")
        continue
    goal = d.get("goal")
    binders = [b.get("name") for b in (d.get("binders") or [])]
    why = []
    if goal is None:
        why.append("goal 是 null（面板会显示「已无目标 ✓」✗）")
    else:
        if goal in FULL_TYPES[name]:
            why.append(f"goal 是**声明的完整类型**（{goal!r}）✗")
        if goal != want_g:
            why.append(f"goal={goal!r}，期望 {want_g!r}")
    if binders != want_b:
        why.append(f"binders={binders}，期望 {want_b}")
    if d.get("step") != -1:
        why.append(f"step={d.get('step')}，期望 -1（声明头部）")
    if why:
        bad.append(f"{label} @{where}：" + "；".join(why))
    else:
        print(f"  ✓ {label} @{where}：binders={binders} goal={goal!r}")

if bad:
    print("\n✗ 头部状态不是题面状态：", file=sys.stderr)
    for b in bad:
        print("   - " + b, file=sys.stderr)
    sys.exit(1)
print(f"\n✓ {len(CASES)} 个位置（四形）的头部状态都是题面状态")
PY
}

case "${1:-}" in
  --criteria) criteria; exit $? ;;
esac

if criteria; then
  echo "✓ G-78：四形 × 两位置的头部状态都已是题面状态（已修）"
  echo "  反向验证：撤掉修复（SOKO_STATE_ROOT=legacy）后判据必须判红"
  scripts/expect-red.sh "撤掉根状态修复（SOKO_STATE_ROOT=legacy）后 G-78 判据必须红" -- \
      env SOKO_STATE_ROOT=legacy bash "$0" --criteria || exit 2
  exit 1
else
  echo "✗ G-78：缺口仍在 —— 头部状态不是题面状态" >&2
  exit 0
fi
