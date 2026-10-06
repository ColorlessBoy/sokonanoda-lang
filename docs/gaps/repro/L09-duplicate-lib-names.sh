#!/usr/bin/env bash
# L-09 复现/守卫件：课程库 **不是 import-闭的** —— 两个模块定义同名声明时，
# 各自单文件判绿 ✓、**同时 import 就炸** ✗（G-72 假绿家族的库级实例）
#
# 历史现场（2026-10-01 修前）：`lib/Fun` 与 `lib/SUnion` **都定义 `Set.pi`** ✗
#   —— `lib/SUnion` 的是非依赖版（`ι → α`，单元㉓ 用 ✓），`lib/Fun` 的是依赖版（无人用 ✗）；
#   **全课程没有任何单元同时 import 两者** ⇒ 冲突潜伏 ✗（是审计探针把 8 个 lib 一起 import 才炸出来的 ✓）。
#
# 退出码约定（同其他 repro 脚本，见 docs/gaps/README.md）：
#   0 = 缺口仍在（闭包失败）· 1 = 已修（闭包 OK）· 2 = 环境不满足
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2
bash courses/set-theory/tools/check-lib-closure.sh >/dev/null 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then
  echo "lib 闭包 OK ⇒ L-09 已修（重名的那份 \`Set.pi\` 已删；守卫：tools/check-lib-closure.sh）"
  exit 1
fi
if [ "$rc" -eq 2 ]; then echo "环境不满足"; exit 2; fi
echo "lib 闭包失败 ⇒ 仍有重名定义（L-09 仍在）"
exit 0
