#!/usr/bin/env bash
# 改内核后的**验收五步**，一条命令跑完（T-K02）。
#
# 为什么要有它：内核解冻（REQUIREMENTS §2 第 1 条）之后，唯一红线是**判定不变**
# ——同一批输入**接受/拒绝不变、事件计数不变、golden 与 `--json` 逐字节不变**。
# 五步就是这条红线的五个面，缺一面就等于没验。
#
#   ① `cargo test --workspace --locked` —— 三层回归里的前两层（kernel 单测 +
#      front 单测 + CLI e2e 都在里面）；
#   ② `scripts/kernel-diff.sh` —— 内核相对**上游**的改动台账（改了什么、为什么）；
#   ③ 课程门禁计数（`courses/set-theory/tools/check.py`）—— `36 目标 · 328 checked ·
#      99 open · 0 判负` 这类计数是**语料级**的红线，内核一动最先在这里露头；
#   ④ `scripts/perf-ledger.sh` —— 性能台账（提速是允许的，退化不行）；
#   ⑤ **语料对拍**（`cargo test -p sokonanoda --test arena`）—— 真 Lean 导出语料
#      的接受/拒绝必须与预期一致。
#
# ⑤ 需要外部语料：`LEAN_KERNEL_ARENA=<lean-kernel-arena 路径>`。**没设就明说跳过**
# （不假装有）：原来 CI 里没设、测试静默空过，"语料对拍"实际上只有一个用例。
# 设了就必须跑过；想放宽体积上限用 `LEAN_KERNEL_ARENA_MAX_BYTES`。
#
# 退出码：0 = 五步全过（含"⑤ 明确跳过"）；非 0 = 有一步没过。
set -uo pipefail
cd "$(dirname "$0")/.." || exit 2
step=0
fail=0

run() {
  step=$((step + 1))
  echo
  echo "== 第 ${step} 步：$1 =="
  shift
  if "$@"; then
    echo "-- 第 ${step} 步通过"
  else
    echo "-- 第 ${step} 步**没过**（退出码 $?）" >&2
    fail=1
  fi
}

run "三层回归（cargo test --workspace --locked）" cargo test --workspace --locked
# ② 全语料逐字节对拍（T-K01）：**改动前后两个二进制**的 stdout + 退出码逐字节比。
# 给了 `KERNEL_DIFF_BASELINE`（改动前的二进制）就跑真对拍；没给就跑 `--self-test`
# ——它证明**这条通道本身能发现差异**（不是安慰剂），并明确说"全量对拍这轮跳过了"。
step=$((step + 1))
echo
echo "== 第 ${step} 步：全语料逐字节对拍（kernel-diff.sh） =="
if [ -n "${KERNEL_DIFF_BASELINE:-}" ]; then
  if bash scripts/kernel-diff.sh --fast "$KERNEL_DIFF_BASELINE" "${KERNEL_DIFF_CURRENT:-./target/debug/sokonanoda}"; then
    echo "-- 第 ${step} 步通过"
  else
    echo "-- 第 ${step} 步**没过**" >&2
    fail=1
  fi
else
  echo "-- 未设 KERNEL_DIFF_BASELINE（改动前的二进制）⇒ 跑 **self-test** 证明这条通道"
  echo "   真能发现差异；**全量对拍这轮跳过了** —— 改内核前请先留一份改动前的二进制："
  echo "     cp target/debug/sokonanoda /tmp/sokonanoda-before"
  echo "   改完再跑：KERNEL_DIFF_BASELINE=/tmp/sokonanoda-before scripts/kernel-check.sh"
  if bash scripts/kernel-diff.sh --self-test ./target/debug/sokonanoda; then
    echo "-- 第 ${step} 步通过（self-test）"
  else
    echo "-- 第 ${step} 步**没过**（self-test 没过 ⇒ 对拍通道本身有问题）" >&2
    fail=1
  fi
fi
run "课程门禁计数" python3 courses/set-theory/tools/check.py
run "性能台账（perf-ledger.sh）" bash scripts/perf-ledger.sh

step=$((step + 1))
echo
echo "== 第 ${step} 步：语料对拍（真 Lean 导出） =="
if [ -n "${LEAN_KERNEL_ARENA:-}" ]; then
  if cargo test -p sokonanoda --test arena --locked; then
    echo "-- 第 ${step} 步通过"
  else
    echo "-- 第 ${step} 步**没过**" >&2
    fail=1
  fi
else
  echo "-- **跳过**：LEAN_KERNEL_ARENA 未设（这一层是"本地可选"：需要外部"
  echo "   lean-kernel-arena 语料）。**别当成跑过了** —— 没设时 step ⑤ 等于没有。"
fi

echo
if [ "$fail" = 0 ]; then
  echo "五步完成：前三步 + 性能台账全过（语料对拍见上面的跳过说明）。"
else
  echo "五步里有**没过**的步骤 —— 内核改动不能提交。" >&2
fi
exit "$fail"
