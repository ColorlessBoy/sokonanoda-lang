#!/usr/bin/env python3
"""L3 复盘工具（C9.5）：读引擎状态或导出日志 → 薄弱 tag/KP、错题、指标与下一批建议。

用法：
    python3 review.py --state ~/.dsh/state/sokonanoda-dsh/quiz/default.json
    python3 review.py --input export.jsonl [--course set-theory] [--out reports/]
    python3 review.py --selftest

数据来源只有一份：引擎状态（`engine.py` 的 `theta` / `kp_theta` / `history` /
`code_done`）。本工具不重新实现判定或掌握度，也不写任何用户内容进仓库；报告是
生成物，落在 `courses/set-theory/knowledge/reports/`（已 gitignore）。

指标口径（C9.5 + 调度 v2）：正确率/覆盖/已解-探针中-待做-薄弱用**结构计数**；每题耗时只汇总展示
（avg/median/max），**不作为通过/失败判据**（仓库纪律：绝对毫秒只兜数量级）。
"""
import argparse
import json
import sys
import tempfile
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

import engine  # noqa: E402  （同一目录的 L2 引擎：状态/掌握度/地图口径只有一份）

DATA = ROOT / "data"
REPO_ROOT = ROOT.parents[2]
SCHEMA = "soko.review/1"


# ───────────────────────── 读入 ─────────────────────────
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


def state_from_events(events: list) -> dict:
    """把 JSONL 事件还原成引擎状态：最后一个 state 快照 + 其后的作答事件。"""
    state = None
    index = -1
    for i, event in enumerate(events):
        if not isinstance(event, dict):
            continue
        if event.get("type") == "state" or ("theta" in event and "history" in event):
            state = event
            index = i
    base = engine.new_state()
    if isinstance(state, dict):
        for key, value in state.items():
            if key != "type":
                base[key] = value
        base.setdefault("history", [])
    seen = {(entry.get("qid"), entry.get("at")) for entry in base.get("history", [])
            if isinstance(entry, dict)}
    for event in events[index + 1:]:
        if not isinstance(event, dict) or not event.get("qid"):
            continue
        key = (event.get("qid"), event.get("at"))
        if key in seen:
            continue
        seen.add(key)
        entry = {"qid": event["qid"], "choice": event.get("choice"),
                 "correct": bool(event.get("correct")), "at": event.get("at"),
                 "decision": event.get("decision")}
        if isinstance(event.get("elapsed_ms"), (int, float)):
            entry["elapsed_ms"] = int(event["elapsed_ms"])
        base["history"].append(entry)
    return base


def load_data(data_dir: Path) -> dict:
    data = engine.load_data(data_dir)
    engine.index_cards(data)
    return data


# ───────────────────────── 聚合 ─────────────────────────
def _mastery(state: dict, data: dict, tag: str) -> float:
    value = (state.get("theta") or {}).get(tag)
    if value is None:
        return engine.sigmoid(float((data.get("params") or {}).get("theta_init", 0.0)))
    return engine.sigmoid(float(value))


def _answer_rows(state: dict, data: dict) -> list:
    rows = []
    for entry in state.get("history", []):
        if not isinstance(entry, dict):
            continue
        question = data["by_id"].get(entry.get("qid"))
        if not question:
            continue
        rows.append({"entry": entry, "question": question})
    return rows


