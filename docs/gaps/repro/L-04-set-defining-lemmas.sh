#!/usr/bin/env bash
# L-04 复现/守卫件：课程标准库缺 Set 的「定义展开」引理
#   （台账标题：subset_def / mem_union / mem_inter / mem_diff / mem_power_iff /
#     not_mem_empty / mem_singleton_iff；today 里另有 mem_singleton_self 与
#     **公理** Set.ext）。
#
# **审计结论（2026-10-07，收口轮）**：这条缺口**已不存在** ✓ —— 那批引理现在都在
# `courses/set-theory/lib/Set.sokonanoda` 里，且名字是 **Loogle 取证版**
# （`Set.notMem_empty` 大写 M · `Set.mem_sdiff` 不是 diff · `Set.mem_powerset_iff`
# 不是 power）；`Set.ext` 也**已落地为公理**（同文件，理由写在文件里：Mathlib 由
# propext + funext 推出，我们两者都没有）。`blocks`（卷 I 单元 1–4）不再被挡：
# 课程门禁 `251 目标 · 2158 checked · 913 open · 0 判负`（exit 0）。
# ⇒ 本文件是**修后形状的回归守卫**：缺口再回来（引理被删/改名、Set.ext 没了、
#    或名字退回试做稿的拼法）⇒ exit 0，逼台账重新开账。
#
# 三臂（每一臂都必须成立才算「已满足」）：
#   ① 结构：**10 条展开引理 + 1 条公理 `ext`** 在册，且名字逐条对上；
#      **防空转** —— 命中数必须等于期望数（不是"grep 没报错"）；
#   ② 可用：**最小夹具**（每条引理各用一次 + `Set.ext` 证一条最小集合等式）在
#      真判卷下 exit 0 · 11 checked · 0 diagnostic；
#   ③ 反向（**咬得住**）：把副本里的 `axiom ext` / `theorem mem_union` 改个名 ⇒
#      同一夹具必须判红且诊断**点名**那条标识符（守卫不许空转）。
#
# ⚠ 夹具跑在**临时模块根**里（`lib/` 是当次从课程库**复制**的）⇒ 不往课程树写
#   任何东西 ✓。⚠ 判卷用 `grade`（模块根 = 入口目录）；**不用** `grade --root` ——
#   `grade` 子命令把 `--root` 吃掉了（`crates/cli/src/env/mod.rs::grade` 硬编码
#   `root: None`，与 `AGENTS.md` 的写法不符 ⇒ 那是**另一笔**，本件不依赖它）。
#
# 退出码约定（`docs/gaps/README.md`）：0 = 缺口仍在 · 1 = 已满足（已修）· 2 = 环境/判据失效
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT" || exit 2

LIB="courses/set-theory/lib/Set.sokonanoda"
SOKO="$ROOT/scripts/soko"

