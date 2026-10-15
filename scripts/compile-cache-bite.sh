#!/usr/bin/env bash
# 咬住证明 —— `scripts/compile-cache.py`（洞 `COMPILE-CACHE-GB-ENVS-NO-GUARD`）
#
# ## 为什么需要它（而不是台账里一条 rc 比较）
#
# 这条洞的形状是「**崩溃 / 静默 而不是判定**」：旧版在 `SOKO_CACHE_BUDGET_GB=abc` 上抛
# `ValueError`（rc=1 + 栈 ✗）；`SOKO_CACHE_RESERVE_GB=-5` 更坏 —— `max(0, …)` 把它**静默**
# 变成 **0 G** ⇒ `reclaim_target` = **判红线本身** ⇒ **`CACHE-GUARD-NO-HEADROOM` 复发**
# （而**没有任何读数会红** ✗）。⇒ **只比 rc 的咬住证明分不出这两者** ✗（判定与崩溃、静默与
# 判红都可能同为 rc=0/1）⇒ 这一条必须断言**形状 + 读数**：
#
#   ① **钉死的旧 rev** 在坏输入上**真的**崩溃（`ValueError`）/ 静默（余量 0 G、目标 = 判红线）
#      —— 这一半证明"那条坏输入**真实存在**、旧版真的没在判" ✓；
#   ② **工作区版**在同一条坏输入上 **rc=2 + 人话（点名变量 + 说清后果 + 给正解）+ 无 `Traceback`**
#      —— 这一半才是"**它咬住了**" ✓；
#   ③ **正对照**：合法值上**旧版与新版的读数逐字节相同**（`budget_bytes` / `reserve_bytes` /
#      `reclaim_target_bytes`）+ `set-budget 60` 写出的 registry **逐字节相同**
#      —— 防"把什么都判红"冒充咬住 ✗，也钉住"**不许收紧**"的另一面 ✓；
#   ④ **空转闸**：钉死的旧 rev 与工作区**逐字节相同** ⇒ **exit 3**（绝不冒充咬住 ✗ ——
#      pr1 实测踩过的坑：坏版本原写 `HEAD`，提交后 `HEAD` 就是修好的那一版 ⇒ 换版逐字节无效）。
#
# ## 用法 / 退出码
#
#   `pass` ⇒ 0 = 当前守卫的判据通道自检绿 / 非 0 = 自检红（原样透传）
#   `bite` ⇒ **1 = 咬住了**（①②③④ 全成立）· **0 = 没咬住**（⇒ 台账 C2 判红 ✓）· 3 = 空转/环境错
#
# ⚠ **零写入**：`env probe` 不写盘；`set-budget` 那两条把 `SOKO_COMPILE_CACHE_ROOT` 指到
#   `mktemp -d` 里（**真 `registry.json` 一字节不动** ✓）；夹具全在临时目录 ✓。
# ⚠ **不碰** `crates/**`、不碰真队列台账、不碰真编译缓存 ✓。
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
GUARD_REL="scripts/compile-cache.py"
GUARD="$ROOT/$GUARD_REL"
PY=${PYTHON:-python3}

# ⚠ **钉死一个固定旧 rev**（不许写 `HEAD` ✗，见文件头 ④）。挑 `12e7b4af` 而不是开工基线
# `1cd4aba0`：两者的 `scripts/compile-cache.py` **逐字节相同**（blob `a5e97be9` —— 本脚本的
# ④ 空转闸会当场核 ✓），而 `12e7b4af` 是 main **深处** 2026-10-15 的那一笔（**离 HEAD 149 笔**）
# ⇒ **land / rebase 之后仍然是祖先** ✓（只写当时的 tip，被 rebase 掉就可能不可达 ⇒ 换版会失败 ✗）。
# ⚠ **不能钉 `9e4a9c3a`**（g1 给 `check-lean4-alignment.py` 用的那个）：它是 2026-10-13，
# **那时 `reserve_bytes()` 还不存在**（2026-10-15 随 `CACHE-GUARD-NO-HEADROOM` 才加的）
# ⇒ 拿它换版会得到 `AttributeError: … has no attribute 'reserve_bytes'` ⇒ **证明咬错了对象** ✗
#（实测踩到过：本脚本第一版就钉的 `9e4a9c3a`，三条断言当场判红 ✓ —— 这就是"钉死旧 rev"
# 必须**逐条核 blob**、不能照抄别人的 rev 的理由）。
BAD_REV="12e7b4af892ef1fd8004573492c3a8f603ad782e"
BAD_REV_SHORT="12e7b4af"

