#!/usr/bin/env bash
# 课程库 **import 闭包** 自检（2026-10-01 立，L-09 的守卫）
#
# 为什么需要它：`lib/` 里**两个模块定义了同名声明**时，各自单文件都判绿 ✓，
# 但**同时 import 就炸** ✗（后加载的那份报重名）—— 这是台账 **G-72「单文件绿、
# import 时红」** 的假绿家族 ✗。课程门禁按**单元**判卷 ⇒ 只要没有单元同时 import
# 那两个模块，冲突就一直潜伏 ✗（L-09 实测：`lib.Fun` 与 `lib.SUnion` 都定义 `Set.pi`，
# 全课程没有任何单元同时 import 两者 ⇒ 潜伏了很久 ✓）。
#
# 判据：生成一个**把所有 lib 模块一次 import 完**的临时文件并判卷 ⇒ 必须 exit 0 ✓
# 退出码：0 = 闭包 OK；1 = 有冲突；2 = 环境不满足
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"        # courses/set-theory

# 第一步：**跨模块重名扫描**（更快、更精确的诊断；重名就是闭包失败的头号原因）
DUPS="$(for f in "$ROOT"/lib/*.sokonanoda; do
          awk '/^(def|theorem|axiom) /{print $2}' "$f"
        done | sort | uniq -d)"
if [ -n "$DUPS" ]; then
  echo "lib 里存在**跨模块重名**声明（同一概念两个家 ✗）："
  for n in $DUPS; do
    echo "--- $n"
    grep -l "^\(def\|theorem\|axiom\) $n " "$ROOT"/lib/*.sokonanoda | sed 's|.*/lib/|   |'
  done
  exit 1
fi
REPO="$(cd "$ROOT/../.." && pwd)"
TMP="$ROOT/units/zzlibclosure.sokonanoda"
{
  for f in "$ROOT"/lib/*.sokonanoda; do
    b="$(basename "$f" .sokonanoda)"
    echo "import lib.$b"
  done
  echo
  echo "theorem lib_closure_ok : True := True.intro"
} > "$TMP"
cd "$REPO" || exit 2
OUT="$(node scripts/soko query check --file "$TMP" 2>&1)"
rc=$?
rm -f "$TMP"
if [ "$rc" -eq 0 ]; then
  echo "lib 闭包 OK（无跨模块重名 · 所有模块可同时 import）"
  exit 0
fi
echo "lib 闭包失败（重名定义？）："
echo "$OUT" | head -20
exit 1
