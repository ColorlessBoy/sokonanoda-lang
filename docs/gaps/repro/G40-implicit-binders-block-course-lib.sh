#!/usr/bin/env bash
# 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：要读原文用 `git log --all -- <path>`。doc-refs: history-only
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

# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE`）：判据是**诊断码**，
# 而「已修」那一档要的正是"一条诊断都没有" ⇒ **量具空转（零事件 / 零输出）时读数与
# 「已修」逐字相同** ✗ = **假绿**（守卫失效却不出声 ✗）。⇒ 先要求这次判卷真的观测到
# **本夹具的声明事件**（可归属 ✓）或**一条诊断**；两样都没有 ⇒ **exit 2 + 人话**，
# **绝不报「已修」** ✗。同族已修好的形状：`G69` 的「无事件行 ⇒ exit 2」·
# `G34` 的「calls==0 ⇒ exit 2」· `G25` 的「observed===0 ⇒ exit 2」。
if ! printf '%s' "$out" | grep -qE '"name":"zero_explicit_arg_constant","type"|"type":"diagnostic"'; then
  echo "G-40：观测不到判卷事件（既没有 zero_explicit_arg_constant 的声明事件，也没有诊断）⇒ 环境/形状异常（exit 2），不是「已修」✗" >&2
  echo "      · 这条判据量的就是诊断码；一条事件都没有时 codes 恒为空 ⇒ 「已修」没有信息量 ✗" >&2
  echo "      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 夹具或模块根不对" >&2
  printf '%s\n' "$out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        → /' >&2
  exit 2
fi

echo "诊断码：${codes:-（无）}"
case "$codes" in
  *elab-tactic-failed*|*kernel-rejected*|*elab-implicit-argument-unsolved*)
    echo "G-40 仍在：零显式实参的常量（∅）补不出隐式 α ⇒ 词项仍是 Pi" >&2
    exit 0 ;;
esac
echo "G-40 已修：零显式实参的常量也能补出隐式 α" >&2
exit 1
