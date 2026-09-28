#!/usr/bin/env bash
# G-68 自断言复现：**项目 build/rebuild 的缓存键是 per-entry-closure ⇒ 共享依赖在每个入口里各编一遍**。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/前置缺失。
#
# 判据（**结构性**，不看墙钟 —— 墙钟受负载影响，见 G-25 的教训）：
#   夹具 = 一个共享依赖 `lib/Shared.sokonanoda`（**恰好一个 `by` 证明**）+ 三个入口
#   a/b/c，每个都 `import lib.Shared`，入口自身是**项风格**（0 个 `by`）。
#   `SOKO_STAGE_STATS=1` 的 `by_calls` 数的是 **`by` 引擎被调用的次数** ⇒
#   依赖每被 elaborate 一次就 +1，于是它就是"共享依赖被编了几次"的**直接读数**：
#     * `build a b c` ⇒ 今天 **3**（Shared 在三条闭包里各编一次）
#     * `build a`     ⇒ 今天 **1**（自校准基线，抵消 prelude 等固定开销）
#   判据：`by_calls(3 入口) >= 3 × by_calls(1 入口)` ⇒ **缺口仍在**（exit 0）。
#   缓存键改成 per-module 之后，第 2、3 个入口应当**复用** Shared ⇒ 比值掉到 1×
#   ⇒ 本脚本 exit 1，回来关账（`python3 scripts/gap.py close G-68 --version …`）。
#
# 根因（读代码定位，非推测）：
#   * `ProjectPlan::digest`（`crates/front/src/project/cache.rs:20-33`）把**整条闭包**
#     （拓扑序上每个模块的名字/源/imports）折进**一个**键 ⇒ 条目是**按入口**存的；
#   * `compile_plan`（`crates/front/src/project/mod.rs:304-327`）把闭包里**每个模块**
#     都交给 `compile_all_units` ⇒ 共享依赖在每个入口的编译里各 elaborate 一次、
#     各过一遍内核检查；
#   * CLI `build`（`crates/cli/src/build.rs:165-190`）逐入口 `load_at`/`store_at`，
#     闭包之间**没有任何共享** ⇒ 42 个入口的课程项目实测 **174 次模块编译（4.14×）**。
#
# 实测（2026-09-28 · Darwin arm64 · 0.78.0）：
#   `courses/set-theory`（42 模块 / 42 入口 / 7451 行）：
#   Σ闭包 = **174** 次模块编译（`lib.Logic` 42 次 · `lib.Set` 37 次 · `lib.Exists` 26 次）
#   ⇒ **4.14×** 重复；rebuild = clean 0.114s + build 221.97s。
#   六项数字、三条命令对照与原始 STAGE_STATS 行：`docs/perf/rebuild-baseline-2026-09-28.md`。
#
# 注意（诚实记录，不夸大）：`by_calls` 数的是**调用次数**；同一份依赖在第 2、3 个入口里
#   的判定会命中 judge 缓存（实测 `hits=2 misses=1`）⇒ **`by` 判定本身**那一小段是被复用的，
#   但**模块的 elaborate + 内核检查**没有（G-29：前端没有"往已有环境里再 elaborate 一条声明"
#   的入口，编译结束环境就没了）⇒ 重复功仍在，只是其中一层有缓存。
set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
command -v node >/dev/null 2>&1 || { echo "需要 node（scripts/soko 是零依赖 Node 启动器）" >&2; exit 2; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
CACHE="$TMP/.cache"          # 全局缓存挪进临时目录：不碰用户缓存、可重复
mkdir -p "$TMP/lib"
printf 'name = "g68-fixture"\n' > "$TMP/sokonanoda.toml"

# 共享依赖：**恰好一个 `by` 证明**（判据的分度就是它）。
cat > "$TMP/lib/Shared.sokonanoda" <<'EOF'
theorem shared_and (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
EOF
# 三个入口：都 import 同一个依赖；自身项风格（不引入额外的 `by`，否则判据被污染）。
for e in a b c; do
  cat > "$TMP/$e.sokonanoda" <<EOF
import lib.Shared
theorem ${e}_thm (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := shared_and P Q hp hq
EOF
done

# 量一次：清缓存 → 冷编 → 取 `by_calls`。$@ = 要编的入口。
measure() {
  rm -rf "$CACHE"
  SOKONANODA_CACHE_DIR="$CACHE" "$SOKO" build --json --clean "$TMP" >/dev/null 2>&1 || return 1
  stats="$(SOKONANODA_CACHE_DIR="$CACHE" SOKO_STAGE_STATS=1 "$SOKO" build --json "$@" 2>&1 >/dev/null \
    | grep '^STAGE_STATS ' | tail -1)"
  [ -n "$stats" ] || return 1
  printf '%s\n' "$stats"
}

shared_line="$(measure "$TMP/a.sokonanoda" "$TMP/b.sokonanoda" "$TMP/c.sokonanoda")" || {
  echo "   → 跑不起来（build 或 SOKO_STAGE_STATS 没生效）" >&2; exit 2; }
one_line="$(measure "$TMP/a.sokonanoda")" || {
  echo "   → 跑不起来（单入口基线）" >&2; exit 2; }

by_calls() { printf '%s\n' "$1" | sed -n 's/.*by_calls=\([0-9]*\).*/\1/p'; }
shared="$(by_calls "$shared_line")"
one="$(by_calls "$one_line")"
[ -n "$shared" ] && [ -n "$one" ] || { echo "   → 解析不出 by_calls" >&2; exit 2; }
[ "$one" -ge 1 ] || { echo "   → 基线异常（单入口 by_calls=${one}）" >&2; exit 2; }

expected=$(( one * 3 ))
echo "   3 入口共享 1 依赖：by_calls=${shared}   （${shared_line#STAGE_STATS }）"
echo "   1 入口（基线）    ：by_calls=${one}   （${one_line#STAGE_STATS }）"
if [ "$shared" -ge "$expected" ]; then
  echo "   → 缺口仍在：共享依赖被 elaborate 了 ${shared} 次（基线 ${one} 次 × 3 个入口）"
  exit 0
fi
echo "   → 已修：共享依赖被复用（by_calls=${shared} < 基线×3=${expected}）"
exit 1
