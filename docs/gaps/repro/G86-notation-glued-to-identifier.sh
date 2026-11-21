#!/usr/bin/env bash
# G-86 复现件：记法符号**粘连在标识符上**时不许抢走标识符 ✓
#
# **退出码约定**（`scripts/gap.py` 的仓内约定 ✓，别写反 ✗）：
#   0 = **缺口仍在**（粘连仍抢走标识符 ⇒ 库层入口仍被判红 ✗）
#   1 = **已修**（全部判据通过 ✓）
#   2 = 环境或形状不对（`scripts/soko` 不在、课程仓缺失、**没有 cargo**…）
#
# **零 cargo**：词法级单测要 Rust 工具链。本机没有 cargo 时它不能把
# 「没跑到」写成「缺口仍在」（那是假读数）⇒ exit 2，交给有工具链的 CI
# 的 ledger 作业跑完。所以仓库检查先跑，cargo 那一步放最后。
#
# **改前**（`5110ddeb` 之前）⇒ exit **0** ✗：`r''` 被切成 `r` + `''` ⇒ 含
# `infixr:80 " '' " => Set.image` 的文件一出现双撇号标识符就**解析失败** ✗，
# 于是 `lib/Prod` / `lib/Equiv` / `lib/Demo` 这些**库层入口**被判红 ✓
# （`elab-notation-unknown-target` + 下游 `elab-tactic-failed` ✓）。
# **改后** ⇒ exit **1** ✓。
#
# 用法：bash docs/gaps/repro/G86-notation-glued-to-identifier.sh
set -u

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SOKO="$ROOT/scripts/soko"
[ -f "$SOKO" ] || { echo "环境不对：找不到 scripts/soko" >&2; exit 2; }
[ -f "$ROOT/courses/set-theory/lib/Prod.sokonanoda" ] || {
  echo "环境不对：课程仓缺失（可分开检出）" >&2
  exit 2
}
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# ① 端到端：记法 `''` 与双撇号标识符同处一个文件 ⇒ 必须 0 条诊断 ✓。
cat > "$TMP/glued.sokonanoda" <<'EOF'
-- 最小复现（G-86）：记法 `''`（像）与**双撇号标识符** `r''` 同处一个文件
infixr:80 " '' " => Set.image

def Set.image (A B : Type) (f : A → B) (s : A → Prop) : B → Prop := fun (b : B) => True

theorem t1 (r'' : Prop) : r'' → r'' := fun (h : r'') => h
EOF
bad="$(cd "$ROOT" && timeout 600 node "$SOKO" grade --json "$TMP/glued.sokonanoda" 2>&1 \
  | grep -c '"type":"diagnostic"')"
if [ "$bad" != "0" ]; then
  echo "缺口仍在 ✗：粘连文件报了 $bad 条诊断（期望 0）" >&2
  exit 0
fi
echo "✓ ① 粘连文件 0 条诊断"

# ② 对照：**有空白**的记法照旧生效 ✓（课程里的 `f '' A` 就是这么写的 ✓）。
cat > "$TMP/spaced.sokonanoda" <<'EOF'
infixr:80 " '' " => Set.image

def Set.image (A B : Type) (f : A → B) (s : A → Prop) : B → Prop := fun (b : B) => True

def f (x : Nat) : Nat := x
def A : Nat → Prop := fun (_ : Nat) => True
theorem t2 : (f '' A) 0 := True.intro
EOF
if (cd "$ROOT" && timeout 600 node "$SOKO" grade --json "$TMP/spaced.sokonanoda" 2>&1 \
  | grep -q '"code":"unexpected-token"'); then
  echo "缺口仍在 ✗：有空白写的记法被误伤（出现 unexpected-token）" >&2
  exit 0
fi
echo "✓ ② 有空白写的记法没被误伤"

# ③ 课程侧：三个曾经判红的库层入口必须 0 条诊断 ✓。
for f in Prod Equiv Demo; do
  n="$(cd "$ROOT" && timeout 900 node "$SOKO" grade --json \
    "$ROOT/courses/set-theory/lib/$f.sokonanoda" 2>&1 | grep -c '"type":"diagnostic"')"
  if [ "$n" != "0" ]; then
    echo "缺口仍在 ✗：lib/$f 有 $n 条诊断（期望 0）" >&2
    exit 0
  fi
  echo "✓ ③ lib/$f 0 条诊断"
done

# ④ 词法级判据（先红那条 ✓）：`r''` 必须是一个标识符，`f '' A` 仍断成记法 ✓。
# 没有 Rust 工具链 ⇒ 环境未知（exit 2），不猜缺口。
command -v cargo >/dev/null 2>&1 || {
  echo "G-86: 需要 cargo（Rust 工具链）跑词法级判据" >&2
  exit 2
}
out="$(cd "$ROOT" && timeout 900 cargo test -p sokonanoda-front --lib \
  token::tests::a_declared_symbol_glued_to_an_identifier_loses --locked 2>&1)"
rc=$?
if [ "$rc" -ne 0 ]; then
  echo "缺口仍在 ✗：词法级判据没过（a_declared_symbol_glued_to_an_identifier_loses）" >&2
  printf '%s\n' "$out" | grep -E "^test |panicked|assertion" | tail -4 >&2
  exit 0
fi
# 测试名漂移时 cargo 仍然 exit 0（0 条用例也算通过）⇒ 必须核对真的跑了 1 条。
if ! printf '%s\n' "$out" | grep -q "test result: ok. 1 passed"; then
  echo "G-86: 词法级判据没跑到 1 条用例（测试名漂移？）" >&2
  exit 2
fi
echo "✓ ④ 词法级判据通过（1 条用例）"

echo "G-86 已修 ✓（全部判据通过）"
exit 1
