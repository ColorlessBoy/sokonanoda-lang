#!/usr/bin/env bash
# **全语料逐字节对拍**（T-K01）：内核解冻之后，这是唯一能**机械**证明
# "判定正确性不变"的判据。
#
#     scripts/kernel-diff.sh <改动前二进制> <改动后二进制>
#     scripts/kernel-diff.sh --fast <前> <后>      # 有代表性的子集（每环节用）
#     scripts/kernel-diff.sh --self-test <二进制>  # 自检：证明它真的能发现差异
#
# 对拍什么（每条都比 **stdout 逐字节** + **退出码**）：
#
#   1. `grade --json`        —— 全量事件流（既有消费者读的就是它）
#   2. `query check --file`  —— 计数/失败摘要
#   3. `query goals --file`  —— 声明列表
#   4. `query holes --file`  —— 洞
#   5. `query project --file`—— 项目闭包视图
#
# 外加**课程门禁计数**逐项比较（`courses/set-theory/tools/check.py --json`）。
#
# 为什么不能只跑测试：内核的改动可能"测试全绿但判定变了"（阈值、边界、
# 归因）。逐字节对拍把**整个语料**的判定结果钉死，这是测试覆盖不到的。
#
# 退出码：0 = 零差异（正确性不变）· 1 = 有差异（**不许合入**）· 2 = 环境/用法错误。

set -u
cd "$(dirname "$0")/.." || exit 2

FAST=0
SELF_TEST=0
ARGS=()
for arg in "$@"; do
  case "$arg" in
    --fast) FAST=1 ;;
    --self-test) SELF_TEST=1 ;;
    -h | --help) sed -n '2,24p' "$0"; exit 0 ;;
    *) ARGS+=("$arg") ;;
  esac
done

if [ "$SELF_TEST" = 1 ]; then
  BIN="${ARGS[0]:-}"
  [ -n "$BIN" ] || { echo "error: --self-test 需要一个二进制" >&2; exit 2; }
  [ -x "$BIN" ] || { echo "error: 不是可执行文件：$BIN" >&2; exit 2; }
  # ① 自己对自己必须零差异。
  if ! bash "$0" --fast "$BIN" "$BIN" >/dev/null 2>&1; then
    echo "self-test FAIL：同一个二进制对拍竟然有差异（对拍器本身不稳）" >&2
    exit 1
  fi
  # ② 与一个"会多吐一个字节"的包装器对拍，**必须**报差异。
  wrapper="$(mktemp)"
  trap 'rm -f "$wrapper"' EXIT
  # **不要用 `exec`**：它会替换掉这个 shell，后面的 `echo` 永远不会执行
  # ⇒ 注入的差异根本没出现，self-test 反而报"没发现差异"（第一版就是这样）。
  cat >"$wrapper" <<WRAP
#!/usr/bin/env bash
"$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")" "\$@"
status=\$?
echo "poison"
exit \$status
WRAP
  chmod +x "$wrapper"
  if bash "$0" --fast "$BIN" "$wrapper" >/dev/null 2>&1; then
    echo "self-test FAIL：对拍器**没能**发现人为注入的差异" >&2
    exit 1
  fi
  echo "kernel-diff self-test: 2/2（同二进制零差异 · 人为差异被抓到）"
  exit 0
fi

[ "${#ARGS[@]}" -eq 2 ] || { echo "用法：scripts/kernel-diff.sh [--fast] <前> <后>" >&2; exit 2; }
BEFORE="${ARGS[0]}"
AFTER="${ARGS[1]}"
for bin in "$BEFORE" "$AFTER"; do
  [ -x "$bin" ] || { echo "error: 不是可执行文件：$bin" >&2; exit 2; }
done

# 语料：全部 `*.sokonanoda`（课程 + 入门课 + 画布 + 例子 + 缺口复现）。
# **不要用 `mapfile`**：macOS 自带 bash 3.2 没有它（实测 `mapfile: command not found`
# ——而且它报错之后 `FILES` 是空的，脚本还会"零差异"地绿过去，最坏的那种失败）。
FILES=()
while IFS= read -r line; do
  [ -n "$line" ] && FILES+=("$line")
# **`-type f` 不能少**（审计 #4，2026-09-25 ✓）：产物目录 `.sokonanoda/` 的名字以
# `.sokonanoda` 结尾 ⇒ `-name '*.sokonanoda'` 会把**目录**也收进来 ✗ —— CLI 对目录
# 报 `Is a directory`，两侧输出**逐字节相同** ⇒ 那些组对拍**恒绿**、"零差异"覆盖被虚报 ✗
# （实测实收 4 个目录 ⇒ 4×5=20 组空转 ✓）。
done < <(find courses course examples docs/gaps/repro -type f -name '*.sokonanoda' 2>/dev/null | sort)
[ -f playground.sokonanoda ] && FILES+=("playground.sokonanoda")
if [ "$FAST" = 1 ]; then
  # 有代表性的子集：单文件 / 项目入口 / 项目依赖 / 坏依赖 / 解析失败 / 空壳。
  #
  # **子集要小**：`--fast` 是每环节跑的，而每个文件要 ×5 个 op ×2 个二进制。
  # unit12 一次 `grade` 就 8.8s ⇒ 光它一个就 88s（实测超时过）。
  # 取"形状各不同但都便宜"的五个：
  KEEP=(
    playground.sokonanoda                                   # 单文件、画布
    course/unit11-project/Exercises.sokonanoda              # 项目入口（有 import）
    docs/gaps/repro/G28-arrow-forall.sokonanoda             # 语法边界
  )
  SUBSET=()
  for keep in "${KEEP[@]}"; do [ -f "$keep" ] && SUBSET+=("$keep"); done
  FILES=("${SUBSET[@]}")
