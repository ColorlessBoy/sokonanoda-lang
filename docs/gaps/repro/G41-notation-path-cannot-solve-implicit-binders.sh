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

echo "诊断码：${codes:-（无）}"
case "$codes" in
  *elab-tactic-failed*|*kernel-rejected*|*elab-implicit-argument-unsolved*|*elab-notation-argument-unsolved*)
    echo "G-41 仍在：记法路径碰上带隐式 binder 的库定义时解不出参数" >&2
    exit 0 ;;
esac
echo "G-41 已修：记法路径与隐式 binder 的库定义可以共存" >&2
exit 1
