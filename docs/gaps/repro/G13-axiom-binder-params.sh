#!/usr/bin/env bash
# G-13 自断言复现：**`axiom` 不吃 binder 参数表**（`def`/`theorem` 吃）
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在（**已修后的期望形状**：三段断言必须全绿，任何一段红 ⇒ 缺口回来了）
#   1 = 行为变了 ⇒ 回来更新台账（写 fixed_in / 收工）
#   2 = 环境不满足
#
# 现场（由 `docs/notes/settheory-survey/` 的调研探针首次发现）：
#   `axiom Foo (α : Type) : Prop`   → parse 阶段 `expected axiom type, found LParen`
#   `axiom Foo : (α : Type) -> Prop` → 正常（柯里化写法）
# 而 `def` / `theorem` **接受** binder 组（`def Set (α : Type) : Type := α -> Prop` 全课程在用）。
# 后果：写库时 axiom 必须全部柯里化，与 def/theorem 的写法不一致；照 Lean 4 习惯写
# `axiom f (x : α) : β`（官方 Lean 完全可以）会被拒。
#
# 加重情节：同一份文件 `query check` 报 `ok:true` + 全零计数（那就是 G-10），
# 所以走 MCP `check` 的 agent 连"有 parse 错误"都看不到。
#
# 期望（官方 Lean 4 语义）：`axiom Foo (α : Type) : Prop` 与 `axiom Foo : (α : Type) -> Prop`
# 等价；binder 参数表是声明语法的一部分，四种声明形式应当一致。
#
# ── 本轮的更新（G-13 修好后）────────────────────────────────────────────
# 本脚本在修好后**翻成 exit 1**（docs/gaps/README.md 的约定：exit 1 = 行为已变 ⇒ 关账）。
# 断言同时改成"修后应有的形状"，所以脚本不会在缺口复发时假绿：
#   ① `axiom Foo (α : Type) : Sort 1`（binder 形式，**codomain 取 Sort 1**：内核要求
#      axiom 的类型是 Sort，`… -> Prop` 这种 Pi 类型会被 kernel-rejected，与写法无关）
#      → 必须 `decl.checked`，不许再出现 unexpected-token；
#   ② 柯里化形式 → 必须 `decl.checked`（与 ① 语义等价）；
#   ③ 对照 `def` 带 binder → 必须 `decl.checked`；
#   ④ `query check` 的 `decl_checked` 计数必须 ≥ 1（G-10 修好后 parse 错误也带诊断）。
# 任一段不满足 ⇒ echo 缺口回来了 + **exit 0**（与缺口仍在的语义一致）。

set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

printf 'axiom Foo (α : Type) : Sort 1\n' > "$TMP/binder.sokonanoda"
printf 'axiom Foo : (α : Type) -> Sort 1\n' > "$TMP/curried.sokonanoda"

echo "== ① binder 形式（修后必须 checked）=="
B="$( node "$SOKO" grade "$TMP/binder.sokonanoda" 2>&1 )"
printf '%s\n' "$B" | tail -1
b_ok=1
printf '%s' "$B" | grep -q '"type":"decl.checked"' && b_ok=0
echo "   → checked（修后预期）：$([ "$b_ok" = 0 ] && echo yes || echo NO)"
echo

echo "== ② 柯里化形式（同一语义）=="
C="$( node "$SOKO" grade "$TMP/curried.sokonanoda" 2>&1 )"
printf '%s\n' "$C" | tail -1
c_ok=1
printf '%s' "$C" | grep -q '"type":"decl.checked"' && c_ok=0
echo "   → 判卷通过（预期）：$([ "$c_ok" = 0 ] && echo yes || echo NO)"
echo

echo "== ③ 对照：def 接受 binder（同一台二进制）=="
printf 'def Bar (α : Type) : Type := α -> α\n' > "$TMP/defbinder.sokonanoda"
D="$( node "$SOKO" grade "$TMP/defbinder.sokonanoda" 2>&1 )"
printf '%s\n' "$D" | tail -1
d_ok=1
printf '%s' "$D" | grep -q '"type":"decl.checked"' && d_ok=0
echo "   → def 带 binder 通过（预期）：$([ "$d_ok" = 0 ] && echo yes || echo NO)"
echo

echo "== ④ 同一份 binder 文件走 query check（G-10 修后：带计数与诊断）=="
Q="$( node "$SOKO" query check --file "$TMP/binder.sokonanoda" --compact | head -1 )"
printf '%s\n' "$Q"
q_ok=1
printf '%s' "$Q" | grep -q '"decl_checked":1' && q_ok=0
echo "   → query check 计数 ≥1（修后预期）：$([ "$q_ok" = 0 ] && echo yes || echo NO)"
echo

if [ "$b_ok" = 0 ] && [ "$c_ok" = 0 ] && [ "$d_ok" = 0 ] && [ "$q_ok" = 0 ]; then
  echo "结论：G-13 已修（axiom 吃 binder、柯里化仍可通过、def 不变）⇒ 行为已变，更新台账。" >&2
  exit 1
fi
echo "结论：缺口回来了（axiom 又不吃 binder / 柯里化回归）——检查 crates/front/src/parser.rs 的 parse_axiom。"
exit 0
