#!/usr/bin/env bash
# G-80 复现：**goal 文本里 λ 绑定的变量被标成 `unknown_ident`**（应为 `binder`）
#
# ── 缺口原文（2026-10-02 值守第 8 单转记，用户实测 ✗）──
#   G-78 修好之后，`query state` 在
#   `courses/set-theory/units/I.6/unit109-ordinal-rec.sokonanoda --line 67 --col 3`
#   仍剩 **8 个 `unknown_ident`**：goal 文本里 λ 绑定的 `w`/`hw`，以及
#   `Nat`/`Acc.intro`。分类器只认**上下文里的**绑元 ⇒ goal 文本内部 `fun (w : α) …`
#   引入的名字掉进 `unknown_ident` ✗ ⇒ Infoview 面板按"未知标识符"着色 ✗。
#
# ── 本复现件钉的**核心**（自足最小件，不依赖课程）──
#   `theorem t2 (α : Type) (a : α) : (fun (w : α) => Box.mk α w) a = mkf α a := by sorry`
#   ⇒ `w` 必须 `binder`（现在 **`unknown_ident`** ✗）；对照组：`Box.mk` 必须 `ctor_use` ✓、
#   `mkf` 必须 `def_use` ✓（这两条现在就对 ⇒ 判据不空转 ✓）。
#
# ⚠ **第二半（`Nat`/`Acc.intro`）另有一个前置缺陷**（本件不复现，记账在 notes 里）：
#   那一格的 goal 文本本身是**折错的** —— `Eq Nat A B`（类型实参写出来时）被显示折叠
#   成 `(Nat = A) B` ✗ ⇒ 文本已经不成句，分类器自然认不出 `Nat`/`Acc.intro`。
#   先修"等式折叠取错操作数"，这一半才有意义。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在   1 = 行为已变（已修）   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-80: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cat > "$WORK/m.sokonanoda" <<'EOF'
inductive Box (α : Type) : Type
ctor Box.mk (a : α) : Box α
end

def mkf (α : Type) (a : α) : Box α := Box.mk α a

theorem t2 (α : Type) (a : α) : (fun (w : α) => Box.mk α w) a = mkf α a := by
  sorry
EOF

out="$("$BIN" query state --file "$WORK/m.sokonanoda" --line 7 --col 1 2>&1)"
rc=$?
if [ "$rc" -ne 0 ]; then
  echo "G-80: query state 失败（rc=${rc}）⇒ 环境/形状变了：$(printf '%s' "$out" | head -c 120)" >&2
  exit 2
fi

# ⚠ 判据体用**文件**传数据（`python3 - <<'PY'` 会把 stdin 换成脚本本身 ✗ —— 实测踩过）
printf '%s' "$out" > "$WORK/state.json"
python3 - "$WORK/state.json" <<'PY'
import json, sys

raw = open(sys.argv[1]).read()
try:
    d = json.loads(raw)["data"]
except Exception:
    print(f"✗ 没拿到 JSON：{raw[:120]!r}", file=sys.stderr)
    sys.exit(2)

kinds = {}
for r in d.get("goal_runs") or []:
    k = r.get("kind")
    if k:
        kinds.setdefault(r.get("text"), set()).add(k)

print("  goal:", d.get("goal"))
for text, ks in kinds.items():
    print(f"    {text!r:14} → {sorted(ks)}")

bad = []
# **判据本体**：λ 绑定的 `w` 必须是 `binder`（不是 `unknown_ident`）
if "binder" not in kinds.get("w", set()):
    bad.append(f"`w`（λ 绑定）被标成 {sorted(kinds.get('w', [])) or '无 kind'}，应为 `binder`")
# **对照组**（现在就对 ⇒ 判据不空转 ✓）
if "ctor_use" not in kinds.get("Box.mk", set()):
    bad.append(f"`Box.mk`（构造子）被标成 {sorted(kinds.get('Box.mk', [])) or '无 kind'}，应为 `ctor_use`")
if "def_use" not in kinds.get("mkf", set()):
    bad.append(f"`mkf`（定义）被标成 {sorted(kinds.get('mkf', [])) or '无 kind'}，应为 `def_use`")

if bad:
    print("✗ 缺口仍在：", file=sys.stderr)
    for b in bad:
        print("   - " + b, file=sys.stderr)
    sys.exit(0)   # 0 = 缺口仍在（与台账 status=open 一致）
print("✓ λ 绑定名已按 `binder` 分类、对照组也都对 ⇒ 行为已变（回来关账）")
sys.exit(1)
PY
