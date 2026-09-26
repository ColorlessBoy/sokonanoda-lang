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
PT = "editor/vscode/project-tree.js"
# **审计 #17 的现状（2026-09-25 实测，别照"看起来对"的写法接 ✓）**：
# `project-tree.js` 读 `project.{root,manifest,counts,modules,diagnostics,requires_warning,entry,module}`
# 与 `module.<12 个字段>`，而它**不在消费点表里** ✗（"有就渲染"的宽容实现 ⇒ 停发字段静默降级 ✓）。
# 我试过直接把 `ProjectResponse` 接进来 ⇒ 报 8 个 MISSING ✗，但**实测是假阳性** ✓：
#   `soko query project --file …` 的响应是 `{project, reason}` ✓（真二进制实测 ✓），
#   那 8 个字段**确实发送**了 ✓ —— 只是嵌在 `ProjectResponse.project`
#   （**front 侧** `crates/front/src/query/types.rs::ProjectView` / `ModuleView` ✓）里，
#   而本守卫只解析 `crates/lsp/src/protocol.rs` ✗ ⇒ **解析不到 ≠ 没发** ✓。
# ⇒ 要真正覆盖它，必须先让 `fields()` 也能读 front 侧那个文件 ✓（下一步 ✓）；
#   在那之前**不要**接 ✗（常红的守卫比没有守卫更糟 ✓）。

# JS 自有的局部字段（不是 wire 字段）。
#
# **2026-09-26 加入 `phase`/`label`/`percent`**（编译进度 P1/P3）：它们由**扩展自己
# 合成**——LSP 发的是标准的 `$/progress`（`WorkDoneProgressBegin/Report/End`），
# 扩展把它翻成 `{type:"progress", phase, label, percent}` 再转给 Infoview ✓。
# ⇒ 它**不是** LSP wire 字段，登记在这里是**如实**而不是放宽 ✗：
# 本守卫的契约是"扩展读了 / **LSP** 从不发"，而这条缝里 LSP 本来就不该发这三个名字 ✓。
LOCAL = {"start", "line", "length", "text", "kind", "phase", "label", "percent"}


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

    # **反向验证注入点**（AGENTS 硬要求：守卫必须能咬住已知历史 bug ✓）：设了这个环境
    # 变量就**模拟 R-1** —— 把 `value_runs` 从"LSP 发送侧"抹掉 ✓，后文必须把它报成
    # MISSING ✓（`--selftest` 用子进程跑这一支 ✓；咬不住的守卫等于没有 ✓）。
    if os.environ.get("SOKO_WIRE_SELFTEST") == "1":
        if "GoalDeclInfo" in wire:
            wire["GoalDeclInfo"] = {f for f in wire["GoalDeclInfo"] if f != "value_runs"}

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
    if "--selftest" in sys.argv:
        # **反向验证**（硬要求 ✓）：抹掉 `value_runs` ⇒ 守卫**必须判红** ✗⇒✓。
        # 为什么值得一条专门通道：这个守卫是**唯一**能咬 R-1（`value_runs` 漏映射 ⇒
        # Infoview 静默降级）的东西 ✓，而它此前**从不自动跑**（审计 #3 ✓）。
        import subprocess

        env = dict(os.environ, SOKO_WIRE_SELFTEST="1")
        probe = subprocess.run(
            [sys.executable, os.path.abspath(__file__)],
            capture_output=True,
            text=True,
            env=env,
        )
        caught = probe.returncode == 1 and "value_runs" in probe.stdout
        if caught:
            print("wire-fields self-test: OK（抹掉 value_runs ⇒ 被抓到 ✓ —— 能咬住 R-1 ✓）")
            raise SystemExit(0)
        print(
            "wire-fields self-test: FAIL（抹掉 value_runs 却没报 ✗ ⇒ 守卫咬不住 R-1 ✓；"
            f"exit={probe.returncode}, out={probe.stdout.strip()[:200]!r}）",
            file=sys.stderr,
        )
        raise SystemExit(1)
    raise SystemExit(main())
