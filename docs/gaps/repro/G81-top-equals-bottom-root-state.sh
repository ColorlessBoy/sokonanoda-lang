#!/usr/bin/env bash
# G-81 复现：**「顶」与「底」必须一致** —— 根状态（Infoview 最上头）与声明卡片
# （`query goals`）对**全部** open 声明逐字相同。
#
# ── 缺口原文（2026-10-02 值守第 9 单，用户真机实测 ✗）──
#   `courses/set-theory/units/I.10/unit24-closures.sokonanoda` 的 `subset_reflClosure`：
#     顶 `query state` ⇒ `binders=[]` ⊢ `∀ (α : Type 0) (r : Rel α α) (a b : α), r a b → …`
#     底 `query goals` ⇒ `binders=[α,r]` ⊢ `(a : α) → (b : α) → r a b → …`
#   ⇒ **底对、顶错**。最小判定（同一个二进制、非缓存）：
#     `theorem (a b : Prop) : a → b → a := by` ⇒ 顶 `[a,b,_,_] ⊢ a` ✗ / 底 `[a,b] ⊢ a → b → a` ✓
#   触发条件 = **语句本身是函数/Π 类型**（`→` 或 `∀`），与 Prop/Type、行数、缓存都无关。
#
# ── 病根 ──
#   根状态原先按 `split_by_value(val)` 的 **λ 链全长** `n` 去 `peel_pi_layers(ty, n)` ✗ ——
#   而前端 lowering 会给**函数型语句**补 λ ⇒ λ 链**含语句自身的 Π 层** ⇒ `n` 偏大 ⇒
#   ① 多剥（目标的箭头全没、上下文多出匿名 `_`）；② 层数不够时 `peel_pi_layers` 返 `None`
#   ⇒ 退回 `ty_text` + 空 binders ⇒ 顶显示整句 `∀` ✗。
#
# ── 修法（0.81.0 · 值守第 9 单）──
#   ① **open 声明**：根状态**直接用 `open_goal` 的 `info`**（= 声明卡片那一路的同源数据）
#      ⇒ 顶 ≡ 底 **by construction** ✓（`walk.rs` 的三处 `PendingOp::OpenExercise`）；
#   ② 其余（失败 / 已证）：`decl_prefix_state()` **只剥源位 λ 链的绑元**（= 声明里写的
#      **具名绑元**），不再剥整条 ∀ 望远镜 ✓。
#
# ── 判据（本脚本 `--criteria`）──
#   扫 `courses/set-theory/units/**` 的**全部** open 声明（画布 + 解答），逐个断言
#     顶（`query state`，光标在声明起始行 = 第一条 tactic 之前）的
#     `binders`（名字 + 类型）与 `goal` **逐字等于** 底（`query goals`）✓。
#   规模参考：本仓 `units/**` 共 222 个 `.sokonanoda`，其中画布 111 个；语句是 Π 的
#   open 声明都要绿（**不许只修一种形状** ✗ —— 上一版"只探一行"的教训）。
#   另：`非 Π` 声明也在扫描范围内（防"一律不剥"的假修 ✓）；unit109 的 4 条 open 声明
#   单独点名断言进了扫描（防空转 ✓）。
#
# ── 反向验证（用户立的回测铁律）──
#   `SOKO_STATE_ROOT=legacy`（撤掉根状态修复的逃生门）⇒ 同一套判据必须**判红** ✓
#   （`scripts/expect-red.sh` 表达 ✓，随台账重放一起跑 ✓）。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-81: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi
[ -d courses/set-theory/units ] || { echo "G-81: 找不到 courses/set-theory/units" >&2; exit 2; }

