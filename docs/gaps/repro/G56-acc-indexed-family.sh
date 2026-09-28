#!/usr/bin/env bash
# G-56 / G-64 复现件：**索引族**（`Acc`）—— 单构造子 + 指标位含递归出现。
#
# 缺口仍在 ⇒ 退出码 0；行为已变（能 checked）⇒ 1；环境错 ⇒ 2。
#
# 2026-09-28 实测（`kernel/acc-indexed-families` 分支起点）：判红原文
#   `kernel-rejected: assertion left == right failed (left: 0 / right: 1)`
# 机制（插桩拿到调用栈后定死）：
#   subst_expr_levels (expr.rs:383) ← assert_nonnested_recursors_def_eq (inductive.rs:1692)
#   ← check_inductive_declar；**精确行 = inductive.rs:1705** 的
#   `subst_expr_levels(old.info().ty, old.info().uparams, st.rec_uparams.unwrap())`
#   ⇒ `old.info().uparams` **空** vs `st.rec_uparams` **1 个** ⇒ 两个来源不一致。
# ⚠ 判红**不在** uniform 检查里：`Acc α r y` 的 args_rev.len()=3 > num_params=2
#   ⇒ `inductive.rs:168` 的卫为假 ⇒ 整支跳过（既不检查也不判红）。
set -u
cd "$(dirname "$0")/../../.."
bin="${SOKONANODA_BIN:-target/debug/sokonanoda}"
[ -x "$bin" ] || { echo "G-56：找不到二进制 $bin ⇒ 环境错" >&2; exit 2; }

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT

# ── 缺口本体：Acc（索引族）────────────────────────────────────────────
cat > "$tmp/acc.sokonanoda" <<'SOKO'
inductive Acc (α : Type) (r : α → α → Prop) : α → Prop
ctor intro (x : α) (h : ∀ (y : α), r y x → Acc α r y) : Acc α r x
end
SOKO
acc_out=$("$bin" --json --no-project "$tmp/acc.sokonanoda" 2>&1)
acc_ok=$(printf '%s' "$acc_out" | grep -c '"type":"decl.checked"')

# ── 对照组：**参数位**（无指标）—— 今天就该过，坏了说明退步 ──────────
cat > "$tmp/param.sokonanoda" <<'SOKO'
inductive Box (α : Type) : Prop
ctor mk (x : α) : Box α
end
SOKO
box_ok=$("$bin" --json --no-project "$tmp/param.sokonanoda" 2>&1 | grep -c '"type":"decl.checked"')

echo "对照组 Box（参数位，应过）: checked=$box_ok"
if [ "$box_ok" -lt 1 ]; then
  echo "G-56：⚠ 对照组 Box **退步了**（参数位归纳也不过了）⇒ 这不是本缺口的形态" >&2
  printf '%s\n' "$box_out" >&2
fi

if [ "$acc_ok" -ge 1 ]; then
  echo "G-56：**缺口已修** —— Acc 索引族能 checked 了 ✓"
  exit 1
fi
echo "G-56：缺口仍在（Acc 索引族仍判红：$(printf '%s' "$acc_out" | head -c 160)）"
exit 0
