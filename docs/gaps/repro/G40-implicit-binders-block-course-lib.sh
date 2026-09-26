#!/usr/bin/env bash
# G-40 复现：**前导类型参数改成 `{α : Type}` 的常量，在"零显式实参"位置上补不出 α**。
#
# 形状（真课程库的写法，去掉上下文后的最小复现）：
#
#     def Set.empty {α : Type} : Set α := fun (x : α) => False
#     notation "∅" => Set.empty
#     theorem t (α : Type) (a : α) : (a ∈ ∅) = False := by rfl
#
# 今天的表现：`∅`（= 零显式实参的 `Set.empty`）**没有被插入隐式实参** ⇒ 词项还是那个
# **Pi**（`{α : Type} → Set α`）⇒ 判定报
# `rfl 判定失败：两边不相等（期望 (Set.[] $1)，实际 Pi (α : Sort(1)), (Set.[] $0)）`。
#
# **这是设计内的推迟**（`docs/design/implicit-arguments.md` §7 第 1 条 / §8 的 X14：
# "从期望类型解"那一档明确推迟）—— 本条目记它不是为了"修 bug"，而是钉住
# **B2 不能先于 B3**：
#   * 只把 `image`/`preimage` 改成 `{α β}` ⇒ 真库自己的定理（`Set.mem_image` 一族）
#     就红（`期望 Sort(1)，实际 Pi (_ : $4), $4`）；
#   * 把核心族（`mem`/`subset`/`singleton`/`union`/`inter`/…）也改 ⇒ 课程门禁从
#     `328 checked · 0 判负` 掉到 **`249 checked · 81 open · 7 判负`** ✗。
# 复现整套：先改 `courses/set-theory/lib/Set.sokonanoda` 的前导 `(α : Type)` 为
# `{α : Type}`，再跑 `python3 courses/set-theory/tools/check.py`。
#
# 退出码（见 docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v python3 >/dev/null 2>&1 || { echo "需要 python3" >&2; exit 2; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cat > "$work/repro.sokonanoda" <<'EOF'
def Set (α : Type) : Type := α -> Prop
def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a
infix:50 " ∈ " => Set.mem
def Set.empty {α : Type} : Set α := fun (x : α) => False
notation "∅" => Set.empty

theorem zero_explicit_arg_constant (α : Type) (a : α) : (a ∈ ∅) = False := by rfl
EOF

out="$(scripts/soko grade --json "$work/repro.sokonanoda" 2>&1)"
codes="$(printf '%s' "$out" | python3 -c '
import sys, json
codes = []
for line in sys.stdin:
    line = line.strip()
    if not line.startswith("{"):
        continue
    try:
        ev = json.loads(line)
    except json.JSONDecodeError:
        continue
    if ev.get("type") == "diagnostic":
        codes.append(ev.get("code", "?"))
print(" ".join(codes))
')"

echo "诊断码：${codes:-（无）}"
case "$codes" in
  *elab-tactic-failed*|*kernel-rejected*|*elab-implicit-argument-unsolved*)
    echo "G-40 仍在：零显式实参的常量（∅）补不出隐式 α ⇒ 词项仍是 Pi" >&2
    exit 0 ;;
esac
echo "G-40 已修：零显式实参的常量也能补出隐式 α" >&2
exit 1
