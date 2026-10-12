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
