#!/usr/bin/env bash
# G-66 —— Infoview 分区标题「目标 / 声明 / 项目」比正文**更小更暗**：
# 它们是 `<h2>`（层级更高）却 `font-size: 0.8em` + `opacity: 0.7` + `text-transform: uppercase`
# + `letter-spacing: 0.05em`（视觉更弱）✗ —— 2026-09-28 用户实测，**一个月内第二次**同形反馈
# （2026-09-26 修过 `.decl-ty`「太小太暗」，当时**没横向排查** ✗）。
#
# 退出码（docs/gaps/README.md「复现脚本的退出码约定」）：
#   0 = 缺口仍在（用户那四条要求里有任一条不成立）· 1 = 已满足 · 2 = 环境不对（跑不起来）
#
# 用户 2026-09-28 的四条要求 ⇒ 逐条落成判据：
#   ① 分区标题**字号 ≥ 正文字号**（从 CSS 实测 em/px）；**横向**：不许再有「小于正文 **且**
#      叠 opacity」的规则（当年实测 7 处 ✗）；
#   ② 分区标题**不叠 `opacity`**（降一档只用颜色：`color: …descriptionForeground` ✓）；
#   ③ 中文标题下**无 `text-transform: uppercase` / `letter-spacing`**；
#   ④ 两个守卫脚本 **exit 0**（`check-infoview-hierarchy.py` 含 `--selftest` 反向验证 ✓）。
#   ⑤ **反向臂**：把 CSS **副本**改回坏形态 ⇒ 两个守卫都必须判红（**不碰真树** ✓）。
#
# ⚠ 判据绑「用户看到什么」（AGENTS.md 验证设计纪律第 0 条 (a)）：
#   `media/infoview.js` 的 `section()` 把三处分区标题渲染成 `<h2 class="section-title">`
#   （**结构臂** ✓ —— 判据绑在用户真的看到的那个 DOM 节点上）+ CSS 给它的**实测**
#   字号 / 透明度 / 大小写变换（**样式臂** ✓）⇒ 覆盖「用户打开 Infoview 看到的那三行
#   标题的字号与暗度」。**真机渲染**（CSS 级联、主题变量解析、VS Code 的字体设置）
#   不在本判据里 —— 那要 VS Code e2e；这里用静态 CSS/JS 判据替代，它咬住的正是
#   **用户可见的那三条退化本身**（字号 < 正文 / 叠 opacity 变暗 / 中文被 uppercase 与
#   letter-spacing 折腾），而不是"链路通不通"✗。
#
# ⚠ 防空转：扫描必须真的扫到东西（规则数 ≥ 20、`< 1em` 命中数 > 0）—— 「没扫到 ≠ 绿」✓。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CSS="$ROOT/editor/vscode/media/infoview.css"
JS="$ROOT/editor/vscode/media/infoview.js"
HIER="$ROOT/scripts/check-infoview-hierarchy.py"
AUDIT="$ROOT/scripts/audit-infoview-css.py"

for f in "$CSS" "$JS" "$HIER" "$AUDIT"; do
  if [ ! -f "$f" ]; then
    echo "G-66: 找不到 $f ⇒ 环境不对 ✗（exit 2）"
    exit 2
  fi
done
if ! command -v python3 >/dev/null 2>&1; then
  echo "G-66: 没有 python3 ⇒ 环境不对 ✗（exit 2）"
  exit 2
fi

# **异常退出 ≠ 已修** ✗（2026-10-07 实测踩到：脚本自己因 `set -u` 崩掉，退出码 1 被读成
# "缺口已修" ⇒ 假绿）。⇒ 只有**显式**给出的结论才算数；其余一律 exit 2（环境/形状不对，
# `gap.py` 会把它判红 —— "跑不起来"绝不能被读成"修好了"✓）。
# ⚠ 陷阱必须**尽早**装（在任何可能崩的语句之前 ✓）。
verdict=""
tmp=""  # 反向臂的临时镜像（见 ⑤）；**清理写进 `finish`** —— 再挂一个 EXIT trap 会把
        # 上面这条结论陷阱**顶掉** ✗（2026-10-07 实测：`trap … EXIT` 是覆盖、不是叠加 ✓）
