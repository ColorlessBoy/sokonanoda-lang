#!/usr/bin/env bash
# 六条用户反馈的**现状**，一条命令看完。
#
# 用户原话（2026-09-21）：「在 sokonanoda 的 project 为 root 目录（比如 set-theory），
# vscode 有很多问题：编译很慢，没有实现编译后的文件加速 vscode 处理，新打开一个
# 文件就有临时编译；infoview 的『声明』栏经常失效……goal 展现没有用 notation 的
# 方式；代码里的 notation 不能跳转，hover 信息也没有对应的原始类型。」
#
# 本脚本按**用户反馈**组织，逐条跑它对应的缺口复现（`docs/gaps/repro/G2x-*`），
# 打印一张状态表。复现的退出码约定见 `docs/gaps/README.md`：
#   0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（需要人看）
#
# 用法：
#   scripts/verify-editor-issues.sh            # 跑复现（秒级到分钟级，取决于 LSP 探针）
#   scripts/verify-editor-issues.sh --only G-22
#   scripts/verify-editor-issues.sh --fast     # 只跑非 LSP 的（跳过要起进程的探针）
#
# 退出码：0 = 六条反馈**全部已修**；1 = 还有缺口仍在；2 = 有环境异常或用法错误。
#
# 分工：本脚本是**人读的全貌**；机器判据是 `python3 scripts/gap.py check`
# （台账契约，进 `scripts/soko gate`）。两者跑的是同一批复现件。
#
# e2e 与性能两段要等 T-017 / T-022 落地后接入（现在会打印「未接入」而不是假装通过）。

set -u
cd "$(dirname "$0")/.." || exit 2

only=""
fast=0
while [ $# -gt 0 ]; do
  case "$1" in
    --only) shift; only="${1:-}" ;;
    --fast) fast=1 ;;
    -h | --help) sed -n '2,26p' "$0"; exit 0 ;;
    *) echo "error: unknown flag: $1" >&2; exit 2 ;;
  esac
  shift
done

# 反馈编号 | 反馈原文（短） | 缺口 | 复现 | 是否 LSP 探针
ROWS=(
  "1|编译很慢|G-25|docs/gaps/repro/G25-lsp-recompiles-on-every-open.sh|lsp"
  "2|没有实现编译后的文件加速|G-24|docs/gaps/repro/G24-project-cache-never-warms.sh|cli"
  "2b|（同上：CLI 预热对 LSP 无效）|G-27|docs/gaps/repro/G27-cache-key-folds-binary-mtime.sh|cli"
  "3|新打开一个文件就有临时编译|G-25|docs/gaps/repro/G25-lsp-recompiles-on-every-open.sh|lsp"
  "4|Infoview『声明』栏失效|G-22|docs/gaps/repro/G22-lsp-goals-empty-for-import-entry.sh|lsp"
  "5|goal 展现没有用 notation|G-26|docs/gaps/repro/G26-goal-text-loses-notation.sh|cli"
  "6|记法不能跳转 / hover 无原始类型|G-23|docs/gaps/repro/G23-notation-navigation.sh|lsp"
  "—|（顺带发现）\`→ ∀\` 不解析|G-28|docs/gaps/repro/G28-arrow-forall.sokonanoda|lang"
)

gap=0
fixed=0
broken=0
skipped=0
ran_g25=0

printf '%-4s %-34s %-6s %-10s %s\n' "反馈" "内容" "缺口" "状态" "复现"
printf '%s\n' "--------------------------------------------------------------------------------"

for row in "${ROWS[@]}"; do
  IFS='|' read -r num label gid repro kind <<<"$row"
  if [ -n "$only" ] && [ "$only" != "$gid" ]; then
    continue
  fi
  # G-25 要起两个 LSP 进程（分钟级），同一张表里跑两次没意义。
  if [ "$gid" = "G-25" ] && [ "$ran_g25" = 1 ]; then
    printf '%-4s %-34s %-6s %-10s %s\n' "$num" "$label" "$gid" "见上" "（同一个复现）"
    continue
  fi
  if [ "$fast" = 1 ] && [ "$kind" = "lsp" ]; then
    printf '%-4s %-34s %-6s %-10s %s\n' "$num" "$label" "$gid" "跳过(--fast)" "$repro"
    skipped=$((skipped + 1))
    continue
  fi
  if [ ! -e "$repro" ]; then
    printf '%-4s %-34s %-6s %-10s %s\n' "$num" "$label" "$gid" "复现缺失" "$repro"
    broken=$((broken + 1))
    continue
  fi

  case "$kind" in
    lsp | cli)
      bash "$repro" >/dev/null 2>&1
      code=$?
      ;;
    lang)
      scripts/soko grade "$repro" >/dev/null 2>&1
      code=$?
      # `.sokonanoda` 复现：0 = 判卷干净（= 已修），非 0 = 仍有失败（= 缺口仍在）
      [ "$code" = 0 ] && code=1 || code=0
      ;;
    *)
      code=2
      ;;
  esac

  case "$code" in
    0) state="缺口仍在"; gap=$((gap + 1)) ;;
    1) state="已修"; fixed=$((fixed + 1)) ;;
    *) state="环境异常"; broken=$((broken + 1)) ;;
  esac
  [ "$gid" = "G-25" ] && ran_g25=1
  printf '%-4s %-34s %-6s %-10s %s\n' "$num" "$label" "$gid" "$state" "$repro"
done

echo
echo "== 另外两段（等 T-017 / T-022 落地后接入）=="
if [ -x scripts/vscode-e2e.sh ] && grep -q -- '--grep' scripts/vscode-e2e.sh 2>/dev/null; then
  echo "  e2e   ：scripts/vscode-e2e.sh --grep <用例名> --profile debug --no-build"
else
  echo "  e2e   ：未接入（T-017 还没做：scripts/vscode-e2e.sh 没有 --grep/--profile/--no-build）"
fi
if [ -f scripts/perf-compare.py ]; then
  echo "  perf  ：python3 scripts/perf-compare.py"
else
  echo "  perf  ：未接入（T-022 还没做：没有 scripts/perf-compare.py）"
fi

echo
printf '小结：已修 %d · 缺口仍在 %d · 环境异常 %d' "$fixed" "$gap" "$broken"
[ "$skipped" -gt 0 ] && printf ' · 跳过 %d' "$skipped"
echo

if [ "$broken" -gt 0 ]; then
  echo "结论：有环境异常，先看上面标了「环境异常 / 复现缺失」的那几条。" >&2
  exit 2
fi
if [ "$gap" -gt 0 ]; then
  echo "结论：还有 $gap 条缺口仍在（机器判据：python3 scripts/gap.py check）。" >&2
  exit 1
fi
echo "结论：六条反馈全部已修。"
exit 0
