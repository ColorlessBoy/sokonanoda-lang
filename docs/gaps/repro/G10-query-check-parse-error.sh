#!/usr/bin/env bash
# G-10 自断言复现：`query check` 对「解析失败」的源文本必须报 **parse 诊断 + 退出码 1**
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在（假绿） · 1 = 已修（修后形状成立） · 2 = 环境/形状异常（需要人看）
#
# 台账 `today`（0.58.0 实测，修前）：counts 全 0、`failed: []`、`ok:true`、**exit 0**
# ——对 agent 是假绿；同一份文本用 `grade` 跑则正确报 `unexpected-token` 并 exit 1。
# 修后契约（0.59.0，WO-003；`docs/protocol.md` 的 `check` 行）：
#   * `check.data.failed` 至少含一条 parse 诊断：`code` ∈ {unexpected-token,
#     unexpected-eof, import-malformed, import-not-a-valid-module-name,
#     import-must-precede-declarations, **notation-shape**}，`name` 为 null，
#     `start`/`end` = 诊断 span 的字节 offset。
#     ⚠ **别把码写死成单个值、也别把 span 写成实测字面量**（2026-09-24 修）：本脚本的
#     输入 `infix:50 " e " => mem` 命中的其实是 **`notation-shape`**（`e` 是普通
#     标识符词，不能当记法符号），span 是 `(9, 14)`；旧判据写死
#     `code=="unexpected-token"` + `start==9 and end==9` ⇒ 比契约更严 ⇒ 判成
#     "既不是旧假绿也不是修后契约" ⇒ **exit 2（环境异常）**，把 CI 环境误判成环境问题 ✗；
#   * ⚠ **只收 stdout**：`2>&1` 会把 Node 的 `[UNDICI-EHPA]` 代理警告收进来，
#     让 JSON 解析失败 ⇒ 同样掉进 exit 2 ✗；
#   * 退出码 **1**（与 `grade` 同口径）；`counts` 保持全 0（诚实：一条声明都没验过）；
#     `ok` 保持 `true`（"问出来了"——答案就是"这份文本解析不了"）。
# 本脚本断言的正是**修后形状**：假绿一旦回来（全零 + `failed` 空 + exit 0），脚本
# 回到 exit 0 ⇒ `gap.py check` 会对 `status=fixed` 的台账报"缺口仍在"。

set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }

SRC='infix:50 " e " => mem'
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
printf '%s\n' "$SRC" > "$TMP/bad.sokonanoda"

echo "== 通道 1：query check（agent 主通道）——两条输入通道各一次 =="
for channel in text file; do
  # **只收 stdout**（2026-09-24 修 CI/本机假红）：`2>&1` 会把 Node 的
  # `[UNDICI-EHPA] Warning: EnvHttpProxyAgent is experimental` 一起收进来
  # （设了代理环境变量就会出现），于是 `Q` 不是合法 JSON ⇒ 判成"形状不对"
  # ⇒ **exit 2（环境异常）**，把好好的 check 信封误判成环境问题 ✗
  # ——本次 CI 那条 `G-10 fixed 环境异常` 就是这么来的。
  # 启动器自己的报错仍会被下游判成"不是可解析的信封"，不需要靠 stderr。
  if [ "$channel" = text ]; then
    Q="$( node "$SOKO" query check --text "$SRC" --compact 2>/dev/null )"
  else
    Q="$( node "$SOKO" query check --file "$TMP/bad.sokonanoda" --compact 2>/dev/null )"
  fi
  q_exit=$?
  printf '  --%s → %s\n' "$channel" "$Q"
  shape=0
  printf '%s' "$Q" | python3 -c '
import json, sys

try:
    d = json.load(sys.stdin)
    data = d["data"]
    counts = data["counts"]
    failed = data["failed"]
except Exception as exc:  # 启动器报错 / 不是 JSON / 信封缺字段
    print("   → 不是可解析的 check 信封：", exc)
    sys.exit(3)

