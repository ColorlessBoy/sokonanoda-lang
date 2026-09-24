#!/usr/bin/env python3
"""执行清单工具：把 `docs/design/vscode-editor-feedback-plan.md` §13 的线性清单
变成可操作的"下一条"。

为什么要有它：那份计划有 100+ 个环节、2000+ 行，按章节读找不到"我该做哪一条"。
`next` 把**下一条未勾的环节**连同它的**完整规格**（判据 / 依赖 / 风险）打出来，
可以直接粘给实现者——与 `scripts/gap.py next` 同一个形状。

用法：
  python3 scripts/plan.py next [--json]   # 下一条要做的环节（含完整规格 + 是否该 bump）
  python3 scripts/plan.py list            # 全部环节 + 进度（⬆ 标出发版点）
  python3 scripts/plan.py bumps           # 全部发版点（13 个）与剩余
  python3 scripts/plan.py done T-001      # 勾掉一条（写回 §13）
  python3 scripts/plan.py check           # 清单与详细规格不漂移（进 gate）

退出码：0 = 正常；1 = `check` 发现漂移；2 = 用法错误。
"""

from __future__ import annotations

import json
import os
import pathlib
import re
import sys

# 默认计划 = E2（`docs/design/e2-plan.md`；E1 的 §13 已于 2026-09-24 全部勾完 ✓）。
# 用 `SOKO_PLAN=<路径>` 可指向别的计划文件（例如回看 E1 的清单 ✓）。
PLAN = pathlib.Path(
    os.environ.get("SOKO_PLAN")
    or (pathlib.Path(__file__).resolve().parent.parent / "docs" / "design" / "e2-plan.md")
)

# §13 的一条：`- [ ] \`T-001\` 标题`（标题里可以再带反引号/加粗）
CHECK_RE = re.compile(r"^- \[([ xX])\] `(T-[A-Z]?\d+[a-z]?)` (.*)$")
# 紧跟其后的发版标记：`  - ⬆ **BUMP**：\`patch\` —— 理由`
BUMP_RE = re.compile(r"^\s+- ⬆ \*\*BUMP\*\*：`(patch|minor|major)` —— (.*)$")
# 详细规格的标题：###/####/##### + 任务 ID（ID 后面可能是空白，也可能是全角括号）
HEAD_RE = re.compile(r"^(#{3,5})\s+(T-[A-Z]?\d+[a-z]?)(.*)$")
BATCH_RE = re.compile(r"^#### (批次 .*)$")
SECTION = "## 13. 执行清单"


def read_plan() -> str:
    try:
        return PLAN.read_text(encoding="utf-8")
    except FileNotFoundError:
        sys.stderr.write(f"plan.py: 找不到计划文件 {PLAN}\n")
        raise SystemExit(2)


def checklist(text: str) -> list[dict]:
    """§13 的线性清单（按出现顺序，带批次与发版标记）。"""
    start = text.index(SECTION)
    items: list[dict] = []
    batch = "?"
    for line in text[start:].splitlines():
        m = BATCH_RE.match(line)
        if m:
            batch = m.group(1)
            continue
        m = CHECK_RE.match(line)
        if m:
            items.append(
                {
                    "done": m.group(1).lower() == "x",
                    "id": m.group(2),
                    "title": m.group(3).strip(),
                    "batch": batch,
                    "bump": None,
                }
            )
            continue
        m = BUMP_RE.match(line)
        if m and items:
            items[-1]["bump"] = {"kind": m.group(1), "why": m.group(2).strip()}
    return items


def specs(text: str) -> dict[str, dict]:
    """每个任务的完整规格（标题层级 → 到下一个同级或更高级标题为止）。"""
    lines = text.splitlines()
    heads = []
    for i, line in enumerate(lines):
        m = HEAD_RE.match(line)
        if m:
            heads.append((i, len(m.group(1)), m.group(2), line))
    out: dict[str, dict] = {}
    for idx, (i, level, tid, line) in enumerate(heads):
        end = len(lines)
        for j, lvl, _tid, _line in heads[idx + 1 :]:
            if lvl <= level:
                end = j
                break
        body = "\n".join(lines[i:end]).rstrip()
        # 同名 ID 取第一次出现（§3–§7 的规格；§13 只是清单）
        out.setdefault(tid, {"heading": line, "body": body})
    return out


def cmd_list(text: str) -> int:
    items = checklist(text)
    done = sum(1 for it in items if it["done"])
    batch = None
    for it in items:
        if it["batch"] != batch:
            batch = it["batch"]
            print(f"\n{batch}")
        mark = "x" if it["done"] else " "
        bump = f"   ⬆ BUMP({it['bump']['kind']})" if it["bump"] else ""
        print(f"  [{mark}] {it['id']}  {it['title']}{bump}")
    bumps = sum(1 for it in items if it["bump"])
    print(f"\n进度：{done}/{len(items)}   发版点：{bumps} 个（python3 scripts/plan.py bumps）")
    return 0


