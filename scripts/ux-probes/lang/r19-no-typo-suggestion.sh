#!/usr/bin/env bash
# r19-no-typo-suggestion.sh —— 变量名打错时不给「你是不是想写 h」的建议
# 现象: `exact hh`（手里只有 `h`）报「`exact` 判定失败：unknown identifier `hh`」——
#       定位准、但**没有任何候选**。初学者最常见的就是拼错名字，而所有 IDE 都会给候选。
#       同一条消息还是中英混排（`unknown identifier` 未本地化，与中文前缀并列）。
# 期望: 「unknown identifier `hh`（在作用域内的相近名字：h）」，并统一语言。
# 判据: exit 0 = 问题仍在（无候选建议）/ exit 1 = 已给建议
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
F="$REPO/.cache/ux/lang/repro/.work/r19.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem typo_var (α : Type) (A : Set α) (a : α) (h : a ∈ A) : a ∈ A := by
  exact hh
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1)
if grep -q 'unknown identifier `hh`' <<<"$out" && ! grep -qE '你是不是|相近|候选|did you mean' <<<"$out"; then
  echo "STILL REPRODUCES: 未给拼写候选"; echo "$out" | head -c 300; echo; exit 0
fi
echo "FIXED: 已给拼写候选"; exit 1