finish() {
  local rc=$?
  if [ -n "$tmp" ]; then rm -rf "$tmp"; fi
  if [ "$rc" -eq 2 ]; then exit 2; fi
  if [ "$verdict" = "satisfied" ] && [ "$rc" -eq 1 ]; then exit 1; fi
  if [ "$verdict" = "present" ] && [ "$rc" -eq 0 ]; then exit 0; fi
  echo "G-66: 脚本异常退出（rc=$rc · verdict='${verdict}'）⇒ 环境/形状不对 ✗（exit 2）"
  exit 2
}
trap finish EXIT

fails=0
note() { printf '  %s\n' "$*"; }

# ── 结构臂：三处分区标题真的是 `<h2 class="section-title">`（判据绑在用户看到的节点上）──
if grep -q 'el("h2", "section-title"' "$JS" &&
   grep -q 'section("目标")' "$JS" &&
   grep -q 'section("声明")' "$JS" &&
   grep -q 'section("项目")' "$JS"; then
  note "结构 ✓ media/infoview.js：section() → <h2 class=\"section-title\">，目标/声明/项目 三处都走它"
else
  note "结构 ✗ media/infoview.js 不再把三处分区标题渲染成 <h2 class=\"section-title\">（判据失去绑定的 DOM 节点）"
  fails=$((fails + 1))
fi

# ── 样式臂 ①②③ + 横向排查：全部实测自 CSS（em/px 都算）─────────────────────────
css_out="$(python3 - "$CSS" <<'PY'
import re
import sys
from pathlib import Path

p = Path(sys.argv[1])
text = p.read_text(encoding="utf-8")
clean = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
rules = [(" ".join(s.split()), b) for s, b in re.findall(r"([^{}]+)\{([^}]*)\}", clean)]
if len(rules) < 20:  # 防空转：「没扫到 ≠ 绿」✗
    print(f"  ✗ 只解析出 {len(rules)} 条规则 ⇒ 扫描没扫到东西（环境/形状不对）")
    sys.exit(2)


def size(body):
    m = re.search(r"font-size:\s*([\d.]+)(em|px)\b", body)
    return (float(m.group(1)), m.group(2)) if m else None


def dim(body):
    m = re.search(r"(?<![-a-z])opacity:\s*([\d.]+)", body)
    return float(m.group(1)) if m and float(m.group(1)) < 1.0 else None


def small(s):
    return s[0] < 1.0 if s[1] == "em" else s[0] < 13.0  # 正文 = 1em = 13px（--vscode-font-size 默认）


def show(s):
    if not s:
        return "inherit（继承正文）"
    px, em = (s[0] * 13.0, s[0]) if s[1] == "em" else (s[0], s[0] / 13.0)
    return f"{s[0]:g}{s[1]}（≈ {px:.1f}px / {em:.2f}em）"


bad = 0

# 分区标题的包装（`.section`）不许偷偷把字号压小 —— 否则「标题 1em」是假象 ✗
for sel, body in rules:
    if re.search(r"(^|[\s,])\.section(\s|,|$)", sel):
        s = size(body)
        if s and small(s):
            print(f"  ✗ ① 包装 `.section` 的 font-size = {show(s)} < 正文 ⇒ 标题跟着变小 ✗")
            bad += 1

titles = [b for sel, b in rules if ".section-title" in sel]
if not titles:
    print("  ✗ 找不到 `.section-title` 规则 ⇒ 判据无从下手 ✗")
    bad += 1
