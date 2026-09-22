#!/usr/bin/env bash
# G-34 复现：**记法/探针消解问内核要类型 ⇒ 每次换前缀就全前缀重编译一趟 pass**。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境异常。
#
# 判据：`grade` 一份**课程库文件**时的 `JUDGE_INFER_SPLIT misses=`。
#
# 坑（bash 3.2）：`$var` 后面**紧跟全角字符**时，那串字节会被吞进变量名
# （`missesï¼: unbound variable`）⇒ 一律写 `${var}` 定界。
# 为什么是"未命中次数"而不是墙钟：墙钟受机器负载影响，而未命中一次 = 前缀被
# 重编译一趟 pass，是**结构**量（同一份输入同一台机器上稳定）。
#
# 已修的标准：记法消解不再走 `judge_infer`（即前端能在**当前 pass 的环境**上
# 就地要类型），于是 misses 应当落到 0 附近。到那天这条复现会 exit 1，回来
# 关账并写 `fixed_in`。
#
# 为什么拿 `lib/Set.sokonanoda` 当夹具：它是课程库里记法最密的一份（六条记法
# + 两条两段式 binder），单文件、零 import、秒级；缺口就在它的闭包里。
set -u
cd "$(dirname "$0")/../../.." || exit 2

FIXTURE="courses/set-theory/lib/Set.sokonanoda"
[ -f "$FIXTURE" ] || { echo "   → 找不到夹具 $FIXTURE" >&2; exit 2; }

# 阈值：修之前实测 28（0.64.0）。取 10 留足余量——只要"每条声明都重编前缀"
# 这个机制还在，数量级就不会掉到个位数。
THRESHOLD=10

CACHE_DIR="$(mktemp -d)"
trap 'rm -rf "$CACHE_DIR"' EXIT

out="$(SOKO_JUDGE_STATS=1 SOKONANODA_CACHE_DIR="$CACHE_DIR" \
  scripts/soko grade "$FIXTURE" 2>&1 >/dev/null)" || {
  echo "   → grade 跑不起来（退出码非 0）" >&2
  echo "$out" | tail -5 >&2
  exit 2
}

line="$(printf '%s\n' "$out" | grep '^JUDGE_INFER_SPLIT ' | tail -1)"
[ -n "$line" ] || { echo "   → 没拿到 JUDGE_INFER_SPLIT（SOKO_JUDGE_STATS 没生效？）" >&2; exit 2; }

misses="$(printf '%s\n' "$line" | sed -n 's/.*misses=\([0-9]*\).*/\1/p')"
calls="$(printf '%s\n' "$out" | sed -n 's/^JUDGE_INFER calls=\([0-9]*\).*/\1/p' | tail -1)"
[ -n "$misses" ] || { echo "   → 解析不出 misses：$line" >&2; exit 2; }

echo "   judge_infer calls=${calls} misses=${misses}（阈值 ${THRESHOLD}）"
if [ "$misses" -ge "$THRESHOLD" ]; then
  echo "   → 缺口仍在：${misses} 次未命中 = 整段前缀被重编译 ${misses} 趟 pass"
  exit 0
fi
echo "   → 已修：记法消解不再全前缀重编译（misses=${misses}）"
exit 1
