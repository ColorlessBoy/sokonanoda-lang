#!/usr/bin/env bash
# G-64 自断言复现：**下标写在返回位**的带索引归纳死在递归子的宇宙代入。
#
# 退出码约定（docs/gaps/README.md）：
#   0 = 缺口仍在（必须报 `left: 0` / `right: 1`）
#   1 = 行为变了（能立起来了 ⇒ 回来关账、把 ST7/ST9 的"卡在良基递归"划掉）
#   2 = 环境不满足
#
# 现场（v0.77.0 · 用户授权的 Acc 可行性探针 variant ②，2026-09-28）：
#   `inductive Acc (α : Type) (r : α → α → Prop) : α → Prop`（**下标在返回位**，
#   即 Lean core `src/Init/WF.lean` 的官方写法）+ `ctor Acc.intro (x : α) (h : …) : Acc α r x`
#   ⇒ `kernel-rejected` /「rejected: assertion `left == right` failed
#        left: 0
#       right: 1」。
#   **断言位置**（本轮用内核静默 hook 插桩 + Backtrace::force_capture 定死）：
#   `crates/kernel/src/expr.rs:383` 的
#       `assert_eq!(self.read_levels(ks).len(), self.read_levels(vs).len());`
#   —— `left: 0` = 目标常量的宇宙参数表为空；`right: 1` = 调用方要代入 1 个层级。
#   ⇒ 内核试图把「带 1 个宇宙参数」的代入用到「没有宇宙参数」的常量上。
#
# 与 G-56 的关系：**两道门**。块头写下标 ⇒ 撞 G-56（`check_ctor` 的 uniform 检查，更早）；
# 下标写返回位 ⇒ 过得了 uniform 与 SPEC0，但撞本条（递归子的宇宙代入，更晚）。
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
  exit 1
fi

# ② 缺口的形状：下标写返回位 ⇒ 必须仍报 left: 0 / right: 1
cat > "$WORK/b.sokonanoda" <<'EOF'
inductive Acc (α : Type) (r : α → α → Prop) : α → Prop
ctor Acc.intro (x : α) (h : ∀ (y : α), r y x → Acc α r y) : Acc α r x
end
EOF
b_out="$("$BIN" --json --no-project "$WORK/b.sokonanoda" 2>&1)"
if ! printf '%s' "$b_out" | grep -q 'left: 0'; then
  echo "G-64 ②：不再报 \`left: 0\` 了 —— 行为已变，回来关账" >&2
  printf '%s\n' "$b_out" | head -3 >&2
  exit 1
fi
if ! printf '%s' "$b_out" | grep -q 'right: 1'; then
  echo "G-64 ②：报的不再是 \`right: 1\` —— 形状变了，回来看一眼" >&2
  printf '%s\n' "$b_out" | head -3 >&2
  exit 1
fi

echo 'G-64：缺口仍在（Even ✓；Acc 下标写返回位 ⇒ `left: 0 / right: 1` ✗）'
exit 0

# ── 2026-09-28 探针（第 2 轮）记录：这条断言可以被安全打开，但下一道门是 G-59 ──
#
# 修法（实测有效，**未合入**，补丁存在 `docs/HANDOFF-0.77.md` 的记录里）：
#   `expr.rs` 的 `subst_expr_levels` 把 `ks.is_empty()` 这一支**单独提前返回**，
#   不再顺手断言 `ks.len() == vs.len()`（`ks` 空 ⇒ 没有可被替换的宇宙参数，
#   `vs` 多出来的层级**没有消费者**，多几个都不改变结果）。
#   改完实测：`left: 0 / right: 1` **消失** ✓，但 variant ② **换成另一条判红**：
#   「类型不匹配：期望 `Pi (α : Sort(1)), … Pi (motive : … , Sort(0)), …`，实际是 `…, Sort(2)`…」
#   ⇒ 递归子的 motive 被钉在 `Sort 0`（**G-59**：`large_elim_test` 对 `Acc` 这种
#     「非 Prop 字段恰好是指标」的单构造子 Prop 块**没放行大消去**）⇒
#   **G-64 只是第一道门，下一道是 G-59**；本版两条都不修。
