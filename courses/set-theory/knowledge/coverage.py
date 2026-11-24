#!/usr/bin/env python3
"""知识点 × 题目覆盖统计（内容门禁）。

知识点是知识点，题目是题目：题目**饱和性覆盖**知识点。本脚本按
`data/knowledge-points.json` 的 `saturation` 配额，统计每道题在 `kps` 里挂到的
知识点，逐 KP 给出：

- 红 red    ：某个 L1/L2/L3 档的题数低于 `saturation.min`（必须补）
- 黄 yellow ：达到了 min，但总题数低于 `saturation.target`（可以补）
- 绿 green  ：总题数达到 target，且各档 min 都满足

任一红存在 → 退出码 1（内容门禁）；报告写 `reports/coverage-<时间>.json`。

用法：
    python3 coverage.py                # 打印汇总 + 写报告，红则 exit 1
    python3 coverage.py --no-report    # 不写报告
    python3 coverage.py --json         # 只打印一份机器可读 JSON（不写报告）
    python3 coverage.py --fail-on-yellow   # 黄也算红（更严的内容门禁）
"""
import argparse
import json
import sys
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DATA = ROOT / "data"
LEVELS = ("L1", "L2", "L3")
STATUS_ORDER = {"red": 0, "yellow": 1, "green": 2}


def load_json(name):
    return json.loads((DATA / name).read_text(encoding="utf-8"))


def build_report(questions_data, kp_data):
    questions = questions_data.get("questions") or []
    kps = kp_data.get("kps") or []
    per_kp = {}
    for kp in kps:
        kp_id = kp.get("id")
        per_kp[kp_id] = {"kp": kp, "counts": {lv: 0 for lv in LEVELS},
                         "total": 0, "question_ids": []}
    unknown_refs = []
    integration_by_chapter = {}
    questions_by_chapter = {}
    questions_by_kind = {}
    questions_by_level = {lv: 0 for lv in LEVELS}
    for question in questions:
        chapter = question.get("chapter", "")
        kind = question.get("kind", "choice")
        raw_level = question.get("level")
        level = "L" + str(raw_level) if raw_level in (1, 2, 3) else None
        questions_by_chapter[chapter] = questions_by_chapter.get(chapter, 0) + 1
        questions_by_kind[kind] = questions_by_kind.get(kind, 0) + 1
        if level in LEVELS:
            questions_by_level[level] += 1
        refs = [ref for ref in (question.get("kps") or []) if isinstance(ref, str)]
        for ref in refs:
            if ref not in per_kp:
                unknown_refs.append((question.get("id"), ref))
                continue
            entry = per_kp[ref]
            entry["total"] += 1
            entry["question_ids"].append(question.get("id"))
            if level in LEVELS:
                entry["counts"][level] += 1
        if len(set(refs)) >= 2:
            integration_by_chapter[chapter] = integration_by_chapter.get(chapter, 0) + 1

    rows = []
    totals = {"kps": len(kps), "red": 0, "yellow": 0, "green": 0}
    kind_totals = {}
    for kp in kps:
        kp_id = kp.get("id")
        entry = per_kp[kp_id]
        minimum = (kp.get("saturation") or {}).get("min") or {}
        target = int((kp.get("saturation") or {}).get("target") or 1)
        missing = {lv: max(0, int(minimum.get(lv, 0)) - entry["counts"][lv]) for lv in LEVELS}
        if any(missing.values()):
            status = "red"
        elif entry["total"] < target:
            status = "yellow"
        else:
            status = "green"
        totals[status] += 1
        kind = kp.get("kind", "?")
        kind_totals.setdefault(kind, {"total": 0, "green": 0, "yellow": 0, "red": 0})
        kind_totals[kind]["total"] += 1
        kind_totals[kind][status] += 1
        rows.append({
            "id": kp_id,
            "chapter": kp.get("chapter", ""),
            "kind": kind,
            "title": kp.get("title", ""),
            "target": target,
            "min": {lv: int(minimum.get(lv, 0)) for lv in LEVELS},
            "counts": entry["counts"],
            "total": entry["total"],
            "missing": missing,
            "deficit": max(0, target - entry["total"]),
            "status": status,
            "question_ids": entry["question_ids"],
        })

    chapter_titles = questions_data.get("chapters") or {}
    chapters = {}
    for chapter in sorted({row["chapter"] for row in rows} | set(questions_by_chapter)):
        chapter_rows = [row for row in rows if row["chapter"] == chapter]
        kp_count = len(chapter_rows)
        have = integration_by_chapter.get(chapter, 0)
        target = (kp_count + 1) // 2  # ≈ KP 数 × 0.5（向上取整）
        chapters[chapter] = {
            "title": chapter_titles.get(chapter, chapter),
            "kps": kp_count,
            "red": sum(1 for row in chapter_rows if row["status"] == "red"),
            "yellow": sum(1 for row in chapter_rows if row["status"] == "yellow"),
            "green": sum(1 for row in chapter_rows if row["status"] == "green"),
            "questions": questions_by_chapter.get(chapter, 0),
            "integration": {"have": have, "target": target, "gap": max(0, target - have)},
            "deficit": sum(row["deficit"] for row in chapter_rows),
        }

    saturation_ratio = round(totals["green"] / totals["kps"], 4) if totals["kps"] else 0.0
    return {
        "schema": "soko.coverage/1",
        "generated_at": datetime.now().astimezone().isoformat(timespec="seconds"),
        "knowledge_points_file": "data/knowledge-points.json",
        "questions_file": "data/questions.json",
        "knowledge": {
            "schema": kp_data.get("schema"),
            "version": kp_data.get("version"),
            "updated": kp_data.get("updated"),
        },
        "questions": {
            "total": len(questions),
            "by_chapter": dict(sorted(questions_by_chapter.items())),
            "by_kind": dict(sorted(questions_by_kind.items())),
            "by_level": questions_by_level,
        },
        "totals": dict(totals, saturation_ratio=saturation_ratio, unknown_refs=len(unknown_refs)),
        "kinds": dict(sorted(kind_totals.items())),
        "chapters": chapters,
        "integration": {
            "by_chapter": {chapter: data["integration"] for chapter, data in chapters.items()},
            "total_have": sum(data["integration"]["have"] for data in chapters.values()),
            "total_target": sum(data["integration"]["target"] for data in chapters.values()),
        },
        "kps": sorted(rows, key=lambda row: (row["chapter"], STATUS_ORDER[row["status"]],
                                             row["deficit"] * -1, row["id"])),
        "status": "red" if totals["red"] > 0 else ("yellow" if totals["yellow"] > 0 else "green"),
        "unknown_refs": [{"question": qid, "kp": ref} for qid, ref in unknown_refs],
    }


