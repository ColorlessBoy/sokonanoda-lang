#!/usr/bin/env bash
# r20-goal-extra-parens.sh —— 目标渲染多出一层括号，与源码写法对不上
# 现象: 源码写 `{a} ⊆ A`，`query state` 渲染成 `({a}) ⊆ A`（多一层括号）；
#       `↔` 目标渲染成 `(({a}) ⊆ A) ↔ (a ∈ A)`。初学者拿 state 的字符串去对照
#       源码/画布时会以为不是同一个命题。
# 期望: 渲染与源码写法一致（`{a} ⊆ A`），或渲染风格在文档里说明。
# 判据: exit 0 = 问题仍在（goal 里出现 `({a})`）/ exit 1 = 已一致
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
# ⚠ **锚点按声明名定位**（2026-10-12）：旧版写死 `--line 80`（旧画布练习 5 的
# 证明体）。画布 `5c6eeff8` 重写后行号漂了 —— 新 80 行是练习 5 的**签名行**，
# 洞在 82 行。签名行与洞上恰好给同一个目标（读数没变），但锚点必须指向**洞**，
# 否则画布一改又会静默落到别处。⇒ 按声明名找洞行号。
DLINE=$(grep -n '^theorem singleton_subset_iff' "$U" | head -1 | cut -d: -f1)
[ -z "$DLINE" ] && { echo "环境不满足：画布里找不到 singleton_subset_iff（锚点失效）⇒ 无法判定"; exit 2; }
HOLE=$(awk -v d="$DLINE" 'NR>d && !/^[[:space:]]*--/ && /sorry/{print NR; exit}' "$U")
[ -z "$HOLE" ] && { echo "环境不满足：singleton_subset_iff 之后找不到 sorry（锚点失效）⇒ 无法判定"; exit 2; }
st=$(timeout 300 "$REPO/scripts/soko" query state --compact --file "$U" --line "$HOLE" --col 3 2>&1)
goal=$(grep -o '"goal":"[^"]*"' <<<"$st" | head -1)
echo "state goal: $goal"
if grep -q '({a})' <<<"$goal"; then
  echo "STILL REPRODUCES: 渲染比源码多一层括号"; exit 0
fi
echo "FIXED: 渲染与源码一致"; exit 1
