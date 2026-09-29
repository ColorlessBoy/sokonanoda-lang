#!/usr/bin/env bash
# **rebuild 基线量具**（G-68 / 切片 1 的收益估算输入）。
#
# 量什么：真课程上"**编译依赖**"占一次**冷编**多少墙钟 ——
#   A 组 = 以某入口为入口编**整条闭包**（今天 `build <file>` 做的事）；
#   B 组 = 把该入口闭包里的 `lib/*` 拷进临时项目、入口只留"import + 一条平凡定理"
#          ⇒ 编的就是**纯依赖**。
#   B/A 就是"每个模块只编一次"能砍掉的那部分（**墙钟**口径）。
#
# 为什么必须有它：`docs/perf/rebuild-baseline-2026-09-28.md` 的六项数字与本文的占比
# 都是**实测**，而当时的量具放在 `/tmp`（会被清掉）⇒ 数字就不可复现了。
#
# ⚠ **口径陷阱**（实测踩到）：整轮冷编里，**后面的入口会因为进程内 `judge_infer` 缓存
# （按前缀命中）而变便宜** ⇒ 单文件独立进程测出的 A 与它在整轮里的那一段**不可比**。
# 所以本量具一律用"**独立进程 + 冷缓存 + 同口径 A/B**"，并且每组测 2 次。
#
# ⚠ **profile 必须写进输出**（用户 2026-09-29 定规）：`build` 子命令在 debug/release 下
# **实测只差 3%**（`docs/perf/course-profile-2026-09-29.md` §5），但**别处**（CLI 集成测试
# 二进制）debug 慢一个量级 ⇒ 数字必须自带 profile，否则没法判断可比性 ✗。
# 默认 **release**（用户侧看到的那个；`docs/perf/ledger.jsonl` 也是 release 口径）。
#
# 用法：scripts/measure-rebuild.sh [入口文件…]
#   （默认测 `courses/set-theory` 的三个代表性入口；二进制默认
#     `target/release/sokonanoda`，可用 `SOKONANODA_BIN` 覆盖。）
#
# 输出：stdout 的 markdown 表（直接贴进 docs/perf/ 记录即可）。
set -u
cd "$(dirname "$0")/.." || exit 2
BIN="${SOKONANODA_BIN:-$PWD/target/release/sokonanoda}"
# 兜底：release 没构建过就明确报错，**不要**悄悄退到 debug ✗（那正是这次踩的坑）。
if [ ! -x "$BIN" ] && [ -z "${SOKONANODA_BIN:-}" ]; then
  echo "找不到 release 二进制：$BIN" >&2
  echo "先跑：cargo build --release -p sokonanoda-cli --locked" >&2
  echo "（要量 debug 请显式：SOKONANODA_BIN=$PWD/target/debug/sokonanoda $0）" >&2
  exit 3
fi
ROOT="$PWD/courses/set-theory"
[ -x "$BIN" ] || { echo "找不到可执行的 CLI：$BIN（先 cargo build，或设 SOKONANODA_BIN）" >&2; exit 2; }
[ -d "$ROOT" ] || { echo "找不到课程根：$ROOT" >&2; exit 2; }

DEFAULT_ENTRIES=(
  "units/solutions/unit08-solution.sokonanoda:Exists,Image,Logic,Set"
  "units/solutions/unit12-solution.sokonanoda:Equiv,Exists,Fun,Image,Logic,Rel,Set"
  "units/unit05-pairs-products.sokonanoda:Logic,Prod,Set"
)
ENTRIES=("$@"); [ "${#ENTRIES[@]}" -gt 0 ] || ENTRIES=("${DEFAULT_ENTRIES[@]}")

timed() { # $1 = 命令数组的字符串（用 bash -c 跑）→ 打印秒数
  local t0 t1
  t0=$(python3 -c 'import time;print(time.time())')
  bash -c "$1" >/dev/null 2>&1
  t1=$(python3 -c 'import time;print(time.time())')
  python3 -c "print(f'{$t1-$t0:.2f}')"
}

full_once() { # $1 = entry rel path
  local c; c=$(mktemp -d)
  SOKONANODA_CACHE_DIR="$c" "$BIN" build --json --clean "$ROOT" >/dev/null 2>&1
  timed "SOKONANODA_CACHE_DIR='$c' '$BIN' build --json '$ROOT/$1'"
  rm -rf "$c"
}

deps_once() { # $1 = comma-separated lib names
  local d c libs="$1" imports="" m
  d=$(mktemp -d); mkdir -p "$d/lib"
  printf 'name = "deps-only"\n' > "$d/sokonanoda.toml"
  for m in ${libs//,/ }; do
    cp "$ROOT/lib/$m.sokonanoda" "$d/lib/"
    imports="${imports}import lib.$m"$'\n'
  done
  printf '%stheorem probe (P : Prop) : P → P := fun h => h\n' "$imports" > "$d/probe.sokonanoda"
  c=$(mktemp -d)
  SOKONANODA_CACHE_DIR="$c" "$BIN" build --json --clean "$d" >/dev/null 2>&1
  timed "SOKONANODA_CACHE_DIR='$c' '$BIN' build --json '$d/probe.sokonanoda'"
  rm -rf "$c" "$d"
}

PROFILE="unknown"
case "$BIN" in
  */target/release/*) PROFILE="release" ;;
  */target/debug/*)   PROFILE="debug" ;;
esac
echo "**profile：\`$PROFILE\`**（口径见 \`docs/perf/course-profile-2026-09-29.md\`；debug/release 在 \`build\` 上实测只差 3%）"
echo "二进制：\`$("$BIN" --version)\` · 靶子：\`courses/set-theory\` · 主机：$(uname -sm) · 核数：$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo '?')"
echo
echo "| 入口 | 闭包 | A 整条闭包（冷, s） | B 只编依赖（冷, s） | B/A |"
echo "|---|---|---|---|---|"
for spec in "${ENTRIES[@]}"; do
  entry="${spec%%:*}"; libs="${spec#*:}"
  [ "$entry" = "$spec" ] && { echo "用法：<入口相对路径>:<逗号分隔的依赖>，例：units/unit05-pairs-products.sokonanoda:Logic,Prod,Set" >&2; exit 2; }
  a1=$(full_once "$entry"); a2=$(full_once "$entry")
  b1=$(deps_once "$libs");   b2=$(deps_once "$libs")
  python3 - "$entry" "$libs" "$a1" "$a2" "$b1" "$b2" <<'PY'
import sys
entry, libs, a1, a2, b1, b2 = sys.argv[1], sys.argv[2], *map(float, sys.argv[3:7])
a, b = (a1 + a2) / 2, (b1 + b2) / 2
import os
closure = len(libs.split(",")) + 1
print(f"| `{os.path.basename(entry)}` | {closure} | {a1:.2f} / {a2:.2f} | {b1:.2f} / {b2:.2f} | {b/a*100:.0f}% |")
PY
done
