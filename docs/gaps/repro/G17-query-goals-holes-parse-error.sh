#!/usr/bin/env bash
# G-17 自断言复现：`query goals` / `query holes` 对「解析失败」的源文本返回**空数组 +
# `ok:true` + 退出码 0**（与 G-10 同族的假绿：读起来就是"这份画布没有声明/没有洞"）。
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在（假绿） · 1 = 已修（修后形状成立） · 2 = 环境/形状异常（需要人看）
#
# 台账 `today`（0.58.0 实测，修前）：
#   query goals --text 'infix:50 " e " => mem' → `data: []`、`ok:true`、exit 0；
#   query holes 同一份文本 → `holes: []` + `navigated: null`、`ok:true`、exit 0；
#   而 `query state` 同一份文本给 `ok:false` + `outside-declarations`（三个 op 三种口径）。
# 修后契约（0.59.0，与 G-10 同轮；`docs/protocol.md` 的 `goals`/`holes` 行）：
#   解析失败 = "问不出来" ⇒ `ok:false` + `error.code = "not-parsable"` + 退出码 1，
#   负载里**没有** `data`（不发假空数组）；"正常的没有"（能解析、只是没有声明/洞）仍是
#   `ok:true` + 空数组 / `navigated: null`（协议 §4.1 的两分法）。
# 本脚本断言的正是**修后形状**：假绿一旦回来（空数组 + `ok:true`），脚本回到 exit 0。

set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }

BAD='infix:50 " e " => mem'
EMPTY='-- 只有注释
'

# stdin 上的 JSON：0 = 旧假绿（缺口重现）、1 = 修后形状、3 = 两者都不是。
judge_bad() {
  python3 -c '
import json, sys

try:
    d = json.load(sys.stdin)
except Exception as exc:
    print("   → 不是 JSON：", exc)
    sys.exit(3)

if d.get("ok") is True:
    print("   → ok:true（空数组假绿 = G-17 仍在）")
    sys.exit(0)
err = d.get("error", {})
ok_shape = d.get("ok") is False and err.get("code") == "not-parsable" and "data" not in d
print("   → ok =", d.get("ok"), "· error.code =", err.get("code"), "· 无 data =", "data" not in d)
sys.exit(1 if ok_shape else 3)
'
}

# 一个坏文本调用：`--text "$BAD"` 放在最后（CLI 的 `--text` 会吃掉紧跟的值，
# 放前面会把下一个 flag 当成源码）。
check_bad() {
  label="$1"
  shift
  out="$( node "$SOKO" query "$@" --text "$BAD" --compact 2>&1 )"
  code=$?
  printf '  %-46s → %s\n' "$label" "$out"
  printf '%s' "$out" | judge_bad
  shape=$?
  echo "   退出码 = ${code}（修后契约 1）"
  if [ "$shape" = 0 ]; then
    echo "结论：G-17 仍在（goals/holes 对解析失败假绿）。" >&2
    exit 0
  fi
  if [ "$shape" = 3 ]; then
    echo "结论：${label} 的形状既不是旧假绿也不是修后契约 ⇒ 需要人看。" >&2
    exit 2
  fi
  if [ "$code" != 1 ]; then
    echo "结论：ok:false 已给出但退出码 = ${code}（契约要求 1）⇒ 需要人看。" >&2
    exit 2
  fi
}

echo "== ① 坏文本：三个调用（goals / holes / holes 导航形态）必须都答 not-parsable + exit 1 =="
check_bad "query goals --text" goals
check_bad "query holes --text" holes
check_bad "query holes --offset 0 --direction next" holes --offset 0 --direction next

echo
echo "== ② 对照组：能解析的空画布仍是 ok:true + 空数组（「正常的没有」不是错误）=="
CONTROL="$( node "$SOKO" query goals --text "$EMPTY" --compact 2>&1 )"
c_exit=$?
printf '  query goals  → %s\n' "$CONTROL"
printf '%s' "$CONTROL" | python3 -c '
import json, sys
d = json.load(sys.stdin)
ok = d.get("ok") is True and d.get("data") == []
print("   → ok:true + data:[] =", ok)
sys.exit(0 if ok else 1)
' || { echo "结论：可解析的空画布不再答空数组（回归）⇒ 需要人看。" >&2; exit 2; }
[ "$c_exit" = 0 ] || { echo "结论：空画布的退出码 = ${c_exit}（应为 0）⇒ 需要人看。" >&2; exit 2; }

CONTROL="$( node "$SOKO" query holes --text "$EMPTY" --compact 2>&1 )"
c_exit=$?
printf '  query holes  → %s\n' "$CONTROL"
printf '%s' "$CONTROL" | python3 -c '
import json, sys
d = json.load(sys.stdin)["data"]
ok = d["holes"] == [] and d["navigated"] is None
print("   → holes:[] + navigated:null =", ok)
sys.exit(0 if ok else 1)
' || { echo "结论：可解析的空画布不再答空洞列表（回归）⇒ 需要人看。" >&2; exit 2; }
[ "$c_exit" = 0 ] || { echo "结论：空画布的退出码 = ${c_exit}（应为 0）⇒ 需要人看。" >&2; exit 2; }

echo
echo "结论：G-17 已修——解析失败对 goals/holes 一律 not-parsable + exit 1（与 check 同族），"
echo "      而「正常的没有」仍是 ok:true + 空数组。"
exit 1
