#!/usr/bin/env bash
# G-10 自断言复现：`query check` 对「解析失败」的源文本必须报 **parse 诊断 + 退出码 1**
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在（假绿） · 1 = 已修（修后形状成立） · 2 = 环境/形状异常（需要人看）
#
# 台账 `today`（0.58.0 实测，修前）：counts 全 0、`failed: []`、`ok:true`、**exit 0**
# ——对 agent 是假绿；同一份文本用 `grade` 跑则正确报 `unexpected-token` 并 exit 1。
# 修后契约（0.59.0，WO-003；`docs/protocol.md` 的 `check` 行）：
#   **2026-09-24 更新**：夹具 `infix:50 " e " => mem` 现在报 **`notation-shape`**
#   （记法子集 0.59.0 之后新增的、更精确的 parse 诊断：`e` 是普通标识符词，不能
#   当记法符号）——它同样是 **parse 阶段**诊断（`name: null`、有 span）。
#   判据的**实质不变**：解析失败必须"报 parse 诊断 + exit 1"，而不是全零假绿。
#   * `check.data.failed` 至少含一条 parse 诊断：`code` ∈ {unexpected-token,
#     notation-shape,
#     unexpected-eof, import-malformed, import-not-a-valid-module-name,
#     import-must-precede-declarations}，`name` 为 null，`start`/`end` = 诊断 span
#     的字节 offset（本次实测 9/9）；
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
  # **只捕获 stdout**（2026-09-24 修）：`2>&1` 会把 Node/undici 的实验性警告
  # （环境里设了代理时必现）并进 JSON 通道 ⇒ 解析失败 ⇒ 本脚本误报 exit 2
  # "环境异常"。stderr **不并进来**：它照旧打到终端，人看得见，不算掩膜。
  if [ "$channel" = text ]; then
    Q="$( node "$SOKO" query check --text "$SRC" --compact )"
  else
    Q="$( node "$SOKO" query check --file "$TMP/bad.sokonanoda" --compact )"
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
ok_shape = (
    d.get("ok") is True
    and zero
    and len(failed) == 1
    and first.get("code") in ("unexpected-token", "notation-shape")
    and first.get("name") is None
    # **span 只要非空**：0.59.0 时这个夹具的诊断正好是 (9, 9)（零宽），
    # 记法子集之后变成 (9, 14)——覆盖住那个非法的符号词。**具体偏移是
    # 过度规定**（诊断变精确是好事），判据的实质是"有一条**带 span** 的 parse
    # 诊断"，所以这里只要求非空、且落在文本内。
    and isinstance(first.get("start"), int)
    and isinstance(first.get("end"), int)
    and 0 <= first["start"] < first["end"] <= len(SRC)
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
    exit 2
  fi
  if [ "$q_exit" != 1 ]; then
    echo "结论：failed 已带 parse 诊断但退出码 = ${q_exit}（契约要求 1）⇒ 需要人看。" >&2
    exit 2
  fi
done

echo
echo "== 通道 2：grade（全量事件流）——同一份文本，同口径 =="
G="$( node "$SOKO" grade "$TMP/bad.sokonanoda" 2>&1 )"
g_exit=$?
printf '%s\n' "$G" | tail -1
printf '%s' "$G" | grep -qE '"code":"(unexpected-token|notation-shape)"' \
  || { echo "结论：grade 不再报 parse 诊断 ⇒ 两条通道口径不一致，需要人看。" >&2; exit 2; }
echo "   退出码 = ${g_exit}（应为 1，与 query check 一致）"
[ "$g_exit" = 1 ] || { echo "结论：grade 的退出码不是 1 ⇒ 需要人看。" >&2; exit 2; }

echo
echo "结论：G-10 已修——query check 与 grade 对同一份坏文本同口径（parse 诊断 + exit 1），"
echo "      假绿（全零 + failed 空 + exit 0）不再出现。"
exit 1