else:
    b = titles[0]
    s = size(b)
    d = dim(b)
    has_color_dim = re.search(r"color:\s*[^;]*descriptionForeground", b) is not None
    print(
        f"  实测 `.section-title`：font-size = {show(s)} · 正文 = 1em（≈ 13.0px）"
        f" · opacity = {f'{d:g} ✗' if d is not None else '无 ✓'}"
        f" · text-transform = {'有 ✗' if 'text-transform' in b else '无 ✓'}"
        f" · letter-spacing = {'有 ✗' if 'letter-spacing' in b else '无 ✓'}"
        f" · 降档色（descriptionForeground）= {'有 ✓' if has_color_dim else '无 ✗'}"
    )
    # ① 标题字号 ≥ 正文（未声明 = 继承正文 ⇒ 也算 ≥ ✓）
    if s and small(s):
        print(f"  ✗ ① 分区标题 {show(s)} **小于正文**（1em / 13px）⇒ 层级更高、视觉更弱 ✗")
        bad += 1
    # ② 标题不叠 opacity（降一档只用颜色）
    if d is not None:
        print(f"  ✗ ② 分区标题叠了 opacity:{d:g} ⇒ 双重压暗（颜色已经暗一档）✗")
        bad += 1
    if not has_color_dim:
        print("  ✗ ② 分区标题没有用颜色降档（`color: …descriptionForeground`）⇒ 与正文抢层级 ✗")
        bad += 1
    # ③ 中文标题下无 uppercase / letter-spacing
    if "text-transform" in b:
        print("  ✗ ③ 分区标题带 text-transform（uppercase 对中文完全无效）✗")
        bad += 1
    if "letter-spacing" in b:
        print("  ✗ ③ 分区标题带 letter-spacing（把中文标题拉得更松散）✗")
        bad += 1

# ① 横向排查（用户要求「不许只改这一处」）：所有「小于正文」的规则，其中叠 opacity 的必须为 0
small_rules = [(sel, b, size(b)) for sel, b in rules if size(b) and small(size(b))]
double_dim = [(sel, b, s) for sel, b, s in small_rules if dim(b) is not None]
print(f"  横向实测：{len(rules)} 条规则里 font-size 小于正文的有 {len(small_rules)} 条，"
      f"其中**又叠 opacity** 的有 {len(double_dim)} 条")
if not small_rules:  # 防空转：一条都没命中说明扫描逻辑坏了 ✗
    print("  ✗ 横向扫描命中数 = 0 ⇒ 没扫到 ≠ 绿（判据空转）✗")
    bad += 1
for sel, b, s in double_dim:
    print(f"  ✗ ① 横向：`{sel}` font-size {show(s)} 又叠 opacity:{dim(b):g} ⇒ 双重压暗 ✗")
    bad += 1

# ③ 横向：整个 media CSS 不许再有 `text-transform`（中文 UI 里它只会误伤拉丁字母）
tt = [sel for sel, b in rules if "text-transform" in b]
if tt:
    print(f"  ✗ ③ 横向：还有 {len(tt)} 条规则带 text-transform：{', '.join(tt[:3])} ✗")
    bad += 1

sys.exit(1 if bad else 0)
PY
)"
css_rc=$?
printf '%s\n' "$css_out"
if [ "$css_rc" -eq 2 ]; then
  echo "G-66: CSS 扫描形状不对 ⇒ 环境不对 ✗（exit 2）"
  exit 2
elif [ "$css_rc" -ne 0 ]; then
  fails=$((fails + 1))
fi

# ── ④ 两个守卫脚本：必须 exit 0；`--selftest` 反向验证也必须过 ────────────────────
run_guard() { # run_guard <名字> <脚本> [参数...]
  local name="$1" script="$2"; shift 2
  local out rc
  out="$(cd "$ROOT" && python3 "$script" "$@" 2>&1)"
  rc=$?
  if [ "$rc" -eq 2 ]; then
    echo "G-66: $name exit 2（环境不对）⇒ exit 2"
    exit 2
  elif [ "$rc" -eq 0 ]; then
    note "守卫 ✓ $name exit 0"
  else
    note "守卫 ✗ $name exit ${rc}（应 0）"
    printf '%s\n' "$out" | tail -3 | sed 's/^/      /'
    fails=$((fails + 1))
  fi
}
run_guard "check-infoview-hierarchy.py" "scripts/check-infoview-hierarchy.py"
run_guard "check-infoview-hierarchy.py --selftest" "scripts/check-infoview-hierarchy.py" --selftest
run_guard "audit-infoview-css.py" "scripts/audit-infoview-css.py"