fi
[ "${#FILES[@]}" -gt 0 ] || { echo "error: 找不到任何 .sokonanoda 语料" >&2; exit 2; }

# 缓存一律关掉：条目命中会跳过编译，对拍就测不到内核了。
export SOKONANODA_NO_CACHE=1

DIFFS=0
CHECKS=0
FIRST_DIFFS=()

compare() {
  local label="$1" file="$2"
  shift 2
  local before after code_before code_after
  before="$("$BEFORE" "$@" "$file" 2>&1)"
  code_before=$?
  after="$("$AFTER" "$@" "$file" 2>&1)"
  code_after=$?
  CHECKS=$((CHECKS + 1))
  if [ "$before" != "$after" ] || [ "$code_before" != "$code_after" ]; then
    DIFFS=$((DIFFS + 1))
    if [ "${#FIRST_DIFFS[@]}" -lt 5 ]; then
      FIRST_DIFFS+=("$label $file（exit $code_before vs $code_after）")
    fi
  fi
}

echo "对拍 ${#FILES[@]} 个文件 × $([ "$FAST" = 1 ] && echo '2' || echo '5') 个 op（$([ "$FAST" = 1 ] && echo 子集 || echo 全量)）"
for file in "${FILES[@]}"; do
  compare "grade" "$file" --json
  compare "check" "$file" query check --file
  if [ "$FAST" = 0 ]; then
    # 三个 `query` op 各自都要**重新编译**（缓存被关掉了，见上）⇒ 每个文件
    # 多花 3 倍时间。全量对拍要它们，每环节的 `--fast` 不要。
    compare "goals" "$file" query goals --file
    compare "holes" "$file" query holes --file
    compare "project" "$file" query project --file
  fi
done

# stdin 通道（`--text` / 管道）：三条形状不同的输入。
for src in 'def a : Nat := 1' 'theorem t : Prop := sorry' 'import Nope'; do
  CHECKS=$((CHECKS + 1))
  b="$(printf '%s\n' "$src" | "$BEFORE" - 2>&1)"; cb=$?
  a="$(printf '%s\n' "$src" | "$AFTER" - 2>&1)"; ca=$?
  if [ "$b" != "$a" ] || [ "$cb" != "$ca" ]; then
    DIFFS=$((DIFFS + 1))
    [ "${#FIRST_DIFFS[@]}" -lt 5 ] && FIRST_DIFFS+=("stdin「$src」")
  fi
done

# 课程门禁计数逐项比较（它读的是 `grade` 的退出码与事件计数）。
#
# **只在全量模式跑**：门禁要 `grade` 36 个目标 ×2 个二进制，实测 **6 分钟**，
# 把 `--fast` 从"每环节能用"拖成"不敢跑"（第一版就是这样）。
if [ "$FAST" = 0 ] && [ -f courses/set-theory/tools/check.py ]; then
  CHECKS=$((CHECKS + 1))
  gate_of() {
    SOKONANODA_BIN="$1" python3 courses/set-theory/tools/check.py --json 2>/dev/null |
      python3 -c 'import json,sys; d=json.load(sys.stdin); print(json.dumps(d.get("summary"), sort_keys=True), d.get("failed"))'
  }
  gb="$(gate_of "$(cd "$(dirname "$BEFORE")" && pwd)/$(basename "$BEFORE")")"
  ga="$(gate_of "$(cd "$(dirname "$AFTER")" && pwd)/$(basename "$AFTER")")"
  if [ "$gb" != "$ga" ]; then
    DIFFS=$((DIFFS + 1))
    FIRST_DIFFS+=("课程门禁计数：$gb → $ga")
  fi
fi

echo
if [ "$DIFFS" -eq 0 ]; then
  echo "零差异：$CHECKS 组对拍全部逐字节相同（判定正确性不变）。"
  exit 0
fi
echo "**有差异：$DIFFS / $CHECKS 组**（内核改动不许合入）：" >&2
for line in "${FIRST_DIFFS[@]}"; do echo "  - $line" >&2; done
[ "$DIFFS" -gt "${#FIRST_DIFFS[@]}" ] && echo "  …（还有 $((DIFFS - ${#FIRST_DIFFS[@]})) 组）" >&2
exit 1
