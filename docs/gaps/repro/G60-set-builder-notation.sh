#!/usr/bin/env bash
# G-60 自断言复现：**集合建构式 `{x ∈ A | P x}` / `{x : α | P x}` 写不出来**。
#
# 退出码约定（docs/gaps/README.md）：
#   0 = 缺口仍在（两种**可写**形状里至少一种仍被拒）
#   1 = 已修（两种形状都能写，且 `{x | P x}` 仍被拒 + 诊断指路）
#   2 = 形状异常（需要人看：边界被打破 / 诊断码变了 / 对照组红了）
#
# 台账 `today`（0.77.0 实测，修前）：① `{x | P x}` ⇒ `set-literal-shape`
# （「集合字面量要写成 {a} 或 {a, b}」）；② `{x : α | x ∈ A}` ⇒ `unexpected-token`
# /「expected `)` to close binder group, found Pipe」。
#
# 修后契约（0.83.0，设计 `docs/design/notation-subset.md` §19）：
#   * `{x : α | P x}` ⇒ `fun (x : α) => P x`（**纯内核**，不依赖任何库）✓
#   * `{x ∈ A | P x}` ⇒ `Set.sep A (fun x => P x)`（与点名 `Set.sep A P` 同头常量）✓
#   * `{x | P x}` **仍被拒**（本语言没有元变量 ⇒ `x` 的类型没有来源），但码换成
#     **`set-builder-shape`** 且文案**指路**（「写 `{x : α | P x}` 或
#     `{x ∈ A | P x}`」）。
#
# ⚠ ① 的断言**按新诊断同步**了（不是删断言 ✗）：它钉的从「旧码 `set-literal-shape`」
#    改成「**新码 + 边界仍在 + 诊断指路**」—— 旧码那句「集合字面量要写成 {a} 或
#    {a, b}」对着 `{x | P x}` **指错路** ✗（它不是字面量，是缺类型来源的建构式）。
#    判据因此**更强**：既要求它仍被拒，又要求它说清楚该写什么 ✓。
#
# ⚠ 夹具**自足**（不 `import lib.Set`）：这样**修前的二进制**也能跑同一份脚本
#    （反证用 ✓ —— 否则它会先在 `lib.Set` 上失败，判词就不是本条的了 ✗）。
#    形状与卷 I 的 `lib/Set` 同款（`Set α = α → Prop`、`Set.sep A P = fun x => A x ∧ P x`）。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-60: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

LIB='def Set (α : Type) : Type := α → Prop
def Set.mem {α : Type} (a : α) (A : Set α) : Prop := A a
def Set.subset {α : Type} (A B : Set α) : Prop := ∀ (x : α), A x → B x
def Set.sep {α : Type} (A : Set α) (P : α → Prop) : Set α := fun (x : α) => A x ∧ P x
infix:50 " ∈ " => Set.mem
infix:50 " ⊆ " => Set.subset'

# 只收 **stdout**（G-10 的教训：`2>&1` 会把环境的告警混进 JSON 里 ✗）。
grade_src() {
  printf '%s\n' "$2" > "$WORK/$1.sokonanoda"
  "$BIN" --json "$WORK/$1.sokonanoda" 2>/dev/null
}

# 事件流 → 判词：`checked:<name>` / `rejected:<code>` / `other`；第二行是诊断全文。
verdict() {
  python3 -c '
import json, sys

want = sys.argv[1]
checked = False
codes = []
text = []
for line in sys.stdin:
    line = line.strip()
    if not line.startswith("{"):
        continue
    try:
        event = json.loads(line)
    except json.JSONDecodeError:
        continue
    kind = event.get("type")
    if kind == "decl.checked" and event.get("name") == want:
        checked = True
    elif kind == "diagnostic":
        codes.append(event.get("code") or "")
        text.append((event.get("message") or "") + (event.get("hint") or ""))
if codes:
    print("rejected:" + ",".join(codes))
    print("TEXT\t" + " ".join(text))
elif checked:
    print("checked:" + want)
else:
    print("other")
' "$1"
}

# ── ① `{x | P x}`：**仍被拒**（边界）+ 专用码 + 诊断**指路** ────────────────
a_out="$(grade_src a 'def Set (α : Type) : Type := α → Prop
def t (α : Type) (P : α → Prop) : Set α := {x | P x}')"
a_verdict="$(printf '%s' "$a_out" | verdict t)"
a_code="$(printf '%s\n' "$a_verdict" | head -1)"
a_text="$(printf '%s\n' "$a_verdict" | sed -n '2p' | cut -f2-)"