def suggestions_for(state: dict, data: dict, rows: list, weak_tags: list, weak_kps: list,
                    wrong: dict, limit: int = 10) -> list:
    """下一批建议：错题 → 变式 → 薄弱 tag → 薄弱 KP → 未接触章节/KP（数据驱动）。"""
    excluded = engine.excluded_ids(data)
    touched_ids = {row["question"]["id"] for row in rows}
    touched_chapters = {row["question"].get("chapter") for row in rows}
    out = []
    chosen = set()

    def add(qid, reason, **extra):
        if not isinstance(qid, str) or qid in chosen or len(out) >= limit:
            return
        question = data["by_id"].get(qid)
        if question is None or qid in excluded:
            return
        chosen.add(qid)
        item = {"qid": qid, "reason": reason}
        item.update(extra)
        out.append(item)

    for qid, count in sorted(wrong.items(), key=lambda kv: -kv[1]):
        add(qid, "wrong", count=count)
    for qid in list(wrong):
        question = data["by_id"].get(qid)
        if not question:
            continue
        for variant in question.get("variants") or []:
            add(variant, "variant-of-wrong")
    for tag in weak_tags:
        for question in data["questions"]:
            if tag in (question.get("tags") or []) and question["id"] not in touched_ids:
                add(question["id"], "weak-tag:" + tag)
                break
    for row in weak_kps:
        for question in engine.kp_questions(data, row["id"]):
            if question["id"] not in touched_ids:
                add(question["id"], "weak-kp:" + row["id"])
                break
    for question in data["questions"]:
        if question.get("chapter") not in touched_chapters:
            add(question["id"], "uncovered-chapter:" + str(question.get("chapter")))
    for row in engine.kp_status(state, data)["kps"]:
        if row["todo"]:
            for question in engine.kp_questions(data, row["id"]):
                add(question["id"], "untouched-kp:" + row["id"])
                break
    return out[:limit]


def aggregate(state: dict, data: dict, course: str = "set-theory") -> dict:
    cfg = data.get("params") or {}
    review_cfg = cfg.get("review") or {}
    weak_below = float(review_cfg.get("weak_mastery_below", 0.5))
    strong_above = float(review_cfg.get("strong_mastery_above", 0.8))
    rows = _answer_rows(state, data)
    by_tag = {}
    wrong = {}
    per_question = {}
    for row in rows:
        entry, question = row["entry"], row["question"]
        correct = entry.get("correct") is True
        for tag in question.get("tags") or []:
            tag_row = by_tag.setdefault(tag, {"asked": 0, "correct": 0})
            tag_row["asked"] += 1
            tag_row["correct"] += 1 if correct else 0
        qid = question["id"]
        per_question.setdefault(qid, {"asked": 0, "correct": 0})
        per_question[qid]["asked"] += 1
        per_question[qid]["correct"] += 1 if correct else 0
        if not correct:
            wrong[qid] = wrong.get(qid, 0) + 1
    for tag_row in by_tag.values():
        tag_row["accuracy"] = round(tag_row["correct"] / tag_row["asked"], 3) if tag_row["asked"] else 0.0

    mastery = {tag: round(_mastery(state, data, tag), 3) for tag in (state.get("theta") or {})}
    weak_tags = sorted((tag for tag, value in mastery.items() if value < weak_below),
                       key=lambda tag: (mastery[tag], tag))
    strong_tags = sorted((tag for tag, value in mastery.items() if value >= strong_above),
                         key=lambda tag: (-mastery[tag], tag))

    kp = engine.kp_status(state, data)
    kp_summary = dict(kp["summary"])
    content_gaps = [{"id": row["id"], "title": row["title"], "chapter": row["chapter"]}
                    for row in kp["kps"] if row["total"] == 0]
    weak_kps = [{"id": row["id"], "title": row["title"], "chapter": row["chapter"],
                 "accuracy": row["accuracy"], "mastery": row["mastery"], "wrong": row["wrong"]}
                for row in kp["kps"] if row["weak"]]
    metrics = engine.metrics_snapshot(state, data, course=course)
    code = engine.code_progress(state, data, course=course)
    wrong_detail = []
    for qid, count in sorted(wrong.items(), key=lambda kv: (-kv[1], kv[0])):
        question = data["by_id"].get(qid) or {}
        wrong_detail.append({
            "qid": qid, "count": count,
            "chapter": question.get("chapter", ""),
            "unit": question.get("unit", ""),
            "tags": list(question.get("tags") or []),
            "kps": list(question.get("kps") or []),
            "prompt": question.get("prompt", ""),
        })
    suggestions = suggestions_for(state, data, rows, weak_tags, weak_kps, wrong)
    answers = len(rows)
    correct_total = sum(1 for row in rows if row["entry"].get("correct") is True)
    return {
        "schema": SCHEMA,
        "generated_at": datetime.now().strftime("%Y-%m-%dT%H:%M:%S"),
        "answers": answers,
        "total_correct": correct_total,
        "accuracy": round(correct_total / answers, 3) if answers else 0.0,
        "timing": metrics["timing"],
        "by_level": metrics["byLevel"],
        "by_chapter": metrics["byChapter"],
        "per_tag": by_tag,
        "mastery": mastery,
        "weak_tags": weak_tags,
        "strong_tags": strong_tags,
        "kp": kp_summary,
        "weak_kps": weak_kps,
        "wrong": wrong_detail,
        "code": {"course": code["course"], "solved": code["solved"], "skipped": code["skipped"],
                 "total": code["total"], "chapters": code["chapters"],
                 "otherCourse": code["otherCourse"]},
        "content_gaps": content_gaps,
        "suggestions": suggestions,
    }


