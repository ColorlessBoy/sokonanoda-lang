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

# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE`）：判据是**诊断码**，
# 而「已修」那一档要的正是"一条诊断都没有" ⇒ **量具空转（零事件 / 零输出）时读数与
# 「已修」逐字相同** ✗ = **假绿**。⇒ 先要求这次判卷真的观测到**本夹具的声明事件**
# （可归属 ✓ —— namespace 内的规范名是 `Foo.subset` / `Foo.powerset`）或**一条诊断**；
# 两样都没有 ⇒ **exit 2 + 人话**，绝不报「已修」✗。
# 同族已修好的形状：`G69` 的「无事件行 ⇒ exit 2」· `G34` 的「calls==0 ⇒ exit 2」。
if ! printf '%s' "$out" | grep -qE '"name":"Foo\.subset","type"|"name":"Foo\.powerset","type"|"type":"diagnostic"'; then
  echo "G-42：观测不到判卷事件（既没有 Foo.subset/Foo.powerset 的声明事件，也没有诊断）⇒ 环境/形状异常（exit 2），不是「已修」✗" >&2
  echo "      · 这条判据量的就是诊断码；一条事件都没有时 codes 恒为空 ⇒ 「已修」没有信息量 ✗" >&2
  echo "      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 夹具或模块根不对" >&2
  printf '%s\n' "$out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        → /' >&2
  exit 2
fi

echo "诊断码：${codes:-（无）}"
case "$codes" in
  *kernel-rejected*|*elab-implicit-argument-unsolved*|*elab-tactic-failed*)
    echo "G-42 仍在：namespace 里的裸名引用查不到规范名 ⇒ 隐式插入不触发" >&2
    exit 0 ;;
esac
echo "G-42 已修：namespace 里的裸名引用也能触发隐式插入" >&2
exit 1
