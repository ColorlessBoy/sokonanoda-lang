#!/usr/bin/env bash
# r15-kernel-span-whole-decl.sh —— kernel-rejected 的 span 覆盖整条声明，粒度与 elab 类不一致
# 现象: 同一个文件里：
#   · elab-tactic-failed  → span = 那一条 tactic（如 L6 C3–C11，`exact hh`），精确可用；
#   · elab-unknown-identifier → span = 那个标识符（如 L5 C71–C72，`𝒬`），精确可用；
#   · kernel-rejected → span = **整条声明**（如 L5 C1 → L7 C5，从 theorem 到函数体结束）。
#       配上内核黑话，用户既不知道哪一行错，也不知道哪个子项错。
# 期望: kernel-rejected 也定位到出错子表达式（或至少报出它在声明内的路径）。
# 判据: exit 0 = 问题仍在（kernel-rejected 的 span 跨多行）/ exit 1 = 已精确
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"
F="$REPO/.cache/ux/lang/repro/.work/r15.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem simple_mismatch (α : Type) (A : Set α) (a b : α) (ha : a ∈ A) :
    b ∈ A :=
  ha
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1)
sl=$(grep -o '"start_line": [0-9]*' <<<"$out" | head -1 | grep -o '[0-9]*')
el=$(grep -o '"end_line": [0-9]*' <<<"$out" | head -1 | grep -o '[0-9]*')
echo "kernel-rejected span: start_line=$sl end_line=$el"
if [ -n "$sl" ] && [ -n "$el" ] && [ "$el" -gt "$sl" ]; then
  echo "STILL REPRODUCES: kernel-rejected 的 span 跨 $((el-sl+1)) 行（整条声明），无法定位出错子项"; exit 0
fi
echo "FIXED: kernel-rejected span 已精确"; exit 1
