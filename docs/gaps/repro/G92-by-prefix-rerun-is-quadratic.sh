#!/usr/bin/env bash
# **G-92**：`by` 判定的**前缀重跑**仍是 O(n²) ✗（与 G-85 那条**不同** ✓）。
#
# 退出码：0 = 缺口仍在（规模翻倍 ⇒ `by_calls` 约 **4×**）· 1 = 行为变了（已线性化）· 2 = 环境不对
#
# ## 它是什么（2026-10-04 第 15 棒实测定位 ✓）
#
# 一个文件里放 N 条**各自需要新判定**的 `by` 声明（每条的目标都不同 ⇒ 判定缓存必然 miss）：
#
# | N | `JUDGE_PREFIX runs` | `by_calls` |
# |---|---|---|
# | 10 | 21 | **451** |
# | 20 | 41（**2.0×** ✓ 线性）| **1996**（**4.4×** ✗ 平方）|
# | 40 | — | 8386（**4.2×** ✗）|
# | 80 | 240（2.0× ✓）| 34366（**4.1×** ✗）|
#
# ⇒ **前缀重跑的"趟数"是线性的 ✓，但每一趟都要把整份前缀重编一遍** ✗
# （`JUDGE_PREFIX bytes_per_run` 5072 → 10157 ⇒ 每趟成本随 N 线性 ⇒ 总成本 N² ✗）。
# 重编前缀时，前缀里那些 `by` 声明**又被 elaborate 一遍**（`by_calls` 数的是
# **`by` 引擎调用** ✓）⇒ 这就是那个 N²。
#
# ## ⚠ 它与 G-85（`own_prefix` 那条）**不是同一处** ✗
#
# G-85 那条是**身份计算**（`canonical_prefix_id` 每命令重解析整份前缀 ✗）—— **已修** ✓
# （`identity_parses` 3062 → **0** ✓）。这一条是**判定本身**要重跑前缀 ✗。
# **两者独立**：本复现件在**修好 G-85 的二进制**与**改前的基线二进制**上读数**完全一致** ✓
# （`by_calls` 8386 / 34366 两边一模一样 ✓）⇒ 它是**既有的**、**没被那次修复碰到** ✓。
#
# ## ⚠ 真实课程**不**呈现它（如实记 ✓）
#
# `unit12-synthesis` 的**逐声明耗时**（`SOKO_DECL_PROFILE=1` ✓）在 index 150+ 是
# **135–150ms 的平线** ✓（不随序号增长 ✓）—— 因为课程里绝大多数 `by` 判定**命中缓存**
# （受信任前缀那条路 ✓，§3.C ✓）。⇒ 本条目是**最坏形状**的缺口 ✓，
# **不是**"课程现在很慢" ✗。
#
# ## 为什么用**结构计数**判（不用毫秒 ✗）
#
# `AGENTS.md` 2026-09-29（同一个病犯过三次 ✓）：共享 runner 上墙钟不可转移 ⇒
# 判据用**机器无关的结构计数** ✓（这里用 `by_calls` ✓）。
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT" || exit 2

BIN="${SOKO_BIN:-}"
if [ -z "$BIN" ]; then
  for cand in "$ROOT/target/release/sokonanoda" "$ROOT/target/debug/sokonanoda"; do
    [ -x "$cand" ] && BIN="$cand" && break
  done
fi
[ -n "$BIN" ] && [ -x "$BIN" ] || { echo "环境不对：找不到 sokonanoda 二进制" >&2; exit 2; }

DIR="$(mktemp -d "${TMPDIR:-/tmp}/soko-g92-XXXXXX")"
trap 'rm -rf "$DIR"' EXIT
printf '[project]\nname = "g92"\n' > "$DIR/sokonanoda.toml"

gen() { # $1 = N，$2 = 输出文件
  python3 - "$1" "$2" <<'PY'
import sys
n, out = int(sys.argv[1]), sys.argv[2]
body = [""]
for k in range(n):
    body.append(
        f"theorem g{k} (α : Type) (f : α → α) (a : α) : f a = f a := by\n"
        f"  exact Eq.refl α (f a)\n"
    )
open(out, "w", encoding="utf-8").write("\n".join(body))
PY
}

by_calls_of() { # $1 = 夹具，$2 = 缓存标记
  rm -rf "$DIR/c$2"
  SOKO_STAGE_STATS=1 SOKONANODA_NO_PROJECT_ARTIFACTS=1 SOKONANODA_CACHE_DIR="$DIR/c$2" \
    timeout 300 "$BIN" build --json "$1" > /dev/null 2> "$DIR/e$2.err" || return 1
  grep -o 'by_calls=[0-9]*' "$DIR/e$2.err" | tail -1 | cut -d= -f2
}

gen 10 "$DIR/n10.sokonanoda"
gen 20 "$DIR/n20.sokonanoda"
b10="$(by_calls_of "$DIR/n10.sokonanoda" 10)" || { echo "环境不对：N=10 那一跑没跑起来" >&2; exit 2; }
b20="$(by_calls_of "$DIR/n20.sokonanoda" 20)" || { echo "环境不对：N=20 那一跑没跑起来" >&2; exit 2; }
[ -n "$b10" ] && [ -n "$b20" ] && [ "$b10" -gt 0 ] || { echo "环境不对：拿不到 by_calls 读数" >&2; exit 2; }

ratio="$(python3 -c "print(f'{$b20/$b10:.2f}')")"
echo "G-92 规模翻倍读数：by_calls(N=10)=$b10 ⇒ by_calls(N=20)=$b20 ⇒ **比值 $ratio**"
echo "   线性 = 2.0 ✓ · 平方 = 4.0 ✗（阈值 3.0）"

# **比值 ≥ 3.0 ⇒ 平方 ⇒ 缺口仍在** ✓（判据与机器快慢无关 ✓）。
if python3 -c "import sys; sys.exit(0 if $ratio >= 3.0 else 1)"; then
  echo "缺口仍在：G-92「by 判定的前缀重跑」仍是 O(n²)（比值 $ratio ≥ 3.0）"
  echo "  根因：趟数线性 ✓（runs 21→41）但**每趟重编整份前缀** ✗ ⇒ 总成本 N² ✗"
  echo "  ⚠ 真实课程**不**呈现它（逐声明耗时平线 ✓）—— 这是**最坏形状**的缺口 ✓"
  exit 0
fi
echo "行为变了：比值 $ratio < 3.0 ⇒ 这条已经线性化（或夹具不再触发）⇒ 回来更新台账"
exit 1