die() { printf 'compile-cache-bite: %s\n' "$*" >&2; exit 3; }

[ -f "$GUARD" ] || die "找不到 $GUARD_REL"
command -v "$PY" >/dev/null 2>&1 || die "没有 python3（**判不了 ≠ 咬住** ✗）"
git -C "$ROOT" cat-file -e "$BAD_REV^{commit}" 2>/dev/null \
  || die "钉死的旧 rev \`$BAD_REV_SHORT\` 取不到（**不许拿 HEAD 顶替** ✗）"

mode=${1:-pass}
if [ "$mode" = "pass" ]; then
  # 正对照：守卫自己的判据通道（含真 CLI 的端到端那几条 ⇒ 它就是"守卫绿着"的读数 ✓）
  exec "$PY" "$GUARD" --selftest
fi
[ "$mode" = "bite" ] || die "用法：$0 [pass|bite]"

TMP=$(mktemp -d "${TMPDIR:-/tmp}/soko-cc-bite.XXXXXX") || die "mktemp 失败"
trap 'rm -rf "$TMP"' EXIT

# 旧版副本（**唯一**的换版动作；只在临时目录里落盘 ⇒ 工作区零写入 ✓）
git -C "$ROOT" show "$BAD_REV:$GUARD_REL" > "$TMP/old-guard.py" || die "取不出旧版文件"

# ── 空转闸（④）──────────────────────────────────────────────────────────────
if cmp -s "$TMP/old-guard.py" "$GUARD"; then
  printf 'compile-cache-bite: ⚠ **空转** —— 钉死旧版（%s）与工作区**逐字节相同**\n' \
    "$BAD_REV_SHORT" >&2
  printf '  ⇒ 换版**没有换到任何东西** ⇒ 绝不冒充咬住 ✗（pr1 的坑：坏版本写 `HEAD`）\n' >&2
  exit 3
fi

# ── 读数探针（**真代码路径**：import 那份守卫、调它的纯函数 ⇒ 不是把正则抄一遍 ✓）──
# ⚠ `reg` 是**夹具 registry**（`{"budget_gb": 60}`）⇒ **不读也不写真 `registry.json`** ✓。
cat > "$TMP/probe.py" <<'PY'
import importlib.util, json, sys
spec = importlib.util.spec_from_file_location("cc_bite_probe", sys.argv[1])
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
reg = {"budget_gb": 60}
out = {}
try:
    out["budget_bytes"] = m.budget_bytes(reg)
    out["reserve_bytes"] = m.reserve_bytes(reg)
    out["reclaim_target_bytes"] = m.reclaim_target_bytes(reg)
    out["headroom_bytes"] = out["budget_bytes"] - out["reclaim_target_bytes"]
    out["rc"] = 0
except Exception as exc:                      # noqa: BLE001 - 夹具要抓住"丢栈"本身
    out["rc"] = 1
    out["exc"] = f"{type(exc).__name__}: {exc}"
print(json.dumps(out, sort_keys=True, ensure_ascii=False))
PY

# ⚠ 两个旋钮一律先 `-u` 清掉 ⇒ 读数**可归属**到这一条证明（不吃外来的环境噪声 ✓）
probe() {  # $1 = 守卫文件 · $2.. = env 赋值
  local f="$1"; shift
  env -u SOKO_CACHE_BUDGET_GB -u SOKO_CACHE_RESERVE_GB "$@" "$PY" "$TMP/probe.py" "$f" 2>/dev/null
}
CLI_RC=0; CLI_OUT=""
run_cli() {  # $1 = 守卫文件 · $2.. = env 赋值（真 CLI：`env probe` ⇒ 零写入 ✓）
  local f="$1"; shift
  CLI_OUT=$(env -u SOKO_CACHE_BUDGET_GB -u SOKO_CACHE_RESERVE_GB "$@" "$PY" "$f" env probe 2>&1)
  CLI_RC=$?
}
SETBUD_RC=0; SETBUD_OUT=""
run_setbudget() {  # $1 = 守卫文件 · $2 = 值 · $3 = 夹具受管根
  SETBUD_OUT=$(env -u SOKO_CACHE_BUDGET_GB -u SOKO_CACHE_RESERVE_GB \
    "SOKO_COMPILE_CACHE_ROOT=$3" "$PY" "$1" set-budget "$2" 2>&1)
  SETBUD_RC=$?
}