def cmd_bumps(text: str) -> int:
    items = checklist(text)
    pending = [it for it in items if it["bump"] and not it["done"]]
    print(f"发版点 {sum(1 for it in items if it['bump'])} 个，还剩 {len(pending)} 个：\n")
    for it in items:
        if not it["bump"]:
            continue
        mark = "x" if it["done"] else " "
        print(f"  [{mark}] {it['id']}  {it['bump']['kind']:5s}  {it['bump']['why']}")
    return 0


def cmd_next(text: str, as_json: bool) -> int:
    items = checklist(text)
    spec = specs(text)
    pending = next((it for it in items if not it["done"]), None)
    if pending is None:
        print("执行清单已全部勾完。")
        return 0
    entry = spec.get(pending["id"])
    if entry is None:
        sys.stderr.write(f"plan.py: 清单里的 {pending['id']} 在计划正文里没有规格（跑 check）\n")
        return 1
    if as_json:
        print(
            json.dumps(
                {
                    "id": pending["id"],
                    "title": pending["title"],
                    "batch": pending["batch"],
                    "bump": pending["bump"],
                    "spec": entry["body"],
                },
                ensure_ascii=False,
                indent=2,
            )
        )
        return 0
    done = sum(1 for it in items if it["done"])
    print(f"# 下一条：{pending['id']} {pending['title']}")
    print(f"# 批次：{pending['batch']}   进度：{done}/{len(items)}")
    if pending["bump"]:
        print(
            f"# ⬆ 这一条做完要 **bump {pending['bump']['kind']}**："
            f"{pending['bump']['why']}（§0.2 的 bump 动作）"
        )
    print()
    print(entry["body"])
    print()
    print("# 完成后：python3 scripts/plan.py done " + pending["id"])
    return 0


def cmd_done(text: str, tid: str) -> int:
    items = checklist(text)
    if not any(it["id"] == tid for it in items):
        sys.stderr.write(f"plan.py: 清单里没有 {tid}\n")
        return 2
    lines = text.splitlines()
    start = next(i for i, l in enumerate(lines) if l.startswith(SECTION))
    patched = 0
    for i in range(start, len(lines)):
        m = CHECK_RE.match(lines[i])
        if m and m.group(2) == tid:
            if m.group(1).lower() == "x":
                print(f"{tid} 已经是勾上的。")
                return 0
            lines[i] = f"- [x] `{tid}` {m.group(3)}"
            patched += 1
    if patched != 1:
        sys.stderr.write(f"plan.py: {tid} 匹配到 {patched} 行，拒绝改写\n")
        return 1
    PLAN.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{tid} 已勾上。")
    return 0


def cmd_check(text: str) -> int:
    items = checklist(text)
    spec = specs(text)
    ids = [it["id"] for it in items]
    problems: list[str] = []

    dup = {i for i in ids if ids.count(i) > 1}
    if dup:
        problems.append(f"清单里有重复 ID：{sorted(dup)}")

    missing_spec = [i for i in ids if i not in spec]
    if missing_spec:
        problems.append(f"清单里有 ID 但正文没有规格：{missing_spec}")

    # 正文里每个任务标题都应该在清单里（T-K21… 这类"范围标题"是刻意不入清单的）
    allow = {"T-K21"}
    not_listed = sorted(set(spec) - set(ids) - allow)
    if not_listed:
        problems.append(f"正文里有环节但清单没有排入：{not_listed}")

    if problems:
        print("plan.py check: 清单与正文漂移")
        for p in problems:
            print(f"  - {p}")
        return 1
    print(f"plan.py check: OK（{len(ids)} 个环节，清单与正文一致）")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) < 2 or argv[1] in {"-h", "--help"}:
        print(__doc__)
        return 0 if len(argv) >= 2 else 2
    text = read_plan()
    cmd = argv[1]
    if cmd == "list":
        return cmd_list(text)
    if cmd == "bumps":
        return cmd_bumps(text)
    if cmd == "next":
        return cmd_next(text, "--json" in argv[2:])
    if cmd == "check":
        return cmd_check(text)
    if cmd == "done":
        if len(argv) != 3:
            sys.stderr.write("用法：python3 scripts/plan.py done <T-xxx>\n")
            return 2
        return cmd_done(text, argv[2])
    sys.stderr.write(f"plan.py: 未知子命令 {cmd}（next / list / bumps / done / check）\n")
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
