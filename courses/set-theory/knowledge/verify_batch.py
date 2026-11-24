#!/usr/bin/env python3
"""B1+ 出题批次验收器（content-author 交草稿 → product-manager 复核）。

不修改任何数据；只做「入账前」检查，并把批次**注入内存**预测覆盖变化：

1. 结构：id 唯一（不与现有题库撞号）、kind/answer|rubric、why/source、difficulty、
   variants/prereq、tags、kps（引用存在 / 章节归属 / 综合题 ≥2 KP）；
2. source 可核验：每条 `文件:行号` 都必须真实存在、行号在文件范围内、该行非空
   （写不了行号/找不到文件的题直接判红，不许进批）；
3. 覆盖预测：把批次并入现有 questions.json，调 coverage.build_report 算出
   目标章节的红/黄/绿与缺口变化；`--chapter I.1` 默认要求该章 0 红；
4. 人工抽检样本：按 level 分层打印 N 个题号，供逐条打开 source 复核答案。

用法：
    python3 verify_batch.py --batch data/questions-b1.json --chapter I.1
    python3 verify_batch.py --batch data/questions-b1.json --sample 8
    python3 verify_batch.py --batch data/questions-b1.json --no-coverage   # 只查结构+source

退出码：0 = 可合入（结构/source 全过且目标章 0 红），1 = 有问题（逐条打印）。
"""
import argparse
import importlib.util
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DATA = ROOT / "data"
COURSE_ROOT = ROOT.parent  # courses/set-theory（source 里的 lib/、units/ 相对它）
LEVELS = ("L1", "L2", "L3")
KINDS = ("choice", "short", "explain", "proof")
SOURCE_RE = re.compile(r"([A-Za-z0-9_][A-Za-z0-9_./-]*\.sokonanoda):(\d+)")


