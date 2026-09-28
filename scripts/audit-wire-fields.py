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
# **front 侧的 wire 结构**（E30，2026-09-27 接上）：`soko/project` 的载荷是
# `ProjectResponse.project`，而 `ProjectView`/`ProjectModule`/`ProjectCounts`/
# `ProjectArtifacts` 定义在 **front**（`crates/front/src/query/types.rs`）——
# 只解析 `protocol.rs` 会"解析不到 ≠ 没发" ✗（审计 #17 早就点名了这一点，
# `project-tree.js` 因此一直不在消费点表里 ✗）。E30 让 Infoview 也读这些字段
# ⇒ 顺手把这条缝补上 ✓（同一个 commit，见 `docs/PLAN-0.74-0.79.md` §E30）。
FRONT_TYPES = "crates/front/src/query/types.rs"
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
LOCAL = {
    "start", "line", "length", "text", "kind", "phase", "label", "percent",
    # **DOM 方法**（2026-09-28 G-67 重设计时实测踩到 ✓）：`row.appendChild(...)`
    # 里的 `row` 是 `document.createElement` 出来的**节点** ✗ 不是 wire 载荷 ⇒
    # 它当然不在任何 `*Info` 结构体里 ✓。**为什么可以放 LOCAL**：`appendChild`
    # **不可能**是 wire 字段（它没有对应的 Rust 结构体字段名）⇒ 全局放行**不会**
    # 关掉任何真判据 ✓（与 `decls` 那种"两边同名"的情形**不同** ✗ —— 那一条
    # 见下面 `ENVELOPE` 的注释：那种必须按 `(path, var)` 定点排除 ✓）。
    "appendChild", "setAttribute", "addEventListener",
}

# **扩展 ↔ webview 的信封**（`infoview.js` 里 `window.addEventListener("message")` 的那个
# `msg`）：它**不是 LSP wire** —— 走的是扩展**自定**的 `INFOVIEW_PROTOCOL`，字段是
# `protocol`/`type`/`fontScale`/`decls`… ⇒ 与 `phase`/`label`/`percent` **同性质** ✓。
#
# ⚠ **不能**把它们塞进 `LOCAL` ✗✓：`decls` **同时**是 `GoalsResponse.decls`（**真的 LSP wire
# 字段**，见下面 `collect(EX, "response", ("GoalsResponse",), only=("uri","decls","version"))`）
# ⇒ 全局放行会**顺手把那条判据也关掉** ✗（这正是"为让判据变绿而放宽守卫"的反面教材 ✓）。
# ⇒ 按 `(path, var)` **定点排除** ✓。
ENVELOPE = {"protocol", "type", "fontScale", "decls"}


def fields(src: str, struct: str) -> set[str]:
    m = re.search(r"struct %s \{(.*?)\n\}" % struct, src, re.S)
    if not m:
        sys.stderr.write(f"audit: 找不到结构体 {struct}\n")
        sys.exit(2)
    # `pub(crate)`（LSP 侧）与 `pub`（front 侧）两种可见性都要认 ✓。
    return set(re.findall(r"pub(?:\(crate\))?\s+(\w+):", m.group(1)))


def wire_struct(struct: str) -> set[str]:
    """结构体定义在 `protocol.rs`（LSP）或 `query/types.rs`（front）——两处都找 ✓。"""
    for path in (PROTO, FRONT_TYPES):
        src = open(path, encoding="utf-8").read()
        if re.search(r"struct %s \{" % struct, src):
            return fields(src, struct)
    sys.stderr.write(f"audit: 两个文件里都找不到结构体 {struct}\n")
    sys.exit(2)


def strip_line_comment(line: str) -> str:
    """去掉 `//` 行注释（**不动字符串**：模板串里的 `${decl.name}` 是**真的**读取点 ✓）。

    ⚠ 顺序很重要：**先去掉 `//`，再找 `/*`** ✗✓ —— 反过来会把散文里的
    `compiled/*.tmp`（出现在一行 `//` 注释里）当成块注释开头 ⇒ `in_block` 卡住
    ⇒ **整份文件后半段被静默跳过**（实测：infoview.js 的 `decl.value_runs`
    因此读不到，`--selftest` 也就不咬了 ✗）。
    """
    i = line.find("//")
    if i >= 0 and line[:i].count('"') % 2 == 0 and line[:i].count("'") % 2 == 0:
        return line[:i]
    return line


