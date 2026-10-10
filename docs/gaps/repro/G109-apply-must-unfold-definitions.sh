#!/usr/bin/env bash
# G109 —— `apply` 的目标匹配必须支持**定义展开（defeq）**，与 Lean 4 的 `Meta.apply` 对齐。
#
# 用户现场（2026-10-10）：
#   theorem mem_of_subset_singleton (α : Type) (A : Set α) (a : α) (h : {a} ⊆ A) : a ∈ A := by
#     apply h
#     exact Eq.refl a        -- ✓ 可以
#     apply Eq.refl a        -- ✗ 「apply 的目标不匹配：Eq.refl a 的结果是 a = a，无法对齐当前目标 a ∈ ({a})」
# 根因：目标 `a ∈ ({a})` 与 `a = a` 是 **defeq**（`Set.mem` 展开 + beta + `Set.singleton` 展开），
# `exact` 走内核判等（defeq）能闭合，而 `apply` 只做**表面匹配**（不 whnf/不展开）⇒ 判不匹配 ✗。
# Lean 4 的 `apply` 基于 `Meta.apply`/`isDefEq`，会展开定义、能成功 ⇒ 本件对齐它。
#
# 夹具自包含（只用 prelude）：`Set.mem a (Set.singleton a)` defeq 于 `a = a`。
# 判据：对照组 `via_exact` **必须** checked（环境自检）；`via_apply` 若也 checked ⇒ 已修（exit 1）；
# 只要它带 `apply … 目标不匹配` 诊断 ⇒ 缺口仍在（exit 0）。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常
set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO=scripts/soko
[ -x "$SOKO" ] || { echo "找不到 $SOKO" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "需要 python3" >&2; exit 2; }

dir=$(mktemp -d) || exit 2
trap 'rm -rf "$dir"' EXIT
cat >"$dir/Apply.sokonanoda" <<'EOF'
def Set (α : Type) : Type := α -> Prop

def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a

def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a

theorem via_exact (α : Type) (a : α) : Set.mem a (Set.singleton a) := by
  exact Eq.refl a

theorem via_apply (α : Type) (a : α) : Set.mem a (Set.singleton a) := by
  apply Eq.refl a
EOF

"$SOKO" grade --json "$dir/Apply.sokonanoda" >"$dir/events.jsonl" 2>"$dir/err.txt" || true
python3 - "$dir/events.jsonl" <<'PY'
import json, sys
checked, failed, messages = set(), set(), []
for line in open(sys.argv[1], encoding="utf-8"):
    line = line.strip()
    if not line.startswith("{"):
        continue
    event = json.loads(line)
    kind = event.get("type")
    if kind == "decl.checked":
        checked.add(event.get("name"))
    elif kind == "decl.failed":
        failed.add(event.get("name"))
    elif kind == "diagnostic":
        messages.append(event.get("message") or "")
if "via_exact" not in checked:
    print("对照组 via_exact 都没通过 ⇒ 环境/形状异常：%s" % (sorted(checked),), file=sys.stderr)
    raise SystemExit(2)
apply_bad = "via_apply" in failed or any("apply" in m and "不匹配" in m for m in messages)
if not apply_bad and "via_apply" in checked:
    print("已修：apply 也能展开定义（defeq）匹配 —— via_apply 通过")
    raise SystemExit(1)
print("缺口仍在：apply 只做表面匹配（未修）—— 诊断：%s" % ([m for m in messages if "不匹配" in m][:1],))
raise SystemExit(0)
PY
