#!/usr/bin/env bash
# G-69：**delta 展开的实参对齐不认前导隐式前缀** ⇒ `intro` 派生的假设（点形式）
# 展开出胡说八道的类型。
#
# 背景：B2（课程库隐式化）把库签名从 `(α : Type)` 改成 `{α : Type}` 之后，**短写**
# （`And.left hx`）要在"假设的类型"上反解隐式参数，而假设的类型是 `intro` **派生**的
# ——它写成点形式 `Set.mem x (Set.inter A B)`（**没有**前导隐式实参，源级 AST 里只有
# 写出来的 2 个实参）。`solve_prefix` 的路线 ① 要 `unfold_to_inductive` 把它展开到
# `And` 归纳头，而 `spine.rs::unfold_one` 把实参**右对齐到 `DefInfo.params`（含隐式）**：
#
#     Set.mem 的形参 = [α, a, A]（3），实参 = [x, Set.inter A B]（2）
#     ⇒ offset = 1 ⇒ a := x, A := Set.inter A B ⇒ 体 `A a` ⇒ `(Set.inter A B) x` ✓
#     再展开一层：Set.inter 的形参 = [α, A, B]（3），实参 = [A, B, x]（3）
#     ⇒ α := A, A := B, B := x ⇒ **胡说八道** ✗
#
# ⇒ 路线 ① 解不出 ⇒ `elab-implicit-argument-unsolved` ⇒ `exact And.left hx` 判红 ✗。
# **对照**（同样 10 行、只差假设是**显式 binder** 还是 `intro` 派生）：显式 binder
# （`(hx : A x ∧ B x)`）**判绿** ✓ ⇒ 触发条件是「假设的类型是**点形式的应用**」✓。
#
# 期望：缺口**仍在**时 exit 0；修好后 exit 1。
#
# ⭐ **零观测闸门**（2026-10-15，洞 `G69-NO-EVENT-CHECK`）：判据是 `"code"` 的**条数**，
# 而「已修」那一档要的正是 `bad=0 good=0` ⇒ **量具空转（一条事件都不输出）时读数与
# 「已修」逐字相同** ✗ = **假绿**（守卫失效却不出声）。⇒ 先要求两次判卷都真的吐出
# 事件行；一条都没有 ⇒ **exit 2（环境/形状异常）+ 说清没观测到什么** ✓。
# 同族已修好的形状：`G25` 的 `observed===0 ⇒ exit 2`、`G27` 的
# `hits1==0 && compiled1==0 ⇒ exit 2`、`G34` 的 `calls==0 ⇒ exit 2`
# （那三条是**假红**方向，本条是**假绿**方向 —— 形状同、危险更大 ✗）。
set -uo pipefail
cd "$(dirname "$0")/../../.." || exit 2
# **优先 `scripts/soko`**（受管环境里 `CARGO_TARGET_DIR` 指向别处 ⇒ 下面那些
# `target/{release,debug}` 路径永远不存在 ⇒ 本件一律 exit 2 = "环境异常"，
# 而 `gap.py check`（`soko gate`/CI 的一步）会因此判红 ✗ —— 2026-10-12 独立复核发现）。
# ⚠ 先探测它**真的能答**：无可信构建时 `scripts/soko` **exit 3 拒跑**，不探测就会
# 掉进"grep 不到复现标记 ⇒ 已修"的**假绿** ✗。
if [ -x scripts/soko ] && scripts/soko version >/dev/null 2>&1; then BIN=scripts/soko
else BIN=./target/release/sokonanoda; fi
[ -x "$BIN" ] || { echo "G-69：先跑 cargo build --release -p sokonanoda-cli" >&2; exit 2; }

DIR=$(mktemp -d)
trap 'rm -rf "$DIR"' EXIT
cat > "$DIR/lib.sokonanoda" <<'EOF'
def Set (α : Type) : Type := α → Prop
namespace Set
def mem {α : Type} (a : α) (A : Set α) : Prop := A a
def inter {α : Type} (A B : Set α) : Set α := fun (x : α) => A x ∧ B x
def subset {α : Type} (A B : Set α) : Prop := ∀ (x : α), A x → B x
end Set
infix:50 " ∈ " => Set.mem
infix:50 " ⊆ " => Set.subset
infixl:70 " ∩ " => Set.inter
EOF
# ① 派生假设（`intro hx`）⇒ 应当判红（缺口仍在）
cat > "$DIR/derived.sokonanoda" <<'EOF'
import lib

theorem t (α : Type) (A B : Set α) : A ∩ B ⊆ A := by
  intro x
  intro hx
  exact And.left hx
EOF
# ② 显式 binder ⇒ 对照组，必须判绿
cat > "$DIR/explicit.sokonanoda" <<'EOF'
import lib

theorem t (α : Type) (A B : Set α) (x : α) (hx : A x ∧ B x) : A x :=
  And.left hx
EOF

# ⚠ 两份输出都**留全**：`bad`/`good` 只是其中的 `"code"` 条数，而"有没有事件"要
# 单独量（见下面的零观测闸门）✗ —— 旧写法把 `grade` 的输出直接喂给 `grep -c`，
# 量具空转时读数与「已修」**逐字相同** ✗。
bad_out=$("$BIN" grade --root "$DIR" "$DIR/derived.sokonanoda" 2>&1)
good_out=$("$BIN" grade --root "$DIR" "$DIR/explicit.sokonanoda" 2>&1)
bad=$(printf '%s\n' "$bad_out" | grep -c '"code"')
good=$(printf '%s\n' "$good_out" | grep -c '"code"')

# ⭐ **零观测 ⇒ exit 2**（不是 exit 1 ✗）：`grade` 的 JSON 事件流**每条都带 `"type"`**
# ⇒ 一条都没有 = 这次判卷**什么都没观测到**（量具空转 / 拒跑 / 夹具或模块根不对）
# ⇒ 此时 `bad=0 good=0` 是**空观测**，与「已修」不可区分 ⇒ **绝不报「已修」** ✗。
# 实测（改前，真环境）：worktree 里没有版本匹配的仓库构建 ⇒ `scripts/soko` 对过期
# 缓存**拒跑**（exit 3 + 人话，零事件）⇒ 本件报「已修」exit 1 ✗（假绿）。
bad_events=$(printf '%s\n' "$bad_out" | grep -c '"type"')
good_events=$(printf '%s\n' "$good_out" | grep -c '"type"')
if [ "$bad_events" -eq 0 ] || [ "$good_events" -eq 0 ]; then
  echo "G-69：观测不到判卷事件（derived=${bad_events} · explicit=${good_events} 条）⇒ 环境/形状异常（exit 2），不是「已修」✗" >&2
  echo "      · 这条判据量的就是「诊断条数」；一条事件都没有时 bad/good 恒为 0 ⇒ 「已修」没有信息量 ✗" >&2
  echo "      · 常见成因：scripts/soko 拒绝跑过期缓存（exit 3）/ 无版本匹配构建 / 夹具或模块根不对" >&2
  printf '%s\n' "$bad_out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        derived  → /' >&2
  printf '%s\n' "$good_out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        explicit → /' >&2
  exit 2
fi

if [ "$bad" != "0" ] && [ "$good" = "0" ]; then
  echo "G-69 仍在：派生假设的点形式展开错位 ⇒ 短写解不出隐式参数" >&2
  exit 0
fi
if [ "$bad" = "0" ] && [ "$good" = "0" ]; then
  echo "G-69 已修：派生假设也能解出隐式参数 ✓" >&2
  exit 1
fi
echo "G-69：形状异常（derived=$bad explicit=${good}）" >&2
exit 2
