#!/usr/bin/env bash
# G-79 复现：**编译缓存把"源码没变但二进制变了"的修复静默挡掉**
#
# ── 缺口原文（2026-10-02 值守第 8 单，用户独立验证 ✗）──
#   代码是对的，用户还是看不到：`query state` 仍是旧的坏行为（`binders=[]`、goal 整句
#   `∀ …`）；**只删掉一条陈旧缓存**（`~/Library/Caches/sokonanoda/compiled/46a5b818a4239be3.json`，
#   含 `ordinal_rec_nat`、**0 处** `by_root`）⇒ 同一条命令立刻变对 ✓。
#   病根：源码没变 ⇒ digest 没变 ⇒ 命中；而那份 report 是**旧二进制**写的 ⇒
#   新字段 `DeclState.by_root` 走 `#[serde(default)]` ⇒ `None` ⇒ 静默退回旧行为 ✗。
#   ⇒ **`#[serde(default)]` 这个"兼容性善意"本身就是掩盖机制**：老缓存不报错、不 miss、
#   只给旧答案 ✗（实测全局 250+ 条里 **0 条**含 `by_root`，整库陈旧）。
#
# ── 修法（0.81.0 · 值守第 8 单）──
#   ① `report::REPORT_SHAPE`（报告形状版本）**进缓存键、进条目本身、进 `meta.json` 的
#      schema** ⇒ 形状一变**整库不命中** ✓；`CACHE_FORMAT` 3→4、`ARTIFACTS_FORMAT` 1→2。
#   ② **拆掉兜底**：`DeclState.by_root` / `CacheFile.project` / `ProjectReport.kernel_checks`
#      的 `#[serde(default)]` 全删 ⇒ 老条目**反序列化直接失败** ⇒ 当 miss 重编 ✓。
#   ③ 逃生门 `SOKO_CACHE_SHAPE=off`（**只给本复现件的反向验证用** ✗）：关掉形状校验 ⇒
#      旧形状条目会被当成命中 ⇒ 静默回放旧答案。
#
# ── 判据（本脚本 `--criteria`）──
#   在一个**临时项目**（`sokonanoda.toml` + `import`）里：① 查一次（写入产物缓存）⇒
#   答案必须是**新**的（`binders=[a,b,h]`、`goal=a ∧ a`）；② 把产物条目**毒化**成旧形状
#   （`shape=0` + 抹掉 `by_root` —— 等价于旧二进制写的那份 ✗）；③ 再查一次 ⇒
#   答案**必须还是新的**（形状不符 ⇒ miss ⇒ 重编/回落到全局缓存 ✓），**不许静默给旧答案** ✗。
#
# ── 反向验证（用户立的回测铁律）──
#   `SOKO_CACHE_SHAPE=off` 撤掉形状校验 ⇒ 同一份毒化条目被当成命中 ⇒ 判据必须**判红** ✓
#   （用仓库自己的 `scripts/expect-red.sh` 表达 ✓，随台账重放一起跑 ✓）。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-79: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# 临时**项目**（模块根 = WORK）：单文件只走全局缓存，项目才走模块根产物那条（实测 ✓）。
printf 'requires = "0.81.0"\n' > "$WORK/sokonanoda.toml"
printf 'def helper : Prop := True\n' > "$WORK/lib.sokonanoda"
cat > "$WORK/m.sokonanoda" <<'EOF'
import lib

theorem t (a b : Prop) (h : a) : a ∧ a := by
  sorry
EOF

# 判据：0 = 全过（= 已修）  1 = 判据不过（= 缺口仍在 / 被旧缓存挡住）
criteria() {
  python3 - "$BIN" "$WORK" <<'PY'
import glob, json, os, subprocess, sys

bin_, work = sys.argv[1], sys.argv[2]
entry_file = os.path.join(work, "m.sokonanoda")


# ⚠ `gap.py` 的 `clean_env()` 故意设了 `SOKONANODA_NO_PROJECT_ARTIFACTS`（把复现固定在
# 「只写全局缓存」语义）⇒ 本件测的正是**模块根产物**那条 ⇒ 自己把它摘掉 ✓
# （仓库约定：要测覆盖的脚本自己 inline 处理 ✓）。
ENV = {k: v for k, v in os.environ.items() if k != "SOKONANODA_NO_PROJECT_ARTIFACTS"}


def ask():
    out = subprocess.run(
        [bin_, "query", "state", "--file", entry_file, "--line", "3", "--col", "1"],
        capture_output=True, text=True, env=ENV,
    )
    try:
        return json.loads(out.stdout)["data"]
    except Exception:
        print(f"✗ query state 没给出 JSON：{(out.stdout or out.stderr)[:120]!r}", file=sys.stderr)
        sys.exit(1)


def verdict(d):
    """(是否新答案, 描述)"""
    binders = [b.get("name") for b in (d.get("binders") or [])]
    goal = d.get("goal")
    ok = binders == ["a", "b", "h"] and goal == "a ∧ a"
    return ok, f"binders={binders} goal={goal!r}"


# ① 先查一次：写入产物缓存（同时断言基线是对的）
ok, desc = verdict(ask())
if not ok:
    print(f"✗ 基线就不对（还没毒化）：{desc}", file=sys.stderr)
    sys.exit(1)
print(f"  ✓ 基线（未毒化）：{desc}")

# ② 毒化：把产物条目改成**旧形状**（等价于旧二进制写的那份）
entries = sorted(glob.glob(os.path.join(work, ".sokonanoda", "compiled", "*.json")))
if not entries:
    print("✗ 找不到模块根产物条目（.sokonanoda/compiled/*.json）⇒ 环境/形状变了", file=sys.stderr)
    sys.exit(2)
poisoned = 0
for path in entries:
    d = json.load(open(path))
    d["shape"] = 0                     # 旧形状版本
    for decl in d.get("report", {}).get("decls", []):
        decl.pop("by_root", None)      # 旧二进制写不出这个字段
    json.dump(d, open(path, "w"))
    poisoned += 1
print(f"  · 已毒化 {poisoned} 条产物条目（shape=0 + 抹掉 by_root）")

# ③ 再查一次：**不许**静默给旧答案
ok, desc = verdict(ask())
if ok:
    print(f"  ✓ 毒化后仍是新答案（形状不符 ⇒ miss ⇒ 重编/回落）：{desc}")
    sys.exit(0)
print(f"✗ 被旧缓存挡住了 —— 毒化后给出的是**旧答案**：{desc}", file=sys.stderr)
sys.exit(1)
PY
}

case "${1:-}" in
  --criteria) criteria; exit $? ;;
esac

if criteria; then
  echo "✓ G-79：旧形状缓存不再能挡住修复（形状版本进键 + 条目，兜底已拆）⇒ 已修"
  echo "  反向验证：关掉形状校验（SOKO_CACHE_SHAPE=off）后判据必须判红"
  scripts/expect-red.sh "撤掉形状校验（SOKO_CACHE_SHAPE=off）后 G-79 判据必须红" -- \
      env SOKO_CACHE_SHAPE=off bash "$0" --criteria || exit 2
  exit 1
else
  echo "✗ G-79：缺口仍在 —— 旧形状缓存能把修复静默挡掉" >&2
  exit 0
fi
