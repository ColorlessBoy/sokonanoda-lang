#!/usr/bin/env bash
# G-43：把课程标准库的**前导类型参数**从显式 `(α : Type)` 改成隐式 `{α : Type}` 之后，
# **库自己**就编不过 ✗（实测：课程门禁从 `328 checked · 99 open · 0 判负` 塌到
# `46 checked · 0 open · 31 判负` ✗）。
#
# 这是 **B2（课程库隐式化）** 的验收件：**它绿 ⇒ B2 可以做了** ✓。
# 只判 `lib/Set.sokonanoda` 一份（快 ✓；全门禁的结论见台账正文 ✓）。
# 脚本退出时 `git checkout` 还原，**不留副作用** ✓。
#
# 期望：缺口**仍在**时 exit 0（库编不过）；修好后 exit 1（库仍绿）。
set -uo pipefail
cd "$(dirname "$0")/../../.." || exit 2
ROOT=$PWD
restore() { git -C "$ROOT" checkout -- courses/set-theory/lib/ 2>/dev/null; }
trap restore EXIT

python3 - <<'PY'
import re
p = 'courses/set-theory/lib/Set.sokonanoda'
out = []
for line in open(p).read().split("\n"):
    if (line.startswith('def ') or line.startswith('theorem ')) and line != 'def Set (α : Type) : Type':
        line = re.sub(r'\((α(?: β)?) : Type\)', r'{\1 : Type}', line, count=1)
    out.append(line)
open(p, 'w').write("\n".join(out))
PY

# 用**仓库里现成**的二进制（`cargo build --release` 由调用者先跑 ✓ —— 别在判据里再编一遍 ✗）。
BIN=./target/release/sokonanoda
[ -x "$BIN" ] || { echo "G-43：先跑 cargo build --release -p sokonanoda-cli" >&2; exit 2; }

out=$("$BIN" grade courses/set-theory/lib/Set.sokonanoda 2>&1)
if printf '%s' "$out" | grep -q '"code"'; then
  echo "G-43 仍在：课程库隐式化之后**库自己**就编不过（B2 仍被挡）" >&2
  printf '%s' "$out" | grep -oE '"message":"[^"]{0,90}' | head -3 >&2
  exit 0
fi
echo "G-43 已修：课程库可以隐式化，库仍绿 ✓" >&2
exit 1
