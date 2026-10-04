#!/usr/bin/env bash
# **「预算耗尽只允许变慢，绝不允许变错」的实验判据** ✓（2026-10-04 值守派单 ✓）。
#
# 用户 13:12 原话：「这种闸我都不能接受」✓。总规矩：
#
# > **预算 / 尺寸 / 规模耗尽，只允许「变慢」，绝不允许「变错 / 变差」。**
#
# ## 这个脚本做什么
#
# 把三道**甲类闸**（「判不了 ⇒ 换路 / 判否」那类 ✗）**全部拧到最小值 1** ✓，
# 再跑同一份语料，然后要求：
#
# ① **闸真的被触发了** ✓（否则这个实验是**空转**的 ✗ —— 「咬不住的守卫等于没有」✓）；
# ② 语料的 `--json` 与默认配置**逐字节相同** ✓（= 只变慢 ✓，答案一个字都没变 ✓）；
# ③ `meta_budget_exhausted == 0` ✓（升级总是够用 ⇒ 没有一条约束落到"弃权" ✓）。
#
# ## 三道闸（读数出口见 `crates/kernel/src/gates.rs` ✓）
#
# | 闸 | 拧到 | 默认 | 出处 |
# |---|---|---|---|
# | `PROBE_CAP`（相等性探查步数） | 1 | 2048 | `kernel/conv.rs`（G-89） |
# | `MAX_TRACKED`（签名位掩码宽度） | 1 | 64 | `kernel/relevance.rs`（G-90） |
# | `maxHeartbeats`（求解步数 = fuel） | 1 | 4096 | `front/compile/meta.rs`（G-88） |
# | `maxRecDepth`（求解递归深度） | 1 | 64 | 同上（G-88） |
#
# ## 用法
#
# ```bash
# scripts/limits-only-slow.sh            # 子集语料（~30s，日常）
# scripts/limits-only-slow.sh --full     # 整本课程（~2min，收口）
# ```
#
# 退出码：0 = 判据全过 ✓ · 1 = 判红 ✗ · 2 = 环境不对（二进制不在等）。
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 2

BIN="${SOKO_BIN:-}"
if [ -z "$BIN" ]; then
  for cand in "$ROOT/target/release/sokonanoda" "$ROOT/target/debug/sokonanoda"; do
    [ -x "$cand" ] && BIN="$cand" && break
  done
fi
if [ -z "$BIN" ] || [ ! -x "$BIN" ]; then
  echo "环境不对：找不到 sokonanoda 二进制（先 cargo build --release -p sokonanoda-cli）" >&2
  exit 2
fi

COURSE="$ROOT/courses/set-theory"
[ -f "$COURSE/sokonanoda.toml" ] || { echo "环境不对：找不到 $COURSE/sokonanoda.toml" >&2; exit 2; }

# ── 语料：默认**子集**（快 ✓），`--full` 才整本 ✓ ────────────────────────────
if [ "${1:-}" = "--full" ]; then
  TARGET="$COURSE"
  LABEL="整本课程"
else
  TARGET="$COURSE/lib/Set.sokonanoda"
  LABEL="子集（lib/Set + unit08 + unit12）"
fi

# **跑一次 build，落盘 `--json` 与闸读数**；$1 = tag，其余 = 环境变量前缀。
run_once() {
  local tag="$1"; shift
  rm -rf "/tmp/soko-limits-$tag"
  env "$@" SOKO_STAGE_STATS=1 SOKONANODA_NO_PROJECT_ARTIFACTS=1 \
      SOKONANODA_CACHE_DIR="/tmp/soko-limits-$tag" \
      timeout 1800 "$BIN" build --json "$TARGET" \
      > "/tmp/soko-limits-$tag.json" 2> "/tmp/soko-limits-$tag.err"
  local rc=$?
  # 剔心跳/进度（它们带**墙钟**与**完成顺序** ⇒ 不是判定内容 ✓）
  grep -v -e '"type":"build.tick"' -e '"type":"build.progress"' \
      "/tmp/soko-limits-$tag.json" > "/tmp/soko-limits-$tag.cmp.json"
  return $rc
}

echo "== limits-only-slow（${LABEL}）=="
echo "   被测二进制: $BIN"

# 子集模式：把两个单元接在 lib 后面一起跑（`build <dir>` 只吃目录 ⇒ 用临时目录铺软链 ✗
# 会改变模块根 ⇒ 直接分三次跑、把三份 `--json` 串起来当"语料输出" ✓）。
if [ "${1:-}" = "--full" ]; then
  FILES=("")
else
  FILES=("$COURSE/lib/Set.sokonanoda" "$COURSE/units/I.3/unit08-images-preimages.sokonanoda" "$COURSE/units/I.4/unit12-synthesis.sokonanoda")
