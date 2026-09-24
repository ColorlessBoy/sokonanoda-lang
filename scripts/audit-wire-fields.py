#!/usr/bin/env python3
"""A∖B 对账：扩展从 LSP 的 wire 载荷里**读**、但 LSP **从不发**的字段。

为什么需要它（2026-09-24 的教训）：扩展是"**有就渲染**"的宽容实现 ⇒
缺字段是**静默降级**（那一行直接不出现），**e2e 天然抓不到** ✗。
R-1（`decl.value_runs` 漏映射）就是这么潜伏的 ✓。这条脚本是它的守卫 ✓。

退出码：0 = 无缺失 ✓；1 = 存在"读了但不发"的字段 ✗。
用法：`python3 scripts/audit-wire-fields.py`（在仓库根跑 ✓）
"""
import re
import sys

PROTO = "crates/lsp/src/protocol.rs"
IV = "editor/vscode/media/infoview.js"
EX = "editor/vscode/extension.js"


def fields(src: str, struct: str) -> set[str]:
    m = re.search(r"struct %s \{(.*?)\n\}" % struct, src, re.S)
    if not m:
        sys.stderr.write(f"audit: 找不到结构体 {struct}\n")
        sys.exit(2)
    return set(re.findall(r"pub\(crate\)\s+(\w+):", m.group(1)))


def reads(path: str, var: str, lo: int = 1, hi: int = 10**9) -> dict[str, list[int]]:
    out: dict[str, list[int]] = {}
    for i, line in enumerate(open(path, encoding="utf-8").read().split("\n"), 1):
        if lo <= i <= hi:
            for k in re.findall(r"\b%s\s*(?:&&\s*)?\??\.\s*(\w+)" % re.escape(var), line):
                out.setdefault(k, []).append(i)
    return out


def main() -> int:
    proto = open(PROTO, encoding="utf-8").read()
    wire = set()
    for s in ("GoalDeclInfo", "StateAtResponse", "StateDeclInfo", "StateGoalInfo",
              "GoalsResponse", "HoleInfo", "SubGoalInfo", "RunInfo", "GoalBinderInfo"):
        wire |= fields(proto, s)

    read: dict[str, list[int]] = {}
    for k, v in reads(IV, "decl", 236, 292).items():       # 声明卡片
        read.setdefault(k, []).extend(v)
    for var in ("msg", "state", "binder", "run"):          # 目标面板
        for k, v in reads(IV, var, 150, 234).items():
            read.setdefault(k, []).extend(v)
    for k, v in reads(EX, "decl").items():                 # 练习树
        read.setdefault(k, []).extend(v)
    for var in ("cursor", "hole", "binder"):
        for k, v in reads(EX, var).items():
            read.setdefault(k, []).extend(v)
    for k, v in reads(EX, "state").items():
        if k in ("goal", "binders"):            # 只看 stateAt 的兜底字段
            read.setdefault(k, []).extend(v)
    for k, v in reads(EX, "response").items():
        if k in ("uri", "decls", "version"):    # 只看 soko/goals 的响应字段
            read.setdefault(k, []).extend(v)

    # JS 自有的局部字段（不是 wire 字段）。
    local = {"start", "line", "length", "text", "kind"}
    missing = {k: sorted(set(v)) for k, v in read.items() if k not in wire and k not in local}
    print("MISSING (扩展读了 / LSP 从不发):", missing or "NONE ✓")
    return 1 if missing else 0


if __name__ == "__main__":
    raise SystemExit(main())
