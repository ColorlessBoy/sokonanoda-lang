#!/usr/bin/env bash
# G-62 自断言复现：**def 形态与展开形态不同一** ⇒ 三条等价律写不出来。
#
# 退出码约定（docs/gaps/README.md）：
#   0 = 缺口仍在（三条律都必须被拒）
#   1 = 行为变了（写得出来了 ⇒ 回来关账）
#   2 = 环境不满足
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

BIN="${SOKONANODA_BIN:-}"
if [ -z "$BIN" ]; then
  # **优先 `scripts/soko`**（harness 中立的启动器，AGENTS.md 的推荐形态）：
  # 受管环境里 `CARGO_TARGET_DIR` 指向别处 ⇒ `target/release/sokonanoda`
  # **永远不存在** ⇒ 这批复现件一律 exit 2，而 `gap.py check`（`soko gate` 与 CI
  # 的一步）把 exit 2 记成"环境异常" ⇒ **门禁在受管环境下必然判红** ✗
  # （2026-10-12 独立复核实测：23 个脚本硬编码路径 · 22 条判环境异常）。
  # ⚠ **先探测它真的能答**：`scripts/soko` 没有可信构建时 **exit 3 拒跑** ——
  # 若不探测就交给它，退出码会掉进"grep 不到复现标记 ⇒ 已修"的**假绿**路径 ✗
  # （同族隐患：干净缓存 + 无构建时 lang 探针会假报 FIXED）。探测不过就往下走旧路径 ✓。
  if [ -x scripts/soko ] && scripts/soko version >/dev/null 2>&1; then BIN=scripts/soko
  elif [ -x target/debug/sokonanoda ]; then BIN=target/debug/sokonanoda
  elif [ -x target/release/sokonanoda ]; then BIN=target/release/sokonanoda
  else echo "G-62: 找不到 sokonanoda 二进制（先 cargo build -p sokonanoda-cli）" >&2; exit 2; fi
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ① 对照组：`Cardinal` 本体**必须**过（缺口不影响核心）
cat > "$WORK/a.sokonanoda" <<'EOF'
import lib.Cardinal
def c1 : Cardinal := Cardinal.mk Nat
theorem c2 (α β : Type) (h : Type.Equiv α β) : Cardinal.mk α = Cardinal.mk β :=
  Cardinal.sound α β h
EOF
a_out="$("$BIN" --json --root courses/set-theory "$WORK/a.sokonanoda" 2>&1)"
if ! printf '%s' "$a_out" | grep -q '"name":"c2","type":"decl.checked"'; then
  echo "G-62 ①：对照组（Cardinal.sound）也不过 —— 缺口描述失真" >&2
  printf '%s\n' "$a_out" | head -3 >&2
  exit 1
fi

# ② 三条律：必须仍被拒
cat > "$WORK/b.sokonanoda" <<'EOF'
import lib.Cardinal
theorem Type.Equiv.refl' (α : Type) : Type.Equiv α α :=
  Iff.mpr (Type.Equiv.iff α α)
    (Exists.intro (α → α)
      (fun (f : α → α) => ∃ (g : α → α),
        (∀ (a : α), g (f a) = a) ∧ (∀ (b : α), f (g b) = b))
      (fun (a : α) => a)
      (Exists.intro (fun (a : α) => a)
        (And.intro
          (fun (a : α) => Eq.refl.{1} α a)
          (fun (b : α) => Eq.refl.{1} α b))))
EOF
b_out="$("$BIN" --json --root courses/set-theory "$WORK/b.sokonanoda" 2>&1)"
if ! printf '%s' "$b_out" | grep -q 'kernel-expected-sort'; then
  echo "G-62 ②：三条律不再被判红了 —— 行为已变，回来关账" >&2
  printf '%s\n' "$b_out" | head -3 >&2
  exit 1
fi

echo 'G-62：缺口仍在（`Cardinal` 核心 ✓，三条等价律判红 ✗）'
exit 0
