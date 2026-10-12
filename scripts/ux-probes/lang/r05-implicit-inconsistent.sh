#!/usr/bin/env bash
# r05-implicit-inconsistent.sh —— 隐式实参「补不补得出来」毫无规律（用户的核心抱怨）
# 现象: 同一个文件里，`And.left h` / `Iff.mp h hp` / `Set.mem_singleton_iff x a` /
#       `And.intro ha (…)` 都**自动补出**隐式实参；而 `Eq.symm h` / `Eq.trans h1 h2`
#       补不出来（报 elab-implicit-argument-unsolved），`congrArg A h` 更糟——直接
#       掉进内核黑话。用户原话：「不知道要不要传入，不传入又会失败」。
# 期望: 同一套反解规则（或至少：报一条可读诊断说明为什么这条补不出来、该写什么）。
# 判据: exit 0 = 问题仍在（四类里既有自动成功又有失败）/ exit 1 = 行为已统一
set -uo pipefail
REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
# ⚠ **M-07：干净缓存纪律**（2026-10-12 实测咬过一次）—— 内建 prelude / 判定逻辑改了以后
# **只重建不清缓存**时，旧条目会继续答旧行为（同一二进制无 import 时 `#print And.left` = theorem、
# `import lib.Set` 时却 = def）⇒ 每条探针都必须在**新 mktemp 缓存 + 关两处产物**下取读数，
# 否则「缺口仍在」可能是**旧答案**（r04 就这么假红过一次：`apply` 结论校验已修好、探针仍报 exit 0）。
FRESH_CACHE=$(mktemp -d)
export SOKONANODA_CACHE_DIR="$FRESH_CACHE"
export SOKONANODA_NO_PROJECT_ARTIFACTS=1
export SOKONANODA_NO_MODULE_ARTIFACTS=1
trap 'rm -rf "$FRESH_CACHE"' EXIT
# 状态/夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏；干净缓存纪律见
# docs/ux-findings-2026-10-12.md「复核方法」。
W="$REPO/.cache/ux/lang/repro/.work"
mkdir -p "$W"
[ -f "$W/sokonanoda.toml" ] || printf 'name = "ux-lang-work"\n' > "$W/sokonanoda.toml"
[ -e "$W/lib" ] || ln -s "$REPO/courses/set-theory/lib" "$W/lib"
F="$REPO/.cache/ux/lang/repro/.work/r05.sokonanoda"
cat > "$F" <<'SOKO'
import lib.Logic
import lib.Set

theorem ok_and_left (p q : Prop) (h : p ∧ q) : p := And.left h
theorem ok_iff_mp (p q : Prop) (h : p ↔ q) (hp : p) : q := Iff.mp h hp
theorem ok_mem_sing (α : Type) (x a : α) (hx : x ∈ {a}) : x = a :=
  Iff.mp (x ∈ {a}) (x = a) (Set.mem_singleton_iff x a) hx
theorem bad_symm (α : Type) (a b : α) (h : a = b) : b = a := Eq.symm h
theorem bad_trans (α : Type) (a b c : α) (h1 : a = b) (h2 : b = c) : a = c := Eq.trans h1 h2
theorem bad_congr (α : Type) (A : Set α) (a b : α) (h : a = b) : A a = A b := congrArg A h
SOKO
out=$(timeout 300 "$REPO/scripts/soko" query check --file "$F" 2>&1); rc=$?
n_ok=$(grep -c 'ok_' <<<"$(grep -o '"name":"[^"]*"' <<<"$out")" || true)
if [ "$rc" -ne 0 ] && grep -q 'elab-implicit-argument-unsolved' <<<"$out" && grep -q '个绑元' <<<"$out"; then
  echo "STILL REPRODUCES: 同文件内隐式实参反解时好时坏（Eq.symm/Eq.trans 报缺参，congrArg 掉内核黑话）"
  echo "$out" | head -c 600; echo; exit 0
fi
echo "FIXED: 隐式实参行为已统一"; echo "$out" | head -c 400; exit 1
