#!/usr/bin/env bash
# r09-unsolved-msg-placeholder.sh —— 缺隐式实参的诊断「例子」是个占位符
# 现象: `Eq.symm h` 报 elab-implicit-argument-unsolved，正文其实相当好（点了常量、
#       给了签名、说了有几个隐式参数、说了反解机制），但**示范写法是字面的
#       `Eq.symm …`**——省略号。用户已经知道要写 Eq.symm，缺的正是「逐位写什么」。
# 期望: 示范写成 `Eq.symm α a b h`（照签名逐位填），或至少标出 α/a/b 从哪来。
# 判据: exit 0 = 问题仍在（hint/message 里的示范含 `…`）/ exit 1 = 已给具体写法
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
F="$REPO/.cache/ux/lang/repro/.work/r09.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem bad_symm (α : Type) (a b : α) (h : a = b) : b = a := Eq.symm h
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1)
if grep -q 'Eq.symm …' <<<"$out"; then
  echo 'STILL REPRODUCES: 示范写法仍是占位符 Eq.symm ...'; echo "$out" | head -c 500; echo; exit 0
fi
echo "FIXED: 已给出可照抄的具体写法"; echo "$out" | head -c 400; exit 1
