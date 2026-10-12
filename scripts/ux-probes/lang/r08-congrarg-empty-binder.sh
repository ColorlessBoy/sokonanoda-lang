#!/usr/bin/env bash
# r08-congrarg-empty-binder.sh —— `congrArg A h` 报错里出现**空名字的绑定** `Pi ( : …)`
# 现象: `congrArg A h`（真实 Lean 里完全合法）在本子集里失败，报错是内核黑话，
#       而且里面有一个**名字为空**的 Pi 绑定：`Pi ( : 第 4 个绑元（α）), (Set.[] …)`。
#       写全成 `congrArg α Prop A h` 也一样红；只有照 AGENTS.md 记的 Lean 顺序
#       `congrArg α Prop a b A h` 才判绿——但用户不可能知道要写 a b 这两个。
# 期望: 报一条像 `Eq.symm` 那样的可读诊断（给出签名 + 该写什么），或直接支持 `congrArg A h`。
# 判据: exit 0 = 问题仍在（报错含 `Pi ( :`）/ exit 1 = 已修
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
F="$REPO/.cache/ux/lang/repro/.work/r08.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem natural_form (α : Type) (A : Set α) (a b : α) (h : a = b) : A a = A b :=
  congrArg A h
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1)
if grep -q 'Pi ( :' <<<"$out"; then
  echo 'STILL REPRODUCES: 报错含空名绑定 Pi ( : ...) 的内核黑话'; echo "$out" | head -c 400; echo; exit 0
fi
echo "FIXED: congrArg 报错已可读"; echo "$out" | head -c 400; exit 1