def print_summary(report):
    totals = report["totals"]
    questions = report["questions"]
    print("KNOWLEDGE COVERAGE  %s" % report["generated_at"])
    print("  题目: %d 道（%s；level %s）" % (
        questions["total"],
        ", ".join("%s=%d" % (key, value) for key, value in questions["by_kind"].items()),
        " ".join("%s=%d" % (lv, questions["by_level"].get(lv, 0)) for lv in LEVELS)))
    print("  知识点: %d 个 → 绿 %d / 黄 %d / 红 %d（饱和率 %.1f%%）" % (
        totals["kps"], totals["green"], totals["yellow"], totals["red"],
        totals["saturation_ratio"] * 100))
    print("  综合题: %d 道（目标 ≈ %d）" % (
        report["integration"]["total_have"], report["integration"]["total_target"]))
    print("")
    print("  章节     知识点  绿   黄   红   题目  综合(有/目标)  缺口题数")
    for chapter, data in report["chapters"].items():
        print("  %-6s %6d %4d %4d %4d %5d  %5d/%-6d %6d" % (
            chapter, data["kps"], data["green"], data["yellow"], data["red"],
            data["questions"], data["integration"]["have"], data["integration"]["target"],
            data["deficit"]))
    if report["unknown_refs"]:
        print("\n  ⚠ 未知 kp 引用: %d" % len(report["unknown_refs"]))
    reds = [row for row in report["kps"] if row["status"] == "red"]
    if reds:
        print("\n  红（某档低于 min，必须补）：")
        for row in reds:
            missing = " ".join("%s缺%d" % (lv, count) for lv, count in row["missing"].items() if count > 0)
            print("  - %-6s %-26s %-2s 总%2d/%-2d  %s" % (
                row["chapter"], row["id"], row["kind"], row["total"], row["target"], missing))
    yellows = [row for row in report["kps"] if row["status"] == "yellow"]
    if yellows:
        print("\n  黄（达到 min，未到 target，可补）：%d 个" % len(yellows))
    print("\n  内容门禁: %s" % ("RED → exit 1" if report["status"] == "red" else report["status"].upper()))


def main():
    parser = argparse.ArgumentParser(description="知识点 × 题目覆盖统计（内容门禁）")
    parser.add_argument("--report-dir", default=str(ROOT / "reports"), help="报告目录（默认 reports/）")
    parser.add_argument("--no-report", action="store_true", help="不写 JSON 报告")
    parser.add_argument("--json", action="store_true", help="只打印机器可读 JSON（不写报告）")
    parser.add_argument("--fail-on-yellow", action="store_true", help="黄也算红（更严门禁）")
    args = parser.parse_args()

    report = build_report(load_json("questions.json"), load_json("knowledge-points.json"))
    if report["unknown_refs"]:
        print("FAIL 存在未知 kp 引用，先跑 validate.py", file=sys.stderr)
    exit_code = 1 if report["status"] == "red" else 0
    if args.fail_on_yellow and report["status"] == "yellow":
        exit_code = 1

    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=1))
        return exit_code

    print_summary(report)
    if not args.no_report:
        report_dir = Path(args.report_dir)
        report_dir.mkdir(parents=True, exist_ok=True)
        stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        path = report_dir / ("coverage-" + stamp + ".json")
        path.write_text(json.dumps(report, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        print("  报告: %s" % path)
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