# 判据：0 = 全过（= 已修）  1 = 有顶≠底（= 缺口仍在 / 被挡）
criteria() {
  python3 - "$BIN" "$ROOT" <<'PY'
import glob, json, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

bin_, root = sys.argv[1], sys.argv[2]
units = os.path.join(root, "courses", "set-theory", "units")
# **画布**（`units/**` 排除 `solutions/`）：与用户手工扫描的口径一致（~110 个文件）。
# 解答是完整证明、没有 open 声明 ⇒ 不扫（省一半时间 ⇒ CI 的复现超时 300s 之内 ✓）。
files = [
    f
    for f in sorted(glob.glob(os.path.join(units, "**", "*.sokonanoda"), recursive=True))
    if os.sep + "solutions" + os.sep not in f
]
if len(files) < 80:
    print(f"✗ 只扫到 {len(files)} 个画布文件（预期 >=80）⇒ 环境/形状变了", file=sys.stderr)
    sys.exit(2)


def run(args):
    out = subprocess.run([bin_] + args, capture_output=True, text=True)
    try:
        return json.loads(out.stdout)
    except Exception:
        return None


def line_of(raw, offset):
    """字节偏移 → 1 基行号。**必须先按 UTF-8 解码**：`start`/`end` 是**字节**偏移 ✗，
    而语料里有中文注释 ⇒ 按字符切会漂（实测把声明定位到注释行 ⇒ `outside-declarations` ✗）。"""
    return 1 + raw[:offset].decode("utf-8", "replace").count("\n")


def check_one(path):
    """一个画布 ⇒ (相对路径, [顶≠底 的说明…], 断言条数, Π 条数, {名字…})。"""
    goals = run(["query", "goals", "--file", path])
    if not goals:
        return path, [], 0, 0, set(), []
    entries = goals.get("data") or []
    if not entries:
        return path, [], 0, 0, set(), []
    raw = open(path, "rb").read()
    bad, checked, pi, names = [], 0, 0, set()
    skipped_empty = 0
    non_pi_seen = 0
    for g in entries:
        name, start = g.get("name"), g.get("start")
        if name is None or start is None:
            continue
        goal_text = g.get("goal") or ""
        if not goal_text:
            # **底没有 goal**（已闭合 / 该条只是列表里的占位）⇒ 没有可比对的"题面" ✓
            # （实测这类 10 条：顶是历史根状态、底是 null ⇒ 属**定义差异**，不是本条缺陷 ✗）
            skipped_empty += 1
            continue
        is_pi = goal_text.startswith("∀") or "→" in goal_text
        if not is_pi:
            # **对照**：非 Π 声明每 3 条取 1（防"一律不剥"的假修 ✓）。
            # 为什么不全查：`query state` 一次 ~6s CPU（判卷要重跑前缀 ✗）⇒ 全量 360 次
            # 查询在这台机器上要 ~9 分钟 ✗（实测超时）⇒ 按类别收敛，Π 类**全查** ✓
            # （那正是被修坏的那一类 ✓）。
            non_pi_seen += 1
            # unit109 的 4 条**永远查**（防空转 ✓：它们必须出现在断言里，不许被抽样甩掉）
            if non_pi_seen % 3 != 1 and "unit109" not in path:
                continue
        names.add(name)
        line = line_of(raw, start)    # 声明起始行 = 根状态的位置（第一条 tactic 之前同一格）
        st = run(["query", "state", "--file", path, "--line", str(line), "--col", "1"])
        if not st or not st.get("data"):
            bad.append(f"{os.path.relpath(path, root)}:{line} `{name}` ⇒ query state 没答上")
            continue
        d = st["data"]
        top = ([(b.get("name"), b.get("ty")) for b in (d.get("binders") or [])], d.get("goal"))
        bot = ([(b.get("name"), b.get("ty")) for b in (g.get("binders") or [])], g.get("goal"))
        checked += 1
        if (bot[1] or "").startswith("∀") or "→" in (bot[1] or ""):
            pi += 1
        if top != bot:
            bad.append(
                f"{os.path.relpath(path, root)}:{line} `{name}`\n"
                f"      顶 binders={[n for n, _ in top[0]]} goal={top[1]!r}\n"
                f"      底 binders={[n for n, _ in bot[0]]} goal={bot[1]!r}"
            )
    return path, bad, checked, pi, names, skipped_empty


# 8 路并行（每个文件一组子进程；结论按路径排序 ⇒ 输出稳定 ✓）
with ThreadPoolExecutor(max_workers=8) as ex:
    results = list(ex.map(check_one, files))

bad, checked, pi_checked, seen_unit109, skipped = [], 0, 0, set(), 0
for path, b, c, pq, names, sk in results:
    bad.extend(b)
    checked += c
    pi_checked += pq
    skipped += sk
    if "unit109" in path:
        seen_unit109 |= names

print(f"  · 扫描 {len(files)} 个画布 · 断言 {checked} 条 open 声明（其中语句含 Π 的 {pi_checked} 条；Π 类**全部**在断言内 ✓，非 Π 每 3 条取 1 作对照 ✓）；另有 {skipped} 条底无 goal ⇒ 跳过（已闭合/占位，无可比对题面 ✓）")
need = {"ordinal_rec_nat_eq", "ordinal_rec_nat_unique", "addsTo_zero", "addsTo_succ"}
missing = need - seen_unit109
if missing:
    bad.append(f"unit109 的 4 条 open 声明没被扫到：{sorted(missing)} ⇒ 判据空转 ✗")
else:
    print("  · unit109 的 4 条 open 声明都在扫描里 ✓（防空转）")

if bad:
    print(f"\n✗ 顶 ≠ 底（{len(bad)} 处）：", file=sys.stderr)
    for b in bad[:12]:
        print("   - " + b, file=sys.stderr)
    if len(bad) > 12:
        print(f"   … 还有 {len(bad) - 12} 处", file=sys.stderr)
    sys.exit(1)
print(f"  ✓ 全部 {checked} 条：顶 ≡ 底（binders 名字+类型 与 goal 逐字一致）")
PY
}

case "${1:-}" in
  --criteria) criteria; exit $? ;;
esac

if criteria; then
  echo "✓ G-81：全部 open 声明的顶 ≡ 底（根状态与声明卡片同源）⇒ 已修"
  echo "  反向验证：撤掉根状态修复（SOKO_STATE_ROOT=legacy）后判据必须判红"
  scripts/expect-red.sh "撤掉根状态修复（SOKO_STATE_ROOT=legacy）后 G-81 判据必须红" -- \
      env SOKO_STATE_ROOT=legacy bash "$0" --criteria || exit 2
  exit 1
else
  echo "✗ G-81：缺口仍在 —— 有 open 声明的顶 ≠ 底" >&2
  exit 0
fi
