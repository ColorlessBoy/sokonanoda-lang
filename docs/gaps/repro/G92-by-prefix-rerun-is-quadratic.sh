#!/usr/bin/env bash
# **G-92**：`by` 判定的**前缀重跑**仍是 O(n²) ✗（与 G-85 那条**不同** ✓）。
#
# 退出码：0 = 缺口仍在（规模翻倍 ⇒ `by_calls` 约 **4×**）· 1 = 行为变了（已线性化）· 2 = 环境不对
#
# ## 它是什么（2026-10-04 第 15 棒实测定位 ✓）
#
# 一个文件里放 N 条**各自需要新判定**的 `by` 声明（每条的目标都不同 ⇒ 判定缓存必然 miss）：
#
# | N | `passes`（趟数 ✓） | `pass_total_ms` | `by_calls` |
# |---|---|---|---|
# | 10 | 13 | 82 | 55 |
# | 20 | 23（**1.77×** ✓） | 156（1.90×） | 210（**3.8×** ✗） |
# | 40 | 43（1.87× ✓） | 338（2.17× ✗） | 820（**3.9×** ✗） |
# | 80 | 83（1.93× ✓） | 860（**2.54×** ✗） | 3240（**3.95×** ✗） |
# | 160 | 163（**1.96×** ✓ **线性**） | **2249（2.62× ✗）** | **12880（3.98× ✗ 平方）** |
# 
# ⇒ **趟数是线性的 ✓，但每趟的成本随 N 涨** ✗（`pass_total_ms` 每个翻倍 **2.6×** ✗）
# ⇒ **总成本仍是 N²** ✗（墙钟 80→160 = **0.52 → 1.27s = 2.44×** ✗）。

# ## ⚠ 它与 G-85（`own_prefix` 那条）**不是同一处** ✗
#
# G-85 那条是**身份计算**（`canonical_prefix_id` 每命令重解析整份前缀 ✗）—— **已修** ✓
# （`identity_parses` 3062 → **0** ✓）。这一条是**判定本身**要重跑前缀 ✗。
# **两者独立** ✓：本复现件在**修好 G-85 的二进制**与**改前的基线二进制**上读数**完全一致** ✓
# ⇒ 它是**既有的**、**没被那次修复碰到** ✓。
#
# ## ⚠ 真实课程**不**呈现它（如实记 ✓）
#
# `unit12-synthesis` 的**逐声明耗时**（`SOKO_DECL_PROFILE=1` ✓）在 index 150+ 是
# **135–150ms 的平线** ✓（不随序号增长 ✓）—— 因为课程里绝大多数 `by` 判定**命中缓存**
# （受信任前缀那条路 ✓，§3.C ✓）。⇒ 本条目是**最坏形状**的缺口 ✓，
# **不是**「课程现在很慢」✗。
#
# ## 为什么用**结构计数**判（不用毫秒 ✗）
#
# `AGENTS.md` 2026-09-29（同一个病犯过三次 ✓）：共享 runner 上墙钟不可转移 ⇒
# 判据用**机器无关的结构计数** ✓（这里用 `by_calls` 的**比值** ✓）。
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
  python3 - "$1" "$2" <<'PYGEN'
import sys
n, out = int(sys.argv[1]), sys.argv[2]
body = [""]
for k in range(n):
    # ⚠ **夹具必须真的编得过** ✗→✓（2026-10-04 第 17 棒修 ✓）：先前用的是
    # `f a = f a := by exact Eq.refl α (f a)` ✗ —— 那个形状**在闭包里会被误拒** ✗
    # （单文件能编 ✓、带 `import` 就判红 ✗；**两个二进制都复现** ✓ ⇒ **既有** ✓，
    #  已单独登记 **G-93** ✓）。拿它当夹具 ⇒ 量到的是**失败路径** ✗
    # （每条声明多跑一次 `mismatch_message` ⇒ 多一次前缀重跑 ✗）
    # ⇒ **读数不是「很多条正常证明」那个形状** ✗。
    # 换成 `P : Prop` + `exact h` ✓（**闭包里真的编得过** ✓，已实测 ✓）。
    body.append(
        f"theorem g{k} (P : Prop) (h : P) : P := by\n"
        f"  exact h\n"
    )
open(out, "w", encoding="utf-8").write("\n".join(body))
PYGEN
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
  echo "缺口仍在：G-92「by 判定的前缀重跑」仍是 O(n²)（by_calls 比值 $ratio ≥ 3.0）"
  echo "  根因：趟数线性 ✓（passes 13→23，1.77× ✓）但**每趟重编整份前缀** ✗"
  echo "  ⚠ 真实课程**不**呈现它（逐声明耗时平线 ✓）—— 这是**最坏形状**的缺口 ✓"
  exit 0
fi
echo "行为变了：比值 $ratio < 3.0 ⇒ 这条已经线性化（或夹具不再触发）⇒ 回来更新台账"
exit 1
