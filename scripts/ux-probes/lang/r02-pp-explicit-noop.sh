#!/usr/bin/env bash
# r02-pp-explicit-noop.sh —— `set_option pp.explicit true` 是静默空操作
# 现象: 画布 unit01 第 35 行写着 `set_option pp.explicit true  -- 显示隐式实参（常量带 @）`，
#       并靠它教学习者看隐式实参。实测开/关**逐字节相同**，没有任何诊断说它被忽略。
#       （用户原话：「不知道要不要传入」——课程提供的正是这个工具，而它不工作。）
# 期望: 要么真的显示隐式实参（`@Eq.subst`），要么给一条 warning 说该选项未实现。
# 判据: exit 0 = 问题仍在（开/关输出相同）/ exit 1 = 已生效
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
printf 'import lib.Set\n\n#print Set.singleton\n#check Set.singleton\n' > "$W/r02-off.sokonanoda"
printf 'import lib.Set\n\nset_option pp.explicit true\n#print Set.singleton\n#check Set.singleton\n' > "$W/r02-on.sokonanoda"
pick() {  # $1=文件 $2=事件类型  → 取该事件的文本
  timeout 300 "$REPO/scripts/soko" grade "$1" 2>&1 | python3 -c "
import json,sys
kind=sys.argv[1]
for line in sys.stdin:
    try: o=json.loads(line)
    except Exception: continue
    if o.get('type')==kind: print(o.get('text') or o.get('human') or '')
" "$2" | head -1
}
p_off=$(pick "$W/r02-off.sokonanoda" decl.printed)
p_on=$(pick "$W/r02-on.sokonanoda" decl.printed)
c_off=$(pick "$W/r02-off.sokonanoda" expr.typed)
c_on=$(pick "$W/r02-on.sokonanoda" expr.typed)
echo "#print off: $p_off"
echo "#print on : $p_on"
echo "#check off: $c_off"
echo "#check on : $c_on"
if [ -n "$p_off" ] && [ "$p_off" != "$p_on" ] && [ "$c_off" = "$c_on" ]; then
  echo "对照成立 ✓：档位对 #print 生效（写回 @-形态）、对 #check 裸常量有意不变（文档边界）"
  exit 0
fi
echo "对照失败 ✗：档位行为与文档不符（#print 该变而没变，或 #check 裸常量不该变而变了）"
exit 1
