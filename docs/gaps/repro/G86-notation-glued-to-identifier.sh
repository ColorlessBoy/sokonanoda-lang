#!/usr/bin/env bash
# G-86 复现件：记法符号**粘连在标识符上**时不许抢走标识符 ✓
#
# **改前**（`5110ddeb` 之前）：本脚本 exit 1 ✗ —— `r''` 被切成 `r` + `''` ⇒ 含
# `infixr:80 " '' " => Set.image` 的文件一出现双撇号标识符就**解析失败** ✗，
# 于是 `lib/Prod` / `lib/Equiv` / `lib/Demo` 这些**库层入口**被判红 ✓
# （`elab-notation-unknown-target` + 下游 `elab-tactic-failed` ✓）。
# **改后**：exit 0 ✓。
#
# 用法：bash docs/gaps/repro/G86-notation-glued-to-identifier.sh
set -u

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SOKO="$ROOT/scripts/soko"
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
  echo "✗ G-86 仍在：粘连文件报了 $bad 条诊断（期望 0）" >&2
  exit 1
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
  echo "✗ 有空白写的记法被误伤（不该出现 unexpected-token）" >&2
  exit 1
fi
echo "✓ ② 有空白写的记法没被误伤"

# ③ 词法级判据（先红那条 ✓）：`r''` 必须是一个标识符，`f '' A` 仍断成记法 ✓。
if ! (cd "$ROOT" && timeout 900 cargo test -q -p sokonanoda-front --lib \
  token::tests::a_declared_symbol_glued_to_an_identifier_loses >/dev/null 2>&1); then
  echo "✗ 词法级判据没过（a_declared_symbol_glued_to_an_identifier_loses）" >&2
  exit 1
fi
echo "✓ ③ 词法级判据通过"

# ④ 课程侧：三个曾经判红的库层入口必须 0 条诊断 ✓。
for f in Prod Equiv Demo; do
  n="$(cd "$ROOT" && timeout 900 node "$SOKO" grade --json \
    "$ROOT/courses/set-theory/lib/$f.sokonanoda" 2>&1 | grep -c '"type":"diagnostic"')"
  if [ "$n" != "0" ]; then
    echo "✗ lib/$f 仍有 $n 条诊断（期望 0）" >&2
    exit 1
  fi
  echo "✓ ④ lib/$f 0 条诊断"
done

echo "G-86 复现件：全部通过 ✓"
