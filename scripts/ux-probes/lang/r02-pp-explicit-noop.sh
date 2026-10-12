#!/usr/bin/env bash
# r02-pp-explicit-noop.sh —— `set_option pp.explicit true` 是静默空操作
# 现象: 画布 unit01 第 35 行写着 `set_option pp.explicit true  -- 显示隐式实参（常量带 @）`，
#       并靠它教学习者看隐式实参。实测开/关**逐字节相同**，没有任何诊断说它被忽略。
#       （用户原话：「不知道要不要传入」——课程提供的正是这个工具，而它不工作。）
# 期望: 要么真的显示隐式实参（`@Eq.subst`），要么给一条 warning 说该选项未实现。
# 判据: exit 0 = 问题仍在（开/关输出相同）/ exit 1 = 已生效
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"
W="$REPO/.cache/ux/lang/repro/.work"
printf 'import lib.Logic\nimport lib.Set\n\n#check Eq.subst\n' > "$W/r02-off.sokonanoda"
printf 'import lib.Logic\nimport lib.Set\n\nset_option pp.explicit true\n#check Eq.subst\n' > "$W/r02-on.sokonanoda"
off=$(timeout 300 "$REPO/scripts/soko" grade --root "$REPO/courses/set-theory" "$W/r02-off.sokonanoda" 2>&1 | grep -o '"human":"[^"]*"' | head -1)
on=$(timeout 300 "$REPO/scripts/soko" grade --root "$REPO/courses/set-theory" "$W/r02-on.sokonanoda" 2>&1 | grep -o '"human":"[^"]*"' | head -1)
echo "off: $off"; echo "on : $on"
if [ "$off" = "$on" ]; then
  echo "STILL REPRODUCES: pp.explicit true 对输出没有任何影响（且无 warning）"; exit 0
fi
echo "FIXED: pp.explicit true 改变了输出"; exit 1
