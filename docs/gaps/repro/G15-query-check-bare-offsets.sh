#!/usr/bin/env bash
# G-15 自断言复现：`query check` 的 `failed[]` / `warnings[]` 只给**裸字节 offset**——
# 没有行列、没有单位、没有坐标空间说明（agent 的 MCP `check` 走的就是这条通道）。
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在（缺行列字段，或行列与事件自带 span 不一致，或 span 不再等于出错命令）
#   1 = 行为变了（已修：两个数组自带 1 基行列，且与 `grade --json` 的 span 逐字段一致）
#   2 = 环境/形状异常（跑不起来 / 不是可解析的 JSON ⇒ 需要人看，不算结果）
#
# ⚠️ 勘误（本文件的前身叫 G15-kernel-error-span-drift.sh，教训见 docs/LESSONS.md）：
# 它断言的是「内核拒绝的 span 会漂到别的声明上」——**那个结论是错的**。`span.offset`
# 是**字节**偏移（`crates/front/src/token.rs`：`self.offset += c.len_utf8()`），旧脚本
# 却拿它当**字符**下标去切 Python 字符串；源里有 α/中文（多字节）⇒ 数出来的行号偏后，
# 看起来就是"溢出到下一个声明"。按**字节**切，span 逐字等于出错命令本身。
#
# 所以本脚本的度量只用两种尺子，**禁止** `src[:offset]` 这类字符切片：
#   ① 事件自带的 `span.start.line/column`（内核给的行列，`grade --json`）；
#   ② `src.encode()` 的**字节**切片（用来验证 `span.offset` 的语义 = 出错命令范围）。
#
# 修后契约（0.59.0，WO-010；`docs/protocol.md` 的 `check` 行）：`failed[]` / `warnings[]`
# 每条带 `start_line/start_col/end_line/end_col`（1 基，与事件 `span` 同一批数字）；
# `start`/`end` 仍是**字节** offset，坐标空间 = **入口文件**（依赖的病只以
# `import-dependency-failed` 出现在入口，见 F′ 形状）。

set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
command -v node >/dev/null 2>&1 || { echo "需要 node（scripts/soko 是零依赖 Node 启动器）" >&2; exit 2; }
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# ── 夹具：一条 import（让 warnings[] 也非空）+ 中间一条坏声明 + 多字节注释 ──────
cat > "$TMP/Lib.sokonanoda" <<'EOF'
axiom P : Prop

example : P := sorry
EOF
cat > "$TMP/Main.sokonanoda" <<'EOF'
-- 中文注释：多字节字符让**字节** offset 与**字符**下标不再相等
import Lib

def good : Prop -> Prop := fun (p : Prop) => p

def bad : Prop -> Type := fun (x : Prop) => x

def after : Prop -> Prop := fun (p : Prop) => p
EOF

echo "== ① 夹具 =="
echo "   入口 Main.sokonanoda 第 6 行是坏声明（1 基），前后各有一条好声明；"
echo "   第 1 行是中文注释 ⇒ 按字符数行会偏，按**字节**数行才对。"
echo

echo "== ② grade --json：事件流的 span（内核给的唯一真相）=="
SOKONANODA_CACHE_DIR="$TMP/cache" node "$SOKO" grade "$TMP/Main.sokonanoda" \
  > "$TMP/grade.jsonl" 2>"$TMP/grade.err"
g_exit=$?
tail -1 "$TMP/grade.jsonl" 2>/dev/null | head -c 400; echo
echo "   grade 退出码 = ${g_exit}（坏声明 ⇒ 期望 1）"
if [ "$g_exit" != 1 ] || [ ! -s "$TMP/grade.jsonl" ]; then
  echo "结论：grade 没跑起来或没判红 ⇒ 环境/夹具异常，需要人看。" >&2
  sed -n '1,5p' "$TMP/grade.err" >&2
  exit 2
fi
echo

echo "== ③ query check（agent 通道）：failed[] / warnings[] 的坐标 =="
SOKONANODA_CACHE_DIR="$TMP/cache" node "$SOKO" query check --file "$TMP/Main.sokonanoda" --compact \
  > "$TMP/query.json" 2>"$TMP/query.err"
q_exit=$?
head -c 600 "$TMP/query.json" 2>/dev/null; echo
echo "   query check 退出码 = ${q_exit}（与 grade 同口径 ⇒ 期望 1）"
if [ ! -s "$TMP/query.json" ]; then
  echo "结论：query check 没吐出 JSON ⇒ 环境异常，需要人看。" >&2
  sed -n '1,5p' "$TMP/query.err" >&2
  exit 2
fi
echo

echo "== ④ 度量：字节切片 vs 事件自带行列（只用这两种尺子）=="
python3 - "$TMP/query.json" "$TMP/grade.jsonl" "$TMP/Main.sokonanoda" <<'PY'
import json, pathlib, sys

query_path, grade_path, src_path = sys.argv[1:4]
src = pathlib.Path(src_path).read_bytes()          # **字节**，不是 str
try:
    env = json.loads(pathlib.Path(query_path).read_text())
    data = env["data"]
except Exception as exc:                            # 不是可解析的 check 信封
    print("   → query check 的输出不是可解析的 JSON：", exc)
    sys.exit(3)

failed = data.get("failed") or []
warnings = data.get("warnings") or []
if not failed:
    print("   → failed[] 是空的（夹具/行为异常，判不了）")
    sys.exit(3)

