#!/usr/bin/env bash
# G-07 自断言复现：**课程清单扁平**（无卷/章/先修/标签/练习配额）
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在（**已修后的期望形状**：下面四段断言必须全绿，任何一段红 ⇒ 缺口回来了）
#   1 = 行为变了 ⇒ 回来更新台账（写 fixed_in / 收工）
#   2 = 环境不满足
#
# 现场（修前）：两个清单都是**扁平 JSON 数组**（`[{file,title,title_en,unit}]`），
# 机器读得到的只有文件/标题/单元号；卷、章、先修、标签、练习配额**只能写进文档**
# （`courses/set-theory/README.md` 的现状表与大纲文档都是人手维护的），于是
# 「文档说有 4 章」与「机器只知道 12 个平铺单元」必然漂移。
#
# 期望（修后，0.60.0 / 台账 G-07）：清单 v2 `soko.course/2`
# （`{schema,name,title,volumes[].chapters[]{id,title,prereqs,tags,quota,units[]}}`），
# **v1 扁平数组继续被接受**（向后兼容是硬要求）；CLI 事件 additive 地多出
# `volume`/`chapter`/`tags` 与 summary 的 `volumes`/`chapters`；课程门禁有 G6。
#
# ── 本轮的更新（G-07 修好后）──────────────────────────────────────────────
# 本脚本在修好后**翻成 exit 1**（docs/gaps/README.md 的约定：exit 1 = 行为已变 ⇒ 关账）。
# 断言同时改成"修后应有的形状"，所以脚本不会在缺口复发时假绿：
#   ① 卷 I 的 course.json 是 v2（schema + volumes[].chapters[]，12 个单元一个不少）；
#   ② `course --json` 的 course.unit 带 volume/chapter/tags，summary 带 volumes/chapters；
#   ③ **兼容**：一份 v1 扁平夹具仍能判卷，且事件里**没有** volume 键；
#   ④ 课程门禁 --json 的 course.chapters 与清单一致（G6 的机器可读面）。
# 任一段不满足 ⇒ echo 缺口回来了 + **exit 0**（与缺口仍在的语义一致）。

set -u
cd "$(dirname "$0")/../../.." || exit 2
SOKO="$PWD/scripts/soko"
COURSE_JSON="$PWD/courses/set-theory/course.json"
command -v node >/dev/null 2>&1 || { echo "需要 node" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "需要 python3" >&2; exit 2; }
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail=0
note() { printf '%s\n' "$*"; }

# ── ① 清单是 v2 ────────────────────────────────────────────────────────────
echo "== ① 卷 I 的清单是 v2（schema + volumes/chapters + 12 单元）=="
V2_REPORT="$(python3 - "$COURSE_JSON" <<'PY'
import json, sys
try:
    data = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception as error:                       # noqa: BLE001 —— 复现件只报结论
    print(f"unreadable: {error}")
    raise SystemExit(0)
if not isinstance(data, dict):
    print("flat-array (v1)")
    raise SystemExit(0)
schema = data.get("schema")
volumes = data.get("volumes") if isinstance(data.get("volumes"), list) else []
chapters = [c for v in volumes if isinstance(v, dict)
            for c in (v.get("chapters") or []) if isinstance(c, dict)]
units = [u for c in chapters for u in (c.get("units") or []) if isinstance(u, dict)]
prereqs = sum(1 for c in chapters if isinstance(c.get("prereqs"), list))
tags = sum(1 for c in chapters if isinstance(c.get("tags"), list))
quotas = sum(1 for c in chapters
             if isinstance(c.get("quota"), dict) and isinstance(c["quota"].get("exercises"), int))
# 判据**与课程规模无关**（2026-10-01 修）：原来这里写死了 `chapters=4 units=12`，
# 于是**合法地加一章**（I.5 序关系与良序，13 个单元）就把复现件判红 ✗ ——
# 那与「课程门禁只判形状、不锁计数」的口径自相矛盾（course-manifest-v2.md §4.2）。
# 现在只判**形状**：v2 schema · 卷/章/单元非空 · 每章都带 prereqs/tags/quota ·
# 单元总数**不少于** 12（历史上的 12 单元一个都不能少，但可以继续长）。
shape_ok = (schema == "soko.course/2" and len(volumes) >= 1 and len(chapters) >= 1
            and len(units) >= 12 and prereqs == len(chapters) and tags == len(chapters)
            and quotas == len(chapters))
