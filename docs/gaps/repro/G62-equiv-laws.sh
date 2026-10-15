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
  # ⚠⚠ **2026-10-15 换探针**（洞 `GATE-FALSE-GREEN-11-MORE`）：`scripts/soko version` 是
  # **启动器自己**答的（**不转发给 CLI** ✗）⇒ 没有可执行 CLI 时也 rc=0 ⇒ 咬不住 ✗。
  # 改用 `--version`（**会被转发给 CLI** ✓）并要求**输出非空**（空转/拒跑 ⇒ 探针不过 ⇒ exit 2 ✓）。
  if [ -x scripts/soko ] && [ -n "$(scripts/soko --version 2>/dev/null)" ]; then BIN=scripts/soko
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
# ⭐ **零观测闸门**（2026-10-15，洞 `GATE-FALSE-GREEN-11-MORE`）：判据是"事件流里有没有
# `c2` 的 `decl.checked`"，而量具空转（零事件 / 零输出）时它**必然**是"没有" ⇒ 逐字掉进
# ①「缺口描述失真」⇒ exit 1 = **假绿** ✗。⇒ 先要求真的观测到**本夹具的声明事件**（可归属 ✓）
# 或**一条诊断**；两样都没有 ⇒ **exit 2 + 人话**。同族形状：G69 的「无事件行 ⇒ exit 2」。
if ! printf '%s' "$a_out" | grep -qE '"name":"c1","type"|"name":"c2","type"|"type":"diagnostic"'; then
  echo "G-62 ①：观测不到判卷事件（既没有 c1/c2 的声明事件，也没有诊断）⇒ 环境/形状异常（exit 2），不是「已修」✗" >&2
  echo "      · 这条判据量的就是 c2 的 decl.checked；一条事件都没有时它恒为「没有」⇒ 结论没有信息量 ✗" >&2
  echo "      · 常见成因：scripts/soko 拒跑（exit 3 / 无版本匹配构建）/ 量具空转 / 模块根不对" >&2
  printf '%s\n' "$a_out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        → /' >&2
  exit 2
fi
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
# ⭐ **零观测闸门**（同上）：② 的判据是"有没有 `kernel-expected-sort`"，零事件时它必然是
# "没有" ⇒ 逐字掉进 ②「行为已变，回来关账」⇒ exit 1 = **假绿** ✗。先要求事件/诊断。
if ! printf '%s' "$b_out" | grep -qE '"name":"[^"]+","type"|"type":"diagnostic"'; then
  echo "G-62 ②：观测不到判卷事件（既没有声明事件，也没有诊断）⇒ 环境/形状异常（exit 2），不是「已修」✗" >&2
  echo "      · 这条判据量的就是诊断码；一条事件都没有时它恒为「没有」⇒ 结论没有信息量 ✗" >&2
  printf '%s\n' "$b_out" | grep -v '^[[:space:]]*$' | head -3 | sed 's/^/        → /' >&2
  exit 2
fi
if ! printf '%s' "$b_out" | grep -q 'kernel-expected-sort'; then
  echo "G-62 ②：三条律不再被判红了 —— 行为已变，回来关账" >&2
  printf '%s\n' "$b_out" | head -3 >&2
  exit 1
fi

echo 'G-62：缺口仍在（`Cardinal` 核心 ✓，三条等价律判红 ✗）'
exit 0
