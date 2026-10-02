#!/usr/bin/env bash
# G-85 复现：**省掉前导隐式实参 + 结果再收一个实参**时，实参被装错位 ✗
#
# ── 缺口原文（2026-10-03 内核线实测 ✗；起因 = 课程线 #5 普查的 86 处「红」）──
#   `And.right : {a b : Prop} → And a b → b`（前导隐式 k=2 · 显式 arity m=1）。
#   实参个数 n 的四种情形实测：
#     n = 1 = m        `And.right h`            ⇒ ✓ 绿（对照）
#     n = 2 = m+1      `And.right h x`          ⇒ ✗ **红**（本条）
#     n = 3 = k+m      `And.right p (∀…) h x`   ⇒ ✓ 绿（前导隐式实参写全 —— 旧写法）
#     n = 4 = k+m+1    `And.right p (∀…) h x y` ⇒ ✓ 绿
#   诊断：`类型不匹配：期望 Sort(0)，实际是 ((And.[] $3) Pi (x : Sort(0)), ($3 $0))`
#   ⇒ `h` 被**按位置**装进 `a : Prop` 那一层 ✗（`b` 也是）。
#
# ── 病根（`crates/front/src/compile/elab.rs` 的 `try_implicit_application`）──
#   路线③（「富余实参落到**结果**上」）靠**展开结果类型**造虚拟层；而 `And.right`
#   的结果是**变量** `b` ✗ —— 不把前导隐式参数解出来就展不动 ⇒ 路线③ 放弃 ⇒
#   落到「旧写法」分支（判据 = 实参个数 > 显式层数）⇒ **按位置**装 ⇒ 类型不匹配 ✗。
#   ⚠ 这一段 `m < n < k+m` 的读法是**唯一**的（n < k+m ⇒ 前导隐式实参根本没写全 ⇒
#     不可能是旧写法 ✓）⇒ 修它**不会**抢走任何既有形状 ✓。
#
# ── 判据（本复现件）──
#   ① `And.right h x` 必须判绿（`b := ∀ (x : Prop), Q x` ⇒ `x` 收到结果上 ✓）；
#   ② 三个**对照**（n=m / n=k+m / n=k+m+1）必须**照旧**判绿 —— 防「为了修①把旧写法
#      抢走」✗（那是路线③ 演进史上**实测踩过**的坑：prelude 当场打红 ✓）。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-85: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cat > "$WORK/shapes.sokonanoda" <<'EOF'
theorem n1 (p q : Prop) (h : p ∧ q) : q := And.right h

theorem n2 (p : Prop) (Q : Prop → Prop) (h : p ∧ (∀ (x : Prop), Q x)) (x : Prop) : Q x :=
  And.right h x

theorem n3 (p : Prop) (Q : Prop → Prop) (h : p ∧ (∀ (x : Prop), Q x)) (x : Prop) : Q x :=
  And.right p (∀ (y : Prop), Q y) h x

theorem n4 (p : Prop) (Q : Prop → Prop → Prop)
    (h : p ∧ (∀ (x : Prop), ∀ (y : Prop), Q x y)) (x y : Prop) : Q x y :=
  And.right p (∀ (u : Prop), ∀ (v : Prop), Q u v) h x y
EOF

python3 - "$BIN" "$WORK/shapes.sokonanoda" <<'PY'
import json, subprocess, sys

bin_, path = sys.argv[1], sys.argv[2]
out = subprocess.run([bin_, "query", "check", "--file", path], capture_output=True, text=True)
try:
    data = json.loads(out.stdout)["data"]
except Exception:
    print(f"✗ query check 没给出 JSON：{(out.stdout or out.stderr)[:160]!r}", file=sys.stderr)
    sys.exit(2)

failed = data.get("failed") or []
checked = data["counts"]["decl_checked"]

# ⚠ **不按声明名判** ✗：内核拒绝的那条在 `failed[].name` 上是 `None`（实测）⇒
# 按名字找会**静默判绿** ✗（本复现件第一版就是这么写的，被下面那条形状守卫逮住 ✓）。
# 结构判据：4 条声明 ⇒ 全绿 = `checked==4 且 failed==0` ✓；缺口那一形红 = `checked==3 且 failed==1` ✓。
print(f"  decl.checked={checked} · failed={len(failed)}")
for f in failed:
    print(f"    ✗ 判红 @line {f.get('start_line')}：{str(f.get('message'))[:120]}")

if checked == 4 and not failed:
    print("✓ 四条形状全绿（`And.right h x` 已能解出前导隐式参数 ✓，三个对照未被抢 ✓）")
    sys.exit(1)   # 1 = 行为已变（已修）
if checked == 3 and len(failed) == 1:
    print("\n✗ 缺口仍在：`And.right h x` 那一形被按位置装错 ⇒ 类型不匹配 ✗", file=sys.stderr)
    sys.exit(0)   # 0 = 缺口仍在（与台账 status=open 一致）
print(f"✗ 形状变了（期望 4 条判完 / 或 3+1）⇒ 回来看看", file=sys.stderr)
sys.exit(2)
PY
