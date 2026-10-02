#!/usr/bin/env bash
# G-56 自断言复现：**`Acc`（良基性）立不起来 ⇒ 语言里没有良基递归**。
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在   1 = 行为已变（已修 / 形状变了，回来关账）   2 = 环境不满足
#
# ── 缺口原文（v0.77.0 · ST1 探针，2026-09-28）──
#   `inductive Acc (α : Type) (r : α → α → Prop) (x : α) : Prop`
#   + `ctor Acc.intro (h : ∀ (y : α), r y x → Acc α r y) : Acc α r x` ⇒ 内核拒。
#   当轮把原因记成「递归出现的下标 `y` 与块参数 `x` 不同 ⇒ 撞 uniform 检查」。
#
# ── **已修（G-56，0.81.0）· 但根因与台账原文不同** ──
#   实测把三件事分开了：
#   ① **下标写在块头**（`(x : α)` 是**参数**）⇒ 递归出现 `Acc α r y` 不是"按参数
#      uniform 套用" ⇒ uniform 检查判红。**这是对的，官方 Lean 同样拒**（Lean 的
#      `Acc` 把下标写在**返回位**：`inductive Acc (r) : α → Prop`）⇒ 这一形**不该放行** ✓；
#   ② **下标写在返回位**（Lean core 的官方写法）过了 uniform，但死在
#      `subst_expr_levels`（**G-64**，已单独修）⇒ 修好后换成**第三道门**：
#      `assert_nonnested_recursors_def_eq` —— 前端派生的 `Acc.rec` 与内核自己构造的
#      那个**宇宙参数个数不一致**（实测：期望 `motive … Sort(0)`、实际 `motive … Sort(u)`）；
#   ③ 第三道门的根因：前端镜像 `field_sort_via_kernel` 靠合成一份 `#check` 问内核
#      「这个字段类型是不是 Prop」，而那份合成源**看不见正在声明的块自己** ⇒
#      `h : ∀ (y : α), r y x → Acc α r y` 里的 `Acc` 解不出来 ⇒ 保守答 false ⇒
#      镜像说"不大消去"、内核说"大消去" ⇒ 两边不一致 ✗。
#   **修法**：`elab.rs::type_is_prop_by_source` —— 源码层补两条**内核一定会答 Prop**
#   的形状（蕴涵/全称链的**尾件**是 `Prop`、或**本块自己的名字**）✓。
#
# 本脚本断言**三件事**（修好后全部成立 ⇒ exit 1）：
#   ① `Acc`（**返回位**下标，官方写法）判绿、且真的能消去到大消去；
#   ② 对照组 `Even` 判绿；
#   ③ **反面**：下标写块头那一形**仍然**被 uniform 检查拒 —— 守卫的牙不许拔
#      （`Acc` 不是靠"放宽 uniform 检查"换来的 ✓）。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-56: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ① 缺口本体：**官方写法**（下标在返回位）—— 必须判绿，且要能真的用起来
cat > "$WORK/acc.sokonanoda" <<'EOF'
inductive Acc (α : Type) (r : α → α → Prop) : α → Prop
ctor Acc.intro (x : α) (h : ∀ (y : α), r y x → Acc α r y) : Acc α r x
end

#check Acc.rec

-- 良基归纳的**用法**：从 `Acc` 消去到 Type（大消去）
def accType (α : Type) (r : α → α → Prop) (x : α) (h : Acc α r x) : Type :=
  Acc.rec α r (fun (w : α) (_ : Acc α r w) => Type)
    (fun (w : α) (_h : ∀ (y : α), r y w → Acc α r y)
         (_ih : ∀ (y : α), r y w → Type) => Nat) x h
EOF
acc_out="$("$BIN" --json --no-project "$WORK/acc.sokonanoda" 2>&1)"
for name in Acc accType; do
  if ! printf '%s' "$acc_out" | grep -q "\"name\":\"$name\",\"type\":\"decl.checked\""; then
    echo "G-56 ①：\`$name\` 没有判绿 —— 缺口仍在（或形状变了）" >&2
    printf '%s\n' "$acc_out" | head -5 >&2
    exit 0
  fi
done
if ! printf '%s' "$acc_out" | grep -q '"text":"Acc.rec","type":"expr.typed"'; then
  echo "G-56 ①：\`#check Acc.rec\` 没有答上 —— 派生的递归子没进环境" >&2
  printf '%s\n' "$acc_out" | head -5 >&2
  exit 0
fi

# ② 对照组：同形状、下标**不变化**的归纳块必须能过（否则缺口描述失真）
cat > "$WORK/even.sokonanoda" <<'EOF'
inductive Even : Nat → Prop
ctor Even.zero : Even 0
ctor Even.succ (n : Nat) (h : Even n) : Even (Nat.succ (Nat.succ n))
end
EOF
even_out="$("$BIN" --json --no-project "$WORK/even.sokonanoda" 2>&1)"
if ! printf '%s' "$even_out" | grep -q '"type":"decl.checked"'; then
  echo "G-56 ②：对照组 \`Even : Nat → Prop\` 也不过了 —— 缺口描述失真" >&2
  printf '%s\n' "$even_out" | head -3 >&2
  exit 2
fi

# ③ **反面**：下标写**块头**那一形（`x` 是参数）**仍然**要被 uniform 检查拒 ——
#    递归出现 `Acc α r y` 不是"按参数 uniform 套用"，官方 Lean 同样拒 ✓。
#    ⚠ 这条是**守卫的牙**：谁哪天把 uniform 检查放宽了，这里立刻判红。
cat > "$WORK/header.sokonanoda" <<'EOF'
inductive Acc (α : Type) (r : α → α → Prop) (x : α) : Prop
ctor Acc.intro (h : ∀ (y : α), r y x → Acc α r y) : Acc α r x
end
EOF
header_out="$("$BIN" --json --no-project "$WORK/header.sokonanoda" 2>&1)"
if ! printf '%s' "$header_out" | grep -q 'inductive occurrence is not applied uniformly to the block parameters and universe levels'; then
  echo "G-56 ③：下标写块头那一形**不再**被 uniform 检查拒了 —— 守卫的牙没了，回来看" >&2
  printf '%s\n' "$header_out" | head -3 >&2
  exit 0
fi

echo 'G-56：症状消失（`Acc`（返回位下标，官方写法）✓ 且能大消去 ✓；对照组 Even ✓；反面：下标写块头那一形仍被 uniform 检查拒 ✓）'
exit 1
