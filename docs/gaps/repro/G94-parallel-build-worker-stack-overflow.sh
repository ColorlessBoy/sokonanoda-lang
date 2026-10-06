#!/usr/bin/env bash
# G-94 自断言复现：**`build` 的入口级并行 worker 用默认 2MB 栈 ⇒ 深递归的编译把它撑爆**。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在（真的崩了）· 1 = 已修（编得过）· 2 = 环境/前置缺失。
#
# 判据（**结构性**：只看退出码与 `build.summary`，不看墙钟）：
#   夹具 = 一个**深度嵌套**的项（`def deep : Nat := ((((…1…))))`，250 层）+ 一个普通文件；
#   `SOKONANODA_BUILD_JOBS=2` ⇒ `deep` 落在**并行 worker** 上（`std::thread::scope` 默认栈 2MB）：
#     * **主线程**（`JOBS=1`，8MB）**编得过** ✓ —— 这一条是**对照**：证明病根是**线程栈**，不是判定；
#     * **worker 线程**（`JOBS=2`，2MB）⇒ `thread '<unknown>' has overflowed its stack`
#       + `Abort trap: 6`（exit **134**）✗。
#
# 实测（2026-10-06 · Darwin arm64 · 仓库 debug 构建 · 0.82.0）：
#   `JOBS=1` ⇒ exit 0 / `compiled=2` ✓    `JOBS=2` ⇒ exit **134** ✗（真语料上同一形状：
#   冷编 `courses/set-theory/units/I.3` · `JOBS=4` ⇒ exit 134 ✗；`JOBS=1` ⇒ 16/16 compiled ✓）。
#
# 根因（读码定位，非推测）：`crates/cli/src/build.rs` 的入口级并行用 `std::thread::scope`
#   + `scope.spawn` ⇒ **平台默认栈**（本机 2MB）；而编译（`elab_expr`）是**深度递归**的。
#   同一个病在 LSP 侧**早就修过**（`crates/lsp/src/lib.rs::run` 的
#   `thread_stack_size(32 * 1024 * 1024)`，注释原文："编译（深度递归的 `elab_expr`）
#   会 `thread 'tokio-rt-worker' has overflowed its stack`"）⇒ **CLI 这条腿漏了** ✗。
#
# 修法（本轮的落地）：worker 线程显式给栈 —— `std::thread::Builder::new()
#   .stack_size(sokonanoda_front::project::COMPILE_STACK_BYTES /* 32MB */)`
#   （CLI 的 `soko-build` 与前端分组会话的 `soko-shared-group` 两处 ✓）。
#   修好后本脚本 exit 1（回来关账：`python3 scripts/gap.py close G-94 --version …`）。
set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
command -v node >/dev/null 2>&1 || { echo "需要 node（scripts/soko 是零依赖 Node 启动器）" >&2; exit 2; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP"
printf 'name = "g94-fixture"\n' > "$TMP/sokonanoda.toml"

# 250 层嵌套：**恰好**卡在 2MB（worker）与 8MB（主线程）之间 —— 实测 n=250 是分界
# （n≤300 主线程过、n≥350 主线程也过不了 ⇒ 那个深度已经不是"线程栈"这条缺口了 ✗）。
python3 - "$TMP/deep.sokonanoda" <<'PY'
import sys
n = 250
open(sys.argv[1], "w", encoding="utf-8").write("def deep : Nat := " + "(" * n + "1" + ")" * n + "\n")
PY
printf 'def other : Nat := 2\n' > "$TMP/other.sokonanoda"

run() { # $1 = JOBS
  rm -rf "$TMP/.cache" "$TMP/.sokonanoda"
  SOKONANODA_CACHE_DIR="$TMP/.cache" SOKONANODA_BUILD_JOBS="$1" \
    "$SOKO" build --json "$TMP" > "$TMP/out-$1.json" 2> "$TMP/err-$1.txt"
  printf '%s' "$?"
}

serial="$(run 1)"
parallel="$(run 2)"
serial_ok="$(grep -o '"compiled":[0-9]*' "$TMP/out-1.json" 2>/dev/null | tail -1)"

echo "   JOBS=1（主线程，8MB）：exit=${serial} ${serial_ok}（**对照**：这一档必须编得过 ✓）"
echo "   JOBS=2（worker，2MB）：exit=${parallel}"

[ "$serial" = "0" ] || {
  echo "   → 对照档就崩了 ⇒ 夹具形状不对（不再是"线程栈"这条缺口）—— 记 exit 2" >&2
  exit 2
}
[ "$serial_ok" = '"compiled":2' ] || {
  echo "   → 对照档没编出 2 个文件（${serial_ok:-无 summary}）⇒ 夹具前提不成立 —— 记 exit 2" >&2
  exit 2
}
if [ "$parallel" != "0" ]; then
  echo "   → 缺口仍在：并行 worker 撑爆了栈（exit=${parallel}）"
  grep -o "has overflowed its stack" "$TMP/err-2.txt" 2>/dev/null | head -1
  exit 0
fi
echo "   → 已修：并行 worker 也编得过（exit=0）"
exit 1
