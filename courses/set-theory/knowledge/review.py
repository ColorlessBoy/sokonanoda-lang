#!/usr/bin/env python3
"""L3 维护层复盘工具：读 L2 导出日志 → 聚合报告 + 下一批建议 + 内容缺口线索。

用法：
    python3 review.py --input export.jsonl [--out reports/]
    python3 review.py --state ~/.dsh/learn/<sessionId>.json [--out reports/]

输入兼容两种行：
  · 作答事件 {"type":"answer","qid":"...","choice":2,"correct":true,...}
  · 状态快照 {"type":"state","theta":{...},"history":[...],...}
输出：reports/review-<时间>.json + 终端摘要（给 L3 agent 做题库/参数迭代用）。
"""
import argparse
import json
import sys
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DATA = ROOT / "data"


def load_data(data_dir: Path) -> dict:
    questions = json.loads((data_dir / "questions.json").read_text(encoding="utf-8"))
    return {"questions": questions["questions"], "by_id": {q["id"]: q for q in questions["questions"]}}


def load_events(path: Path) -> list:
    events = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            events.append(json.loads(line))
        except Exception:
            pass
    return events


def sigmoid(x: float) -> float:
    import math
    x = max(-30.0, min(30.0, x))
    return 1.0 / (1.0 + math.exp(-x))


def aggregate(events: list, data: dict) -> dict:
    answers = []
    state = None
    for event in events:
        if event.get("type") == "state" or ("theta" in event and "history" in event):
            state = event
        elif event.get("qid"):
            answers.append(event)
        elif event.get("type") == "answer":
            answers.append(event)
    by_tag = {}
    wrong = {}
    per_question = {}
    for event in answers:
        qid = event.get("qid")
        question = data["by_id"].get(qid)
        if not question:
            continue
        correct = bool(event.get("correct"))
        for tag in question.get("tags", []):
            entry = by_tag.setdefault(tag, {"asked": 0, "correct": 0})
            entry["asked"] += 1
            entry["correct"] += 1 if correct else 0
        per_question[qid] = per_question.get(qid, {"asked": 0, "correct": 0})
        per_question[qid]["asked"] += 1
        per_question[qid]["correct"] += 1 if correct else 0
        if not correct:
            wrong[qid] = wrong.get(qid, 0) + 1

    theta = (state or {}).get("theta", {})
    mastery = {tag: round(sigmoid(value), 3) for tag, value in theta.items()}
    weak_tags = sorted([tag for tag, value in mastery.items() if value < 0.5],
                       key=lambda tag: mastery[tag])
    strong_tags = sorted([tag for tag, value in mastery.items() if value >= 0.8],
                         key=lambda tag: -mastery[tag])

    # 下一批建议：错题 → 同 tag 变式 → 同 tag 新题 → 未接触章节
    suggestions = []
    for qid, count in sorted(wrong.items(), key=lambda kv: -kv[1]):
        suggestions.append({"qid": qid, "reason": "wrong", "count": count})
    suggestion_ids = {item["qid"] for item in suggestions}
    for qid in list(wrong):
        question = data["by_id"].get(qid)
        if not question:
            continue
        for variant in question.get("variants", []):
            if variant not in suggestion_ids:
                suggestions.append({"qid": variant, "reason": "variant-of-wrong"})
                suggestion_ids.add(variant)
    for tag in weak_tags:
        for question in data["questions"]:
            if tag in question.get("tags", []) and question["id"] not in suggestion_ids:
                suggestions.append({"qid": question["id"], "reason": "weak-tag:" + tag})
                suggestion_ids.add(question["id"])
                break
    touched_chapters = set()
    for qid in per_question:
        question = data["by_id"].get(qid)
        if question:
            touched_chapters.add(question["chapter"])
    for question in data["questions"]:
        if question["chapter"] not in touched_chapters and question["id"] not in suggestion_ids:
            suggestions.append({"qid": question["id"], "reason": "uncovered-chapter:" + question["chapter"]})
            suggestion_ids.add(question["id"])
        if len(suggestions) >= 10:
            break

    tags_asked = {tag: entry["asked"] for tag, entry in by_tag.items()}
    report = {
        "generated_at": datetime.now().strftime("%Y-%m-%dT%H:%M:%S"),
        "answers": len(answers),
        "total_correct": sum(1 for event in answers if event.get("correct")),
        "accuracy": round(sum(1 for event in answers if event.get("correct")) / max(1, len(answers)), 3),
        "per_tag": by_tag,
        "weak_tags": weak_tags,
        "strong_tags": strong_tags,
        "mastery": mastery,
        "wrong": wrong,
        "suggestions": suggestions[:10],
        "content_gaps": sorted(tag for tag in ("membership", "extensionality", "rel", "order", "function",
                                               "image", "preimage", "cardinal", "ordinal", "choice")
                               if tags_asked.get(tag, 0) == 0),
    }
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description="L3 学习日志复盘")
    parser.add_argument("--input", help="导出 JSONL 路径")
    parser.add_argument("--state", help="~/.dsh/learn/<sessionId>.json 路径")
    parser.add_argument("--data-dir", default=str(DATA))
    parser.add_argument("--out", default=str(ROOT / "reports"))
    args = parser.parse_args()
    data = load_data(Path(args.data_dir))
    if args.input:
        events = load_events(Path(args.input))
    elif args.state:
        raw = json.loads(Path(args.state).expanduser().read_text(encoding="utf-8"))
        events = [{"type": "state", **raw}]
        events += [{"type": "answer", **entry} for entry in raw.get("history", [])]
    else:
        parser.print_help()
        return 2
    report = aggregate(events, data)
    out_dir = Path(args.out).expanduser()
    out_dir.mkdir(parents=True, exist_ok=True)
    target = out_dir / ("review-" + datetime.now().strftime("%Y%m%d-%H%M%S") + ".json")
    target.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    print("== 复盘 ==")
    print("作答 %d 次，正确率 %.1f%%" % (report["answers"], report["accuracy"] * 100))
    print("薄弱 tag：" + (", ".join(report["weak_tags"]) or "无"))
    print("错题：" + (", ".join("%s×%d" % (qid, count) for qid, count in report["wrong"].items()) or "无"))
    print("从未接触：" + (", ".join(report["content_gaps"]) or "无"))
    print("下一批建议：" + ", ".join("%s(%s)" % (item["qid"], item["reason"]) for item in report["suggestions"][:5]))
    print("报告：" + str(target))
    return 0


if __name__ == "__main__":
    sys.exit(main())
