#!/usr/bin/env python3
"""按 STYLE.md 扫描「不是完整句子」的文本，输出报告（不判红，供内容作者批量改写）。

用法：
  python3 text_audit.py                 # 摘要
  python3 text_audit.py --list prompt   # 列出某类问题
  python3 text_audit.py --json          # 机读
判据（启发式）：
  - 空串 / 只有符号
  - 不以 。？！ 结尾，且长度 < 12
  - 结尾是「的 / 是 / 吧 / 呢 / ：」这类未完成形式
  - 含「是？」「用？」「取出？」这类无主语残句
  - `option.length`：含中文的选项不在 8–60 字（STYLE §2）
  - `option.length-leak`：正确项比最长干扰项长 >15 字（长度泄漏答案）
  - `why.short`：`why` 少于 2 个完整句子（STYLE §3 要 2–4 句）
"""
import json
import re
import sys

CJK_RE = re.compile(r"[\u4e00-\u9fff]")
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DATA = ROOT / "data"

END = ("。", "？", "！", "?", "!")
WEAK_TAIL = ("的", "是", "吧", "呢", "：", ":", "，", ",", "、")


def is_sentence(text: str) -> bool:
    t = (text or "").strip()
    if t == "":
        return False
    bare = re.sub(r"[`*\s]", "", t)
    if bare == "":
        return False
    if len(t) < 8:
        return False
    if t.endswith(END):
        return True
    if t.endswith(WEAK_TAIL):
        return False
    # 没有句末标点但足够长（≥24 字）也算可接受（长句无标点时可能被截断，单独提示）
    return len(t) >= 24 and any(mark in t for mark in ("，", "；", "、", ":", "："))


STEM_MARKERS = re.compile(r"(是什么|哪些|哪一个|哪个|怎么做|应当|应该|是否|为什么|哪条|如何|下列|下面|正确|错误|能否|会怎样|说法|取决于|意味着|属于|采用)")


def stem_ok(text: str) -> bool:
    """题干：必须以？结尾，且含明确提问标记（避免「XX 是？」这类残句）。"""
    t = (text or "").strip()
    if not t.endswith(("？", "?")):
        return False
    return bool(STEM_MARKERS.search(t)) and is_sentence(t)


ENDING = re.compile(r"[。！？!?]")
OPTION_MIN, OPTION_MAX = 8, 60
LEAK_MARGIN = 15


def option_length_issues(question):
    """含中文的选项不在 8–60 字（STYLE §2；纯代码/符号选项豁免）。"""
    issues = []
    for index, option in enumerate(question.get("options") or []):
        text = str(option).strip()
        if CJK_RE.search(text) and not (OPTION_MIN <= len(text) <= OPTION_MAX):
            issues.append(("options[%d]" % index, text))
    return issues


def option_leak_issue(question):
    """正确项比最长干扰项长 > LEAK_MARGIN 字 ⇒ 选项长度泄漏答案（返回 delta 或 None）。"""
    options = question.get("options") or []
    answer = question.get("answer")
    if not isinstance(answer, int) or isinstance(answer, bool) or len(options) < 2:
        return None
    if not (1 <= answer <= len(options)):
        return None
    lengths = [len(str(option).strip()) for option in options]
    others = [lengths[index] for index in range(len(lengths)) if index != answer - 1]
    delta = lengths[answer - 1] - max(others)
    return delta if delta > LEAK_MARGIN else None


def why_short_issue(question):
    """`why` 少于 2 个完整句子（STYLE §3 要 2–4 句）。"""
    why = str(question.get("why") or "")
    return why if len(ENDING.findall(why)) < 2 else None


def scan():
    problems = {}

    def add(kind, item_id, field, text):
        problems.setdefault(kind, []).append({"id": item_id, "field": field, "text": text})

    bank = json.loads((DATA / "questions.json").read_text(encoding="utf-8"))
    for q in bank["questions"]:
        if not stem_ok(q.get("prompt")):
            add("prompt", q["id"], "prompt", q.get("prompt", ""))
        for index, option in enumerate(q.get("options") or []):
            if is_sentence(option):
                continue
            # 纯代码/符号选项不算问题（见 STYLE.md §2 例外）
            if CJK_RE.search(option) is None:
                continue
            add("option", q["id"], "options[%d]" % index, option)
        if q.get("why") and not is_sentence(q["why"]):
            add("why", q["id"], "why", q["why"])
        for field, text in option_length_issues(q):
            add("option.length", q["id"], field, text)
        delta = option_leak_issue(q)
        if delta is not None:
            add("option.length-leak", q["id"], "answer",
                "正确项比最长干扰项长 %d 字" % delta)
        if why_short_issue(q):
            add("why.short", q["id"], "why", q.get("why", ""))

    cards = json.loads((DATA / "cards.json").read_text(encoding="utf-8"))
    for card in cards["cards"]:
        if card.get("oneLiner") and not is_sentence(card["oneLiner"]):
            add("card.oneLiner", card["id"], "oneLiner", card["oneLiner"])
        for section in card.get("sections") or []:
            if section.get("text") and not is_sentence(section["text"]):
                add("card.section", card["id"], section.get("heading", ""), section["text"])
            for item in section.get("items") or []:
                if not is_sentence(item):
                    add("card.item", card["id"], section.get("heading", ""), item)

    lessons = json.loads((DATA / "lessons.json").read_text(encoding="utf-8")) if (DATA / "lessons.json").exists() else {"lessons": []}
    for lesson in lessons.get("lessons") or []:
        for field in ("goal", "intuition"):
            if lesson.get(field) and not is_sentence(lesson[field]):
                add("lesson." + field, lesson.get("unit", ""), field, lesson[field])

    glossary = json.loads((DATA / "glossary.json").read_text(encoding="utf-8"))
    for entry in glossary["entries"]:
        if entry.get("oneLiner") and not is_sentence(entry["oneLiner"]):
            add("glossary", entry.get("term", ""), "oneLiner", entry["oneLiner"])

    return problems


def main() -> int:
    problems = scan()
    if "--list" in sys.argv:
        kind = sys.argv[sys.argv.index("--list") + 1]
        rows = problems.get(kind, [])
        for row in rows:
            print("%s | %s | %s" % (row["id"], row["field"], row["text"][:90]))
        print("total: %d" % len(rows))
        return 0
    if "--json" in sys.argv:
        print(json.dumps({kind: len(rows) for kind, rows in problems.items()}, ensure_ascii=False))
        return 0
    total = sum(len(rows) for rows in problems.values())
    print("TEXT AUDIT: %d 处不是完整句子" % total)
    for kind in sorted(problems, key=lambda k: -len(problems[k])):
        print("  %-16s %d" % (kind, len(problems[kind])))
    return 0


if __name__ == "__main__":
    sys.exit(main())
