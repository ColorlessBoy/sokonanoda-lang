#!/usr/bin/env bash
# G-42 复现：隐式插入查签名表用 **AST 上的裸名**，而签名表按**规范名**建
# ⇒ `namespace Foo` 里的裸名引用**整条不触发** ✗。
#
# 最小复现（5 行；**去掉 `namespace` 就好** —— 这正是它躲过所有既有测试的原因）：
#
#     def Foo (α : Type) : Type := α -> Prop
#     namespace Foo
#     def subset {α : Type} (A B : Foo α) : Prop := forall (x : α), A x -> B x
#     def powerset {α : Type} (A : Foo α) : Foo (Foo α) := fun (B : Foo α) => subset B A
#     end Foo
#     ⇒ 类型不匹配：期望 Sort(1)，实际是 (Foo.[] $2)
#
# ⚠ **显然的修法（把裸名先 `resolve_known` 成规范名）不能直接上** ✗：
# 它修好本条，却**回归** `#check some Nat`（`Nat -> Option Nat` ⇒ `Option Type 0` ✗，
# 撞红既有测试 `parameterized_option_checks_and_derives_recursor`）—— 根因是
# 「构造子把隐式位**逐位写出来**」（`some Nat`）与「**短写**」（`some a`）
# 在**实参个数上同形**（都 1 个），钩子一开就选了短写那一支 ✗。
# ⇒ 修这条必须**同时**解决那个歧义（判据：既有 Option 测试 + 本复现件都绿 ✓）。
#
# 退出码（见 docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常。
set -u
cd "$(dirname "$0")/../../.." || exit 2
command -v python3 >/dev/null 2>&1 || { echo "需要 python3" >&2; exit 2; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cat > "$work/repro.sokonanoda" <<'EOF'
def Foo (α : Type) : Type := α -> Prop
namespace Foo
def subset {α : Type} (A B : Foo α) : Prop := forall (x : α), A x -> B x
def powerset {α : Type} (A : Foo α) : Foo (Foo α) := fun (B : Foo α) => subset B A
end Foo
EOF

out="$(scripts/soko grade --json "$work/repro.sokonanoda" 2>&1)"
codes="$(printf '%s' "$out" | python3 -c '
import sys, json
codes = []
for line in sys.stdin:
    line = line.strip()
    if not line.startswith("{"):
        continue
    try:
        ev = json.loads(line)
    except json.JSONDecodeError:
        continue
    if ev.get("type") == "diagnostic":
        codes.append(ev.get("code", "?"))
print(" ".join(codes))
')"

echo "诊断码：${codes:-（无）}"
case "$codes" in
  *kernel-rejected*|*elab-implicit-argument-unsolved*|*elab-tactic-failed*)
    echo "G-42 仍在：namespace 里的裸名引用查不到规范名 ⇒ 隐式插入不触发" >&2
    exit 0 ;;
esac
echo "G-42 已修：namespace 里的裸名引用也能触发隐式插入" >&2
exit 1