# ── ⑤ 反向臂：CSS **副本**改坏 ⇒ 两个守卫都必须判红（真树一个字不动 ✓）──────────
tmp="$(mktemp -d)"   # 清理在 `finish` 里（⚠ 不要再挂第二个 EXIT trap ✗）
mkdir -p "$tmp/scripts" "$tmp/editor/vscode/media"
cp "$HIER" "$AUDIT" "$tmp/scripts/"
cp "$CSS" "$tmp/editor/vscode/media/infoview.css"
MIRROR_CSS="$tmp/editor/vscode/media/infoview.css"

mirror_guard() { # mirror_guard <脚本相对路径> ⇒ 打印退出码
  local rc
  (cd "$tmp" && python3 "$1" >/dev/null 2>&1)
  rc=$?
  printf '%s' "$rc"
}

# 反向臂基线：**没改坏**的副本必须两个都绿（否则"红"是路径/环境造出来的假象 ✗）
base_h="$(mirror_guard scripts/check-infoview-hierarchy.py)"
base_a="$(mirror_guard scripts/audit-infoview-css.py)"
if [ "$base_h" = "0" ] && [ "$base_a" = "0" ]; then
  note "反向臂基线 ✓ 未改坏的副本：hierarchy=$base_h · audit=$base_a"
else
  note "反向臂基线 ✗ 未改坏的副本就没绿（hierarchy=$base_h · audit=${base_a}）⇒ 反向臂不可信 ✗"
  fails=$((fails + 1))
fi

# 三种坏形态（都是历史/用户要求点名的退化）：① 原样 0.8em+opacity+uppercase+letter-spacing
# ② 字号回 1em 但**仍用 opacity 降档**（2026-10-07 审计实测：原先 gate 里那条守卫**不咬** ✗）
# ③ `px` 形态 12px（原先**两条守卫都不咬** ✗）
for variant in original 1em-opacity px-12; do
  python3 - "$CSS" "$MIRROR_CSS" "$variant" <<'PY'
import re
import sys
from pathlib import Path

src, dst, variant = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
bad = {
    "original": (".section-title {\n  margin: 0 0 4px;\n  font-size: 0.8em;\n  font-weight: 600;\n"
                 "  text-transform: uppercase;\n  letter-spacing: 0.05em;\n  opacity: 0.7;\n}"),
    "1em-opacity": (".section-title {\n  margin: 0 0 4px;\n  font-size: 1em;\n  font-weight: 600;\n"
                    "  opacity: 0.7;\n  color: var(--vscode-descriptionForeground);\n}"),
    "px-12": ".section-title {\n  margin: 0 0 4px;\n  font-size: 12px;\n  font-weight: 600;\n}",
}[variant]
new, n = re.subn(r"\.section-title\s*\{[^}]*\}", bad, src.read_text(encoding="utf-8"), count=1)
assert n == 1, "没找到 .section-title 规则"
dst.write_text(new, encoding="utf-8")
PY
  h="$(mirror_guard scripts/check-infoview-hierarchy.py)"
  a="$(mirror_guard scripts/audit-infoview-css.py)"
  if [ "$h" = "1" ] && [ "$a" = "1" ]; then
    note "反向臂 ✓ 改坏（${variant}）⇒ hierarchy=1 · audit=1（都判红 ✓）"
  else
    note "反向臂 ✗ 改坏（${variant}）却没判红：hierarchy=$h · audit=$a ⇒ 守卫咬不住 ✗"
    fails=$((fails + 1))
  fi
done

# ── 结论（**显式**结论才作数 —— 见上面的 `finish` 陷阱）──────────────────────────
if [ "$fails" -eq 0 ]; then
  echo "G-66: ✓ 四条要求全部满足（标题 ≥ 正文 · 不叠 opacity、只用颜色降档 · 无 uppercase/letter-spacing · 横向 0 处双重压暗 · 两个守卫 exit 0 且反向臂判红）⇒ 缺口已修（本脚本 exit 1）"
  verdict="satisfied"
  exit 1
fi
echo "G-66: ✗ $fails 项不满足 ⇒ 缺口仍在（本脚本 exit 0）"
verdict="present"
exit 0
