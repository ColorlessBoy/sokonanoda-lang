#!/usr/bin/env bash
# expect-red.sh —— **断言一条命令必须失败（判红）**（2026-09-25 round 172 ✓）
#
# 为什么需要它 ✓（本 session 的教训，**同一个坑三次** ✗）：
#   ① round 138：`set -e` 遇上 `cargo clippy … | grep …` ⇒ 管道退出码是 **grep** 的 ✗
#      ⇒ clippy 失败**不会**让脚本停 ⇒ 我把没修好的提交推了上去 ✗；
#   ② 同轮第二次：同样的写法，同样的结果 ✗；
#   ③ round 171：`--self-test` 明明打印 `FAIL` ✓，我却让脚本继续 ⇒ 把**弄瞎守卫**的版本推了上去 ✗。
# ⇒ 结论 ✓：**"记得按退出码拦"靠人守不住** ✗ ⇒ 把它变成**命令的形状** ✓ ——
#   要表达"这条必须红"就写 `scripts/expect-red.sh <说明> -- <cmd…>` ✓，
#   写成 `cmd && echo ok` 这种**看不出退出码**的形状就写不出来了 ✓。
#
# 用法：
#   scripts/expect-red.sh "关掉折叠时判据必须红" -- \
#       env SOKO_NO_NOTATION_FOLD=1 cargo test -p sokonanoda-front --lib <判据名>
# 退出码：0 = 那条命令**确实失败**了 ✓ / 1 = 它**成功了**（= 断言失败 ✗）
set -uo pipefail
desc="${1:?用法: expect-red.sh <说明> -- <cmd…>}"; shift
[ "${1:-}" = "--" ] || { echo "expect-red.sh：参数里要有一个 \`--\` ✗" >&2; exit 2; }
shift
if "$@" >/tmp/soko-expect-red.log 2>&1; then
  printf '  ✗ %s —— 但它**成功了** ✗（本该判红）\n' "$desc"
  printf '    输出尾部：\n'; tail -5 /tmp/soko-expect-red.log | sed 's/^/      /'
  exit 1
fi
printf '  ✓ %s（如预期判红 ✗）\n' "$desc"
