#!/usr/bin/env bash
# r12-mixed-notation-in-error.sh —— 同一条报错里一半记法一半点名
# 现象: 画布 p03 的实况（Eq.subst 的 x a 写反）报
#       「`exact` 类型不匹配：期望 `x ∈ A`，实际是 `Set.mem a A`」——
#       同一个谓词，一边写 `∈` 一边写 `Set.mem`；而且两边变量还不同（x vs a），
#       初学者会以为「这两个看着一样啊」。
# 期望: 一条报错内渲染风格统一（都用记法），并把「x 与 a 不同」说清楚。
# 判据: exit 0 = 问题仍在（同一条 message 里既有 `∈` 又有 `Set.mem`）/ exit 1 = 已统一
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# ⚠ **M-07：干净缓存纪律**（2026-10-12 实测咬过一次）—— 内建 prelude / 判定逻辑改了以后
# **只重建不清缓存**时，旧条目会继续答旧行为（同一二进制无 import 时 `#print And.left` = theorem、
# `import lib.Set` 时却 = def）⇒ 每条探针都必须在**新 mktemp 缓存 + 关两处产物**下取读数，
# 否则「缺口仍在」可能是**旧答案**（r04 就这么假红过一次：`apply` 结论校验已修好、探针仍报 exit 0）。
FRESH_CACHE=$(mktemp -d)
export SOKONANODA_CACHE_DIR="$FRESH_CACHE"
export SOKONANODA_NO_PROJECT_ARTIFACTS=1
export SOKONANODA_NO_MODULE_ARTIFACTS=1
trap 'rm -rf "$FRESH_CACHE"' EXIT
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"
F="$REPO/.cache/ux/lang/repro/.work/r12.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem subset_of_mem_singleton (α : Type) (A : Set α) (a : α) (h : a ∈ A) :
    {a} ⊆ A := by
  intro x hx
  unfold Set.singleton Set.mem at hx
  apply Eq.subst α (fun (y : α) => y ∈ A) x a
  apply Eq.symm α x a hx
  exact h
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1)
msg=$(grep -o '"message": "[^"]*"' <<<"$out" | head -1)
echo "message: $msg"
if grep -q '∈' <<<"$msg" && grep -q 'Set.mem' <<<"$msg"; then
  echo "STILL REPRODUCES: 同一条报错混用 `∈` 记法与 `Set.mem` 点名"; exit 0
fi
echo "FIXED: 报错渲染已统一"; exit 1
