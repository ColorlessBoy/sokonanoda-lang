#!/usr/bin/env bash
# r06-parse-blames-import.sh —— 文件自身的语法错误被报成「导入失败」，并给出**误导性建议**
# 现象: 受控对照（唯一变量 = 有没有用课程记法 `∈`）：
#   · 不用 `∈` 的文件里写 `·` 子弹 → 诚实的 `unexpected-token`（stage=parse）
#   · 用了 `∈` 的文件里写同一个 `·` → `import-module-invalid`（stage=import），
#     hint 说「**被导入的文件**本身没解析成功：先打开它、修好那里的语法错误」——而
#     lib/Logic、lib/Set 完全没问题，学习者会去翻库文件；
#     同时多出一条 `notation-unknown-symbol`：「符号 `∈` 在本文件里还没有声明过记法；
#     先用 infix/notation 命令声明它」——可文件第 2 行就 `import lib.Set`，
#     照它做等于让学习者自己重声明课程记法。
# 期望: 语法错误永远报在**本文件**的出错位置，不要牵连 import，也不要级联假诊断。
# 判据: exit 0 = 问题仍在（出现 import-module-invalid 或假 notation 诊断）/ exit 1 = 已修
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
W="$REPO/.cache/ux/lang/repro/.work"
printf 'import lib.Logic\nimport lib.Set\n\ntheorem t (p q : Prop) (hp : p) (hq : q) : p /\\ q := by\n  constructor\n  \xc2\xb7 exact hp\n  \xc2\xb7 exact hq\n' > "$W/r06-nomem.sokonanoda"
printf 'import lib.Logic\nimport lib.Set\n\ntheorem t (α : Type) (A : Set α) (a : α) (h : a \xe2\x88\x88 A) : a \xe2\x88\x88 A := by\n  constructor\n  \xc2\xb7 exact h\n' > "$W/r06-mem.sokonanoda"
a=$(timeout 300 "$REPO/scripts/soko" query check --file "$W/r06-nomem.sokonanoda" 2>&1)
b=$(timeout 300 "$REPO/scripts/soko" query check --file "$W/r06-mem.sokonanoda" 2>&1)
echo "不用记法: $(grep -o '"code": "[^"]*"' <<<"$a" | tr '\n' ' ')"
echo "用了记法: $(grep -o '"code": "[^"]*"' <<<"$b" | tr '\n' ' ')"
if grep -q 'import-module-invalid' <<<"$b" && grep -q 'notation-unknown-symbol' <<<"$b"; then
  echo "STILL REPRODUCES: 本文件语法错误被报成 import 失败 + 级联假记法诊断"; exit 0
fi
echo "FIXED: 语法错误已如实报在本文件"; exit 1
