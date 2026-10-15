#!/usr/bin/env bash
# G-117 —— **同一次编译的两个视图读数不一致**：`query check` 与 `grade --json`
# 在"入口文件有 parse 错"时对**同一份输入**给出不同答案。
#
# ⚠ **台账登记状态**：本条**尚未**写进 `docs/gaps/ledger.jsonl` ——
# 红线 X2 规定**共享台账只在 `main` 上改** ✗（`scripts/soko gate --fast` 会判红 ✓）。
# 本复现件与它的读数先落在分支里当证据 ✓；登记（一行 JSON）由 `main` 侧补 ✓。
# 证据与完整读数：`.cache/ux/fix-ui1/VERDICT.md` §8.3 ✓。
#
# 现场（2026-10-15，`AGENT-DOES-NOT-PRODUCE-USER-INPUTS` 的生成式夹具第一次跑起来
# 就撞上的 —— 不是人报的 ✓）：
#
#   夹具（三条声明：`a` 好 / `broken` **坏** / `c` 好）：
#     theorem a (p : Prop) : p → p := by
#       intro h
#       exact h
#
#     theorem broken (p : Prop) : := by      ← 缺类型，故意坏
#       intro h
#
#     theorem c (r : Prop) : r → r := by
#       intro h
#       exact h
#
#   实测读数：
#     query check --file <夹具>  ⇒ counts = {decl_checked: 2, …}      ← 恢复式解析救回的
#                                                                      声明**被算上了** ✓
#     grade  --json <夹具>       ⇒ 只有 1 条 diagnostic，**零** decl.checked 事件 ✗
#
# 为什么这是缺口（**仓库自己的文档写着**）：`crates/cli/tests/query.rs` 的头注释 ——
#   "`query check` and `--json` must agree, because they are two views of the same
#    compile — a divergence would mean a second source of truth (the exact failure
#    mode this design exists to prevent)."
# ⇒ 这就是那个分叉 ✗。而且用户 2026-10-15 亲报的形态正是「**声明列表不回来**」——
# 走 `grade` 通道的人（编辑器/`soko grade`）**看不到**那两条好声明 ✓。
#
# 根因线索（**已定性，未修** —— 这是本条目要交出去的东西）：
#   `crates/cli/src/check.rs:102-126` 的单文件回退分支在 `parsed` 为 `Err(diag)` 时
#   **直接印诊断就返回**，没有走恢复式解析；而 `query check` 那条路
#   （`QueryDoc::check()`）会恢复 ⇒ **两个消费者各走一条路** ✗。
#   ⚠ 与 `SUB-parse-error-kills-whole-file`（已修）**不是**同一处：那次修的是
#   `project/graph.rs` 的闭包入口与 `judge.rs` 的合成前缀，**没动** CLI 的单文件回退。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常
#
# ⚠ **零观测闸门**（2026-10-15 G-25 在 CI 上的实测教训）：若两个通道**都**什么都没
# 观测到（`query` 的 counts 全 0 **且** `grade` 无事件）⇒ 那是"我没看见"、不是
# "缺口还在" ⇒ **exit 2**，绝不许报 0 ✗。

set -u
cd "$(dirname "$0")/../../.." || exit 2

if [ -x "$PWD/target/debug/sokonanoda" ]; then
  SOKO="$PWD/target/debug/sokonanoda"
elif [ -x "$PWD/target/release/sokonanoda" ]; then
  SOKO="$PWD/target/release/sokonanoda"
else
  SOKO="$PWD/scripts/soko"
fi

TMP="$(mktemp -d)" || exit 2
trap 'rm -rf "$TMP"' EXIT

cat > "$TMP/Fixture.sokonanoda" <<'EOF'
theorem a (p : Prop) : p → p := by
  intro h
  exact h

theorem broken (p : Prop) : := by
  intro h

theorem c (r : Prop) : r → r := by
  intro h
  exact h
EOF

export SOKONANODA_NO_CACHE=1

# ① `query check` 的读数（**必须能答**，否则是环境问题）。
Q="$("$SOKO" query check --file "$TMP/Fixture.sokonanoda" 2>/dev/null)" || true
# ② `grade --json` 的事件流。
G="$("$SOKO" grade "$TMP/Fixture.sokonanoda" --json 2>/dev/null)" || true

python3 - "$Q" "$G" <<'PY'
import json, sys

raw_q, raw_g = sys.argv[1], sys.argv[2]

# ① query check 的 counts（答不出来 ⇒ 环境问题 exit 2）。
try:
    q = json.loads(raw_q)
    counts = q["data"]["counts"]
    q_checked = int(counts.get("decl_checked", 0)) + int(counts.get("example_checked", 0))
except Exception as exc:  # noqa: BLE001
    print(f"   → 读不出 `query check` 的 counts（{exc}）⇒ 环境异常")
    sys.exit(2)

# ② grade --json 的声明事件数（逐行 NDJSON）。
decl_events, diagnostics, bad_lines = 0, 0, 0
for line in raw_g.splitlines():
    line = line.strip()
    if not line:
        continue
    try:
        ev = json.loads(line)
    except Exception:  # noqa: BLE001
        bad_lines += 1
        continue
    kind = ev.get("type")
    if kind in ("decl.checked", "exercise.open", "example.checked"):
        decl_events += 1
    elif kind == "diagnostic":
        diagnostics += 1

print(f"   query check ⇒ decl_checked+example_checked = {q_checked}")
print(f"   grade --json ⇒ 声明事件 {decl_events} 条 · 诊断 {diagnostics} 条"
      f"（非 JSON 行 {bad_lines}）")

# ⚠ **零观测闸门**：两个通道都空 ⇒ "我没看见"，不是"缺口还在" ✗
if q_checked == 0 and decl_events == 0:
    print("   → **两个通道都是零观测** ⇒ 判不了（环境/夹具问题）⇒ exit 2")
    sys.exit(2)

# 坏命令**必须**留下诊断（阳性那一半两个通道都该有；没有 ⇒ 夹具没走到那条路）。
if diagnostics == 0:
    print("   → 坏命令**没有留下诊断** ⇒ 夹具没走到该走的路 ⇒ exit 2")
    sys.exit(2)

if q_checked >= 2 and decl_events == 0:
    print("   → ⭐ 缺口仍在：`query check` 算上了救回的声明，`grade --json` 一条都不发 ✗")
    sys.exit(0)

if decl_events >= 2:
    print("   → ✓ 已修：两个通道对同一份输入给出一致的声明读数 ✓")
    sys.exit(1)

print(f"   → 形状既不是「缺口仍在」也不是「已修」（query={q_checked} · grade={decl_events}）"
      " ⇒ exit 2")
sys.exit(2)
PY
