#!/usr/bin/env bash
# G-49 自断言复现：**类型错误的报错文本不许外泄裸 de Bruijn 编号**（`$4` ✗），
# 而且要**点名**到 binder（`$4(β)` ⇒ 「第 4 个绑元（β）」✓）。
#
# 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
#   0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（需要人看）
#
# ⚠ **判据不是那条 `.sokonanoda` 的退出码** ✗：夹具**本身有真类型错误**
#   （对照组才 checked ✓）⇒ 报错文本修好之后它**仍然**判红 ✓。本条的病在**文本**，
#   所以本脚本断言三向：
#     ① 夹具**仍被拒**（`kernel-rejected` + exit≠0 ✓ —— 错是真的 ✓）
#        **且**诊断文本里**没有裸 `$N`**（`$` 紧跟数字的独立形态 ✓）
#        **且**报错**点名**到 binder（`β` / `α` ✓）；
#     ② **对照组**（各用各的论域，同一夹具里的 `Set.eq3`）⇒ `decl.checked` 且
#        **只有那一条**诊断（对照组分不到诊断 ✓）；
#     ③ **防空转**：诊断条数 > 0 且真的扫过文本（"一条都没扫到"不算绿 ✗）。
#
# 换掉旧复现件（`G49-type-error-leaks-de-bruijn.sokonanoda`）的原因：
# 它走 `.sokonanoda` 判据 = **退出码**，而本条的病在**文本** ⇒ 修好之后它照样 exit 1
# ⇒ 判据永远"与 open 自洽"、**永远咬不住** ✗（守卫空转 ✗）。旧文件现在只当**夹具**。

set -u
cd "$(dirname "$0")/../../.." || exit 2
ROOT="$PWD"
FIXTURE="$ROOT/docs/gaps/repro/G49-type-error-leaks-de-bruijn.sokonanoda"

if [ ! -f "$FIXTURE" ]; then
  echo "环境异常：夹具不存在：$FIXTURE" >&2
  exit 2
fi

OUT="$(scripts/soko grade "$FIXTURE" --json 2>&1)"
CODE=$?

printf '%s\n' "$OUT" | CODE="$CODE" python3 -c '
import json, os, re, sys

code = int(os.environ.get("CODE", "0"))
ev = []
for line in sys.stdin.read().splitlines():
    line = line.strip()
    if line.startswith("{"):
        try:
            ev.append(json.loads(line))
        except json.JSONDecodeError:
            pass

diags = [e for e in ev if e.get("type") == "diagnostic"]
checked = [e.get("name") for e in ev if e.get("type") == "decl.checked"]

# 形状异常（环境/夹具坏了）⇒ exit 2：不许被读成"已修" ✗。
if not ev:
    print("环境异常：判卷没有输出任何事件（跑不起来）", file=sys.stderr)
    sys.exit(2)
if not diags:
    print("形状异常：夹具应当判红（它本身是真类型错误），却一条诊断都没有", file=sys.stderr)
    sys.exit(2)
if code == 0:
    print("形状异常：夹具被接受了（真类型错误应当被拒）—— 判定被放宽了？", file=sys.stderr)
    sys.exit(2)

# ① 裸 de Bruijn：只认「`$` 紧跟数字」的**独立形态**（正常文本里的 `$` 不算 ✗）。
BARE = re.compile(r"\$[0-9]")
scanned = 0
leaks = []
texts = []
for d in diags:
    for field in ("message", "hint"):
        t = d.get(field) or ""
        scanned += len(t)
        texts.append(t)
        if BARE.search(t):
            leaks.append("%s=%r" % (field, t))

rejected = [d for d in diags if d.get("code") == "kernel-rejected"]
named = any(("β" in t and "α" in t) for t in texts)

ok_reject = code != 0 and bool(rejected)
ok_no_leak = not leaks
ok_named = named
ok_control = ("Set.eq3" in checked) and len(diags) == 1
ok_scan = scanned > 0 and len(diags) > 0

print("① 仍被拒：exit=%d · kernel-rejected=%d 条 · 裸 `$N` 命中=%d 处"
      % (code, len(rejected), len(leaks)))
print("   点名到 binder（β/α）= %s" % named)
print("② 对照组 Set.eq3 checked=%s · 诊断条数=%d（应当只有那一条）"
      % ("Set.eq3" in checked, len(diags)))
print("③ 防空转：诊断 %d 条 · 扫过 %d 字符" % (len(diags), scanned))
if leaks:
    print("   漏出：%s" % " ｜ ".join(leaks), file=sys.stderr)

if ok_reject and ok_no_leak and ok_named and ok_control and ok_scan:
    print("结论：报错文本已是人话（点名到 binder）、判定没变宽 ⇒ G-49 已修。")
    sys.exit(1)
print("结论：诊断文本仍外泄内核记号（或判定/对照坏了）⇒ G-49 仍在。")
sys.exit(0)
'
