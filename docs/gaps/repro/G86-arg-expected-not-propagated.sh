#!/usr/bin/env bash
# G-86 复现：**短写的隐式调用当实参时吃不到期望类型** ⇒ 未约束的隐式参数被"兄弟同形"兜底填错 ✗
#
# ── 缺口原文（课程线 `f5ada75a` 交来的最小复现 · 内核线逐点复核 ✓）──
#   axiom P : Prop
#   axiom Q : Prop
#   theorem t3 (h : ¬ (P ∨ Q)) (hp : P) : False := h (Or.inl hp)
#   报 `类型不匹配：期望 ((Or P) Q)，实际是 ((Or P) P)` ✗
#   ⇒ `Or.inl ?A ?B hp` 里 `hp` 只约束 `?A := P`，`?B` **没有实参可问** ✗，
#     于是落到「按兄弟同形兜底」⇒ `?B := ?A := P` ✗。
#
# ── 三点刻画（同一份源码、逐点判卷 ✓）──
#   | `t  (hp : P) : P ∨ Q := Or.inl hp`（**期望位 = 显式目标**）| 绿 ✓ |
#   | `t2 … := h (Or.inl P Q hp)`（前导写全，对照）              | 绿 ✓ |
#   | `t3 … := h (Or.inl hp)`（**期望位来自 `h` 的域**）         | **红 ✗** |
#
# ── 病根（实测钉死，**不在求解器** ✗）──
#   `SOKO_U2_TRACE` 逐次打印 `Or.inl` 的 `expected` 与解：
#     `expected=Some("P ∨ Q")` ⇒ `solved=["P","Q"]` ✓✓（**求解器本来就会** ✓）
#     `expected=None`          ⇒ `solved=["P","P"]` ✗✗（兜底填错 ✓）
#   ⇒ 差的是**期望类型有没有送到实参位** ✗：`elab.rs` 通用应用路径只对
#     `needs_expected_type(arg)`（零元记法 / 集合字面量 ✓）算期望类型，
#     **普通应用实参一律 `None`** ✗ ⇒ `Or.inl hp` 拿不到 `h` 的域 ✓。
#
# ── 归属（**不是 U1/U2** ✗，别含糊 ✓）──
#   这是设计 `docs/design/metavar-engine.md` **§2.10 范围 B 的 B1**
#   （「期望类型传播：实参位**逐层**拿期望类型」✓），并且正是 `elab.rs` 里
#   **parked 的 TODO(G-21)**（原文见 `docs/design/course-lean-style.md` §9）：
#   放宽那条闸当场**栈溢出**（本轮实测 exit 134 ✗ —— 算期望类型要 `judge_infer`
#   头的类型，判定**再入** elaborate ⇒ 递归 ✗）；加深度守卫后**不崩也不生效** ✗。
#   ⇒ 最小验证路径：① 给 `application_arg_expected` 加**递归守卫**（或在
#     `needs_expected_type` 处加"已在算"标记 ✓）；② 修「**显式实参写在隐式位上**」
#     的实参↔形参对齐（`Eq.subst.{1} (Set α) …` 那个 parked 回归 ✓）；
#     ③ 再开闸，判据 = 本件 exit 1 ✓ + 全语料对拍 ✓。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-86: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cat > "$WORK/g86.sokonanoda" <<'EOF'
axiom P : Prop
axiom Q : Prop

theorem t (hp : P) : P ∨ Q := Or.inl hp
theorem t2 (h : ¬ (P ∨ Q)) (hp : P) : False := h (Or.inl P Q hp)
theorem t3 (h : ¬ (P ∨ Q)) (hp : P) : False := h (Or.inl hp)
EOF

"$BIN" query check --file "$WORK/g86.sokonanoda" | python3 -c '
import json, sys
d = json.load(sys.stdin)["data"]
failed = sorted(f.get("start_line") or 0 for f in (d.get("failed") or []))
checked = d["counts"]["decl_checked"]
print(json.dumps({"checked": checked, "failed_lines": failed}))
' > "$WORK/out.json"

python3 - "$WORK/out.json" <<'PY'
import json, sys

r = json.load(open(sys.argv[1]))
# 行号（见 heredoc）：t=4 · t2=5 · t3=6
T, T2, T3 = 4, 5, 6
print(f"  decl.checked={r['checked']} · 判红行={r['failed_lines']}")

# 两个对照必须**始终**绿 ✓（它们绿而 t3 红，才说明病根在"期望位来自假设的域" ✓）
if T in r["failed_lines"] or T2 in r["failed_lines"]:
    print("✗ 对照（`Or.inl hp` 期望位=显式目标 / 前导写全）不该红 —— 形状变了，回来看看", file=sys.stderr)
    sys.exit(2)

if r["failed_lines"] == [T3] and r["checked"] == 4:
    print("✗ 缺口仍在：`h (Or.inl hp)` 判红（`?B` 被兄弟兜底填成 `?A` ✗）", file=sys.stderr)
    sys.exit(0)   # 0 = 缺口仍在（与台账 status=open 一致 ✓）
if not r["failed_lines"] and r["checked"] == 5:
    print("✓ 已修：`h (Or.inl hp)` 判绿（期望类型送到了实参位 ✓）")
    sys.exit(1)   # 1 = 行为已变（已修）
print(f"✗ 形状变了（期望 4+1红 或 5全绿）⇒ 回来看看", file=sys.stderr)
sys.exit(2)
PY