OLD="$TMP/old-guard.py"
fail=0
ok()   { printf '  ✓ %s\n' "$*"; }
nope() { printf '  ✗ %s\n' "$*" >&2; fail=1; }

printf 'compile-cache-bite: 钉死旧版 `%s`（与工作区**不同** ⇒ 空转闸过 ✓）· 夹具 `%s`\n' \
  "$BAD_REV_SHORT" "$TMP"

# ── ① 旧版：坏输入上真的"崩溃 / 静默" ────────────────────────────────────────
# ⚠ 断言**逐片段**（`grep -F`），不靠 JSON 的键序 —— `sort_keys=True` 的键序与肉眼读的
# 顺序不同，写死顺序的 `case` 会假红（实测踩到过一次 ✓）。
has() { printf '%s' "$1" | grep -qF "$2"; }
old_budget_abc=$(probe "$OLD" "SOKO_CACHE_BUDGET_GB=abc")
if has "$old_budget_abc" '"exc": "ValueError'; then
  ok "①a 旧版 \`SOKO_CACHE_BUDGET_GB=abc\` ⇒ **崩溃**：$(printf '%s' "$old_budget_abc" | sed 's/.*"exc": "//; s/".*//')"
else
  nope "①a 旧版在 \`SOKO_CACHE_BUDGET_GB=abc\` 上**没有崩溃**（读数：${old_budget_abc}）⇒ 那条坏输入的形状变了 ⇒ 本条证明失效 ✗"
fi
old_reserve_abc=$(probe "$OLD" "SOKO_CACHE_RESERVE_GB=abc")
if has "$old_reserve_abc" '"exc": "ValueError'; then
  ok "①b 旧版 \`SOKO_CACHE_RESERVE_GB=abc\` ⇒ **崩溃**（同族崩溃形；⚠ 它只在 \`status\`/\`reclaim\` 那条路上被消费 ⇒ 这里用探针走到同一个纯函数 ✓）"
else
  nope "①b 旧版在 \`SOKO_CACHE_RESERVE_GB=abc\` 上**没有崩溃**（读数：${old_reserve_abc}）✗"
fi
old_reserve_m5=$(probe "$OLD" "SOKO_CACHE_RESERVE_GB=-5")
if has "$old_reserve_m5" '"reserve_bytes": 0' && has "$old_reserve_m5" '"reclaim_target_bytes": 64424509440' \
   && has "$old_reserve_m5" '"headroom_bytes": 0' && has "$old_reserve_m5" '"rc": 0'; then
  ok "①c 旧版 \`SOKO_CACHE_RESERVE_GB=-5\` ⇒ **静默**：reserve=0 B · reclaim_target=**64424509440**（= 判红线 60 G 本身）· headroom=0 · **rc=0（没有任何读数会红）** ✗"
else
  nope "①c 旧版 \`SOKO_CACHE_RESERVE_GB=-5\` 的静默形状变了（读数：${old_reserve_m5}）✗"
fi
old_budget_m5=$(probe "$OLD" "SOKO_CACHE_BUDGET_GB=-5")
if has "$old_budget_m5" '"budget_bytes": -5368709120'; then
  ok "①d 旧版 \`SOKO_CACHE_BUDGET_GB=-5\` ⇒ 预算 **-5368709120**（负数 ⇒ 每棵树都「超预算」= 判红线永远红；**响亮但不是判据** ✗）"
else
  nope "①d 旧版 \`SOKO_CACHE_BUDGET_GB=-5\` 的读数变了（${old_budget_m5}）✗"
