#!/usr/bin/env bash
# G-82 复现：**声明卡片对「证明体里有 tactic」的题不剥绑元** —— 同一形状，只因证明体不同，
# 卡片一会儿剥（`by sorry`）一会儿不剥（`by constructor; sorry`）✗。
#
# ── 缺口原文（2026-10-02 值守第 9 单顺带实测 ✗）──
#   同一句话、同一组绑元，只改**证明体的第一个 tactic**：
#     `theorem plain (a b : Prop) (h : a) : a ∧ a := by sorry`
#       ⇒ 卡片 `binders=[a,b,h]` ⊢ `a ∧ a`                    ✓（剥了）
#     `theorem with_tactic (a b : Prop) (h : a) : a ∧ a := by constructor; sorry`
#       ⇒ 卡片 `binders=[]` ⊢ `(a : Prop) → (b : Prop) → (h : a) → a ∧ a`  ✗（没剥）
#   ⇒ 学习者看到的是「题目参数忽然跑到目标里去了」，而且**顶（`query state`）跟着卡片一起错**
#     （G-81 之后顶 ≡ 底 ✓ —— 但底本身在这类声明上是错的 ✗）。
#
# ── 病根（定位到分支，未修）──
#   `crates/front/src/compile/goals.rs` 的 `goal_under_binders`：值位是
#   `Expr::Lambda … Expr::Hole` 时按 λ 链逐层剥（✓ 正常）；但证明体里**有 tactic**
#   （`by constructor; …`）时走的是**兜底分支**（`binders: Vec::new()` + `render_expr(ty)`）
#   ⇒ 卡片退回整句声明类型 ✗。
#
# ── 判据（本复现件）──
#   两条声明的卡片**都**必须是 `binders=[a,b,h]` ⊢ `a ∧ a`（= 题面状态 ✓）。
#   现在 `with_tactic` 那条不过 ⇒ exit 0（缺口仍在 ✓，与台账 `open` 一致）。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-82: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cat > "$WORK/plain.sokonanoda" <<'EOF'
theorem plain (a b : Prop) (h : a) : a ∧ a := by
  sorry
EOF

cat > "$WORK/with_tactic.sokonanoda" <<'EOF'
theorem with_tactic (a b : Prop) (h : a) : a ∧ a := by
  constructor
  sorry
EOF

python3 - "$BIN" "$WORK" <<'PY'
import json, os, subprocess, sys

bin_, work = sys.argv[1], sys.argv[2]
bad = []
for name in ("plain", "with_tactic"):
    path = os.path.join(work, f"{name}.sokonanoda")
    out = subprocess.run(
        [bin_, "query", "goals", "--file", path], capture_output=True, text=True
    )
    try:
        data = json.loads(out.stdout).get("data") or []
    except Exception:
        print(f"✗ {name}: query goals 没给出 JSON：{(out.stdout or out.stderr)[:120]!r}", file=sys.stderr)
        sys.exit(2)
    entry = next((g for g in data if g.get("name") == name), None)
    if entry is None:
        print(f"✗ {name}: query goals 里没有这条声明（形状变了）", file=sys.stderr)
        sys.exit(2)
    binders = [b.get("name") for b in (entry.get("binders") or [])]
    goal = entry.get("goal")
    ok = binders == ["a", "b", "h"] and goal == "a ∧ a"
    print(f"  {'✓' if ok else '✗'} {name:12} 卡片 binders={binders} goal={goal!r}")
    if not ok:
        bad.append(name)

if bad:
    print(f"\n✗ 缺口仍在：{bad} 的声明卡片没剥绑元（应 = 题面状态 `[a,b,h] ⊢ a ∧ a`）",
          file=sys.stderr)
    sys.exit(0)   # 0 = 缺口仍在（与台账 status=open 一致）
print("✓ 两条声明的卡片都是题面状态 ⇒ 行为已变（回来关账）")
sys.exit(1)
PY
