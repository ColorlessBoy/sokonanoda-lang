#!/usr/bin/env bash
# G-64 自断言复现：**下标写在返回位**的带索引归纳（`Acc` —— Lean core 的官方写法）。
#
# 退出码约定（docs/gaps/README.md）：
#   0 = 缺口仍在   1 = 行为已变（已修 / 形状变了，回来关账）   2 = 环境不满足
#
# ── 缺口原文（v0.77.0 · 用户授权的 Acc 可行性探针 variant ②，2026-09-28）──
#   `inductive Acc (α : Type) (r : α → α → Prop) : α → Prop`
#   + `ctor Acc.intro (x : α) (h : ∀ (y : α), r y x → Acc α r y) : Acc α r x`
#   ⇒ `kernel-rejected` /「rejected: assertion `left == right` failed
#        left: 0 / right: 1」
#   **断言位置**：`crates/kernel/src/expr.rs::subst_expr_levels` 的第一行
#   —— `left: 0` = 目标常量的宇宙参数表为空；`right: 1` = 调用方要代入 1 个层级。
#
# ── **已修（G-64，0.81.0）** ──
#   `ks` 空 ⇒ **没有要代入的宇宙参数** ⇒ 原样返回；不再顺手断言
#   `ks.len() == vs.len()`（`vs` 多出来的层级**没有消费者**）。`ks` **非空**时的
#   长度断言**保留**（真正的个数不匹配仍判红 ⇒ 反向判据，见
#   `crates/kernel/tests/memory_api.rs::subst_expr_levels_tolerates_a_longer_vs_when_ks_is_empty`）。
#
#   ⚠ **下一道门（本轮也一起修了）**：修好本条之后 `Acc` 换成了
#   `assert_nonnested_recursors_def_eq` 的判红（前端派生的 `Acc.rec` 与内核自己
#   构造的那个宇宙参数个数不一致 —— 前端的 `field_sort_via_kernel` 合成 `#check`
#   看不见**正在声明的块自己** ⇒ 把 `h : ∀ y, r y x → Acc α r y` 误判成非 Prop）。
#   那条属 **G-56**（前端镜像），已单独修 ✓。
#
#   本脚本因此断言**三件事**：对照组 ✓、`Acc`（返回位下标）✓、`TC`（两次递归出现
#   都在指标位上）✓ —— 三条都判绿 ⇒ **症状消失** ⇒ exit 1 ✓。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-64: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ① 对照组：**不带索引**的 Prop 值归纳块**必须**过（证明不是"Prop 归纳块不能用"）
cat > "$WORK/a.sokonanoda" <<'EOF'
inductive Even : Nat → Prop
ctor Even.zero : Even 0
ctor Even.succ (n : Nat) (h : Even n) : Even (Nat.succ (Nat.succ n))
end
EOF
a_out="$("$BIN" --json --no-project "$WORK/a.sokonanoda" 2>&1)"
if ! printf '%s' "$a_out" | grep -q '"name":"Even","type":"decl.checked"'; then
  echo "G-64 ①：对照组（Even，下标不变）也不过 —— 缺口描述失真" >&2
  printf '%s\n' "$a_out" | head -3 >&2
  exit 2
fi

# ② 缺口本体：下标写返回位 + **真的消去到 Type**（大消去）—— 必须判绿
cat > "$WORK/b.sokonanoda" <<'EOF'
inductive Acc (α : Type) (r : α → α → Prop) : α → Prop
ctor Acc.intro (x : α) (h : ∀ (y : α), r y x → Acc α r y) : Acc α r x
end

def accType (α : Type) (r : α → α → Prop) (x : α) (h : Acc α r x) : Type :=
  Acc.rec α r (fun (w : α) (_ : Acc α r w) => Type)
    (fun (w : α) (_h : ∀ (y : α), r y w → Acc α r y)
         (_ih : ∀ (y : α), r y w → Type) => Nat) x h

#check Acc.rec
EOF
b_out="$("$BIN" --json --no-project "$WORK/b.sokonanoda" 2>&1)"
# 派生的 `Acc.rec` **不发** `decl.checked`（与 `And.rec`/`Nat.rec` 同一条：它是
# 随块一起登记的受信任声明）⇒ 用 `#check` 证明它真的在环境里且类型可打印 ✓。
if ! printf '%s' "$b_out" | grep -q '"text":"Acc.rec","type":"expr.typed"'; then
  echo "G-64 ②：\`#check Acc.rec\` 没有答上 —— 派生的递归子没进环境" >&2
  printf '%s\n' "$b_out" | head -5 >&2
  exit 0
fi
for name in Acc accType; do
  if ! printf '%s' "$b_out" | grep -q "\"name\":\"$name\",\"type\":\"decl.checked\""; then
    echo "G-64 ②：\`$name\` 没有判绿 —— 缺口仍在（或形状变了）" >&2
    printf '%s\n' "$b_out" | head -5 >&2
    exit 0
  fi
done
if printf '%s' "$b_out" | grep -q 'left: 0'; then
  echo "G-64 ②：还在报 \`left: 0\` —— 断言没修好" >&2
  printf '%s\n' "$b_out" | head -3 >&2
  exit 0
fi

# ③ **两次递归出现、下标都变**的带索引归纳（传递闭包，G-56 的 blocks 里第一条）
cat > "$WORK/c.sokonanoda" <<'EOF'
inductive TC (α : Type) (r : α → α → Prop) : α → α → Prop
ctor TC.step (a b : α) (h : r a b) : TC α r a b
ctor TC.trans (a b c : α) (h1 : TC α r a b) (h2 : TC α r b c) : TC α r a c
end
EOF
c_out="$("$BIN" --json --no-project "$WORK/c.sokonanoda" 2>&1)"
if ! printf '%s' "$c_out" | grep -q '"name":"TC","type":"decl.checked"'; then
  echo "G-64 ③：\`TC\`（两次递归出现都在指标位）没有判绿 —— 回来看看" >&2
  printf '%s\n' "$c_out" | head -5 >&2
  exit 0
fi

echo 'G-64：症状消失（Even ✓；`Acc`（返回位下标）✓ 且能大消去 ✓；`TC` ✓）'
exit 1