print(f"schema={schema} volumes={len(volumes)} chapters={len(chapters)} units={len(units)} "
      f"prereqs={prereqs} tags={tags} quotas={quotas} shape_ok={1 if shape_ok else 0}")
PY
)"
note "  $V2_REPORT"
v2_ok=1
case "$V2_REPORT" in
  *"shape_ok=1"*) v2_ok=0 ;;
esac
echo "   → v2 形状（修后预期，与规模无关）：$([ "$v2_ok" = 0 ] && echo yes || echo NO)"
echo

# ── ② CLI 事件 additive 地带上卷/章/标签 ───────────────────────────────────
echo "== ② course --json：unit 带 volume/chapter/tags，summary 带 volumes/chapters =="
EVENTS="$(node "$SOKO" course "$COURSE_JSON" --json 2>/dev/null)"
UNIT="$(printf '%s\n' "$EVENTS" | head -1)"
SUMMARY="$(printf '%s\n' "$EVENTS" | tail -1)"
note "  unit:    $(printf '%s' "$UNIT" | cut -c1-160)"
note "  summary: $SUMMARY"
cli_ok=1
printf '%s' "$UNIT" | grep -q '"volume"' || cli_ok=1
printf '%s' "$UNIT" | grep -q '"chapter"' || cli_ok=1
printf '%s' "$UNIT" | grep -q '"tags"' || cli_ok=1
printf '%s' "$SUMMARY" | grep -q '"volumes":1' || cli_ok=1
# 章数**不写死**（2026-10-01）：summary 报的章数必须与清单里的章数**相等**（真判据），
# 而不是等于某个历史快照值 —— 否则合法加一章就把复现件判红。
CLI_CHAPTERS="$(printf '%s' "$SUMMARY" | sed -n 's/.*"chapters":\([0-9]*\).*/\1/p')"
MANIFEST_CHAPTERS="$(python3 -c "
import json,sys
d=json.load(open(sys.argv[1],encoding='utf-8'))
print(sum(len(v.get('chapters') or []) for v in (d.get('volumes') or []) if isinstance(v,dict)))
" "$COURSE_JSON")"
[ "$CLI_CHAPTERS" = "$MANIFEST_CHAPTERS" ] || cli_ok=1
if printf '%s' "$UNIT" | grep -q '"volume"' \
   && printf '%s' "$UNIT" | grep -q '"chapter"' \
   && printf '%s' "$UNIT" | grep -q '"tags"' \
   && printf '%s' "$SUMMARY" | grep -q '"volumes":1' \
   && [ "$CLI_CHAPTERS" = "$MANIFEST_CHAPTERS" ]; then
  cli_ok=0
fi
echo "   → v2 事件字段（修后预期）：$([ "$cli_ok" = 0 ] && echo yes || echo NO)"
echo

# ── ③ 兼容：v1 扁平夹具照旧，且**不多出** v2 键 ────────────────────────────
echo "== ③ 向后兼容：v1 扁平清单仍可判卷，事件里没有 volume 键 =="
mkdir -p "$TMP/v1/units"
cat > "$TMP/v1/course.json" <<'JSON'
[{"file":"units/a.sokonanoda","title":"A","unit":1}]
JSON
cat > "$TMP/v1/units/a.sokonanoda" <<'SOKO'
theorem g07_compat (P : Prop) (h : P) : P := by
  exact h
SOKO
V1_OUT="$(node "$SOKO" course "$TMP/v1/course.json" --json 2>&1)"
V1_UNIT="$(printf '%s\n' "$V1_OUT" | head -1)"
note "  v1 unit: $(printf '%s' "$V1_UNIT" | cut -c1-160)"
v1_ok=1
if printf '%s' "$V1_UNIT" | grep -q '"type":"course.unit"' \
   && ! printf '%s' "$V1_UNIT" | grep -q '"volume"' \
   && ! printf '%s' "$V1_UNIT" | grep -q '"chapter"' \
   && ! printf '%s' "$V1_UNIT" | grep -q '"tags"'; then
  v1_ok=0
fi
echo "   → v1 事件不带 v2 键（修后预期）：$([ "$v1_ok" = 0 ] && echo yes || echo NO)"
echo

