#!/usr/bin/env bash
# **G-31**：`by` 判定**每批重跑整份前缀** —— 量「前缀里那些 `by` 是不是又被 elaborate 一遍」。
#
# 退出码（与 `docs/gaps/README.md` 的约定一致）：**0 = 缺口仍在 · 1 = 已修 · 2 = 环境不对**
#
# ## ⚠ 它**不是** G-29 那条（两条判据**不同** ✗）
#
# * **G-29**（`docs/gaps/repro/G29-edit-recompiles-whole-closure.sh`）量的是**编辑器
#   按键延迟**（改一行 ⇒ 重编整条闭包；`prefix_runs`/`modules`/墙钟）。
# * **本件**量的是**判定内部**的前缀重跑（**与编辑器无关**：一条 `build` 就够 ✓）。
#
# G-31 的台账 `repro` 先前**错指** G-29 那条 ✗（拿"按键延迟"当"前缀重跑"的判据）——
# 2026-10-07 换成本件 ✓（见台账 `notes`）。
#
# ## 口径：`by_calls / N`（**结构计数**，与机器快慢无关 ✓）
#
# 夹具：N 条**各自需要新判定**的 `theorem g{k} (P : Prop) (h : P) : P := by exact h`
# （每条的目标都不同 ⇒ 判定缓存必然 miss ⇒ 每次判定都要合成"整份前缀 + `_soko_judge_k`"
# 再跑一趟 ✓ —— 这正是 G-31 说的"每批重跑整份前缀" ✓）。
#
# `by_calls` = **`by` 引擎被调用了几次**（`check/mod.rs::lower_by_val` ✓）。
# * 缺口在 ⇒ 合成文档把前缀里前面那些 `by` **又 elaborate 一遍** ✗
#   ⇒ `by_calls = N + Σ(1..N-1)` ⇒ **`by_calls / N` 随 N 增长** ✗
#   （实测 2026-10-04：N=10 ⇒ 55/10 = **5.5** · N=20 ⇒ 210/20 = **10.5** ✗）。
# * 修好 ⇒ 每条 `by` **只跑自己那一次** ⇒ **`by_calls / N == 1.0`（与 N 无关 ✓）**。
#
# **判据（阈值 1.5，取在 1.0 与 5.5 之间、且远离两端 ⇒ 噪声免疫 ✓）**：
# `by_calls(N=20)/20 >= 1.5` ⇒ **缺口仍在**（exit 0）；`< 1.5` ⇒ **已修**（exit 1）。
#
# ⚠ **为什么不用毫秒** ✗：`AGENTS.md` 2026-09-29（同一个病犯过三次 ✓）—— 共享 runner 上
# 墙钟不可转移 ⇒ 判据一律用**结构计数**或**比值** ✓。
#
# ⚠ **它同时也是 G-92 的同根因判据** ✓（G-92 = 同一处，量的是**比值 ≥ 3.0**）。
# 两条都在册、口径不同（本条 = **绝对目标 1.0**，G-92 = **翻倍比值**）⇒ 互相印证 ✓。
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2

BIN="${SOKO_BIN:-}"
if [ -z "$BIN" ]; then
  # ⚠ **取较新的那一份** ✓（2026-10-07）：固定 `release` 优先会让**陈旧的 release
  # 产物遮蔽工作树的 debug 构建** ✗ ⇒ 判据量到的是**旧二进制**（G-92 那条实测踩到 ✓）。
  # 本判据是**结构计数** ⇒ 与构建模式无关 ✓ ⇒ 取新的就对 ✓。
  rel="$ROOT/target/release/sokonanoda"
  dbg="$ROOT/target/debug/sokonanoda"
  if [ -x "$rel" ] && { [ ! -x "$dbg" ] || [ "$rel" -nt "$dbg" ]; }; then
    BIN="$rel"
  else
    BIN="$dbg"
  fi
fi
[ -n "$BIN" ] && [ -x "$BIN" ] || { echo "环境不对：找不到 sokonanoda 二进制" >&2; exit 2; }
# **构建身份**（仓规：跨构建不可比 ⇒ 读数必须自带身份 ✓）。
echo "构建身份：${BIN} sha256=$(shasum -a 256 "$BIN" | cut -c1-16) mtime=$(stat -f '%Sm' "$BIN" 2>/dev/null || stat -c '%y' "$BIN")"

DIR="$(mktemp -d "${TMPDIR:-/tmp}/soko-g31-XXXXXX")"
trap 'rm -rf "$DIR"' EXIT
printf '[project]\nname = "g31"\n' > "$DIR/sokonanoda.toml"