def load_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def load_coverage_module():
    spec = importlib.util.spec_from_file_location("knowledge_coverage", ROOT / "coverage.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def check_source(raw_source, problems, qid):
    """返回 (ok, detail)。source 至少要有 文件:行号 且文件/行真实存在。"""
    text = str(raw_source or "")
    matches = SOURCE_RE.findall(text)
    if not matches:
        problems.append(qid + ": source 缺少 `文件:行号` 格式：" + repr(text[:120]))
        return False
    ok = True
    for rel, line_text in matches:
        candidate = (COURSE_ROOT / rel).resolve()
        if not candidate.exists():
            problems.append(qid + ": source 文件不存在：" + rel)
            ok = False
            continue
        try:
            lines = candidate.read_text(encoding="utf-8").splitlines()
        except Exception as error:  # noqa: BLE001 - 只报问题，不中断批次
            problems.append(qid + ": source 文件读不了：" + rel + "（" + str(error) + "）")
            ok = False
            continue
        line_no = int(line_text)
        if line_no < 1 or line_no > len(lines):
            problems.append(qid + ": source 行号越界：" + rel + ":" + str(line_no) + "（文件共 " + str(len(lines)) + " 行）")
            ok = False
            continue
        if not lines[line_no - 1].strip():
            problems.append(qid + ": source 指向空行：" + rel + ":" + str(line_no))
            ok = False
    return ok


def validate_question(question, index, problems, kp_by_id, existing_ids):
    qid = question.get("id")
    if not isinstance(qid, str) or not qid.strip():
        problems.append("batch[%d]: id missing" % index)
        return None
    if qid in existing_ids:
        problems.append(qid + ": id 与现有题库撞号")
    for key in ("chapter", "unit"):
        if not isinstance(question.get(key), str) or not question[key].strip():
            problems.append(qid + ": " + key + " missing")
    if question.get("level") not in (1, 2, 3):
        problems.append(qid + ": level must be 1/2/3")
    tags = question.get("tags")
    if not isinstance(tags, list) or not tags or any(not isinstance(tag, str) or not tag for tag in tags):
        problems.append(qid + ": tags must be a non-empty string list")
    if not isinstance(question.get("prompt"), str) or not question["prompt"].strip():
        problems.append(qid + ": prompt missing")
    kind = question.get("kind", "choice")
    if kind not in KINDS:
        problems.append(qid + ": unknown kind " + repr(kind))
    if kind == "choice":
        options = question.get("options")
        if not isinstance(options, list) or not (2 <= len(options) <= 6):
            problems.append(qid + ": options must be 2-6 items")
        else:
            cleaned = [str(option).strip() for option in options]
            if any(not option for option in cleaned) or len(set(cleaned)) != len(cleaned):
                problems.append(qid + ": options must be non-empty and unique")
            answer = question.get("answer")
            if not isinstance(answer, int) or isinstance(answer, bool) or not (1 <= answer <= len(options)):
                problems.append(qid + ": answer must be a 1-based option index")
    else:
        rubric = question.get("rubric")
        if not isinstance(rubric, dict):
            problems.append(qid + ": open question needs rubric")
        else:
            points = rubric.get("points")
            if not isinstance(points, list) or not points:
                problems.append(qid + ": rubric.points missing")
            else:
                for point in points:
                    if not isinstance(point, dict) or not point.get("any"):
                        problems.append(qid + ": every rubric point needs non-empty any[]")
            if not isinstance(rubric.get("minScore"), (int, float)) or not (0 < float(rubric["minScore"]) <= 1):
                problems.append(qid + ": rubric.minScore must be in (0,1]")
        if not isinstance(question.get("reference"), str) or not question["reference"].strip():
            problems.append(qid + ": open question needs reference")
    for key in ("why", "source"):
        if not isinstance(question.get(key), str) or not question[key].strip():
            problems.append(qid + ": " + key + " missing")
    if question.get("difficulty") not in (-1.5, 0, 0.0, 1.5):
        problems.append(qid + ": difficulty must be -1.5 / 0 / 1.5")
    for key in ("variants", "prereq"):
        value = question.get(key)
        if not isinstance(value, list) or any(not isinstance(item, str) for item in value):
            problems.append(qid + ": " + key + " must be a string list")
    refs = question.get("kps")
    if not isinstance(refs, list) or not refs:
        problems.append(qid + ": kps must be a non-empty list")
    else:
        if len(set(refs)) != len(refs) or len(refs) > 4:
            problems.append(qid + ": kps must be unique and <=4 ids")
        for ref in refs:
            kp = kp_by_id.get(ref)
            if kp is None:
                problems.append(qid + ": kps refs unknown knowledge-point " + repr(ref))
            elif kp.get("kind") != "prep" and kp.get("chapter") != question.get("chapter"):
                problems.append(qid + ": kp " + ref + " belongs to " + repr(kp.get("chapter"))
                               + ", question is " + repr(question.get("chapter")))
    check_source(question.get("source"), problems, qid)
    return qid


def main():
    parser = argparse.ArgumentParser(description="出题批次验收（结构 + source + 覆盖预测）")
    parser.add_argument("--batch", required=True, help="草稿 JSON（{questions:[...]} 或直接数组）")
    parser.add_argument("--chapter", default="I.1", help="本批目标章节（默认 I.1）")
    parser.add_argument("--sample", type=int, default=6, help="人工抽检样本数（按 level 分层，默认 6）")
    parser.add_argument("--no-coverage", action="store_true", help="只查结构与 source，不预测覆盖")
    args = parser.parse_args()

    problems = []
    existing_data = load_json(DATA / "questions.json")
    existing = existing_data.get("questions") or []
    existing_ids = {question.get("id") for question in existing}
    kp_data = load_json(DATA / "knowledge-points.json")
    kp_by_id = {kp.get("id"): kp for kp in (kp_data.get("kps") or [])}

    raw = load_json(args.batch)
    batch = raw.get("questions") if isinstance(raw, dict) else raw
    if not isinstance(batch, list) or not batch:
        print("FAIL 草稿里没有 questions 数组：" + args.batch, file=sys.stderr)
        return 1

    batch_ids = []
    for index, question in enumerate(batch):
        if not isinstance(question, dict):
            problems.append("batch[%d]: not an object" % index)
            continue
        qid = validate_question(question, index, problems, kp_by_id, existing_ids)
        if qid:
            if qid in batch_ids:
                problems.append(qid + ": 批内 id 重复")
            batch_ids.append(qid)

    # ---- 覆盖预测（内存注入，不写盘） ----
    coverage_summary = None
    if not args.no_coverage:
        coverage_module = load_coverage_module()
        merged = dict(existing_data)
        merged["questions"] = existing + [question for question in batch if isinstance(question, dict)]
        before = coverage_module.build_report(existing_data, kp_data)
        after = coverage_module.build_report(merged, kp_data)
        chapter_before = before["chapters"].get(args.chapter)
        chapter_after = after["chapters"].get(args.chapter)
        coverage_summary = {"before": before["totals"], "after": after["totals"],
                            "chapter_before": chapter_before, "chapter_after": chapter_after,
                            "after_status": after["status"]}
        if chapter_after is None:
            problems.append("目标章节 " + args.chapter + " 不在知识点注册表里")
        elif chapter_after["red"] > 0:
            problems.append("覆盖预测：" + args.chapter + " 仍有 %d 个红 KP（需继续补）" % chapter_after["red"])

    print("BATCH VERIFY  " + args.batch)
    print("  批次: %d 题（kind %s / level %s）" % (
        len(batch),
        ", ".join("%s=%d" % (kind, sum(1 for question in batch if question.get("kind", "choice") == kind))
                  for kind in sorted({question.get("kind", "choice") for question in batch if isinstance(question, dict)})),
        " ".join("L%d=%d" % (level, sum(1 for question in batch if question.get("level") == level)) for level in (1, 2, 3))))
    multi = [question for question in batch if isinstance(question, dict) and len(set(question.get("kps") or [])) >= 2]
    print("  kps 引用: %d（综合题 %d）" % (
        sum(len(question.get("kps") or []) for question in batch if isinstance(question, dict)), len(multi)))
    if coverage_summary is not None:
        before_ch = coverage_summary["chapter_before"]
        after_ch = coverage_summary["chapter_after"]
        print("  覆盖预测 %s: 红 %d→%d / 黄 %d→%d / 绿 %d→%d；缺口 %d→%d" % (
            args.chapter, before_ch["red"], after_ch["red"], before_ch["yellow"], after_ch["yellow"],
            before_ch["green"], after_ch["green"], before_ch["deficit"], after_ch["deficit"]))
        print("  全库: 红 %d→%d / 黄 %d→%d / 绿 %d→%d；综合 %d→%d" % (
            coverage_summary["before"]["red"], coverage_summary["after"]["red"],
            coverage_summary["before"]["yellow"], coverage_summary["after"]["yellow"],
            coverage_summary["before"]["green"], coverage_summary["after"]["green"],
            coverage_summary["chapter_before"]["integration"]["have"],
            coverage_summary["chapter_after"]["integration"]["have"]))

    sample = []
    for level in (1, 2, 3):
        pool = [question for question in batch if isinstance(question, dict) and question.get("level") == level]
        per_level = max(1, args.sample // 3)
        sample.extend(question.get("id") for question in pool[:per_level])
    print("  人工抽检（逐条打开 source 核验答案/why）: " + ", ".join(sample[:args.sample]))

    if problems:
        print("\nFAIL %d 个问题：" % len(problems), file=sys.stderr)
        for problem in problems[:80]:
            print("  - " + problem, file=sys.stderr)
        if len(problems) > 80:
            print("  … 其余 %d 条省略" % (len(problems) - 80), file=sys.stderr)
        return 1
    print("\nOK 结构/source/覆盖预测全部通过，可合入题库（合入后仍要跑 validate.py + coverage.py）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
