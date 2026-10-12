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

# ⚠ **读数必须可归属**（2026-10-12 独立复核发现的两条同族病，一次治齐）：
#
# ① **假绿**：`scripts/soko` 没有可信构建时会拒跑（exit 3）或回落缓存，而本类探针的
#    判定形态是「**grep 不到复现标记 ⇒ exit 1（已修）**」⇒ 拒跑的输出里当然没有那个
#    标记 ⇒ 掉进假绿。复核实测（未建任何二进制时）`r07` 打印 `FIXED: …` 并 exit 1，
#    而下方正是 `refusing to run an unverified sokonanoda binary` ✗。
# ② **判错二进制**：无自建构建时 `scripts/soko` 会解析到**用户缓存**——实测
#    `cli.source = cache(STALE: expected 0.90.0 darwin-arm64, found 0.83.0)`，
#    即**拿 0.83.0 的二进制去判 0.90.0 的仓库**，读数与仓库状态**毫无关系** ✗。
#
# ⇒ 预检两条：**(a) 启动器答得上** · **(b) 它给的是 `repo-build`（不是缓存）**。
#    任一不满足 ⇒ **exit 2 = 环境不满足**（`judge_rc` 把 2 判成 `error`、`gap.py` 判成
#    「环境异常」⇒ 它是**失败**，绝不会被读成"已修" ✓）。
#    ⚠ 这就是 `AGENTS.md`「探针读数必须带构建身份」的**机器化**：读不出来就不许判 ✓。
_SOKO_VER="$("$REPO/scripts/soko" version --json 2>/dev/null)"
if [ -z "$_SOKO_VER" ]; then
  echo "环境不满足：scripts/soko 答不上（无可信构建）⇒ 本件**无法判定**（exit 2）" >&2
  exit 2
fi
if ! printf '%s' "$_SOKO_VER" | grep -q '"source": *"repo-build"'; then
  echo "环境不满足：解析到的**不是仓库构建**（是缓存/扩展）⇒ 读数无法归属本仓库 ⇒ 本件**无法判定**（exit 2）" >&2
  printf '%s' "$_SOKO_VER" | head -c 200 >&2; echo >&2
  exit 2
fi

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

# ⚠ **判据收紧（2026-10-13 · FRONT-E）**：旧版只做**否定 grep**（"输出里没有
# `Sort(0)` / 没有占位废话"）⇒ 它**只证明坏消息不见了**，不证明"这条声明判过了" ✗
# —— 一条把声明判红的**新**坏消息（或干脆没判）照样能让旧判据 exit 1（假绿）。
# 实测（`main@d8904285` 的仓库构建）：四条变体今天**真的修好了**（M-03/M-04/M-06
# `decl_checked=1`、M-05 的消息两侧都是 `=` 记法）⇒ 判据改成**正面断言**
# （判过 / 消息两侧同形），只收紧、不放宽 ✓。
# 判据读数口径：`checked=<decl_checked> failed=<len(failed)>` + 消息原文。
probe() {
  python3 - "$REPO/scripts/soko" "$W/$1.sokonanoda" <<'PY'
import json, subprocess, sys
out = subprocess.run([sys.argv[1], "query", "check", "--file", sys.argv[2]],
                     capture_output=True, text=True).stdout
try:
    d = json.loads(out)
except Exception:
    print("PARSE-ERROR " + out[:200].replace("\n", " "))
    sys.exit(0)
data = d.get("data", {})
counts = data.get("counts", {})
failed = data.get("failed", [])
print("checked=%d failed=%d" % (counts.get("decl_checked", 0), len(failed)))
for f in failed:
    print("MSG " + f.get("code", "") + " | " + f.get("message", ""))
PY
}
r3=$(probe m03); r4=$(probe m04); r5=$(probe m05); r6=$(probe m06)
for r in "$r3" "$r4" "$r5" "$r6"; do
  case "$r" in PARSE-ERROR*) echo "环境不满足：query check 不是 JSON（$r）⇒ 无法判定"; exit 2;; esac
done

hit=0
# M-03 / M-04 / M-06：**必须判过**（checked=1 且没有 failed）——
# 旧现象分别是"报成宇宙层级错"/"占位废话"/"层级默认成 0"，三者都以**判红**收场。
grep -q '^checked=1 failed=0' <<<"$r3" || { echo "M-03 STILL REPRODUCES: \`Eq.{1} α a x\` 目标没判过"; hit=1; }
grep -q '^checked=1 failed=0' <<<"$r4" || { echo "M-04 STILL REPRODUCES: \`Eq α a x\` 目标没判过"; hit=1; }
grep -q '^checked=1 failed=0' <<<"$r6" || { echo "M-06 STILL REPRODUCES: \`@Eq α x a\` 目标没判过"; hit=1; }
# M-05：**必须判红**（这是真错），且消息两侧都写 `=` 记法（`a = x` / `x = a`）——
# 旧现象是实际侧漏成点名 `Eq x a`（期望/实际自相矛盾 ✗）。
if ! grep -q '^checked=0 failed=1' <<<"$r5"; then
  echo "M-05 STILL REPRODUCES: 反着的等式目标没判红"; hit=1
elif ! grep -q '期望 `a = x`' <<<"$r5" || ! grep -q '实际是 `x = a`' <<<"$r5"; then
  echo "M-05 STILL REPRODUCES: 消息两侧不是同一种记法"; hit=1
fi
[ "$hit" = 1 ] && { echo "（命中 ≥1 条；逐条原文见 .cache/ux/main-findings.md M-03…M-06）"
  echo "M-03: $r3"; echo "M-04: $r4"; echo "M-05: $r5"; echo "M-06: $r6"; exit 0; }
echo "FIXED: 四种变体都给了可读诊断（M-03/M-04/M-06 判过 · M-05 判红且两侧同记法）"
echo "M-05: $r5"; exit 1
