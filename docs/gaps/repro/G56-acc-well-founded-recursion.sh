#!/usr/bin/env bash
# G-56 自断言复现：**`Acc`（良基性）立不起来 ⇒ 语言里没有良基递归**。
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在（**当前期望形状**：`Acc` 那一段必须被内核拒，且诊断逐字为
#       "inductive occurrence is not applied uniformly to the block parameters
#        and universe levels"）
#   1 = 行为变了（`Acc` 能立起来了 ⇒ 回来关账、把 ST7/ST9 的"卡在良基递归"划掉）
#   2 = 环境不满足（二进制找不到）
#
# 现场（v0.77.0 · ST1 探针，2026-09-28 实测；决策记录 docs/design/v077-st1-boundary.md）：
#   `inductive Acc (α : Type) (r : α → α → Prop) (x : α) : Prop` +
#   `ctor Acc.intro (h : ∀ (y : α), r y x → Acc α r y) : Acc α r x` ⇒ 内核拒。
#   递归出现 `Acc α r y` 的**下标 `y` 与块参数 `x` 不同** ⇒ 撞"uniform"检查
#   （crates/kernel/src/inductive.rs:180）。**对照组成立**：同形状但下标不变化的
#   `Even : Nat → Prop`（`Even.succ : Even n → Even (n+2)`）**能过** ⇒ 被拒的是
#   "下标会变"，不是"载体是函数/Prop 值"。
#
# 为什么它重要：谓词式序数（ST1 探针 ②）**写得出来**，但「秩 rank」「超限递归」
# 「V 层级」（ST7/ST9）全要良基递归。Mathlib 的 `Ordinal.rank` 走 `Acc.recOn`
# （`src/Init/WF.lean` 全文件 `Quot` 出现 0 次）；Isabelle `ZFC_in_HOL` 把良基性
# 做成**公理** `foundation: "wf {(x,y). x ∈ elts y}"` 再用 `wfrec`。**两条路都要
# "良基递归可用"**，区别只是它由归纳类型给还是由公理给。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-56: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ① 缺口本体：`Acc` 必须**仍然**被拒，且诊断逐字固定
cat > "$WORK/acc.sokonanoda" <<'EOF'
inductive Acc (α : Type) (r : α → α → Prop) (x : α) : Prop
ctor Acc.intro (h : ∀ (y : α), r y x → Acc α r y) : Acc α r x
end
EOF
acc_out="$("$BIN" --json --no-project "$WORK/acc.sokonanoda" 2>&1)"
if ! printf '%s' "$acc_out" | grep -q 'inductive occurrence is not applied uniformly to the block parameters and universe levels'; then
  echo "G-56 ①：`Acc` 不再被那句 uniform 检查拒了 —— 行为已变，回来关账" >&2
  printf '%s\n' "$acc_out" | head -3 >&2
  exit 1
fi

# ② 对照组：同形状、下标**不变化**的归纳块必须能过（否则缺口描述失真）
cat > "$WORK/even.sokonanoda" <<'EOF'
inductive Even : Nat → Prop
ctor Even.zero : Even 0
ctor Even.succ (n : Nat) (h : Even n) : Even (Nat.succ (Nat.succ n))
end
EOF
even_out="$("$BIN" --json --no-project "$WORK/even.sokonanoda" 2>&1)"
if ! printf '%s' "$even_out" | grep -q '"type":"decl.checked"'; then
  echo "G-56 ②：对照组 `Even : Nat → Prop` 也不过了 —— 缺口描述失真（不只是 uniform 那条）" >&2
  printf '%s\n' "$even_out" | head -3 >&2
  exit 1
fi

echo 'G-56：缺口仍在（`Acc` 被 uniform 检查拒；对照组 Even 正常）'
exit 0
