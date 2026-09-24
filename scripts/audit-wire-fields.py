#!/usr/bin/env python3
"""A∖B 对账：扩展从 LSP 的 wire 载荷里**读**、但 LSP **从不发**的字段。

为什么需要它（2026-09-24 的教训）：扩展是"**有就渲染**"的宽容实现 ⇒
缺字段是**静默降级**（那一行直接不出现），**e2e 天然抓不到** ✗。
R-1（`decl.value_runs` 漏映射）就是这么潜伏的 ✓。这条脚本是它的守卫 ✓。

**按消费者分组**（T-A5 补强）：每个读取点只许落在它**真正会拿到**的那几个结构体
里。以前是"全体结构体取并集"✗ —— 同名字段会在别的结构体里**顶包**：`goal_runs`
也是 `soko/stateAt` 的字段 ⇒ 声明侧漏了它，并集判定**不报** ✗（T-A5 实测：回退
`GoalDeclInfo` 的两个新字段，只报得出 `goals_runs`）。分组之后两个都报 ✓。

退出码：0 = 无缺失 ✓；1 = 存在"读了但不发"的字段 ✗。
用法：`python3 scripts/audit-wire-fields.py`（在仓库根跑 ✓）
"""
import os
import re
import sys

PROTO = "crates/lsp/src/protocol.rs"
IV = "editor/vscode/media/infoview.js"
EX = "editor/vscode/extension.js"

# JS 自有的局部字段（不是 wire 字段）。
LOCAL = {"start", "line", "length", "text", "kind"}


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
    wire = {
        name: fields(proto, name)
        for name in (
            "GoalDeclInfo",
            "StateAtResponse",
            "StateDeclInfo",
            "StateGoalInfo",
            "GoalsResponse",
            "HoleInfo",
            "SubGoalInfo",
            "RunInfo",
            "GoalBinderInfo",
        )
    }

    missing: dict[str, list[str]] = {}

    def collect(path, var, structs, only=None, lo=1, hi=10**9):
        """`path` 里 `var.<field>` 的每个读取点：字段必须在 `structs` 里。

        `only` 限定只看哪几个字段名（用在同名变量承载多种载荷的地方）。"""
        allowed = set().union(*(wire[s] for s in structs))
        where = os.path.basename(path)
        for k, lines in reads(path, var, lo, hi).items():
            if k in allowed or k in LOCAL:
                continue
            if only is not None and k not in only:
                continue
            missing.setdefault(k, []).extend(f"{where}:{i}" for i in lines)

    # 声明条目（`soko/goals` 的 decls[]）：`renderGoals` 的 `msg.decl` 是
    # StateDeclInfo、`renderDecls` 的循环变量是 GoalDeclInfo —— 同一个变量名，
    # 两个结构体，所以这两个都算"会拿到"。**扫整份文件**（不写死行区间：区间会
    # 随改动漂移，新加的渲染代码一落到区间外守卫就瞎了 —— T-A5 加目标行时踩到）。
    collect(IV, "decl", ("GoalDeclInfo", "StateDeclInfo"))
    # 目标面板（`soko/stateAt`）及其嵌套结构。
    for var in ("msg", "state", "binder", "run"):
        collect(
            IV,
            var,
            ("StateAtResponse", "StateGoalInfo", "GoalBinderInfo", "RunInfo"),
            lo=150,
            hi=234,
        )
    # 练习树：`soko/goals` 的 decls[] + `soko/stateAt` + `soko/nextHole`。
    collect(EX, "decl", ("GoalDeclInfo",))
    collect(EX, "cursor", ("StateAtResponse",))
    collect(EX, "hole", ("HoleInfo",))
    collect(EX, "binder", ("GoalBinderInfo",))
    # `state`/`response` 只数那两个请求自己的兜底字段：同名变量也承载别的载荷
    # （`soko/hints` 的 `response.hints`、树视图自己的 `state`）。
    collect(EX, "state", ("StateAtResponse",), only=("goal", "binders"))
    collect(EX, "response", ("GoalsResponse",), only=("uri", "decls", "version"))

    print(
        "MISSING (扩展读了 / LSP 从不发):",
        {k: sorted(set(v)) for k, v in missing.items()} or "NONE ✓",
    )
    return 1 if missing else 0


if __name__ == "__main__":
    raise SystemExit(main())
