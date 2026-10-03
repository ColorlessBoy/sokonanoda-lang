#!/usr/bin/env bash
# **B1 片的端到端正例**（范围 B · 开关 `SOKO_ARG_EXPECTED`，默认关 ✓）——
# **短写的隐式调用当实参时，期望类型必须送到实参位** ✓（台账 **G-86** ✗ 的那一条）。
#
# ── 形状（一句话）──────────────────────────────────────────────────────────
#   `theorem t3 (h : ¬ (P ∨ Q)) (hp : P) : False := h (Or.inl hp)`
#   `Or.inl hp` 只约束 `?A := P` ✓，`?B` **没有实参可问** ✗ ⇒ 修前落到「按兄弟同形兜底」
#   ⇒ `?B := ?A := P` ✗ ⇒ 内核 `期望 ((Or P) Q)，实际 ((Or P) P)` ✗。
#   `?B` 的唯一来源 = **实参位的期望类型**（`h` 的域 `P ∨ Q` ✓）。
#
# ── 修法（**零内核调用、零递归** ✓）────────────────────────────────────────
#   头是**局部变量**时，它的**书写类型**就在 `scope.src_tys` 里 ✓ ⇒ 直接剥 Π 到实参位
#   （`¬ X` 是 def 头 ⇒ δ 展开一次 ✓）。**不走** `application_arg_expected` ✗ ——
#   那条要 `judge_infer` 头的类型，判定**再入** elaborate ⇒ 对"每个应用实参都算期望类型"
#   的形状**栈溢出**（实测 exit 134 ✗，见 G-86 的 notes ✓）。
#
# ── 判据（四条，缺一不算成立）────────────────────────────────────────────
#   ① **开关关**：`t3` 必须判红 ✓（基线行为 = 台账 `open` 的依据 ✓）；
#   ② **开关开**：`t3` 判绿 ✓（正例成立 ✓）；
#   ③ **对照**：`t`（期望位=显式目标 ✓）与 `h (Or.inl P Q hp)`（前导写全 ✓）**两态都绿** ✓；
#   ④ **反面**：真的**解不出**的隐式实参（`ignores 3`，`{α}` 在实参类型与期望类型里都不出现 ✓）
#      在**开关开**时**仍须判红** ✓ —— 证明不是"什么都放行" ✗。
#
# ── 退出码约定（⚠ 与 `docs/gaps/README.md` **相反**，按本片的约定 ✓）──────────
#   0 = **正例成立**（①②③④ 全过 ✓）    1 = 判据不成立 ✗    2 = 环境不满足
#   （本件**不进台账**：它是 B1 切片在**开关**下的验收 ✓；台账 **G-86 仍 `open`** ✓ ——
#    开关默认关 ⇒ 缺口对用户仍然存在 ✓。）
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "B1: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cat > "$WORK/b1.sokonanoda" <<'EOF'
axiom P : Prop
axiom Q : Prop

theorem t (hp : P) : P ∨ Q := Or.inl hp
theorem t2 (h : ¬ (P ∨ Q)) (hp : P) : False := h (Or.inl P Q hp)
theorem t3 (h : ¬ (P ∨ Q)) (hp : P) : False := h (Or.inl hp)

def ignores {α : Type} (n : Nat) : Nat := n
def uses : Nat := ignores 3
EOF

run_state() {
  local out="$1" mode="$2"
  SOKO_ARG_EXPECTED="$mode" "$BIN" query check --file "$WORK/b1.sokonanoda" \
    | python3 -c '
import json, sys
d = json.load(sys.stdin)["data"]
failed = sorted(f.get("start_line") or 0 for f in (d.get("failed") or []))
print(json.dumps({"checked": d["counts"]["decl_checked"], "failed_lines": failed}))
' > "$out"
}

run_state "$WORK/off.json" "0"
run_state "$WORK/on.json" "1"

python3 - "$WORK/off.json" "$WORK/on.json" <<'PY'
import json, sys

off = json.load(open(sys.argv[1]))
on = json.load(open(sys.argv[2]))
# 行号（见 heredoc）：P=1 Q=2 t=4 t2=5 t3=6 ignores=8 uses=9
T, T2, T3, USES = 4, 5, 6, 9
bad = []

print(f"  开关关：checked={off['checked']} 判红行={off['failed_lines']}")
print(f"  开关开：checked={on['checked']} 判红行={on['failed_lines']}")

if off["failed_lines"] != [T3, USES]:
    bad.append(f"① 开关关时必须只有 `t3` 与 `uses` 判红（缺口面 ✓），实际 {off['failed_lines']}")
if on["failed_lines"] != [USES]:
    bad.append(f"② 开关开时必须只有 `uses` 判红（`t3` 转绿 ✓），实际 {on['failed_lines']}")
if T in on["failed_lines"] or T in off["failed_lines"] or T2 in on["failed_lines"] or T2 in off["failed_lines"]:
    bad.append("③ 两个对照（期望位=显式目标 / 前导写全）两态都必须绿 ✗")
if USES not in on["failed_lines"]:
    bad.append("④ 反面 `ignores 3`（真解不出）在开关开时必须仍判红 ✗ —— 否则是「什么都放行」✗")
if off["checked"] != 5:
    bad.append(f"① 开关关时应判过 5 条（7 条声明 − t3 − uses ✓），实际 {off['checked']}")
if on["checked"] != 6:
    bad.append(f"② 开关开时应判过 6 条（t3 转绿 ✓，只剩 uses 红 ✓），实际 {on['checked']}")

if bad:
    print("✗ 判据不成立：", file=sys.stderr)
    for b in bad:
        print("  -", b, file=sys.stderr)
    sys.exit(1)
print("✓ 四条判据全过：开关关 `h (Or.inl hp)` 红 ✓ / 开关开转绿（`?B := Q` ✓）· 对照两态绿 ✓ · 反面仍红 ✓")
PY
