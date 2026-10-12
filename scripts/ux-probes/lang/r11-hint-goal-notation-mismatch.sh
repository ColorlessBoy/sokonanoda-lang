#!/usr/bin/env bash
# r11-hint-goal-notation-mismatch.sh —— hint 的「目标形态」用点名写法，state 用记法，对不上
# 现象: 练习 4 的 hint 第 76 行写「目标是 A x」；而 `query state` 在那一行给的是 `x ∈ A`。
#       两者是同一个命题的两种渲染，但初学者没法把 `A x` 与 `x ∈ A` 对上——
#       这正是用户抱怨「不知道当前目标长什么样」的来源。
# 期望: hint 的「目标形态」直接抄 state 的渲染（`x ∈ A`），或两种都写。
# 判据: exit 0 = 问题仍在（hint 写点名 `A x`、state 写 `x ∈ A`）/ exit 1 = 已统一
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)

# ⚠ **读数必须可归属**（2026-10-12 独立复核发现的两条同族病，一次治齐）：
#
# ① **假绿**：`scripts/soko` 没有可信构建时会拒跑（exit 3）或回落缓存，而本类探针的
#    判定形态是「**grep 不到复现标记 ⇒ exit 1（已修）**」⇒ 拒跑的输出里当然没有那个
#    标记 ⇒ 掉进假绿。复核实测（未建任何二进制时）`r07` 打印 `FIXED: …` 并 exit 1，
#    而下方正是 `refusing to run an unverified sokonanoda binary` ✗。
# ② **判错二进制**：无自建构建时 `scripts/soko` 会解析到**用户缓存**——实测
#    `cli.source = cache(STALE: expected 0.90.0 darwin-arm64, found 0.83.0)`，
#    即**拿 0.83.0 的二进制去判 0.90.0 的仓库**，读数与仓库状态**毫无关系** ✗。
#
# ⇒ 预检两条：**(a) 启动器答得上** · **(b) 它给的是 `repo-build`（不是缓存）**。
#    任一不满足 ⇒ **exit 2 = 环境不满足**（`judge_rc` 把 2 判成 `error`、`gap.py` 判成
#    「环境异常」⇒ 它是**失败**，绝不会被读成"已修" ✓）。
#    ⚠ 这就是 `AGENTS.md`「探针读数必须带构建身份」的**机器化**：读不出来就不许判 ✓。
_SOKO_VER="$("$REPO/scripts/soko" version --json 2>/dev/null)"
if [ -z "$_SOKO_VER" ]; then
  echo "环境不满足：scripts/soko 答不上（无可信构建）⇒ 本件**无法判定**（exit 2）" >&2
  exit 2
fi
if ! printf '%s' "$_SOKO_VER" | grep -q '"source": *"repo-build"'; then
  echo "环境不满足：解析到的**不是仓库构建**（是缓存/扩展）⇒ 读数无法归属本仓库 ⇒ 本件**无法判定**（exit 2）" >&2
  printf '%s' "$_SOKO_VER" | head -c 200 >&2; echo >&2
  exit 2
fi

# ⚠ **M-07：干净缓存纪律**（2026-10-12 实测咬过一次）—— 内建 prelude / 判定逻辑改了以后
# **只重建不清缓存**时，旧条目会继续答旧行为（同一二进制无 import 时 `#print And.left` = theorem、
# `import lib.Set` 时却 = def）⇒ 每条探针都必须在**新 mktemp 缓存 + 关两处产物**下取读数，
# 否则「缺口仍在」可能是**旧答案**（r04 就这么假红过一次：`apply` 结论校验已修好、探针仍报 exit 0）。
FRESH_CACHE=$(mktemp -d)
export SOKONANODA_CACHE_DIR="$FRESH_CACHE"
export SOKONANODA_NO_PROJECT_ARTIFACTS=1
export SOKONANODA_NO_MODULE_ARTIFACTS=1
trap 'rm -rf "$FRESH_CACHE"' EXIT
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"
U="$REPO/courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda"
hint=$(sed -n '76p' "$U")
st=$(timeout 300 "$REPO/scripts/soko" query state --compact --file "$U" --line 81 --col 3 2>&1)
goal=$(grep -o '"goal":"[^"]*"' <<<"$st" | head -1)
echo "hint 目标形态: $hint"; echo "state goal   : $goal"
if grep -q '目标是 A x' <<<"$hint" && grep -q 'x ∈ A' <<<"$goal"; then
  echo "STILL REPRODUCES: hint 用点名 `A x`，state 用记法 `x ∈ A`，两种渲染没打通"; exit 0
fi
echo "FIXED: hint 与 state 的渲染已一致"; exit 1