command -v node >/dev/null 2>&1 || { echo "L-04: 需要 node（scripts/soko 是 Node 启动器）⇒ 环境不满足" >&2; exit 2; }
[ -f "$LIB" ] || { echo "L-04: 找不到 $LIB ⇒ 环境不满足" >&2; exit 2; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# ── ① 结构：在册 + 名字正确 ────────────────────────────────────────────────
# 10 条 = 标题那 7 个（按**真名**写）+ `today` 的 mem_singleton_self
# + 同族在册的 mem_empty_iff_false / mem_univ；逐条按**真名**断言。
REQUIRED="subset_def mem_empty_iff_false notMem_empty mem_univ mem_singleton_iff
mem_singleton_self mem_union mem_inter_iff mem_sdiff mem_powerset_iff"
# 试做稿用错、真名不同的三个拼法：库里**不许**出现（真名才是契约）。
LEGACY="not_mem_empty mem_power_iff mem_diff"

decls=$(grep -cE '^[[:space:]]*(def|theorem|axiom|inductive|abbrev)[[:space:]]' "$LIB")
found=0
missing=""
for name in $REQUIRED; do
  if grep -qE "^[[:space:]]*theorem[[:space:]]+${name}([[:space:]]|$)" "$LIB"; then
    found=$((found + 1))
  else
    missing="$missing $name"
  fi
done
legacy_hits=""
for name in $LEGACY; do
  if grep -qE "^[[:space:]]*(theorem|def|axiom|abbrev)[[:space:]]+${name}([[:space:]]|$)" "$LIB"; then
    legacy_hits="$legacy_hits $name"
  fi
done
ext_ok=0
grep -qE '^[[:space:]]*axiom[[:space:]]+ext([[:space:]]|$)' "$LIB" && ext_ok=1

echo "① 结构：${LIB} —— 声明总数 ${decls} · 真名命中 ${found} / 10 · axiom ext 在册：$([ "$ext_ok" = 1 ] && echo yes || echo NO)"
echo "   防空转：判据是「逐条命中数 == 期望数」（不是「grep 没报错」）⇒ 库空/文件缺 ⇒ 命中 0 ⇒ 走 exit 0 ✗"
if [ -n "$missing" ]; then
  echo "   ✗ 缺（或名字不对）：$missing" >&2
fi
if [ -n "$legacy_hits" ]; then
  echo "   ✗ 用了试做稿的旧拼法（真名见 docs/design/course-stdlib.md §3.1）：$legacy_hits" >&2
fi

if [ "$found" != 10 ] || [ "$ext_ok" != 1 ] || [ -n "$legacy_hits" ]; then
  echo "结论：L-04 缺口仍在（课程标准库缺 Set 的定义展开引理 / 名字不合 Loogle 取证版）⇒ exit 0" >&2
  exit 0
fi

# ── 夹具（每条引理各用一次；`Set.ext` 一条最小集合等式）────────────────────
cat > "$TMP/probe.sokonanoda" <<'SOKO'
import lib.Set

theorem l04_subset_def (α : Type) (A B : Set α) (h : A ⊆ B) (x : α) (hx : x ∈ A) : x ∈ B :=
  (fun (hu : ∀ (y : α), y ∈ A → y ∈ B) => hu x hx)
    ((fun (hv : A ⊆ B ↔ ∀ (y : α), y ∈ A → y ∈ B) => Iff.mp hv h) (Set.subset_def α A B))

theorem l04_mem_empty_iff_false (α : Type) (a : α) (h : a ∈ ∅) : False :=
  Iff.mp (Set.mem_empty_iff_false a) h

theorem l04_notMem_empty (α : Type) (a : α) : ¬ (a ∈ ∅) :=
  Set.notMem_empty a

theorem l04_mem_univ (α : Type) (a : α) : a ∈ Set.univ α :=
  Set.mem_univ a

theorem l04_mem_singleton_iff (α : Type) (a b : α) (h : a ∈ {b}) : a = b :=
  Iff.mp (Set.mem_singleton_iff a b) h

theorem l04_mem_singleton_self (α : Type) (a : α) : a ∈ {a} :=
  Set.mem_singleton_self a

theorem l04_mem_union (α : Type) (a : α) (A B : Set α) (h : a ∈ A ∪ B) : a ∈ A ∨ a ∈ B :=
  Iff.mp (Set.mem_union a A B) h

theorem l04_mem_inter_iff (α : Type) (a : α) (A B : Set α) (h : a ∈ A ∧ a ∈ B) : a ∈ A ∩ B :=
  Iff.mpr (Set.mem_inter_iff a A B) h

theorem l04_mem_sdiff (α : Type) (a : α) (A B : Set α) (h : a ∈ A \ B) : a ∈ A ∧ ¬ (a ∈ B) :=
  Iff.mp (Set.mem_sdiff a A B) h

theorem l04_mem_powerset_iff (α : Type) (A B : Set α) (h : B ⊆ A) : B ∈ 𝒫 A :=
  Iff.mpr (Set.mem_powerset_iff A B) h

theorem l04_ext (α : Type) (A B : Set α) (h : ∀ (x : α), x ∈ A ↔ x ∈ B) : A = B :=
  Set.ext A B h
SOKO

# 判卷一次、读数一次（`--json` 事件流：一行一个 JSON）。
grade_probe() { # $1 = 模块根
  ( cd "$ROOT" && timeout 300 node "$SOKO" grade "$1/probe.sokonanoda" 2>&1 )
}

# ── ② 可用：真判卷下判绿 ──────────────────────────────────────────────────
mkdir -p "$TMP/ok/lib"
cp "$ROOT"/courses/set-theory/lib/*.sokonanoda "$TMP/ok/lib/"
cp "$TMP/probe.sokonanoda" "$TMP/ok/probe.sokonanoda"
OUT="$(grade_probe "$TMP/ok")"
rc=$?
checked=$(printf '%s' "$OUT" | grep -c '"type":"decl.checked"')
diag=$(printf '%s' "$OUT" | grep -c '"type":"diagnostic"')
echo "② 可用：探针判卷 exit=$rc · decl.checked=${checked}（期望 11）· diagnostic=${diag}（期望 0）"

if [ "$rc" = 124 ]; then
  echo "结论：判卷超时（环境慢，非形状问题）⇒ exit 2" >&2
  exit 2
fi
if [ "$rc" != 0 ] || [ "$checked" != 11 ] || [ "$diag" != 0 ]; then
  echo "结论：L-04 缺口仍在（引理在册但**用不起来**：夹具判红）⇒ exit 0" >&2
  printf '%s\n' "$OUT" | grep -o '"message":"[^"]*"' | head -3 >&2
  exit 0
fi

# ── ③ 反向：改名 ⇒ 同一夹具必须判红且点名（守卫不许空转）─────────────────
# 只动**副本**；课程库一个字节都不改。
neg_arm() { # $1 = 标签 · $2 = sed 表达式 · $3 = 诊断里必须出现的名字
  local tag="$1" expr="$2" want="$3" dir="$TMP/neg-$1"
  mkdir -p "$dir/lib"
  cp "$ROOT"/courses/set-theory/lib/*.sokonanoda "$dir/lib/"
  sed "$expr" "$LIB" > "$dir/lib/Set.sokonanoda"
  cp "$TMP/probe.sokonanoda" "$dir/probe.sokonanoda"
  local out rc
  out="$(grade_probe "$dir")"
  rc=$?
  if [ "$rc" = 124 ]; then
    echo "   ✗ 反向臂 ${tag}：判卷超时（环境慢）⇒ 本臂未验证" >&2
    return 2
  fi
  if [ "$rc" = 0 ]; then
    echo "   ✗ 反向臂 ${tag}：改坏后**仍判绿** ⇒ 守卫空转（判据失效）" >&2
    return 2
  fi
  if ! printf '%s' "$out" | grep -q "$want"; then
    echo "   ✗ 反向臂 ${tag}：判红了，但诊断**没有点名** ${want} ⇒ 判据失效" >&2
    return 2
  fi
  echo "   反向臂 ${tag}：改坏 ⇒ exit=${rc} 且诊断点名 ${want} ✓"
  return 0
}

echo "③ 反向（副本改名，课程库不动）："
neg_arm ext 's/^axiom ext /axiom ext_renamed /' 'Set.ext' || exit 2
neg_arm union 's/^theorem mem_union /theorem mem_union_renamed /' 'Set.mem_union' || exit 2

echo "结论：L-04 已满足（10 条展开引理 + 公理 Set.ext 在册、名字照 Loogle 取证版、真判卷可用；反向臂咬得住）⇒ exit 1" >&2
exit 1