events = [json.loads(line) for line in pathlib.Path(grade_path).read_text().splitlines() if line.strip()]
# 只取**入口**的诊断/警告：依赖的诊断带 `file`/`module`（别的坐标空间，G-15 的教训
# 之一就是别拿入口的文本去折算依赖的 offset）。
kernel = [e for e in events
          if e.get("type") == "diagnostic" and e.get("code") == "kernel-rejected" and "file" not in e]
warns = [e for e in events if e.get("type") == "warning" and "file" not in e]
if not kernel:
    print("   → 入口没有 kernel-rejected 诊断（夹具/行为异常，判不了）")
    sys.exit(3)

def line_at(offset: int) -> int:
    """按**字节**数行——这是唯一正确的量法（旧脚本按字符数 ⇒ 假缺口）。"""
    return src[:offset].count(b"\n") + 1

def span_of(ev):
    s, e = ev["span"]["start"], ev["span"]["end"]
    return (s["line"], s["column"], e["line"], e["column"])

diag = kernel[0]
f = failed[0]
s, e = f.get("start"), f.get("end")
block = src[s:e].decode() if isinstance(s, int) and isinstance(e, int) else ""
sl, sc, el, ec = span_of(diag)
print("   事件 span（行:列）= %d:%d → %d:%d · 字节 offset %d..%d"
      % (sl, sc, el, ec, diag["span"]["start"]["offset"], diag["span"]["end"]["offset"]))
if isinstance(s, int) and isinstance(e, int):
    print("   按**字节**重算的行 = %d → %d" % (line_at(s), line_at(e)))
    print("   字节切片 = %r" % block)
else:
    print("   offset 不是整数：", s, e)

KEYS = ("start_line", "start_col", "end_line", "end_col")
has_fields = all(k in f for k in KEYS)
if not has_fields:
    print("   failed[0] 的字段 = %s" % sorted(f))
    print("   → query check 只给裸 offset（缺 %s）：**G-15 仍在**。"
          % ", ".join(k for k in KEYS if k not in f))
    print("     （事件自带的 span 一直是精确的：%d:%d → %d:%d 就是第 %d 行的坏声明；"
          % (sl, sc, el, ec, line_at(diag["span"]["start"]["offset"])))
    print("      旧脚本把它当**字符**下标去量，才看到「漂移」。）")
    sys.exit(0)

print("   failed[0] 行列 = %d:%d → %d:%d · 字节 offset %d..%d"
      % (f["start_line"], f["start_col"], f["end_line"], f["end_col"], s, e))
EXPECTED = "def bad : Prop -> Type := fun (x : Prop) => x"
mismatch = []
if (f["start_line"], f["start_col"], f["end_line"], f["end_col"]) != span_of(diag):
    mismatch.append("failed[] 的行列与事件 span 不一致：%s vs %s" % (
        (f["start_line"], f["start_col"], f["end_line"], f["end_col"]), span_of(diag)))
if (s, e) != (diag["span"]["start"]["offset"], diag["span"]["end"]["offset"]):
    mismatch.append("failed[] 的字节 offset 与事件 span 不一致")
if f["start_line"] != line_at(s) or f["end_line"] != line_at(e):
    mismatch.append("failed[] 的行号与**按字节**重算的行号不一致（行列的单位/坐标空间没说清）")
if block.strip() != EXPECTED:
    mismatch.append("字节切片不是出错的那条命令（span 真的漂了）：%r" % block)
if warnings:
    w, wev = warnings[0], (warns[0] if warns else None)
    wkeys = ("start_line", "start_col", "end_line", "end_col")
    if not all(k in w for k in wkeys):
        mismatch.append("warnings[0] 仍缺行列字段（%s）" % sorted(w))
    elif wev is not None and (w["start_line"], w["start_col"], w["end_line"], w["end_col"]) != span_of(wev):
        mismatch.append("warnings[0] 的行列与事件 span 不一致")
    print("   warnings[0] 行列 = %s · code = %s"
          % ([(w.get(k)) for k in wkeys], w.get("code")))
else:
    print("   warnings[] 为空（本次没验到——夹具本该在 import 行给一条 warning）")
if mismatch:
    for m in mismatch:
        print("   → " + m)
    sys.exit(0)

print("   → failed[] / warnings[] 自带行列、单位（字节 offset）与坐标空间（入口文件）")
print("     与事件流逐字段一致；字节切片逐字等于第 6 行的坏声明。")
sys.exit(1)
PY
shape=$?
echo

if [ "$shape" = 0 ]; then
  echo "结论：G-15 仍在（query check 的 failed[]/warnings[] 没有可信的行列；span 本身是精确的）"
  echo "      —— 与台账一致。"
  exit 0
fi
if [ "$shape" = 3 ]; then
  echo "结论：形状既不是旧裸 offset 也不是修后契约 ⇒ 需要人看。" >&2
  exit 2
fi
[ "$q_exit" = 1 ] || { echo "结论：行列已带，但 query check 退出码 = ${q_exit}（契约要求 1）⇒ 需要人看。" >&2; exit 2; }
echo "结论：G-15 已修——query check 的 failed[] / warnings[] 自带 1 基行列（与 grade --json 的"
echo "      span 逐字段一致），start/end 仍是字节 offset（坐标空间 = 入口文件）。"
exit 1