fi
run_setbudget "$OLD" abc "$TMP/old-cache-a"
if [ "$SETBUD_RC" -ne 0 ] && printf '%s' "$SETBUD_OUT" | grep -q Traceback; then
  ok "①e 旧版 \`set-budget abc\` ⇒ **Traceback + rc=$SETBUD_RC**（同族崩溃形 ✗）"
else
  nope "①e 旧版 \`set-budget abc\` **没有丢栈**（rc=${SETBUD_RC}）⇒ 那条形状变了 ✗"
fi

# ── ② 工作区版：同一条坏输入 ⇒ **判定**（rc=2 + 人话 + 无栈）─────────────────
check_bite() {  # $1 = 说明 · $2 = 期望人话关键字 · $3.. = env 赋值
  local desc="$1" want="$2"; shift 2
  run_cli "$GUARD" "$@"
  if [ "$CLI_RC" -ne 2 ]; then
    nope "② $desc ⇒ rc=${CLI_RC}（期望 **2**）· 输出尾：$(printf '%s' "$CLI_OUT" | tail -1 | cut -c1-120)"
  elif printf '%s' "$CLI_OUT" | grep -q Traceback; then
    nope "② $desc ⇒ rc=2 但**丢栈** ✗（判据是「判定」不是「崩溃」）"
  elif ! printf '%s' "$CLI_OUT" | grep -q "$want"; then
    nope "② $desc ⇒ rc=2 但人话没点名 \`$want\`（尾：$(printf '%s' "$CLI_OUT" | tail -1 | cut -c1-120)）"
  elif ! printf '%s' "$CLI_OUT" | grep -q '正解'; then
    nope "② $desc ⇒ 人话没给**正解**（尾：$(printf '%s' "$CLI_OUT" | tail -1 | cut -c1-120)）"
  else
    ok "② $desc ⇒ **rc=2** + 人话（点名变量/后果/正解），无栈 ✓"
  fi
}
check_bite "\`SOKO_CACHE_BUDGET_GB=abc\`（旧：栈 + rc=1 ✗）" "不是数字" "SOKO_CACHE_BUDGET_GB=abc"
check_bite "\`SOKO_CACHE_BUDGET_GB=-5\`（旧：判红线永远红 ✗）" "正数" "SOKO_CACHE_BUDGET_GB=-5"
check_bite "\`SOKO_CACHE_RESERVE_GB=abc\`（旧：栈 ✗）" "不是数字" "SOKO_CACHE_RESERVE_GB=abc"
check_bite "\`SOKO_CACHE_RESERVE_GB=-5\`（旧：**rc=0 静默** ⇒ 余量 0 G ✗）" "正数" "SOKO_CACHE_RESERVE_GB=-5"
check_bite "\`SOKO_CACHE_RESERVE_GB=0\`（余量 0 G = 那个 bug 的状态本身 ✗）" "正数" "SOKO_CACHE_RESERVE_GB=0"
check_bite "\`SOKO_CACHE_BUDGET_GB=nan\`（旧：\`int(nan)\` 丢栈 ✗）" "有限" "SOKO_CACHE_BUDGET_GB=nan"

# `set-budget`：坏值 ⇒ rc=2 + **一个字节都不写** ✓
for bad in abc -5 0; do
  rm -rf "$TMP/new-cache-bad"; mkdir -p "$TMP/new-cache-bad"
  run_setbudget "$GUARD" "$bad" "$TMP/new-cache-bad"
  if [ "$SETBUD_RC" -ne 2 ]; then
    nope "② \`set-budget $bad\` ⇒ rc=${SETBUD_RC}（期望 2）· 尾：$(printf '%s' "$SETBUD_OUT" | tail -1 | cut -c1-120)"
  elif printf '%s' "$SETBUD_OUT" | grep -q Traceback; then
    nope "② \`set-budget $bad\` ⇒ rc=2 但**丢栈** ✗"
  elif [ -e "$TMP/new-cache-bad/registry.json" ]; then
    nope "② \`set-budget $bad\` 被拒了**却写了 registry** ✗（零写入是判据的一半）"
  else
    ok "② \`set-budget $bad\` ⇒ rc=2 + 人话，且 **registry 一字节没写** ✓"
  fi
done