def reads(path: str, var: str, lo: int = 1, hi: int = 10**9) -> dict[str, list[int]]:
    """`path` 里 `var.<field>` 的读取点。

    ⚠ **注释与字符串里的"读取"不是读取** ✗✓（2026-09-27 E30 接 `project-tree.js`
    时实测到三个假阳性）：① 一句散文 `answer. Answers for another document` 被
    `\\.\\s*(\\w+)` 当成 `answer.Answers` ✗；② 字符串字面量
    `"sokonanoda.project.module"` 被当成 `project.module` ✗；③ Node 的
    `module.exports` 被当成 `ProjectModule.exports` ✗。
    ⇒ 扫之前**去注释**，命中处**在引号里就丢掉**（奇偶引号计数），`exports`
    另在调用点定点排除 —— **收紧扫描器，而不是放宽判据** ✓。
    """
    out: dict[str, list[int]] = {}
    in_block = False
    for i, raw in enumerate(open(path, encoding="utf-8").read().split("\n"), 1):
        line = raw
        if in_block:
            end = line.find("*/")
            if end < 0:
                continue
            line = line[end + 2 :]
            in_block = False
        line = strip_line_comment(line)
        start = line.find("/*")
        if start >= 0:
            end = line.find("*/", start + 2)
            if end < 0:
                line = line[:start]
                in_block = True
            else:
                line = line[:start] + line[end + 2 :]
        if not (lo <= i <= hi):
            continue
        for m in re.finditer(r"\b%s\s*(?:&&\s*)?\??\.\s*(\w+)" % re.escape(var), line):
            # 命中点**在引号里** ⇒ 那是字符串，不是读取 ✗。
            before = line[: m.start()]
            if before.count('"') % 2 == 1 or before.count("'") % 2 == 1:
                continue
            out.setdefault(m.group(1), []).append(i)
    return out


