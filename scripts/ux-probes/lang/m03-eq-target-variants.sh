#!/usr/bin/env bash
# m03-eq-target-variants.sh —— 目标写成 `Eq` 的四种变体时，报错是**误导**或**废话**（主会话 M-03…M-06）
# 现象: 同一个「`Eq.symm hx` 隐参补不出」的失败，目标写法一变，报错就换一副面孔：
#       M-03 `: Eq.{1} α a x`      ⇒ kernel-rejected「期望 Sort(0)，实际是 Sort(1)」（像宇宙层级错）
#       M-04 `: Eq α a x`（无层级）⇒ 「…（值的类型也读不出来，检查它里面的名字）」（废话）
#       M-05 `: Eq α a x := by exact hx` ⇒ 「期望 `a = x`，实际是 `Eq x a`」（pp 丢隐参 ⇒ 自相矛盾）
#       M-06 `@Eq α x a` 在类型位   ⇒ 同 M-03 的 Sort(0)/Sort(1)（`@` 但不写层级 ⇒ 层级默认成 0）
# 对照: 同一语句把值写成 `Eq.symm.{1} α x a hx`（参数写全）⇒ 判绿（M-03 的 s4）。
# 期望: 至少说「`Eq.symm` 的隐式参数补不出来，请写全参数」，而不是宇宙层级错 / 空话。
# 判据: exit 0 = 问题仍在（三条误导/废话文案至少命中一条）/ exit 1 = 已给出可读诊断
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

cat > "$W/m03.sokonanoda" <<'SOKO'
theorem m03 (α : Type) (x a : α) (hx : x = a) : Eq.{1} α a x := Eq.symm hx
SOKO
cat > "$W/m04.sokonanoda" <<'SOKO'
theorem m04 (α : Type) (x a : α) (hx : x = a) : Eq α a x := by
  exact Eq.symm hx
SOKO
cat > "$W/m05.sokonanoda" <<'SOKO'
theorem m05 (α : Type) (x a : α) (hx : x = a) : Eq α a x := by
  exact hx
SOKO
cat > "$W/m06.sokonanoda" <<'SOKO'
theorem m06 (α : Type) (x a : α) (hx : @Eq α x a) : @Eq α a x := by
  exact Eq.symm hx
SOKO

run() { timeout 300 "$REPO/scripts/soko" query check --file "$W/$1.sokonanoda" 2>&1; }
o3=$(run m03); o4=$(run m04); o5=$(run m05); o6=$(run m06)

hit=0
grep -q 'Sort(0)' <<<"$o3" && { echo "M-03 STILL REPRODUCES: 隐参失败被报成宇宙层级错"; hit=1; }
grep -q '值的类型也读不出来' <<<"$o4" && { echo "M-04 STILL REPRODUCES: 诊断是占位废话"; hit=1; }
grep -q '实际是 `Eq x a`' <<<"$o5" && { echo "M-05 STILL REPRODUCES: pp 丢隐参 ⇒ 期望/实际自相矛盾"; hit=1; }
grep -q 'Sort(0)' <<<"$o6" && { echo "M-06 STILL REPRODUCES: \`@Eq\` 不写层级 ⇒ 层级默认成 0"; hit=1; }
[ "$hit" = 1 ] && { echo "（命中 ≥1 条；逐条原文见 .cache/ux/main-findings.md M-03…M-06）"; exit 0; }
echo "FIXED: 四种变体都给了可读诊断"; echo "$o3" | head -c 300; exit 1
