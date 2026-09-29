#!/usr/bin/env python3
"""**文档垃圾回收（报告式）** —— 报「哪些活文档已经没人引用、也没在维护」，不自删 ✓。

起因（用户 2026-09-29）：「一堆 HANDOVER 都可以删了，没必要」+「没有建立起来很好的
doc 清理机制」。⇒ 清理**不能靠人记得**，要有一条**可重复跑**的命令：
本脚本只**报告候选**，删除永远由人/agent 决定并落 commit（评审可见 ✓）。

判据（三条任一命中即列为候选，**不做自动判定**）：
  ① **零引用**：没有其它活文档提到它的文件名（`docs/` + 仓库根 *.md 里 grep）；
  ② **不在必读/权威链上**：不是 `docs/ONBOARDING.md` 必读表、不是 `docs-budget.json`
     的 `frozen` 之外的门禁输入（`plan.py` 的计划文件、`gap.py` 的 repro、
     `docs/archive/README.md` 索引）；
  ③ **久未变动**：`git log -1 --format=%cs -- <文件>` 早于 `--days`（默认 30）天。

用法：
  python3 scripts/docs-gc.py                 # 报告全部候选
  python3 scripts/docs-gc.py --days 14       # 只报 14 天没动过的
  python3 scripts/docs-gc.py --json          # 机器可读
退出码：0 = 跑完（**候选数不是失败** ✓）；2 = 用法错误。
"""
import json, pathlib, re, subprocess, sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
KEEP_ALWAYS = {
    "docs/ONBOARDING.md", "docs/README.md", "docs/NEXT.md",
    "docs/STATUS-ARCHIVE.md", "docs/architecture.md", "docs/protocol.md",
    "docs/TESTING.md", "docs/PERF.md", "docs/RELEASE.md", "docs/E2E.md",
    "docs/LESSONS.md", "docs/CI-FAILURES.md", "docs/vscode-dev-guide.md",
    "docs/design/e2-plan.md",
}
GUARDED = ("docs/archive/", "docs/perf/", "docs/e2e/", "docs/gaps/")


def live_docs() -> list[pathlib.Path]:
    out = []
    for p in sorted(DOCS.rglob("*.md")):
        rel = p.relative_to(ROOT).as_posix()
        if rel.startswith(GUARDED) or rel in KEEP_ALWAYS:
            continue
        out.append(p)
    return out


def corpus() -> str:
    parts = []
    for p in list(DOCS.rglob("*.md")) + sorted(ROOT.glob("*.md")):
        try:
            parts.append(p.read_text(encoding="utf-8"))
        except OSError:
            pass
    return "\n".join(parts)


def last_touch(p: pathlib.Path) -> str:
    r = subprocess.run(["git", "log", "-1", "--format=%cs", "--", str(p.relative_to(ROOT))],
                       cwd=ROOT, capture_output=True, text=True)
    return r.stdout.strip() or "?"


def days_since(iso: str) -> int:
    r = subprocess.run(["git", "log", "-1", "--format=%cs", "--", "."], cwd=ROOT,
                       capture_output=True, text=True)
    today = r.stdout.strip()
    if not iso or iso == "?" or not today:
        return 0
    import datetime as dt
    a = dt.date.fromisoformat(iso); b = dt.date.fromisoformat(today)
    return (b - a).days


def main(argv: list[str]) -> int:
    unknown = [a for a in argv if a.startswith("-") and a not in {"--json", "--days"}]
    if unknown:
        sys.stderr.write(f"docs-gc: 未知参数 {unknown}（只认 --json / --days N）\n")
        return 2
    days = 30
    if "--days" in argv:
        try:
            days = int(argv[argv.index("--days") + 1])
        except (IndexError, ValueError):
            sys.stderr.write("docs-gc: --days 需要一个整数\n")
            return 2
    text = corpus()
    rows = []
    for p in live_docs():
        rel = p.relative_to(ROOT).as_posix()
        name = p.name
        cited = len(re.findall(re.escape(name), text)) > 1  # >1 = 除了它自己以外还有人提
        touched = last_touch(p)
        age = days_since(touched)
        reasons = []
        if not cited:
            reasons.append("零引用")
        if age >= days:
            reasons.append(f"{age} 天没动")
        if reasons:
            rows.append({"file": rel, "lines": len(p.read_text(encoding='utf-8').splitlines()),
                         "reasons": reasons, "last": touched})
    if "--json" in argv:
        print(json.dumps({"candidates": rows, "days": days}, ensure_ascii=False, indent=2))
        return 0
    print(f"docs-gc: 活文档 {len(live_docs())} 份，候选 {len(rows)} 份"
          f"（判据：零引用 / 不在权威链 / ≥{days} 天没动 —— **只报不删** ✓）")
    for r in rows:
        print(f"  {r['file']:60s} {r['lines']:5d} 行  last={r['last']}  ← {' · '.join(r['reasons'])}")
    if rows:
        print("\n处置（人工/agent 决定，落 commit ⇒ 评审可见）：① 并入 `docs/NEXT.md` 后删除"
              "（**默认**，用户 2026-09-29 口径：不需要归档）② 移进 `docs/archive/` 并在"
              " `docs/archive/README.md` 点名 ③ 加回本脚本的 KEEP_ALWAYS 并写明理由。")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
