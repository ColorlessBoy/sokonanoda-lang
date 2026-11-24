#!/usr/bin/env python3
"""知识库校验：data/cards.json（知识卡）+ data/questions.json（选择题）结构自检。退出码 1 = 不合格。"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
problems = []
def fail(message):
    problems.append(message)

# ---------- 知识卡 ----------
cards_data = json.loads((ROOT / "data" / "cards.json").read_text(encoding="utf-8"))
if cards_data.get("schema") != "soko.knowledge-cards/1":
    fail("cards.json: schema")
cards = cards_data.get("cards") or []
if not isinstance(cards, list) or len(cards) == 0:
    fail("cards.json: cards missing")
card_ids = set()
for index, card in enumerate(cards):
    at = "cards[%d]" % index + ((" (" + card["id"] + ")") if isinstance(card, dict) and card.get("id") else "")
    if not isinstance(card, dict):
        fail(at + ": not an object")
        continue
    for key in ("id", "kicker", "title", "oneLiner", "source"):
        if not isinstance(card.get(key), str) or card[key].strip() == "":
            fail(at + ": " + key + " missing")
    if isinstance(card.get("id"), str):
        if not re.match(r"^[a-z0-9-]+$", card["id"]):
            fail(at + ": id must be lower-kebab-case")
        elif card["id"] in card_ids:
            fail(at + ": duplicate id")
        else:
            card_ids.add(card["id"])
    definition = card.get("definition")
    if not isinstance(definition, dict) or not isinstance(definition.get("code"), str) or definition["code"].strip() == "":
        fail(at + ": definition.code missing")
    sections = card.get("sections")
    if not isinstance(sections, list) or not sections:
        fail(at + ": sections missing")
        continue
    for sindex, section in enumerate(sections):
        sat = at + ".sections[%d]" % sindex
        if not isinstance(section, dict) or not isinstance(section.get("heading"), str):
            fail(sat + ": heading missing")
            continue
        kind = section.get("kind")
        if kind == "table":
            columns = section.get("columns")
            rows = section.get("rows")
            if not isinstance(columns, list) or len(columns) < 2:
                fail(sat + ": table columns missing")
            if not isinstance(rows, list) or not rows:
                fail(sat + ": table rows missing")
            elif isinstance(columns, list):
                for rindex, row in enumerate(rows):
                    if not isinstance(row, list) or len(row) != len(columns):
                        fail(sat + ".rows[%d]: width != columns" % rindex)
        elif kind == "list":
            items = section.get("items")
            if not isinstance(items, list) or not items or any(not isinstance(item, str) or not item.strip() for item in items):
                fail(sat + ": list items missing")
        elif kind == "paragraph":
            if not isinstance(section.get("text"), str) or not section["text"].strip():
                fail(sat + ": paragraph text missing")
        else:
            fail(sat + ": unknown kind " + repr(kind))
    tags = card.get("tags")
    if not isinstance(tags, list) or not tags or any(not isinstance(tag, str) or not tag.strip() for tag in tags):
        fail(at + ": tags must be a non-empty list of strings")
    prereq = card.get("prereq")
    if not isinstance(prereq, list) or any(not isinstance(item, str) for item in prereq):
        fail(at + ": prereq must be a list of card ids")

# ---------- 选择题 ----------
questions_data = json.loads((ROOT / "data" / "questions.json").read_text(encoding="utf-8"))
if questions_data.get("schema") != "soko.choice-bank/1":
    fail("questions.json: schema")
questions = questions_data.get("questions") or []
chapters = set((questions_data.get("chapters") or {}).keys())
qids = set()
qcount = {}
levels = {1: 0, 2: 0, 3: 0}
for index, q in enumerate(questions):
    at = "questions[%d]" % index + ((" (" + q["id"] + ")") if isinstance(q, dict) and q.get("id") else "")
    if not isinstance(q, dict):
        fail(at + ": not an object")
        continue
    if not isinstance(q.get("id"), str) or not re.match(r"^I\.\d+\.[a-z0-9-]+$", q["id"]):
        fail(at + ": id must look like I.2.rel-1")
    elif q["id"] in qids:
        fail(at + ": duplicate id")
    else:
        qids.add(q["id"])
    if q.get("chapter") not in chapters:
        fail(at + ": unknown chapter " + repr(q.get("chapter")))
    if not isinstance(q.get("unit"), str) or not q["unit"].strip():
        fail(at + ": unit missing")
    if q.get("level") not in (1, 2, 3):
        fail(at + ": level must be 1/2/3")
    else:
        levels[q["level"]] += 1
    if not isinstance(q.get("tags"), list) or any(not isinstance(tag, str) or not tag for tag in q["tags"]):
        fail(at + ": tags must be non-empty strings")
    if not isinstance(q.get("prompt"), str) or not q["prompt"].strip():
        fail(at + ": prompt missing")
    kind = q.get("kind", "choice")
    if kind not in ("choice", "short", "explain", "proof"):
        fail(at + ": unknown kind " + repr(kind))
    if kind == "choice":
        options = q.get("options")
        if not isinstance(options, list) or not (2 <= len(options) <= 6):
            fail(at + ": options must be 2-6 items")
        else:
            cleaned = [str(option).strip() for option in options]
            if any(not option for option in cleaned):
                fail(at + ": every option needs text")
            if len(set(cleaned)) != len(cleaned):
                fail(at + ": duplicate option text")
        answer = q.get("answer")
        if not isinstance(answer, int) or isinstance(answer, bool) or not (1 <= answer <= (len(options) if isinstance(options, list) else 0)):
            fail(at + ": answer must be 1-based option index")
    else:
        rubric = q.get("rubric")
        points = (rubric or {}).get("points") if isinstance(rubric, dict) else None
        if not isinstance(points, list) or not points:
            fail(at + ": open question needs rubric.points")
        else:
            for pindex, point in enumerate(points):
                if not isinstance(point, dict) or not point.get("any"):
                    fail(at + ".rubric.points[%d]: needs non-empty any[] keywords" % pindex)
        if not isinstance((rubric or {}).get("minScore"), (int, float)):
            fail(at + ": rubric.minScore missing")
        if not isinstance(q.get("reference"), str) or not q["reference"].strip():
            fail(at + ": open question needs reference answer")
    for key in ("why", "source"):
        if not isinstance(q.get(key), str) or not q[key].strip():
            fail(at + ": " + key + " missing")
    if q.get("difficulty") not in (-1.5, 0, 0.0, 1.5):
        fail(at + ": difficulty must be -1.5 / 0 / 1.5, got " + repr(q.get("difficulty")))
    for key in ("variants", "prereq"):
        value = q.get(key)
        if not isinstance(value, list) or any(not isinstance(item, str) for item in value):
            fail(at + ": " + key + " must be a list of question ids")
    qcount[q["chapter"]] = qcount.get(q["chapter"], 0) + 1

# ---------- 知识点注册表（data/knowledge-points.json） ----------
kp_data = json.loads((ROOT / "data" / "knowledge-points.json").read_text(encoding="utf-8"))
if kp_data.get("schema") != "soko.knowledge-points/1":
    fail("knowledge-points.json: schema")
kp_list = kp_data.get("kps") or []
if not isinstance(kp_list, list) or not kp_list:
    fail("knowledge-points.json: kps missing")
KP_KINDS = ("definition", "concept", "technique", "boundary", "integration", "prep")
KP_SAT_LEVELS = ("L1", "L2", "L3")
kp_by_id = {}
for kp_index, kp in enumerate(kp_list):
    kp_at = "knowledge-points.kps[%d]" % kp_index
    if not isinstance(kp, dict):
        fail(kp_at + ": not an object")
        continue
    kp_id = kp.get("id")
    if not isinstance(kp_id, str) or not re.match(r"^kp\.[a-z0-9]+(\.[a-z0-9]+)*$", kp_id):
        fail(kp_at + ": id must look like kp.i1.mem")
        continue
    kp_at += " (" + kp_id + ")"
    if kp_id in kp_by_id:
        fail(kp_at + ": duplicate id")
    else:
        kp_by_id[kp_id] = kp
    if kp.get("chapter") not in chapters:
        fail(kp_at + ": unknown chapter " + repr(kp.get("chapter")))
    if not isinstance(kp.get("title"), str) or not kp["title"].strip():
        fail(kp_at + ": title missing")
    if kp.get("kind") not in KP_KINDS:
        fail(kp_at + ": kind must be one of " + "/".join(KP_KINDS))
    if not isinstance(kp.get("prereq"), list) or any(not isinstance(ref, str) for ref in kp["prereq"]):
        fail(kp_at + ": prereq must be a list of kp ids")
    sat = kp.get("saturation")
    if not isinstance(sat, dict):
        fail(kp_at + ": saturation missing")
        continue
    minimum = sat.get("min")
    if not isinstance(minimum, dict) or set(minimum.keys()) != set(KP_SAT_LEVELS):
        fail(kp_at + ": saturation.min must have exactly L1/L2/L3")
    else:
        for lv in KP_SAT_LEVELS:
            value = minimum[lv]
            if not isinstance(value, int) or isinstance(value, bool) or value < 0:
                fail(kp_at + ": saturation.min." + lv + " must be a non-negative int")
    target = sat.get("target")
    if not isinstance(target, int) or isinstance(target, bool) or target < 1:
        fail(kp_at + ": saturation.target must be a positive int")
    elif isinstance(minimum, dict) and isinstance(minimum.get("L1"), int):
        if sum(minimum.get(lv, 0) for lv in KP_SAT_LEVELS) > target:
            fail(kp_at + ": saturation.min sums above target")
# prereq 交叉引用（知识点之间）
for kp_id, kp in kp_by_id.items():
    for ref in kp.get("prereq") or []:
        if ref not in kp_by_id:
            fail("knowledge-point " + repr(kp_id) + ": prereq refs unknown id " + repr(ref))
        if ref == kp_id:
            fail("knowledge-point " + repr(kp_id) + ": prereq refs itself")

# ---------- 每题的 kps 引用（一题可挂多个；≥2 个算综合题） ----------
integration_by_chapter = {}
for question in questions:
    q_at = "question " + repr(question.get("id"))
    refs = question.get("kps")
    if not isinstance(refs, list) or not refs:
        fail(q_at + ": kps must be a non-empty list (每题至少挂 1 个知识点)")
        continue
    if any(not isinstance(ref, str) for ref in refs):
        fail(q_at + ": kps must contain only kp ids")
        continue
    if len(set(refs)) != len(refs):
        fail(q_at + ": kps has duplicates")
    if len(refs) > 4:
        fail(q_at + ": a question may reference at most 4 kps, got %d" % len(refs))
    for ref in refs:
        kp = kp_by_id.get(ref)
        if kp is None:
            fail(q_at + ": kps refs unknown knowledge-point " + repr(ref))
            continue
        if kp.get("kind") != "prep" and kp.get("chapter") != question.get("chapter"):
            fail(q_at + ": kp " + repr(ref) + " belongs to " + repr(kp.get("chapter"))
                 + " but the question is " + repr(question.get("chapter")))
    if len(set(refs)) >= 2 and question.get("chapter") in chapters:
        integration_by_chapter[question["chapter"]] = integration_by_chapter.get(question["chapter"], 0) + 1

# ---------- 交叉引用（variants/prereq 必须指向存在的 id） ----------
all_ids = {q.get("id") for q in questions}
for q in questions:
    for key in ("variants", "prereq"):
        for ref in q.get(key) or []:
            if ref not in all_ids:
                fail("question " + repr(q.get("id")) + ": " + key + " refs unknown id " + repr(ref))
            if ref == q.get("id"):
                fail("question " + repr(q.get("id")) + ": " + key + " refs itself")

# ---------- 调度参数 ----------
params_data = json.loads((ROOT / "data" / "params.json").read_text(encoding="utf-8"))
if params_data.get("schema") != "soko.learn-params/1":
    fail("params.json: schema")
weights = params_data.get("weights") or {}
for key in ("gap", "challenge", "novelty", "coverage"):
    if not isinstance(weights.get(key), (int, float)):
        fail("params.json: weights." + key + " missing")
if weights and abs(sum(weights.get(k, 0) for k in ("gap", "challenge", "novelty", "coverage")) - 1.0) > 1e-6:
    fail("params.json: weights must sum to 1.0")
if not (0 < params_data.get("eta", 0) < 1):
    fail("params.json: eta must be in (0,1)")

# ---------- 代码练习库 ----------
code_path = ROOT / "data" / "code-exercises.json"
code_bank = None
if code_path.exists():
    code_bank = json.loads(code_path.read_text(encoding="utf-8"))
    if code_bank.get("schema") != "soko.code-exercises/1":
        fail("code-exercises.json: schema")
    kp_ids = {kp["id"] for kp in (json.loads((ROOT / "data" / "knowledge-points.json").read_text(encoding="utf-8")).get("kps") or [])}
    seen_code = set()
    for index, ex in enumerate(code_bank.get("exercises") or []):
        at = "code-exercises[%d]" % index + ((" (" + ex.get("id", "?") + ")") if isinstance(ex, dict) else "")
        if not isinstance(ex, dict):
            fail(at + ": not an object"); continue
        for key in ("id", "chapter", "unit", "title", "prompt", "template", "solution"):
            if not isinstance(ex.get(key), str) or not ex[key].strip():
                fail(at + ": " + key + " missing")
        if ex.get("id") in seen_code:
            fail(at + ": duplicate id")
        seen_code.add(ex.get("id"))
        if "sorry" not in str(ex.get("template", "")):
            fail(at + ": template must contain a sorry hole")
        hints = ex.get("hints")
        if not isinstance(hints, list) or not hints or any(not isinstance(h, str) or not h.strip() for h in hints):
            fail(at + ": hints must be a non-empty string list")
        for kp in ex.get("kps") or []:
            if kp not in kp_ids:
                fail(at + ": unknown kp " + repr(kp))

# ---------- 引理索引（选项/题干里的引理名） ----------
lemmas_path = ROOT / "data" / "lemmas.json"
lemma_count = 0
if lemmas_path.exists():
    lemma_bank = json.loads(lemmas_path.read_text(encoding="utf-8"))
    if lemma_bank.get("schema") != "soko.lemmas/2":
        fail("lemmas.json: schema")
    seen_names = set()
    for index, lemma in enumerate(lemma_bank.get("lemmas") or []):
        at = "lemmas[%d]" % index
        if not isinstance(lemma, dict):
            fail(at + ": not an object"); continue
        for key in ("name", "signature", "oneLiner", "source"):
            if not isinstance(lemma.get(key), str) or not lemma[key].strip():
                fail(at + ": " + key + " missing")
        if lemma.get("name") in seen_names:
            fail(at + ": duplicate name " + repr(lemma.get("name")))
        seen_names.add(lemma.get("name"))
        lemma_count += 1

if problems:
    for problem in problems:
        print("FAIL " + problem, file=sys.stderr)
    print("\nKNOWLEDGE VALIDATE: %d problem(s)" % len(problems), file=sys.stderr)
    sys.exit(1)

print("KNOWLEDGE VALIDATE OK")
print("  cards: %d 张（%s）" % (len(cards), ", ".join(sorted(card_ids))))
print("  questions: %d 题 / %d 章（%s）" % (len(questions), len(qcount), ", ".join("%s:%d" % (k, v) for k, v in sorted(qcount.items()))))
print("  levels: L1=%d L2=%d L3=%d" % (levels[1], levels[2], levels[3]))
print("  params: eta=%.2f weights=%s" % (params_data["eta"], params_data["weights"]))
from collections import Counter as _Counter
kinds = _Counter(q.get("kind", "choice") for q in questions)
print("  kinds: " + ", ".join("%s=%d" % (key, value) for key, value in sorted(kinds.items())))
print("  code-exercises: %d" % (len(code_bank.get("exercises") or []) if code_bank else 0))
print("  lemmas: %d" % lemma_count)
print("  knowledge-points: %d 个（%s）" % (len(kp_by_id), ", ".join(
    "%s:%d" % (chapter, sum(1 for kp in kp_by_id.values() if kp.get("chapter") == chapter))
    for chapter in sorted({kp.get("chapter") for kp in kp_by_id.values()}))))
print("  kps refs: %d 个（综合题 %d 道：%s）" % (
    sum(len(q.get("kps") or []) for q in questions),
    sum(integration_by_chapter.values()),
    ", ".join("%s:%d" % (key, value) for key, value in sorted(integration_by_chapter.items())) or "-"))
