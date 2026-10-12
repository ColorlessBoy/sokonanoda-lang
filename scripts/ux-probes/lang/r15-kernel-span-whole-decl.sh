#!/usr/bin/env bash
# r15-kernel-span-whole-decl.sh —— kernel-rejected 的 span 覆盖整条声明，粒度与 elab 类不一致
# 现象: 同一个文件里：
#   · elab-tactic-failed  → span = 那一条 tactic（如 L6 C3–C11，`exact hh`），精确可用；
#   · elab-unknown-identifier → span = 那个标识符（如 L5 C71–C72，`𝒬`），精确可用；
#   · kernel-rejected → span = **整条声明**（如 L5 C1 → L7 C5，从 theorem 到函数体结束）。
#       配上内核黑话，用户既不知道哪一行错，也不知道哪个子项错。
# 期望: kernel-rejected 也定位到出错子表达式（或至少报出它在声明内的路径）。
# 判据: exit 0 = 问题仍在（kernel-rejected 的 span 跨多行）/ exit 1 = 已精确
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
F="$REPO/.cache/ux/lang/repro/.work/r15.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem simple_mismatch (α : Type) (A : Set α) (a b : α) (ha : a ∈ A) :
    b ∈ A :=
  ha
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1)
sl=$(grep -o '"start_line": [0-9]*' <<<"$out" | head -1 | grep -o '[0-9]*')
el=$(grep -o '"end_line": [0-9]*' <<<"$out" | head -1 | grep -o '[0-9]*')
echo "kernel-rejected span: start_line=$sl end_line=$el"
if [ -n "$sl" ] && [ -n "$el" ] && [ "$el" -gt "$sl" ]; then
  echo "STILL REPRODUCES: kernel-rejected 的 span 跨 $((el-sl+1)) 行（整条声明），无法定位出错子项"; exit 0
fi
echo "FIXED: kernel-rejected span 已精确"; exit 1
