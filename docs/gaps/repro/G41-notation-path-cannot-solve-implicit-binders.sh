#!/usr/bin/env bash
# G-41 复现：**记法路径**（`∈` 一族）碰上**带隐式 binder 的库定义**时解不出参数。
#
# 最小复现（4 条 def + 2 条定理；`Set.mem`/`Set.univ` 的前导类型参数是 `{α}`）：
#
#     theorem unfold_mem_univ (α : Type) (x : α) : (x ∈ Set.univ) = Set.univ x := by rfl
#     ⇒ rfl 判定失败：两边不相等（期望 Sort(1)，实际 $1）
#     theorem mem_univ_holds (α : Type) (x : α) : x ∈ Set.univ := by
#       exact (fun (h : Set.univ x) => h) True.intro
#     ⇒ exact 类型不匹配：期望 x ∈ Set.univ，实际是 Set.univ（那个 Pi）
#
# 真库的后果（2026-09-26 实测）：把 `courses/set-theory/lib/Set.sokonanoda` 与
# `lib/Image.sokonanoda` 的前导类型参数改成 `{α : Type}` / `{α β : Type}` 之后，
# 课程门禁从 `36 目标 · 328 checked · 99 open · 0 判负` 掉到
# **`226 checked · 81 open · 14 判负`** ✗（`lib/Image` 的 `Set.mem_image`、
# `unit11` 的 `x ∈ Set.univ`、`unit04` 的 `cases` …）⇒ **B2 仍被挡住** ✓。
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
def Set.univ {α : Type} : Set α := fun (x : α) => True

theorem unfold_mem_univ (α : Type) (x : α) : (x ∈ Set.univ) = Set.univ x := by rfl
theorem mem_univ_holds (α : Type) (x : α) : x ∈ Set.univ := by
  exact (fun (h : Set.univ x) => h) True.intro
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
# 「已修」逐字相同** ✗ = **假绿**。⇒ 先要求这次判卷真的观测到**本夹具的声明事件**
# （可归属 ✓）或**一条诊断**；两样都没有 ⇒ **exit 2 + 人话**，绝不报「已修」✗。
# 同族已修好的形状：`G69` 的「无事件行 ⇒ exit 2」· `G34` 的「calls==0 ⇒ exit 2」。
if ! printf '%s' "$out" | grep -qE '"name":"unfold_mem_univ","type"|"name":"mem_univ_holds","type"|"type":"diagnostic"'; then
  echo "G-41：观测不到判卷事件（既没有本夹具两条定理的声明事件，也没有诊断）⇒ 环境/形状异常（exit 2），不是「已修」✗" >&2
  echo "      · 这条判据量的就是诊断码；一条事件都没有时 codes 恒为空 ⇒ 「已修」没有信息量 ✗" >&2
  echo "      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 夹具或模块根不对" >&2
  printf '%s\n' "$out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        → /' >&2
  exit 2
fi

echo "诊断码：${codes:-（无）}"
case "$codes" in
  *elab-tactic-failed*|*kernel-rejected*|*elab-implicit-argument-unsolved*|*elab-notation-argument-unsolved*)
    echo "G-41 仍在：记法路径碰上带隐式 binder 的库定义时解不出参数" >&2
    exit 0 ;;
esac
echo "G-41 已修：记法路径与隐式 binder 的库定义可以共存" >&2
exit 1