zero = all(v == 0 for v in counts.values())
if zero and failed == [] and d.get("ok") is True:
    print("   → 全零 + failed 空 + ok:true = 假绿（G-10 仍在）")
    sys.exit(0)
first = failed[0] if failed else {}
# **判据按契约，不按某一次实测的字面量**（2026-09-24 修 CI 假红）：
# 契约（本脚本头部 + docs/protocol.md）说的是"**至少一条** parse 诊断、code ∈ 五个
# 文档化取值、name 为 null、start/end 是**诊断自身的**字节 span"——
# 而旧判据写死成 `len(failed)==1` + `code=="unexpected-token"` + `start==9 and end==9`，
# 比契约更严 ⇒ CI 上只要多一条诊断、或 span 起止差一个字节，就掉进"既不是旧假绿
# 也不是修后契约"⇒ **exit 2（环境异常）**，把 CI 环境误判成环境问题 ✗。
# 放宽的**只有**这些无关紧要的字面量；区分"旧假绿"的那条（全零 + failed 空 + ok:true）
# 一个字没动 ⇒ 假绿照样会被判出来。
PARSE_CODES = {
    "unexpected-token",
    "unexpected-eof",
    "import-malformed",
    "import-not-a-valid-module-name",
    "import-must-precede-declarations",
    # 记法声明的形状不对（`infix:50 " e " => mem` 里 `e` 不是符号）——
    # 这条输入命中的就是它。
    "notation-shape",
}
ok_shape = (
    d.get("ok") is True
    and zero
    and len(failed) >= 1
    and first.get("code") in PARSE_CODES
    and first.get("name") is None
    and isinstance(first.get("start"), int)
    and isinstance(first.get("end"), int)
    and first.get("end") >= first.get("start")
)
print("   → parse 诊断 =", first.get("code"), "· span =", (first.get("start"), first.get("end")),
      "· counts 全零 =", zero, "· ok =", d.get("ok"))
# 0 = 缺口重现（假绿）；1 = 修后形状成立；3 = 两者都不是
sys.exit(1 if ok_shape else 3)
'
  shape=$?
  echo "   退出码 = ${q_exit}（修后契约 1）"
  if [ "$shape" = 0 ]; then
    echo "结论：G-10 仍在（query check 对解析失败假绿、退出码 ${q_exit}）。" >&2
    exit 0
  fi
  if [ "$shape" = 3 ]; then
    echo "结论：query check 的形状既不是旧假绿也不是修后契约 ⇒ 需要人看。" >&2
    echo "      （判据按契约：ok:true + counts 全零 + failed ≥1 + code ∈ 五个文档化取值" >&2
    echo "        + name:null + start/end 是整数且 end ≥ start；上面那行已打出实测值。）" >&2
    exit 2
  fi
  if [ "$q_exit" != 1 ]; then
    echo "结论：failed 已带 parse 诊断但退出码 = ${q_exit}（契约要求 1）⇒ 需要人看。" >&2
    exit 2
  fi
done

echo
echo "== 通道 2：grade（全量事件流）——同一份文本，同口径 =="
G="$( node "$SOKO" grade "$TMP/bad.sokonanoda" 2>/dev/null )"
g_exit=$?
printf '%s\n' "$G" | tail -1
# 同一条纪律：**按契约**（五个文档化的 parse 码之一）判断，别写死单个码。
printf '%s' "$G" | grep -qE '"code":"(unexpected-token|unexpected-eof|import-malformed|import-not-a-valid-module-name|import-must-precede-declarations|notation-shape)"' \
  || { echo "结论：grade 不再报 parse 诊断 ⇒ 两条通道口径不一致，需要人看。" >&2; exit 2; }
echo "   退出码 = ${g_exit}（应为 1，与 query check 一致）"
[ "$g_exit" = 1 ] || { echo "结论：grade 的退出码不是 1 ⇒ 需要人看。" >&2; exit 2; }

echo
echo "结论：G-10 已修——query check 与 grade 对同一份坏文本同口径（parse 诊断 + exit 1），"
echo "      假绿（全零 + failed 空 + exit 0）不再出现。"
exit 1