gen() { # $1 = N，$2 = 输出文件
  python3 - "$1" "$2" <<'PYGEN'
import sys
n, out = int(sys.argv[1]), sys.argv[2]
body = [""]
for k in range(n):
    # ⚠ 夹具必须**真的编得过**（G-92 第 17 棒的坑 ✓）：先前那个
    # `f a = f a := by exact Eq.refl α (f a)` 形状在闭包里会被误拒（另有 G-93 ✓）
    # ⇒ 量到的是**失败路径** ✗。`P : Prop` + `exact h` 在闭包里真的编得过 ✓。
    body.append(
        f"theorem g{k} (P : Prop) (h : P) : P := by\n"
        f"  exact h\n"
    )
open(out, "w", encoding="utf-8").write("\n".join(body))
PYGEN
}

read_stats() { # $1 = 夹具，$2 = 缓存标记 ⇒ 打 `by_calls=<n> passes=<n> judge_prefix_runs=<n>`
  rm -rf "$DIR/c$2"
  SOKO_STAGE_STATS=1 SOKO_JUDGE_STATS=1 SOKONANODA_NO_PROJECT_ARTIFACTS=1 \
    SOKONANODA_CACHE_DIR="$DIR/c$2" \
    timeout 300 "$BIN" build --json "$1" > /dev/null 2> "$DIR/e$2.err" || return 1
  local by passes runs
  by="$(grep -o 'by_calls=[0-9]*' "$DIR/e$2.err" | tail -1 | cut -d= -f2)"
  # ⚠ 要**带前导空格**匹配 ✗→✓：`doc_passes=0` 也含 `passes=` ⇒ 不加锚会取到它（实测踩到 ✓）。
  passes="$(grep -oE ' passes=[0-9]*' "$DIR/e$2.err" | tail -1 | cut -d= -f2)"
  runs="$(grep -o 'JUDGE_PREFIX runs=[0-9]*' "$DIR/e$2.err" | tail -1 | cut -d= -f2)"
  [ -n "$by" ] || return 1
  echo "$by $passes ${runs:-0}"
}

gen 10 "$DIR/n10.sokonanoda"
gen 20 "$DIR/n20.sokonanoda"
r10="$(read_stats "$DIR/n10.sokonanoda" 10)" || { echo "环境不对：N=10 那一跑没跑起来" >&2; exit 2; }
r20="$(read_stats "$DIR/n20.sokonanoda" 20)" || { echo "环境不对：N=20 那一跑没跑起来" >&2; exit 2; }
set -- $r10; b10="$1"; p10="$2"; j10="$3"
set -- $r20; b20="$1"; p20="$2"; j20="$3"
[ "$b10" -gt 0 ] && [ "$b20" -gt 0 ] || { echo "环境不对：拿不到 by_calls 读数" >&2; exit 2; }

per10="$(python3 -c "print(f'{$b10/10:.2f}')")"
per20="$(python3 -c "print(f'{$b20/20:.2f}')")"
ratio="$(python3 -c "print(f'{$b20/$b10:.2f}')")"
echo "G-31 读数（N=10 ⇒ N=20）：by_calls ${b10} ⇒ ${b20}（比值 ${ratio}）· passes ${p10} ⇒ ${p20}"
echo "   判据 = **by_calls / N**：N=10 ⇒ **${per10}** · N=20 ⇒ **${per20}**（修好 = 1.00，与 N 无关 ✓；阈值 1.5）"
echo "   ⚠ 'JUDGE_PREFIX runs'（类型推断那条路重编前缀的趟数）：${j10} ⇒ ${j20} —— **不随 N 变**"
echo "     ⇒ 它不是本条的判据（放大不在 infer 那条路上 ✓），只作旁证报出来 ✓。"

if python3 -c "import sys; sys.exit(0 if $per20 >= 1.5 else 1)"; then
  echo "缺口仍在：G-31「判定每批重跑整份前缀」—— 每条声明的 by 引擎调用数 ${per20} > 1.0"
  echo "  机理：每次判定合成「整份前缀 + _soko_judge_k」再跑一趟，而**受信任前缀只跳"
  echo "  内核检查、不跳 elaborate** ⇒ 前缀里那些 by 声明**又被 elaborate 一遍** ✗"
  echo "  ⇒ by_calls = N + Σ(1..N-1)（N=20 ⇒ 20 + 190 = 210 ✓）。"
  echo "  ⚠ 真实课程**不**呈现它（逐声明耗时平线 ✓）—— 这是**最坏形状**的缺口 ✓。"
  exit 0
fi
echo "已修：每条声明的 by 引擎调用数 ${per20} < 1.5（= 1.0 ⇒ 每条 by 只跑自己那一次 ✓）"
echo "  读数：by_calls ${b10} ⇒ ${b20}（比值 ${ratio}，线性 ≈ 2.0 ✓）· passes ${p10} ⇒ ${p20}"
exit 1