fi

collect() {
  local tag="$1"; shift
  : > "/tmp/soko-limits-$tag.cmp.json"
  : > "/tmp/soko-limits-$tag.err"
  for f in "${FILES[@]}"; do
    if [ -z "$f" ]; then
      run_once "$tag" "$@" || return 1
      cat "/tmp/soko-limits-$tag.cmp.json" >> "/tmp/soko-limits-$tag.all.json"
      cat "/tmp/soko-limits-$tag.err" >> "/tmp/soko-limits-$tag.all.err"
    else
      rm -rf "/tmp/soko-limits-$tag"
      env "$@" SOKO_STAGE_STATS=1 SOKONANODA_NO_PROJECT_ARTIFACTS=1 \
          SOKONANODA_CACHE_DIR="/tmp/soko-limits-$tag" \
          timeout 900 "$BIN" build --json "$f" \
          > "/tmp/soko-limits-$tag.json" 2> "/tmp/soko-limits-$tag.err" || return 1
      grep -v -e '"type":"build.tick"' -e '"type":"build.progress"' \
          "/tmp/soko-limits-$tag.json" >> "/tmp/soko-limits-$tag.all.json"
      cat "/tmp/soko-limits-$tag.err" >> "/tmp/soko-limits-$tag.all.err"
    fi
  done
  return 0
}

rm -f /tmp/soko-limits-default.all.json /tmp/soko-limits-default.all.err
rm -f /tmp/soko-limits-min.all.json /tmp/soko-limits-min.all.err
collect default || { echo "✗ 默认配置那一跑没跑起来" >&2; exit 2; }
collect min \
  SOKO_LIMIT_PROBE_CAP=1 SOKO_LIMIT_MAX_TRACKED=1 \
  SOKO_LIMIT_MAX_HEARTBEATS=1 SOKO_LIMIT_MAX_REC_DEPTH=1 \
  || { echo "✗ 拧到 1 那一跑没跑起来" >&2; exit 2; }

rc=0

# ① **闸真的被触发**（防空转 ✗）
read_counter() { grep -o "$2=[0-9]*" "$1" | tail -1 | cut -d= -f2; }
probe="$(read_counter /tmp/soko-limits-min.all.err probe_exhausted)"
sig="$(read_counter /tmp/soko-limits-min.all.err sig_overflow)"
clamp="$(read_counter /tmp/soko-limits-min.all.err sig_arity_clamped)"
escalated="$(read_counter /tmp/soko-limits-min.all.err meta_budget_escalated)"
exhausted="$(read_counter /tmp/soko-limits-min.all.err meta_budget_exhausted)"
echo "   拧到 1 时的闸读数: probe_exhausted=${probe:-?} sig_overflow=${sig:-?} sig_arity_clamped=${clamp:-?} meta_budget_escalated=${escalated:-?} meta_budget_exhausted=${exhausted:-?}"
if [ "${probe:-0}" = "0" ] || [ "${sig:-0}" = "0" ] || [ "${escalated:-0}" = "0" ]; then
  echo "✗ 判据空转 ✗：拧到 1 了闸却没被触发（probe=$probe sig=$sig escalated=${escalated}）" >&2
  echo "   ⇒ 这份语料**证明不了**「只变慢不变错」✗ —— 换一份真会触发的语料 ✓" >&2
  rc=1
fi

# ② **输出逐字节相同**（= 只变慢 ✓，答案一个字没变 ✓）
if cmp -s /tmp/soko-limits-default.all.json /tmp/soko-limits-min.all.json; then
  echo "✓ ② 默认 vs 全部拧到 1 ⇒ 判定输出**逐字节相同** ✓（$(wc -l < /tmp/soko-limits-default.all.json | tr -d ' ') 行）"
else
  echo "✗ ② 拧小预算**改变了判定输出** ✗ —— 那就是「判不了 ⇒ 当成否」✗" >&2
  diff /tmp/soko-limits-default.all.json /tmp/soko-limits-min.all.json | head -20 >&2
  rc=1
fi

# ③ **升级总是够用**（没有一条约束落到"弃权"）
if [ "${exhausted:-0}" != "0" ]; then
  echo "✗ ③ 有 $exhausted 条约束**升级到底仍弃权** ✗（预算不够 ⇒ 该调默认值）" >&2
  rc=1
else
  echo "✓ ③ meta_budget_exhausted=0 ✓（升级总是够用 ⇒ 没有约束落到「弃权」）"
fi

if [ "$rc" = "0" ]; then
  echo "✓ limits-only-slow 全过：三道甲类闸拧到 1 只**变慢**、判定**一字不变** ✓"
fi
exit "$rc"