# ───────────────────────── 输出 ─────────────────────────
def display_path(target: Path) -> str:
    """报告路径：仓库内给相对路径；仓库外给文件名（不暴露家目录）。"""
    try:
        return str(target.resolve().relative_to(REPO_ROOT.resolve()))
    except Exception:
        return target.name


def summary_lines(report: dict) -> list:
    lines = ["== 复盘（C9.5）=="]
    lines.append("作答 %d 次 · 正确率 %.1f%%" % (report["answers"], report["accuracy"] * 100))
    timing = report.get("timing") or {}
    if timing.get("timed", 0) > 0:
        lines.append("每题耗时（只汇总，不作判据）：%d 题有记录 · 中位 %.1f 秒 · 均值 %.1f 秒" % (
            timing["timed"], timing.get("medianMs", 0) / 1000.0, timing.get("avgMs", 0) / 1000.0))
    else:
        lines.append("每题耗时（只汇总，不作判据）：暂无记录")
    kp = report.get("kp") or {}
    lines.append("知识点：已解 %d · 探针中 %d · 待做 %d · 薄弱 %d（覆盖 %.0f%% · 共 %d，%d 个已有题）" % (
        kp.get("solved", 0), kp.get("probing", 0), kp.get("todo", 0), kp.get("weak", 0),
        (kp.get("coverage", 0.0) or 0.0) * 100,
        kp.get("total", 0), kp.get("withQuestions", 0)))
    if report.get("weak_tags"):
        lines.append("薄弱 tag（<阈值）：" + ", ".join(report["weak_tags"][:8]))
    if report.get("weak_kps"):
        lines.append("薄弱知识点：" + "；".join(
            "%s %s（正确率 %.0f%%，错 %d）" % (
                row["id"], row["title"], row["accuracy"] * 100, row["wrong"])
            for row in report["weak_kps"][:5]))
    if report.get("wrong"):
        lines.append("错题 %d 道：" % len(report["wrong"]) + "；".join(
            "%s×%d" % (row["qid"], row["count"]) for row in report["wrong"][:8]))
    else:
        lines.append("错题：无")
    code = report.get("code") or {}
    if code:
        lines.append("代码题：已解 %d / 共 %d（跳过 %d）" % (
            code.get("solved", 0), code.get("total", 0), code.get("skipped", 0)))
    if report.get("content_gaps"):
        lines.append("内容缺口（无题 KP）%d 个：%s" % (
            len(report["content_gaps"]),
            ", ".join(row["id"] for row in report["content_gaps"][:5])))
    if report.get("suggestions"):
        lines.append("下一批建议：" + ", ".join(
            "%s(%s)" % (item["qid"], item["reason"]) for item in report["suggestions"][:8]))
    return lines


def write_report(report: dict, out_dir: Path) -> Path:
    out_dir.mkdir(parents=True, exist_ok=True)
    target = out_dir / ("review-" + datetime.now().strftime("%Y%m%d-%H%M%S") + ".json")
    target.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return target


