#!/usr/bin/env python3
"""**dry-run 校验** `gaps/census-batch-plan.json`（#5 落地清单）：能回放、能数条数、**不改任何内容** ✓。

用法：
    python3 courses/set-theory/tools/census-plan.py            # 回放 + 计数（exit 0 = 清单与树一致 ✓）
    python3 courses/set-theory/tools/census-plan.py --json     # 单 JSON 对象（给下一棒/CI 用 ✓）

口径 ✓：
  · `ready` / `pending-*` 三类是**机械改写** ⇒ 逐条用 `scripts/notation-lint.py` 的改写器
    **在当前树上重算** `before`/`after`，与清单逐字比对（对不上 = 清单过期 ⇒ exit 1 ✓）；
  · `manual` 只校验 `file:line` 上**确有**该规则的命中 ✓；
  · 本脚本**只读**：不写课程文件、不删豁免、不动标记 ✓（dry-run ✓）。
"""
from __future__ import annotations
import argparse, importlib.util, json, subprocess, sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
PLAN = REPO / "courses/set-theory/gaps/census-batch-plan.json"


def _lint():
    spec = importlib.util.spec_from_file_location("nl", REPO / "scripts/notation-lint.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    mod.Linter([mod.REPO / r for r in mod.DEFAULT_ROOTS])   # ⚠ 派生记法表必须先建 ✓
    return mod


_HEAD_CACHE: dict[str, list[str]] = {}


def _head_line(rel: str, line: int) -> str | None:
    """该文件在 `HEAD` 里第 `line` 行的文本（stripped ✓）—— `done` 条目的落地判据 ✓。"""
    if rel not in _HEAD_CACHE:
        out = subprocess.run(["git", "show", f"HEAD:{rel}"], cwd=REPO,
                             capture_output=True, text=True).stdout
        _HEAD_CACHE[rel] = out.splitlines()
    b = _HEAD_CACHE[rel]
    return b[line - 1].strip() if 0 < line <= len(b) else None


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="census-plan.py")
    ap.add_argument("--json", action="store_true", help="单 JSON 对象输出")
    a = ap.parse_args(argv)
    if not PLAN.exists():
        print(f"census-plan: 找不到清单 {PLAN} ⇒ 无法判定（exit 2）", file=sys.stderr)
        return 2
    nl = _lint()
    plan = json.loads(PLAN.read_text(encoding="utf-8"))
    counts: dict[str, int] = {}
    stale: list[dict] = []
    for e in plan["entries"]:
        counts[e["group"]] = counts.get(e["group"], 0) + 1
        if e["group"] == "manual":
            f = REPO / e["file"]
            if not f.exists():
                stale.append({"file": e["file"], "line": e["line"], "why": "文件不存在"})
                continue
            hits = [h for h in nl.census_hits(f)
                    if h["line"] == e["line"] and h["rule"] == e["rule"]]
            if not hits:
                stale.append({"file": e["file"], "line": e["line"], "why": "命中不在（清单过期）"})
            continue
        f = REPO / e["file"]
        if not f.exists():
            stale.append({"file": e["file"], "line": e["line"], "why": "文件不存在"})
            continue
        lines = f.read_text(encoding="utf-8").splitlines()
        hits = [h for h in nl.census_hits(f) if h["line"] == e["line"] and h["rule"] == e["rule"]]
        if e["group"] == "done":
            # `done` = 已落地 ⇒ **按文本验**（行漂免疫 ✓）：基线（HEAD）那一行的原文必须已不在文件里 ✓
            #   ⚠ 早先按 (line, rule) 判 ⇒ 行号漂移后会把**邻居**命中当成本条 ⇒ 假红 ✗（实测追了 3 轮 ✗）
            base = _head_line(e["file"], e["line"])
            if base and base in f.read_text(encoding="utf-8"):
                stale.append({"file": e["file"], "line": e["line"],
                              "why": "标 done 但基线原文仍在（疑未落地）"})
            continue
        if not hits:
            stale.append({"file": e["file"], "line": e["line"], "why": "命中不在（清单过期）"})
            continue
        cand, _kind = nl.census_candidate(lines, hits[0])
        got_before = lines[e["line"] - 1].strip()
        got_after = cand[e["line"] - 1].strip() if cand else None
        if got_before != e["before"] or got_after != e["after"]:
            stale.append({"file": e["file"], "line": e["line"], "why": "改前/改后与清单不符"})
    ok = not stale
    if a.json:
        print(json.dumps({"ok": ok, "schema": plan["schema"], "counts": counts,
                          "total": len(plan["entries"]), "stale": len(stale),
                          "stale_sample": stale[:5]}, ensure_ascii=False, indent=2))
        return 0 if ok else 1
    print(f"census-plan（dry-run · 只读 ✓）：清单 {len(plan['entries'])} 条 · 与树不符 {len(stale)} 条")
    for g in ("ready", "pending-u1u2", "pending-other", "manual", "done"):
        print(f"  {g:<14} {counts.get(g, 0):>5}  {plan.get('group_labels', {}).get(g, '')}")
    for s in stale[:5]:
        print(f"  ✗ {s['file']}:{s['line']} — {s['why']}")
    print("✓ 清单与当前树逐条一致（可回放 ✓）" if ok else "✗ 清单已过期 ⇒ 重生成 ✓")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
