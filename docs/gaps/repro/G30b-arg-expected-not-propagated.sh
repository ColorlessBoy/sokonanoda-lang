#!/usr/bin/env bash
# **G-30 的第二件复现**（`G30b` ✓）：**短写的隐式调用当实参时吃不到期望类型** ⇒
# 未约束的隐式参数被「兄弟同形」兜底填错 ✗
#
# ⚠ **编号来历**（2026-10-04 值守记账复核 ✓）：本条原登记为 **G-86**（10-03 07:55 · `39d9507b` ✓，
# open / painful ✓），但 10-04 内核线把**另一件事**（记法粘连抢走标识符）也登记成 G-86 ✗
# ⇒ 顶掉了它 ✗。经复核：本条与 **G-30**（期望类型不传播到嵌套应用的实参 ✓）是**同一道闸**
# —— 老 G-86 的 `where` 自己写着「只对零元记法/集合字面量算实参的期望类型 ✗，普通应用实参一律 `None` ✗」✓
# = G-30 的 `where` ✓ ⇒ **并入 G-30** ✓（原文与定位链全挂在 G-30 名下 ✓），本文件按 G-30 命名 ✓。
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
# ── 2026-10-06 收口（第 124 棒 ✓）────────────────────────────────────────
#   ③ 已完成 ✓：闸门 `SOKO_ARG_EXPECTED` **翻默认开** ⇒ 本件 **exit 0 → 1** ✓。
#   判据 = ① **默认档**：`h (Or.inl hp)` 判绿（`?B := Q` ✓）+ 两个对照仍绿 ✓；
#          ② **逃生门** `SOKO_ARG_EXPECTED=0`：**回到缺口**（`t3` 判红 ✓）—— 反向验证 ✓，
#             证明翻的是**默认值**、不是把判据拆掉 ✓；
#          ③ **判定中性**（在 `crates/front` 侧的记账里）：整本课程 `build --json`
#             （剔 `build.tick`/`build.progress`）**逐字节相同** ✓（50065 行 · diff 0 ✓）。
#   ⇒ 两态都在本脚本内断言 ✓（不再靠"调用者记得设环境变量" ✗ —— 那样默认值被翻反也照样绿 ✗）。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
#   ⚠ 本件现在是**双态**判据 ✓：**默认档必须已修**（否则 exit 0）**且**逃生门必须仍是缺口
#   （否则 exit 2 —— 说明判据被拆掉了 ✗）。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-30b: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
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

# 两态各跑一次 ✓：`default` = **显式摘掉**环境变量（量的就是默认值 ✓，用 `env -u` ✓）；
# `escape` = 显式 `SOKO_ARG_EXPECTED=0`（逃生门 ✓，必须回到缺口 ✓）。
# ⚠ **`env -u` 不是装饰** ✗：调用者的 shell 若设过 `SOKO_ARG_EXPECTED`，不摘就会**继承**它
# ⇒ 「默认档」量的是环境值而不是默认值 ✗（实测：`SOKO_ARG_EXPECTED=0 bash 本件` ⇒ 假「缺口仍在」✗）。
run_state() {
  local out="$1" mode="$2"
  if [ "$mode" = default ]; then
    env -u SOKO_ARG_EXPECTED "$BIN" query check --file "$WORK/g86.sokonanoda" > "$WORK/raw.json"
  else
    env SOKO_ARG_EXPECTED=0 "$BIN" query check --file "$WORK/g86.sokonanoda" > "$WORK/raw.json"
  fi
  python3 -c '
import json, sys
d = json.load(open(sys.argv[1]))["data"]
failed = sorted(f.get("start_line") or 0 for f in (d.get("failed") or []))
checked = d["counts"]["decl_checked"]
print(json.dumps({"checked": checked, "failed_lines": failed}))
' "$WORK/raw.json" > "$out"
}

run_state "$WORK/default.json" default
run_state "$WORK/escape.json" escape

python3 - "$WORK/default.json" "$WORK/escape.json" <<'PY'
import json, sys

d = json.load(open(sys.argv[1]))
e = json.load(open(sys.argv[2]))
# 行号（见 heredoc）：t=4 · t2=5 · t3=6
T, T2, T3 = 4, 5, 6
print(f"  默认档：checked={d['checked']} 判红行={d['failed_lines']}")
print(f"  逃生门：checked={e['checked']} 判红行={e['failed_lines']}")

# 两个对照**两态都**必须绿 ✓（它们绿而 t3 红，才说明病根在"期望位来自假设的域" ✓）
for tag, r in (("默认档", d), ("逃生门", e)):
    if T in r["failed_lines"] or T2 in r["failed_lines"]:
        print(f"✗ {tag}：对照（期望位=显式目标 / 前导写全）不该红 —— 形状变了，回来看看",
              file=sys.stderr)
        sys.exit(2)

# ① 默认档必须**已修**：`t3` 绿 ⇒ 5 条全过 ✓
if d["failed_lines"] == [T3] and d["checked"] == 4:
    print("✗ 缺口仍在：**默认档** `h (Or.inl hp)` 判红（`?B` 被兄弟兜底填成 `?A` ✗）"
          "—— ① 闸门默认值被翻回去了，**或** ② 调用者环境里设了 `SOKO_ARG_EXPECTED=0`"
          "（⚠ 本件用 `env -u` 摘过它 ⇒ ② 只在 `env -u` 失效/被改坏时才会命中）✓",
          file=sys.stderr)
    sys.exit(0)   # 0 = 缺口仍在（与台账 status 一致 ✓）
if d["failed_lines"] or d["checked"] != 5:
    print(f"✗ 默认档形状变了（期望 5 全绿）⇒ 回来看看：{d}", file=sys.stderr)
    sys.exit(2)

# ② 逃生门必须**仍是缺口**：判据没被拆掉 ✓（反向验证 ✓）
if e["failed_lines"] != [T3] or e["checked"] != 4:
    print("✗ 逃生门 `SOKO_ARG_EXPECTED=0` 没回到缺口 ⇒ 判据被拆掉了（不是翻默认值 ✗）"
          f"：{e}", file=sys.stderr)
    sys.exit(2)

print("✓ 已修（默认档 `h (Or.inl hp)` 判绿 · `?B := Q` ✓）· 两个对照两态绿 ✓ · "
      "逃生门 `=0` 回到缺口 ✓（翻的是默认值，不是拆判据 ✓）")
sys.exit(1)   # 1 = 行为已变（已修）
PY