# ── ③ 正对照：合法值 ⇒ 旧版与新版的**读数逐字节相同**（不许收紧 ✗）────────────
# 逐值覆盖：不设 · `200`（CI 口径）· `60` · `0.5` · `" 200"` / `"+200"`（旧写法不许收紧 ✗）·
# `0.000000001`（**既有判据在用** ✓）· 余量 `2` / `" 2"` / `"+2"`。
ctl_bad=0; ctl_seen=""
for kv in "" "SOKO_CACHE_BUDGET_GB=200" "SOKO_CACHE_BUDGET_GB=60" "SOKO_CACHE_BUDGET_GB=0.5" \
          "SOKO_CACHE_BUDGET_GB= 200" "SOKO_CACHE_BUDGET_GB=+200" "SOKO_CACHE_BUDGET_GB=0.000000001" \
          "SOKO_CACHE_RESERVE_GB=2" "SOKO_CACHE_RESERVE_GB= 2" "SOKO_CACHE_RESERVE_GB=+2"; do
  if [ -z "$kv" ]; then
    o=$(probe "$OLD"); n=$(probe "$GUARD"); ctl_seen="$ctl_seen · （不设）"
  else
    o=$(probe "$OLD" "$kv"); n=$(probe "$GUARD" "$kv"); ctl_seen="$ctl_seen · \`${kv#SOKO_CACHE_}\`"
  fi
  if [ "$o" != "$n" ]; then
    nope "③ 正对照 \`${kv:-（不设）}\` 的读数**变了** ✗：旧=$o 新=${n}（**不许收紧** ✗）"
    ctl_bad=1
  fi
  # ⚠ 必须**整条**当一个参数传（`${kv:+"$kv"}` 会被词分割 ⇒ `" 200"` 那条裂成两个词 ✗ 实测踩到）
  if [ -z "$kv" ]; then run_cli "$GUARD"; else run_cli "$GUARD" "$kv"; fi
  [ "$CLI_RC" -eq 0 ] || { nope "③ 正对照 \`${kv:-（不设）}\` 的 CLI rc=${CLI_RC}（期望 0）✗"; ctl_bad=1; }
done
[ "$ctl_bad" -eq 0 ] && ok "③ 正对照 10 个合法值：旧/新**读数逐字节相同** + CLI rc=0 ✓（${ctl_seen# · }）"

# `set-budget 60`：合法值 ⇒ rc=0，且**写出的 registry 与旧版逐字节相同** ✓
rm -rf "$TMP/old-cache-ok" "$TMP/new-cache-ok"; mkdir -p "$TMP/old-cache-ok" "$TMP/new-cache-ok"
run_setbudget "$OLD" 60 "$TMP/old-cache-ok"; old_rc=$SETBUD_RC
run_setbudget "$GUARD" 60 "$TMP/new-cache-ok"; new_rc=$SETBUD_RC
if [ "$old_rc" -eq 0 ] && [ "$new_rc" -eq 0 ] \
   && cmp -s "$TMP/old-cache-ok/registry.json" "$TMP/new-cache-ok/registry.json"; then
  ok "③ \`set-budget 60\` ⇒ 旧/新都 rc=0，写出的 \`registry.json\` **逐字节相同**（$(cat "$TMP/new-cache-ok/registry.json" | tr -d '\n ')）✓"
else
  nope "③ \`set-budget 60\` 的正对照不成立（旧 rc=$old_rc · 新 rc=$new_rc · registry 是否相同：$(cmp -s "$TMP/old-cache-ok/registry.json" "$TMP/new-cache-ok/registry.json" && echo 是 || echo 否)）✗"
fi

# ── ④ 空转闸已在上面判过（`cmp -s`）；这里再钉一次"两份确实不同"的读数 ────────
ok "④ 空转闸：钉死旧版（\`$BAD_REV_SHORT\`）与工作区**逐字节不同**（否则本脚本早就 exit 3）✓"

if [ "$fail" -eq 0 ]; then
  printf 'compile-cache-bite: **咬住了** ✓（①②③④ 全成立）⇒ bite exit 1（台账 `expect_rc: 1`）\n'
  exit 1
fi
printf 'compile-cache-bite: **没咬住** ✗ ⇒ exit 0（台账 C2 会因此判红 ✓）\n' >&2
exit 0
