#!/usr/bin/env bash
# G-86 复现件：记法符号**粘连在标识符上**时不许抢走标识符
#
# 退出码（`scripts/gap.py` 约定）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状不对。
# 词法级单测要 cargo，本机没有就 exit 2；仓库检查先跑，cargo 那步放最后。
#
# 改前（`5110ddeb` 之前）`r''` 被切成 `r` + `''`，含 `infixr:80 " '' "` 的文件
# 一遇双撇号标识符即解析失败；改后 ⇒ exit 1。
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

# ① 记法 `''` 与双撇号标识符同处一个文件 ⇒ 0 条诊断。
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

# ② 有空白写的记法照旧生效（课程里的 `f '' A` 就是这么写的）。
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

# ③ 三个曾判红的库层入口 ⇒ 0 条诊断。
for f in Prod Equiv Demo; do
  n="$(cd "$ROOT" && timeout 900 node "$SOKO" grade --json \
    "$ROOT/courses/set-theory/lib/$f.sokonanoda" 2>&1 | grep -c '"type":"diagnostic"')"
  if [ "$n" != "0" ]; then
    echo "缺口仍在 ✗：lib/$f 有 $n 条诊断（期望 0）" >&2
    exit 0
  fi
  echo "✓ ③ lib/$f 0 条诊断"
done

# ④ 词法级判据：`r''` 是一个标识符，`f '' A` 仍断成记法；没有工具链就 exit 2。
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
# 测试名漂移时 cargo 仍 exit 0（0 条也算过）⇒ 核对真跑了 1 条。
if ! printf '%s\n' "$out" | grep -q "test result: ok. 1 passed"; then
  echo "G-86: 词法级判据没跑到 1 条用例（测试名漂移？）" >&2
  exit 2
fi
echo "✓ ④ 词法级判据通过（1 条用例）"

echo "G-86 已修 ✓（全部判据通过）"
exit 1
