#!/usr/bin/env bash
# G-43：课程库隐式化（**B2**）之后，**用它的单元**还不能保持全绿 ✗。
#
# 实测（2026-09-26，**修掉转换脚本自己的 bug 之后**）：
#   · 只改**签名**（`def Set` 的 α 必须**保持显式** ✓ —— 它是集合类型构造子）
#     + 库里**自己的调用点**（`univ α` ⇒ `univ` ✓）⇒ **`lib/Set` 编得干净（0 诊断 ✓）**；
#   · 但**单元**里还写着 `Set.subset α A B` 这类**多写了类型实参**的调用点
#     ⇒ 整门课 `286 checked · 86 open · 18 判负` ✗（红线是 `328 · 99 · 0` ✗）；
#   · 再用**一刀切正则**改 197 处调用点 ⇒ **反而更差**（`279 · 88 · 20` ✗）
#     ⇒ 调用点必须**逐文件**改 ✗（纪律：同一处连红 3 次 ⇒ 换思路 ✓）。
#
# 期望：缺口**仍在**时 exit 0（单元还红）；B2 做完后 exit 1（全绿）。
# 只判**一个单元**（快 ✓）；`trap` 还原，**不留副作用** ✓。
set -uo pipefail
cd "$(dirname "$0")/../../.." || exit 2
ROOT=$PWD
restore() { git -C "$ROOT" checkout -- courses/set-theory/ 2>/dev/null; }
trap restore EXIT

python3 - <<'PY'
import re
NAMES = 'mem|subset|empty|univ|singleton|pair|union|inter|sdiff|compl|powerset'
for p in ('courses/set-theory/lib/Set.sokonanoda', 'courses/set-theory/lib/Image.sokonanoda'):
    lines = open(p).read().split("\n")
    for i, l in enumerate(lines):
        # ⚠ `def Set` 必须排除（α 显式 ✓）—— 第一次漏了它 ⇒ 整门课塌掉、误判成语言缺口 ✗。
        if (l.startswith('def ') or l.startswith('theorem ')) and not l.startswith('def Set '):
            lines[i] = re.sub(r'\((α(?: β)?) : Type\)', r'{\1 : Type}', l, count=1)
    src = "\n".join(lines)
    # 库里自己的调用点：`univ α` ⇒ `univ`（只动**裸名 + α**，不碰 `Set.` 前缀的单元调用 ✓）
    src = re.sub(r'\b(' + NAMES + r') α\b', r'\1', src)
    open(p, 'w').write(src)
PY

BIN=./target/release/sokonanoda
[ -x "$BIN" ] || { echo "G-43：先跑 cargo build --release -p sokonanoda-cli" >&2; exit 2; }
UNIT=courses/set-theory/units/unit03-union-inter-powerset.sokonanoda
out=$("$BIN" grade "$UNIT" 2>&1)
if printf '%s' "$out" | grep -q '"code"'; then
  echo "G-43 仍在：库隐式化之后单元 $UNIT 还红（B2 未完成）" >&2
  printf '%s' "$out" | grep -oE '"message":"[^"]{0,84}' | head -2 >&2
  exit 0
fi
echo "G-43 已修：课程库可以隐式化，单元仍绿 ✓" >&2
exit 1
