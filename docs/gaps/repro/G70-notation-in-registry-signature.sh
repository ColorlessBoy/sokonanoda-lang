#!/usr/bin/env bash
# G-70：注册表 `signature` 里的**记法**让隐式插入整条不触发。
#
# `KnownName::Decl::signature` 存的是 `render_expr(ty)`，而 `implicit::telescope`
# 要用 `parse_expr_text`（**空记法表**）把它**回读**成望远镜 —— 记法节点回读不了
# （`∈` 是未声明符号）⇒ 解析失败 ⇒ 望远镜拿不到 ⇒ **隐式插入整条不触发** ✗
# ⇒ 实参落进隐式位：
#
#     theorem mem_univ {α : Type} (a : α) : a ∈ univ := …   -- 签名里带 `∈`
#     exact Set.mem_univ x
#     ⇒ 类型不匹配：期望 `Set.univ α x`，实际是 `(a : x) -> Set.mem a Set.univ`
#
# 修法：注册表 signature 走**判定路径**的去记法渲染（`decl_signature` =
# `render_expr(expand_notations(ty))`）⇒ 产物能被空记法表回读 ✓。
#
# 期望：缺口**仍在**时 exit 0；修好后 exit 1。
set -uo pipefail
cd "$(dirname "$0")/../../.." || exit 2
# **优先 `scripts/soko`**（受管环境里 `CARGO_TARGET_DIR` 指向别处 ⇒ 下面那些
# `target/{release,debug}` 路径永远不存在 ⇒ 本件一律 exit 2 = "环境异常"，
# 而 `gap.py check`（`soko gate`/CI 的一步）会因此判红 ✗ —— 2026-10-12 独立复核发现）。
# ⚠ 先探测它**真的能答**：无可信构建时 `scripts/soko` **exit 3 拒跑**，不探测就会
# 掉进"grep 不到复现标记 ⇒ 已修"的**假绿** ✗。
# ⚠⚠ **2026-10-15 换探针**（同族：`GATE-FALSE-GREEN-11-MORE`）：`scripts/soko version`
# 是**启动器自己**答的（**不转发给 CLI** ✗）⇒ 没有任何可执行 CLI 时也 **rc=0** ⇒ 探针
# 咬不住 ✗。改用 `--version`（**会被转发给 CLI** ✓）并要求**输出非空**：只有真的 exec
# 到二进制、且它答了话才算过（空转/拒跑 ⇒ 探针不过 ⇒ exit 2 ✓）。详见 G-43 的同类注释。
if [ -x scripts/soko ] && [ -n "$(scripts/soko --version 2>/dev/null)" ]; then BIN=scripts/soko
else BIN=./target/release/sokonanoda; fi
[ -x "$BIN" ] || { echo "G-70：先跑 cargo build --release -p sokonanoda-cli" >&2; exit 2; }

DIR=$(mktemp -d)
trap 'rm -rf "$DIR"' EXIT
cat > "$DIR/lib.sokonanoda" <<'EOF'
def Set (α : Type) : Type := α → Prop
namespace Set
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def univ {α : Type} : Set α := fun (x : α) => True
def subset {α : Type} (A B : Set α) : Prop := ∀ (x : α), A x → B x
end Set
infix:50 " ∈ " => Set.mem
infix:50 " ⊆ " => Set.subset
namespace Set
-- 签名里**带记法**（`a ∈ univ`）—— 这正是触发条件
theorem mem_univ {α : Type} (a : α) : a ∈ univ := True.intro
end Set
EOF
cat > "$DIR/main.sokonanoda" <<'EOF'
import lib

theorem t (α : Type) (A : Set α) : A ⊆ Set.univ α := by
  intro x
  intro hx
  exact Set.mem_univ x
EOF

out=$("$BIN" grade --root "$DIR" "$DIR/main.sokonanoda" 2>&1)

# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE`）：判据是"输出里有没有
# `"code"`"，而「已修」那一档要的正是"一条诊断都没有" ⇒ **量具空转（零事件 / 零输出）
# 时读数与「已修」逐字相同** ✗ = **假绿**。⇒ 先要求这次判卷真的观测到**本夹具的声明
# 事件**（可归属 ✓ —— 入口的 `t` 或库里的 `Set.mem_univ`）或**一条诊断**。
if ! printf '%s' "$out" | grep -qE '"name":"t","type"|"name":"Set\.mem_univ","type"|"type":"diagnostic"'; then
  echo "G-70：观测不到判卷事件（既没有 t / Set.mem_univ 的声明事件，也没有诊断）⇒ 环境/形状异常（exit 2），不是「已修」✗" >&2
  echo "      · 这条判据量的就是诊断码；一条事件都没有时 codes 恒为空 ⇒ 「已修」没有信息量 ✗" >&2
  echo "      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 夹具或模块根不对" >&2
  printf '%s\n' "$out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        → /' >&2
  exit 2
fi

if printf '%s' "$out" | grep -q '"code"'; then
  echo "G-70 仍在：签名里的记法让望远镜回读失败 ⇒ 隐式插入不触发" >&2
  printf '%s' "$out" | grep -oE '"message":"[^"]{0,84}' | head -2 >&2
  exit 0
fi
echo "G-70 已修：签名去记法后可回读，隐式插入正常 ✓" >&2
exit 1