def main() -> int:
    proto = open(PROTO, encoding="utf-8").read()
    wire = {
        name: wire_struct(name)
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
            # E30：`soko/project` 一族（定义在 front 的 query/types.rs ✓）。
            "ProjectResponse",
            "ProjectView",
            "ProjectModule",
            "ProjectCounts",
            "ProjectArtifacts",
        )
    }

    # **反向验证注入点**（AGENTS 硬要求：守卫必须能咬住已知历史 bug ✓）：设了这个环境
    # 变量就**模拟 R-1** —— 把 `value_runs` 从"LSP 发送侧"抹掉 ✓，后文必须把它报成
    # MISSING ✓（`--selftest` 用子进程跑这一支 ✓；咬不住的守卫等于没有 ✓）。
    if os.environ.get("SOKO_WIRE_SELFTEST") == "1":
        if "GoalDeclInfo" in wire:
            wire["GoalDeclInfo"] = {f for f in wire["GoalDeclInfo"] if f != "value_runs"}
        # ⚠ **扩到 binder/run**（E00 切片 B 第 4 条：原来只覆盖 9 个结构体里的 1 个 ✗）
        if "GoalBinderInfo" in wire:
            wire["GoalBinderInfo"] = {f for f in wire["GoalBinderInfo"] if f != "ty_runs"}
        
    missing: dict[str, list[str]] = {}

    def collect(path, var, structs, only=None, lo=1, hi=10**9, skip=None):
        """`path` 里 `var.<field>` 的每个读取点：字段必须在 `structs` 里。

        `only` 限定只看哪几个字段名（用在同名变量承载多种载荷的地方）；
        `skip` **定点排除**若干字段名（同一个变量名承载**两种载荷**时用，见 `ENVELOPE`）。"""
        allowed = set().union(*(wire[s] for s in structs))
        where = os.path.basename(path)
        for k, lines in reads(path, var, lo, hi).items():
            if k in allowed or k in LOCAL or (skip and k in skip):
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
    # ⚠ **不许写死行区间** ✗✓（2026-09-27 E00 切片 B 的 P0-1，**已实测复现** ✓）：
    #   这里原来写死 `lo=150, hi=234` ⇒ `binder`/`run` 的读取点
    #   （实测 binder 在 236/240/241、run 在 54/55）**全在区间外** ✗ ⇒
    #   **抹掉 `GoalBinderInfo.ty_runs` / `RunInfo.kind` 仍印 `NONE ✓` 且 exit 0** ✗
    #   —— 与紧邻上面那句「**扫整份文件**（不写死行区间：区间会随改动漂移，新加的渲染
    #   代码一落到区间外守卫就瞎了）」**自相矛盾** ✗✓。实测（先判红）：
    #   同时抹 ty_runs + kind ⇒ 输出里**只有** `value_runs`，binder/run 一个字都没有 ✗。
    for var in ("msg", "state", "binder", "run"):
        collect(
            IV,
            var,
            # `msg` 兼作 webview 信封，承载多种载荷 ⇒ 它的允许集合要含
            # `ProjectResponse`（E30 的 `{type:"project", project, reason}` ✓）。
            ("StateAtResponse", "StateGoalInfo", "GoalBinderInfo", "RunInfo", "ProjectResponse")
            if var == "msg"
            else ("StateAtResponse", "StateGoalInfo", "GoalBinderInfo", "RunInfo"),
            skip=ENVELOPE if var == "msg" else None,   # `msg` 兼作 webview 信封 ⇒ 定点排除 ✓
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
    # **E30：项目一族**（`soko/project`）。`msg.project`/`msg.reason` 已由上面那条
    # `msg` 的允许集合覆盖（ProjectResponse ✓）；这里是渲染器里的局部变量：
    collect(IV, "project", ("ProjectView",))
    collect(IV, "mod", ("ProjectModule",))
    collect(IV, "counts", ("ProjectCounts",))
    collect(IV, "artifacts", ("ProjectArtifacts",))
    # **项目树**（`project-tree.js`）—— 审计 #17 点名的那个"7 处 wire 读取永远不被扫" ✗；
    # 现在 front 侧结构能解析了，把它接上 ✓（`project` 是 ProjectView、`module` 是
    # ProjectModule、`answer` 是 ProjectResponse 一族）。
    collect(PT, "answer", ("ProjectResponse",))
    collect(PT, "project", ("ProjectView",))
    # `skip={"exports"}`：`module.exports = {…}`（Node 的模块导出语法）不是
    # `ProjectModule.exports` ✗ —— 定点排除，**不是**放宽（`module.<其它字段>`
    # 仍然逐个受检 ✓）。
    collect(PT, "module", ("ProjectModule",), skip={"exports"})
    collect(PT, "counts", ("ProjectCounts",))


    print(
        "MISSING (扩展读了 / LSP 从不发):",
        {k: sorted(set(v)) for k, v in missing.items()} or "NONE ✓",
    )
    return 1 if missing else 0


if __name__ == "__main__":
    # ⚠ **不许静默回落** ✗✓（E00 切片 B 实测 ✓）：这几个脚本原来用 `"--x" in argv`
    # **子串**判模式 ⇒ **错拼的参数被静默忽略、回落成全量检查并 exit 0** ✗ ⇒
    # 表现是「**自检没跑，退出码却是绿的**」✗（我上次"它没有自检"的错结论就是这么来的 ✓）。
    # ⇒ 未知参数一律 **exit 2**（只有 `notation-lint.py` 本来就用 argparse、是对的 ✓）。
    unknown = [a for a in sys.argv[1:] if a.startswith("-") and a not in {"--selftest"}]
    if unknown:
        print(f"✗ 未知参数 {unknown} ⇒ 拒绝执行（**不许静默回落全量检查** ✗）", file=sys.stderr)
        raise SystemExit(2)
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
        # ⚠ 断言**三个**名字都出现在 MISSING 里（原来只断言 value_runs 一个 ✗）——
        #   E00 切片 B 第 4 条：反向验证只抹 9 个结构体里的 1 个 ⇒ 其余 8 个的
        #   "读取点在不在扫描域内"**从来没被验证过** ✗（binder/run 就是这样瞎了 ✓）。
        # ⚠ **不**断言 `kind` ✗：它在 `LOCAL` 里、**按设计**豁免 ✓（我第一版把它写进
        #   期望，实测当场判红 ✓ —— 是**我的期望错**，不是守卫错）。
        want = ("value_runs", "ty_runs")
        absent = [n for n in want if n not in probe.stdout]
        if probe.returncode == 1 and not absent:
            print("wire-fields self-test: OK（抹掉 value_runs + ty_runs ⇒ **两个都被抓到** ✓"
                  " —— 能咬住 R-1，且 binder 不再瞎 ✓）")
            raise SystemExit(0)
        print(
            f"wire-fields self-test: FAIL（抹掉后没报全 ✗；缺 {absent}；"
            f"exit={probe.returncode}, out={probe.stdout.strip()[:200]!r}）",
            file=sys.stderr,
        )
        raise SystemExit(1)
    raise SystemExit(main())