# ── ④ 课程门禁的机器可读面带上章计数（G6）──────────────────────────────────
echo "== ④ 课程门禁 --json：course.chapters 与清单一致（G6）=="
# **`--only "单元 1"` 不是省事，是这条断言的正确作用域**：本段要验的是 G6 的
# **机器可读面**（`course.volumes` / `course.chapters` 与清单一致），而那两项来自
# **清单本身**，与"判了几个目标"无关。整卷判一遍是 315s（2026-09-21 实测，
# release 二进制），会把 `gap.py check` 变成 CI 上最慢的一步；只判一个目标是 3s，
# 而 G6 该验的东西一个没少。`course.units` 会随选择变化（=1），所以"12 个单元"
# 由第 ① 段直接从 `course.json` 断言，不在这里重复。
GATE_JSON="$TMP/gate.json"
GATE_BIN="${SOKONANODA_BIN:-$PWD/target/debug/sokonanoda}"
GATE_ERR="$TMP/gate.err"
# ⚠ **不许把门禁的 stderr 丢掉** ✗✓（2026-09-27 实测）：原来是 `2>/dev/null` ✗ ⇒
#   门禁以 **exit 2（前置/环境问题）** 收场时，**原因被扔了**，本复现件于是把
#   「**环境坏了**」读成「**缺口回来了**」✗ ⇒ 让**过期的 debug 产物**伪装成**行为回退** ✗✓。
#   （实测：`target/debug/sokonanoda` 自报 **0.72.0**、仓库钉 **0.73.0** ⇒
#    `--bin` 分支的版本核对**正确地** exit 2 ✓ —— 那不是缺口 ✗。）
#   ⇒ 保留 stderr，并在下面把 **exit 2 单独认成「环境/形状异常」** ✓
#     （与本复现件的契约一致：`0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常` ✓）。
if [ -x "$GATE_BIN" ]; then
  python3 "$PWD/courses/set-theory/tools/check.py" --bin "$GATE_BIN" \
    --only "单元 1" --json > "$GATE_JSON" 2>"$GATE_ERR"
  GATE_EXIT=$?
else
  python3 "$PWD/courses/set-theory/tools/check.py" --only "单元 1" --json \
    > "$GATE_JSON" 2>"$GATE_ERR"
  GATE_EXIT=$?
fi
if [ "$GATE_EXIT" = 2 ]; then
  echo "  ⚠ 门禁 **exit 2 = 前置/环境问题**，**不是**「缺口回来了」✗ —— 本复现件不作 G-07 的判断" >&2
  sed 's/^/      /' "$GATE_ERR" >&2
  echo "  ⇒ 按契约报 **2 = 环境/形状异常** ✓（先修环境，别改台账 ✗）" >&2
  exit 2
fi
GATE_COUNTS="$(python3 - "$GATE_JSON" <<'PY'
import json, sys
try:
    report = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception:                                # noqa: BLE001
    print("no-report")
    raise SystemExit(0)
course = report.get("course") or {}
summary = report.get("summary") or {}
print(f"volumes={course.get('volumes')} chapters={course.get('chapters')} "
      f"units={course.get('units')} rejected={summary.get('rejected')} "
      f"quota_notes={len(report.get('quota_notes') or [])}")
PY
)"
note "  门禁 exit=$GATE_EXIT · $GATE_COUNTS"
gate_ok=1
# 章数同样**不写死**（2026-10-01）：门禁报的章数要与清单一致，其余形状逐条判。
GATE_CHAPTERS="$(printf '%s' "$GATE_COUNTS" | sed -n 's/.*chapters=\([0-9]*\).*/\1/p')"
if [ "$GATE_EXIT" = 0 ] \
   && printf '%s' "$GATE_COUNTS" | grep -q 'volumes=1 ' \
   && printf '%s' "$GATE_COUNTS" | grep -q 'units=1 rejected=0' \
   && [ "$GATE_CHAPTERS" = "$MANIFEST_CHAPTERS" ]; then
  gate_ok=0
fi
echo "   → G6 机器可读面（修后预期）：$([ "$gate_ok" = 0 ] && echo yes || echo NO)"
echo

if [ "$v2_ok" = 0 ] && [ "$cli_ok" = 0 ] && [ "$v1_ok" = 0 ] && [ "$gate_ok" = 0 ]; then
  echo "结论：G-07 已修（清单 v2 + 事件 additive 字段 + v1 仍兼容 + 门禁 G6）⇒ 行为已变，更新台账。" >&2
  exit 1
fi
echo "结论：缺口回来了（清单又变回扁平 / v2 字段没发出来 / v1 兼容破了 / 门禁没有 G6）——" >&2
echo "      检查 courses/set-theory/course.json、crates/cli/src/course/{mod,manifest}.rs、tools/check.py。" >&2
exit 0