# ───────────────────────── 自检 ─────────────────────────
def selftest(data: dict) -> int:
    failures = []

    def check(name, condition, detail=""):
        if condition:
            print("PASS " + name + ("  " + detail if detail else ""))
        else:
            print("FAIL " + name + ("  " + detail if detail else ""))
            failures.append(name)

    state = engine.new_state()
    questions = [q for q in data["questions"] if q.get("kind", "choice") == "choice"]
    # 三题：两对一错（不同 tag），耗时用显式值，回报复盘要能结构统计。
    picks = []
    used_tags = set()
    for question in questions:
        tags = set(question.get("tags") or [])
        if tags and not (tags & used_tags):
            used_tags.update(tags)
            picks.append(question)
        if len(picks) == 3:
            break
    elapsed = [1200, 3400, 800]
    for index, question in enumerate(picks):
        correct = index != (len(picks) - 1)
        choice = int(question.get("answer", 1)) if correct else (
            1 if int(question.get("answer", 1)) != 1 else 2)
        engine.apply_answer(state, data, question, choice, elapsed_ms=elapsed[index])
    code_bank = data.get("code_exercises") or []
    if code_bank:
        state.setdefault("code_done", {})[code_bank[0]["id"]] = True
    report = aggregate(state, data)

    check("review-fixture", len(picks) == 3, str(len(picks)))
    expected_accuracy = round((len(picks) - 1) / len(picks), 3) if picks else 0.0
    check("review-answers", report["answers"] == len(picks), str(report["answers"]))
    check("review-accuracy", report["accuracy"] == expected_accuracy, str(report["accuracy"]))
    check("review-timing", (report["timing"]["timed"] == len(picks)
                            and report["timing"]["medianMs"] == sorted(elapsed)[len(picks) // 2]),
          str(report["timing"]))
    check("review-wrong", len(report["wrong"]) == 1, str([row["qid"] for row in report["wrong"]]))
    check("review-weak-tag", bool(report["weak_tags"]), str(report["weak_tags"][:3]))
    check("review-kp-summary",
          (report["kp"]["solved"] + report["kp"]["weak"] + report["kp"].get("probing", 0)) >= 1
          and report["kp"].get("probing", 0) >= 1,
          str(report["kp"]))
    check("review-content-gaps", all(row["id"].startswith("kp.") for row in report["content_gaps"]),
          str(len(report["content_gaps"])))
    check("review-suggestions",
          bool(report["suggestions"]) and all(item["qid"] in data["by_id"] for item in report["suggestions"]),
          str(report["suggestions"][:2]))
    if code_bank:
        check("review-code-merge", report["code"]["solved"] >= 1,
              str((report["code"]["solved"], report["code"]["total"])))
    blob = json.dumps(report, ensure_ascii=False)
    home = str(Path.home())
    check("review-no-personal-paths", home not in blob and "/Users/" not in blob,
          "" if (home not in blob and "/Users/" not in blob) else "home path leaked")
    check("review-schema", report["schema"] == SCHEMA, report["schema"])

    with tempfile.TemporaryDirectory() as tmp:
        target = write_report(report, Path(tmp))
        check("review-writes-report", target.exists() and target.name.startswith("review-"),
              target.name)

    if failures:
        print("\nREVIEW SELFTEST: %d failed" % len(failures), file=sys.stderr)
        return 1
    print("\nREVIEW SELFTEST: all passed")
    return 0


# ───────────────────────── CLI ─────────────────────────
def main() -> int:
    parser = argparse.ArgumentParser(description="L3 学习日志复盘（C9.5）")
    parser.add_argument("--input", help="导出 JSONL 路径")
    parser.add_argument("--state", help="引擎状态 JSON 路径（~/.dsh/…/quiz/*.json）")
    parser.add_argument("--data-dir", default=str(DATA))
    parser.add_argument("--course", default="set-theory", help="代码题进度按哪个课程统计")
    parser.add_argument("--out", default=str(ROOT / "reports"))
    parser.add_argument("--json", action="store_true", dest="as_json",
                        help="stdout 输出报告 JSON（仍写 reports/）")
    parser.add_argument("--selftest", action="store_true", help="用合成状态自检聚合器")
    args = parser.parse_args()
    data = load_data(Path(args.data_dir).expanduser())
    if args.selftest:
        return selftest(data)
    if args.input:
        events = load_events(Path(args.input).expanduser())
        state = state_from_events(events)
    elif args.state:
        state = engine.load_state(Path(args.state).expanduser())
    else:
        parser.print_help()
        return 2
    report = aggregate(state, data, course=args.course)
    target = write_report(report, Path(args.out).expanduser())
    if args.as_json:
        print(json.dumps(report, ensure_ascii=False))
    else:
        for line in summary_lines(report):
            print(line)
        print("报告：" + display_path(target))
    return 0


if __name__ == "__main__":
    sys.exit(main())