# ① 的三档：新码 + 指路 ✓ / 旧码（= 修前形状）/ 别的（边界破了）。
a_state=broken
case "$a_code" in
  rejected:set-builder-shape)
    if printf '%s' "$a_text" | grep -q '{x : α | P x}' \
       && printf '%s' "$a_text" | grep -q '{x ∈ A | P x}'; then
      a_state=guided
      echo "① ✓ \`{x | P x}\` 仍被拒（set-builder-shape），文案指路两种可写形状"
    else
      echo "① ✗ 码对了但**没有指路**（没说该写 \`{x : α | P x}\` / \`{x ∈ A | P x}\`）" >&2
      printf '%s\n' "$a_text" >&2
      exit 2
    fi
    ;;
  rejected:set-literal-shape)
    a_state=prefix
    echo "① … \`{x | P x}\` 被拒，但码是**修前**的 set-literal-shape（缺口形状）"
    ;;
  rejected:*)
    echo "① ✗ 诊断码既不是 set-builder-shape 也不是修前的 set-literal-shape：${a_code}" >&2
    exit 2
    ;;
  *)
    echo "① ✗ \`{x | P x}\` 变成能写了 —— 边界被打破（没有元变量 ⇒ 类型没有来源）" >&2
    printf '%s\n' "$a_out" | head -3 >&2
    exit 2
    ;;
esac

# ── ② `{x : α | x ∈ A}`：**能写**（修后形状）────────────────────────────────
b_out="$(grade_src b "$LIB
def t2 (α : Type) (A : Set α) : Set α := {x : α | x ∈ A}")"
b_verdict="$(printf '%s' "$b_out" | verdict t2 | head -1)"

# ── ③ `{x ∈ A | P x}`：**能写**（修后形状）──────────────────────────────────
c_out="$(grade_src c "$LIB
def t3 (α : Type) (A : Set α) (P : α → Prop) : Set α := {x ∈ A | P x}")"
c_verdict="$(printf '%s' "$c_out" | verdict t3 | head -1)"

# ── ④ 对照组：点名写法**必须仍能用**（`Set.sep` 已落库）─────────────────────
d_out="$(grade_src d "$LIB
def t4 (α : Type) (A : Set α) (P : α → Prop) : Set α := Set.sep A P")"
d_verdict="$(printf '%s' "$d_out" | verdict t4 | head -1)"

# ── ⑤ 边界回归：既有花括号形状**逐字不变**（`{a}` / `{a, b}` 仍是字面量）────
e_out="$(grade_src e "$LIB
def Set.singleton {α : Type} (a : α) : Set α := fun (x : α) => x = a
def Set.pair {α : Type} (a b : α) : Set α := fun (x : α) => x = a ∨ x = b
def one (α : Type) (a : α) : Set α := {a}
def two (α : Type) (a b : α) : Set α := {a, b}")"
e_one="$(printf '%s' "$e_out" | verdict one | head -1)"
e_two="$(printf '%s' "$e_out" | verdict two | head -1)"

# ── ⑥ 建构式在 `by` 块里**可用**（配套修复 `render_binder` 在位）────────────
f_out="$(grade_src f "$LIB
theorem sep_subset (α : Type) (A : Set α) (P : α → Prop) : Set.sep A P ⊆ A := by
  intro x
  intro h
  exact And.left h
theorem t6 (α : Type) (A : Set α) (P : α → Prop) : {x ∈ A | P x} ⊆ A := by
  exact sep_subset α A P")"
f_verdict="$(printf '%s' "$f_out" | verdict t6 | head -1)"

echo "② {x : α | x ∈ A} ⇒ ${b_verdict}"
echo "③ {x ∈ A | P x}   ⇒ ${c_verdict}"
echo "④ 点名 Set.sep     ⇒ ${d_verdict}"
echo "⑤ {a} / {a, b}     ⇒ ${e_one} / ${e_two}"
echo "⑥ 目标含建构式的 by 块 ⇒ ${f_verdict}"

[ "$d_verdict" = "checked:t4" ] || { echo "形状异常：④ 实际 ${d_verdict}" >&2; exit 2; }
[ "$e_one" = "checked:one" ] || { echo "形状异常：⑤ {a} 实际 ${e_one}" >&2; exit 2; }
[ "$e_two" = "checked:two" ] || { echo "形状异常：⑤ {a, b} 实际 ${e_two}" >&2; exit 2; }

if [ "$b_verdict" = "checked:t2" ] && [ "$c_verdict" = "checked:t3" ]; then
  # 两种形状都能写了 ⇒ 再要求「边界仍在 + 指路」与「by 块可用」都成立。
  [ "$a_state" = guided ] || { echo "形状异常：能写了但 ① 不是「新码 + 指路」（${a_state}）" >&2; exit 2; }
  [ "$f_verdict" = "checked:t6" ] || { echo "形状异常：⑥ 目标含建构式的 by 块没证出来（${f_verdict}）" >&2; exit 2; }
  echo '结论：G-60 已修 —— `{x : α | P x}` 与 `{x ∈ A | P x}` 都能写（后者与点名 `Set.sep` 同头常量、`by` 块可用），'
  echo '      `{x | P x}` 仍被拒且诊断指路（set-builder-shape）。'
  exit 1
fi

echo "结论：G-60 仍在 —— 两种可写形状里至少一种仍被拒（② $b_verdict · ③ ${c_verdict}）。" >&2
exit 0
