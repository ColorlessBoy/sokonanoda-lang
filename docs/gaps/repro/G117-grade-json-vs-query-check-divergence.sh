#!/usr/bin/env bash
# G-117 —— **同一次编译的两个视图读数不一致**：`query check` 与 `grade --json`
# 在"入口文件有 parse 错"时对**同一份输入**给出不同答案。
#
# ✅ **2026-10-15 已修**（本复现件自断言 ⇒ 现在 exit 1「已修」✓）：
# 修法 = `crates/cli/src/check_recovery.rs` —— 单文件回退改走 `Session::update`
# （与 `query check` 的 `QueryDoc::set_text` **同一个编译者** ⇒ 一致性是构造出来的 ✓）。
# ⚠ 台账（`docs/gaps/ledger.jsonl`）的关账由 `main` 侧做（红线 X2：共享台账只在
# `main` 上改 ✗）—— 在它落之前 `scripts/gap.py check` 会**如实判红**（复现件已翻成
# exit 1 = 已修，而台账还写 `open`）✓，那正是这套门禁设计的行为 ✓。
# 证据与完整读数：`.cache/ux/fix-ui1/VERDICT.md` §8.3（发现轮）·
# `.cache/ux/fix-g117/VERDICT.md`（收口轮）✓。
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
#   实测读数（**改前**）：
#     query check --file <夹具>  ⇒ counts = {decl_checked: 2, …}      ← 恢复式解析救回的
#                                                                      声明**被算上了** ✓
#     grade  --json <夹具>       ⇒ 只有 1 条 diagnostic，**零** decl.checked 事件 ✗
#   实测读数（**改后**）：
#     grade  --json <夹具>       ⇒ 2 条 decl.checked + 1 条 diagnostic · exit 1 ✓
#                                  （那一条诊断与改前**逐字节相同** · 事件流纯新增 2 行 ✓）
#
# 为什么这是缺口（**仓库自己的文档写着**）：`crates/cli/tests/query.rs` 的头注释 ——
#   "`query check` and `--json` must agree, because they are two views of the same
#    compile — a divergence would mean a second source of truth (the exact failure
#    mode this design exists to prevent)."
# ⇒ 这就是那个分叉 ✗。
#
# ⚠ **影响面（2026-10-15 用户更正，实测复核 ✓）**：**不是**编辑器 —— LSP 那条路走
# `QueryDoc`（会恢复），实测 `soko/stateAt` 在这份夹具上 `line 2`（`a` 的 `intro`）与
# `line 8`（`c` 的 `intro`）都答 `goals=1 · binders=1` ✓、`documentSymbol` = 2 ✓
# ⇒ **编辑器一直看得见好的声明** ✓。它影响的是 **`grade` 这一条 CLI 通道**
# （命令行 / agent / CI 用的那条）✗ —— 台账 `blocks` 里"编辑器 Infoview 看不到"
# 那句**不准确**，已在 `main` 侧更正 ✓。
#
# 根因（**已定性并已修**）：`crates/cli/src/check.rs:102-126` 的单文件回退分支在
# `parsed` 为 `Err(diag)` 时**直接印诊断就返回**，没有走恢复式解析；而 `query check`
# 那条路（`QueryDoc::set_text` → `Session::update`）会恢复 ⇒ **两个消费者各走一条路** ✗。
#   ⚠ 与 `SUB-parse-error-kills-whole-file`（已修）**不是**同一处：那次修的是
#   `project/graph.rs` 的闭包入口与 `judge.rs` 的合成前缀，**没动** CLI 的单文件回退。
#
# 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常
#
# ⚠ **零观测闸门**（2026-10-15 G-25 在 CI 上的实测教训）：若两个通道**都**什么都没
# 观测到（`query` 的 counts 全 0 **且** `grade` 无事件）⇒ 那是"我没看见"、不是
# "缺口还在" ⇒ **exit 2**，绝不许报 0 ✗。
#
# ⚠ **读数必须来自与 HEAD 同一份构建**（2026-10-15 实测踩到，`探针身份` 纪律）：
# 本件在 `target/debug/sokonanoda`（20:49）**早于** `bc2ab0ff`（21:39，修
# `前缀源码无法解析`）时读到 `query=1`（**改前**行为）⇒ 三档都不匹配 ⇒ exit 2 ✗；
# 同一份 HEAD 重新构建（22:04）后读到 `query=2` ⇒ **exit 0** ✓。
# ⇒ 看到 exit 2 时**先重建**（`cargo build -p sokonanoda-cli --bin sokonanoda`）再读，
# 别把它读成"形状异常" ✗ —— `gap.py` 对 exit 2 的口径正是「环境异常 ⇒ 修环境 ✓」。

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
