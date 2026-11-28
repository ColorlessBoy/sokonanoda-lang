#!/usr/bin/env python3
"""教学表达底线（文本公理 X1–X10）机检门禁。

公理正文、误报治理与人工评审流程见
`skills/sokonanoda-prep/references/text-axioms.md`；本文件只做**可机检**的那部分
（X1/X3/X4/X5/X6/X7/X8），X2/X9/X10 用 `--review` 出**待人工勾选**清单。

用法（默认扫描课程知识库的全部文本资产）：

    python3 courses/set-theory/knowledge/text_lint.py              # 门禁：按基线判红
    python3 courses/set-theory/knowledge/text_lint.py --selftest   # 红/绿对照 fixtures
    python3 courses/set-theory/knowledge/text_lint.py --review     # X2/X9/X10 人工清单
    python3 courses/set-theory/knowledge/text_lint.py --json-out   # 机读 findings
    python3 courses/set-theory/knowledge/text_lint.py --json X.json      # 任意 JSON 文本
    python3 courses/set-theory/knowledge/text_lint.py --tex deck.tex    # 讲解图（可选）
    python3 courses/set-theory/knowledge/text_lint.py --update-baseline # 收紧存量基线

判据（错误即红，警告按基线只减不增）：
  * error   = X1 残句 / X5 单句 >72 字；
  * warning = X3 定义记号无白话 / X3 术语首现无白话 / X4 抽象断言无例子 /
              X5 单句 >48 字 / X5 逗号粘连 / X6 段首代词 / X7 空洞评价 /
              X8 故事页四要素缺一（--tex）。
存量欠账冻结在 `text-lint-baseline.json`（键 = file+id+field+code，只许减不许增：
新条目判红，已修条目没同步收紧也判红）。

退出码：0 = 通过；1 = 有判红/基线漂移；2 = 用法错误。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

# 仓库根：courses/set-theory/knowledge/text_lint.py → parents[2] = 仓库根。
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
BASELINE = HERE / "text-lint-baseline.json"
FIXTURES = HERE / "text-lint-fixtures.json"
DATA = HERE / "data"

F_LESSONS = "courses/set-theory/knowledge/data/lessons.json"
F_CARDS = "courses/set-theory/knowledge/data/cards.json"
F_QUESTIONS = "courses/set-theory/knowledge/data/questions.json"
F_GLOSSARY = "courses/set-theory/knowledge/data/glossary.json"

# ── 判据常量（原型 v1 的正规化版；每一条改动都要在 text-axioms.md 里写清理由）──

#: 合法的句末/分句收尾（原型口径；`；`/`：` 本身是分句收尾，不算残句）。
TERMINAL = "。！？；：.!?;:"
WEAK_TAIL = "的是吧呢"
META_LINE = re.compile(r"^(卷\s*[IVX]+|[IVX]+\.\d|§)")
CJK = re.compile(r"[\u4e00-\u9fff]")

CONNECTIVES = (
    "因为", "所以", "因此", "于是", "但是", "然而", "而且", "并且",
    "如果", "那么", "也就是说", "换句话说", "即", "而是",
    "同时", "另外", "例如", "比如", "只要", "以及", "然后", "但", "并",
)
EMPTY_PRAISE = (
    "很重要", "非常重要", "很有用", "非常有用", "意义重大",
    "特别重要", "很关键", "至关重要", "极其重要", "必不可少", "不可或缺", "不可忽视",
)
CONSEQUENCE = (
    "因为", "在于", "体现在", "意味着", "所以", "否则", "不然", "导致",
    "后果", "才能", "一旦", "如果", "要是", "使得", "从而",
)
EXAMPLE_MARK = ("例如", "比如", "举个例子", "例子", "如：", "反例", "示例", "譬如")
PLAIN_MARK = (
    "白话", "也就是说", "就是说", "意思是", "换句话说", "一句话", "简单说",
    "直白", "就是", "指的是", "用白话说", "说白了",
)
DEF_MARK = ("定义为", ":=", "称为", "叫做", "记为", "记作")
ABSTRACT_MARK = ("定理", "命题", "性质", "结论")
#: X10（人工评审）的机器提示词：这些词出现处要人工确认没有跳过推理。
SKIP_REASONING = ("显然", "很容易看出", "容易看出", "不言而喻", "众所周知", "显而易见")
STORY_TITLE = ("故事", "历史")
PRONOUN_INITIAL = re.compile(
    r"^[\s`*「『\"“]*(它|这|那|该|此|其|他|她)"
    r"(?![一条种个类些位项章节步两三四五六七八九十])")

#: X5 阈值：>ERROR 字判红，>WARN 字提示（占位符按 1 字计，见 normalize）。
X5_ERROR = 72
X5_WARN = 48
X5_SPLICE_MIN = 40

CODE_SPAN = re.compile(r"`[^`\n]*`")
MD_LINK = re.compile(r"!?\[([^\]]*)\]\([^)]*\)")
MD_EMPHASIS = re.compile(r"\*\*|__|(?<!\*)\*(?!\*)")
LATEX_MATH = re.compile(r"\$[^$]*\$")
LATEX_CMD = re.compile(r"\\[a-zA-Z]+\*?(?:\[[^\]]*\])?(?:\{[^{}]*\})?")


# ── 文本规范化 ────────────────────────────────────────────────────────────────


def normalize(text: str, tex: bool = False) -> str:
    """给「句长」用的规范化：数学占位符按 1 字计（误报治理，见 text-axioms.md §3）。

    行内代码 / LaTeX 数学 / 链接目标 / 强调标记都被折成单个占位符；正文汉字不丢。
    """
    t = text or ""
    if tex:
        t = LATEX_MATH.sub("□", t)
        t = LATEX_CMD.sub(" ", t)
        t = t.replace("{", " ").replace("}", " ").replace("\\\\", " ")
    else:
        t = CODE_SPAN.sub("□", t)
        t = MD_LINK.sub(r"\1", t)
        t = MD_EMPHASIS.sub("", t)
        # Lean 标识符（`Quot.lift` 写在外面没加反引号时）也按 1 字计：代码不该撑爆「句长」。
        t = re.sub(r"[A-Za-z][A-Za-z0-9_.']*", "□", t)
    return re.sub(r"\s+", " ", t).strip()


def sentences(text: str, split_semicolons: bool = False) -> list[str]:
    """切句：`。！？` 是句末；`split_semicolons` 时把 `；` 也当分句边界（目标/清单行）。"""
    pattern = r"(?<=[。！？!?；;])" if split_semicolons else r"(?<=[。！？!?])"
    return [part.strip() for part in re.split(pattern, text) if part.strip()]


# ── 数据结构 ──────────────────────────────────────────────────────────────────


@dataclass
class Unit:
    """一个用户可见的文本单元（一行 prompt、一段 intuition、一张卡的一个列表项…）。"""

    file: str
    id: str
    field: str
    text: str
    #: 列表项：列表续行豁免 X1（见 text-axioms.md §3）。
    is_list_item: bool = False
    #: 标题/元信息行：不受 X1/X4 约束（标题可以是名词短语）。
    is_meta: bool = False
    #: 目标/清单行：X5 按 `；` 分句量长度。
    split_semicolons: bool = False
    #: 术语/定义的白话检查用：正文单元（非标题）才参与术语首现扫描。
    ordinal: int = 0


def finding(unit: Unit, axiom: str, code: str, level: str, note: str, text: str = "") -> dict:
    return {
        "axiom": axiom,
        "code": code,
        "level": level,
        "file": unit.file,
        "id": unit.id,
        "field": unit.field,
        "note": note,
        "text": (text or unit.text)[:160],
    }


# ── 单条公理检查 ──────────────────────────────────────────────────────────────


def check_unit(unit: Unit) -> list[dict]:
    """对单个文本单元跑 X1/X3/X4/X5/X6/X7（X8 在 check_story 里，按页聚合）。"""
    out: list[dict] = []
    raw = (unit.text or "").strip()
    text = normalize(raw)
    if text == "":
        if not unit.is_meta and not unit.is_list_item:
            out.append(finding(unit, "X1", "X1-empty", "error", "正文为空"))
        return out
    meta = unit.is_meta or bool(META_LINE.match(text))

    # X1 完整：正文必须以句末/分句标点收尾（标题、元信息行、列表续行豁免）。
    if not unit.is_list_item and not meta:
        last = text[-1]
        if last not in TERMINAL:
            if last in "，、":
                out.append(finding(unit, "X1", "X1-comma-tail", "error", "残句：逗号/顿号收尾"))
            elif len(text) < 12 and last in WEAK_TAIL:
                out.append(finding(unit, "X1", "X1-weak-tail", "error", "残句：未完成形式收尾"))
            elif len(text) >= 12:
                out.append(finding(unit, "X1", "X1-terminal", "error", "正文没有句末标点"))

    # X5 一句一义：单句过长；并列分句缺连接词的分号/逗号粘连。
    # 长度按占位符折过的规范文本量（误报治理），展示用原文。
    for raw_sentence in sentences(raw, unit.split_semicolons):
        sentence = normalize(raw_sentence)
        if len(sentence) > X5_ERROR:
            out.append(finding(unit, "X5", "X5-long", "error",
                               f"单句超过 {X5_ERROR} 字（占位符按 1 字计）", raw_sentence))
        elif len(sentence) > X5_WARN:
            out.append(finding(unit, "X5", "X5-long", "warning",
                               f"单句超过 {X5_WARN} 字（占位符按 1 字计）", raw_sentence))
        # 逗号/连接词也在**规范化后**的句子上数：行内代码（`(a,b)`）里的逗号不算。
        commas = sentence.count("，") + sentence.count(",")
        if (commas >= 2 and len(sentence) >= X5_SPLICE_MIN
                and not any(mark in sentence for mark in CONNECTIVES)
                and "；" not in sentence):
            out.append(finding(unit, "X5", "X5-splice", "warning",
                               "逗号粘连：并列分句缺连接词或分号", raw_sentence))

    # X3 先定义后使用（机检部分）：出现定义记号却没给白话。
    if not unit.is_meta and any(mark in text for mark in DEF_MARK):
        if not any(mark in text for mark in PLAIN_MARK):
            out.append(finding(unit, "X3", "X3-no-plain", "warning",
                               "出现定义记号但没有白话句"))

    # X4 抽象必配例：定理/命题/性质/结论直给而全文没有例子。
    # 只查解释性正文：题干是在「问」性质、术语行是名词注解，都不承担举例义务。
    field = unit.field.rsplit(".", 1)[-1]
    if (not unit.is_meta and not unit.is_list_item
            and field != "prompt" and not field.startswith("type:")):
        if (any(mark in text for mark in ABSTRACT_MARK)
                and not any(mark in text for mark in EXAMPLE_MARK)
                and "□" not in text):
            out.append(finding(unit, "X4", "X4-no-example", "warning",
                               "抽象断言没有配例子"))

    # X6 指代明确：段首代词（列表项可能有上一条的先行词；术语行的「这是…」指术语本身，
    # 都按误报治理豁免）。
    if (not unit.is_meta and not unit.is_list_item
            and not field.startswith("type:") and PRONOUN_INITIAL.match(text)):
        out.append(finding(unit, "X6", "X6-pronoun", "warning",
                           "段首代词，需确认先行词在上一句"))

    # X7 拒绝空话：空洞评价没有给后果。
    for raw_sentence in sentences(raw):
        sentence = normalize(raw_sentence)
        if any(word in sentence for word in EMPTY_PRAISE) and not any(
                mark in sentence for mark in CONSEQUENCE):
            out.append(finding(unit, "X7", "X7-empty-praise", "warning",
                               "空洞评价，没有给后果", raw_sentence))

    return out


def check_story(page: str, titles: list[str]) -> list[dict]:
    """X8 故事四要素：人物/时间/冲突/结果，四缺二判红、缺一提示（仅 --tex）。"""
    if not any(any(mark in title for mark in STORY_TITLE) for title in titles):
        return []
    text = normalize(page, tex=True)
    has = {
        "time": bool(re.search(r"(\d{3,4}\s*年?|约\s*\d{3,4}|世纪)", text)),
        "person": bool(
            re.search(r"[\u4e00-\u9fff]{2,4}(在|于)\s*约?\s*\d{3,4}\s*年", text)
            or any(mark in text for mark in ("数学家", "学者", "他", "她"))
        ),
        "conflict": bool(
            re.search(r"(但|却|矛盾|反驳|质疑|困境|悖论|挑战|难题|问题|危机)", text)
        ),
        "outcome": bool(
            re.search(r"(所以|于是|因此|最终|结果|后来|由此|这就证明|终于)", text)
        ),
    }
    missing = [name for name, present in has.items() if not present]
    if len(missing) >= 2:
        level, note = "error", "故事页四要素缺 %s" % "、".join(missing)
    elif len(missing) == 1:
        level, note = "warning", "故事页缺 %s" % missing[0]
    else:
        return []
    unit = Unit(file="<tex>", id="（页面）", field="page", text=page, is_meta=True)
    return [finding(unit, "X8", "X8-story", level, note)]


# ── 数据资产：默认扫描范围 ────────────────────────────────────────────────────


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def collect_default_units() -> list[Unit]:
    """按「读者接触顺序」收集课程知识库的用户可见文本。

    范围：lessons（goal/intuition）、cards（oneLiner/段落/列表项/术语行）、
    questions（prompt/why）、glossary（oneLiner）。
    不扫描：标题/标题行、表格单元格（速记格，不是段落）、`definition.code`、
    题目 options（样式由 `text_audit.py` 报告，不是表达公理）。
    """
    units: list[Unit] = []
    order = 0

    def add(**kwargs) -> None:
        nonlocal order
        units.append(Unit(ordinal=order, **kwargs))
        order += 1

    lessons = _load(DATA / "lessons.json").get("lessons") or []
    for lesson in lessons:
        unit = lesson.get("unit") or lesson.get("chapter") or "?"
        add(file=F_LESSONS, id=unit, field="goal", text=lesson.get("goal") or "",
            is_meta=True, split_semicolons=True)
        add(file=F_LESSONS, id=unit, field="intuition", text=lesson.get("intuition") or "")

    cards = _load(DATA / "cards.json").get("cards") or []
    for card in cards:
        cid = card.get("id") or "?"
        add(file=F_CARDS, id=cid, field="oneLiner", text=card.get("oneLiner") or "")
        for section in card.get("sections") or []:
            heading = section.get("heading") or section.get("kind") or "section"
            if section.get("kind") == "paragraph":
                add(file=F_CARDS, id=cid, field=f"section:{heading}",
                    text=section.get("text") or "")
            for index, item in enumerate(section.get("items") or []):
                add(file=F_CARDS, id=cid, field=f"items:{heading}[{index}]",
                    text=item, is_list_item=True)
        for type_entry in card.get("types") or []:
            add(file=F_CARDS, id=cid, field=f"type:{type_entry.get('term', '')}",
                text=type_entry.get("oneLiner") or "")

    questions = _load(DATA / "questions.json").get("questions") or []
    for question in questions:
        qid = question.get("id") or "?"
        add(file=F_QUESTIONS, id=qid, field="prompt", text=question.get("prompt") or "")
        add(file=F_QUESTIONS, id=qid, field="why", text=question.get("why") or "")

    glossary = _load(DATA / "glossary.json").get("entries") or []
    for entry in glossary:
        gid = entry.get("id") or entry.get("term") or "?"
        add(file=F_GLOSSARY, id=gid, field="oneLiner", text=entry.get("oneLiner") or "")

    return units


# ── 任意 JSON（--json，原型兼容；用于临时副本反例验证）────────────────────────

TEXT_KEYS = ("prompt", "why", "goal", "intuition", "oneLiner", "text", "keyPoint")


def collect_json_units(path: Path) -> list[Unit]:
    """通用遍历：文本字段 + `items[]` 字符串（与原型 `walk_json` 同口径）。"""
    display = _display_path(path)
    data = _load(path)
    units: list[Unit] = []
    order = 0

    def add(file: str, id_: str, field_: str, text: str, **kwargs) -> None:
        nonlocal order
        units.append(Unit(file=file, id=id_, field=field_, text=text, ordinal=order, **kwargs))
        order += 1

    def walk(obj, path_: str) -> None:
        if isinstance(obj, dict):
            ident = str(obj.get("id") or obj.get("unit") or obj.get("term")
                        or obj.get("title") or path_)
            for key, value in obj.items():
                here = f"{path_}.{key}" if path_ else str(key)
                if key in TEXT_KEYS and isinstance(value, str):
                    add(display, ident, here, value)
                elif key == "items" and isinstance(value, list):
                    for index, item in enumerate(value):
                        if isinstance(item, str):
                            add(display, ident, f"{here}[{index}]", item, is_list_item=True)
                else:
                    walk(value, here)
        elif isinstance(obj, list):
            for index, item in enumerate(obj):
                walk(item, f"{path_}[{index}]")

    walk(data, "")
    return units


def _display_path(path: Path) -> str:
    try:
        return path.resolve().relative_to(REPO).as_posix()
    except ValueError:
        return path.as_posix()


# ── 术语首现（X3 的完整版）────────────────────────────────────────────────────

TERM_LEAD = re.compile(r"([\u4e00-\u9fff]{2,10})")


def load_terms() -> list[str]:
    """术语来源 = knowledge-points 的 title 主体 + cards.types[].term。"""
    terms: set[str] = set()
    try:
        kps = _load(DATA / "knowledge-points.json").get("kps") or []
    except OSError:
        kps = []
    for kp in kps:
        title = str(kp.get("title") or "")
        match = TERM_LEAD.search(title)
        if match:
            terms.add(match.group(1))
    try:
        cards = _load(DATA / "cards.json").get("cards") or []
    except OSError:
        cards = []
    for card in cards:
        for type_entry in card.get("types") or []:
            term = str(type_entry.get("term") or "").strip()
            if term:
                terms.add(term)
    return sorted(terms)


def check_term_first_occurrence(units: list[Unit], terms: list[str]) -> list[dict]:
    """X3 完整版：术语首次出现所在的讲解段落必须给白话。

    「白话句」= 该段落里有 ≥8 个汉字的句子（且不是纯公式/代码）。术语在正文里
    从未出现（只出现在标题/表格）不算缺口——本检查不要求术语必须出现。
    """
    out: list[dict] = []
    body = [unit for unit in units
            if not unit.is_meta and not unit.is_list_item and unit.text]
    for term in terms:
        first = next((unit for unit in body if term in (unit.text or "")), None)
        if first is None:
            continue
        plain = False
        for sentence in sentences(normalize(first.text)):
            if len(CJK.findall(sentence)) >= 6:
                plain = True
                break
        if not plain:
            out.append(finding(first, "X3", "X3-term-first", "warning",
                               f"术语「{term}」首次出现处没有白话句"))
    return out


# ── 扫描入口 ──────────────────────────────────────────────────────────────────


def scan(units: list[Unit], *, with_terms: bool = True) -> list[dict]:
    findings: list[dict] = []
    for unit in units:
        findings.extend(check_unit(unit))
    if with_terms:
        findings.extend(check_term_first_occurrence(units, load_terms()))
    return dedupe(findings)


def dedupe(findings: list[dict]) -> list[dict]:
    """同一 (file,id,field,code) 的重复只留一条（基线键的粒度就是它）。"""
    seen: set[tuple] = set()
    out: list[dict] = []
    for item in findings:
        key = (item["file"], item["id"], item["field"], item["code"])
        if key in seen:
            continue
        seen.add(key)
        out.append(item)
    return sorted(out, key=lambda f: (f["file"], f["id"], f["field"], f["code"]))


# ── LaTeX 讲解图（--tex，可选输入；源文件不入库）─────────────────────────────


def brace_body(source: str, start: int) -> str:
    depth = 0
    for index in range(start, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return source[start + 1:index]
    return ""


def tex_nodes(source: str) -> list[tuple[int, str, str]]:
    """返回每个 `\\node[...]{...}` 的 (行号, 正文, 选项)。"""
    nodes: list[tuple[int, str, str]] = []
    index = 0
    while True:
        pos = source.find("\\node", index)
        if pos < 0:
            break
        line = source.count("\n", 0, pos) + 1
        brace = source.find("{", pos)
        if brace < 0:
            break
        options = ""
        bracket = source.find("[", pos)
        if 0 <= bracket < brace:
            depth = 0
            for cursor in range(bracket, brace):
                if source[cursor] == "[":
                    depth += 1
                elif source[cursor] == "]":
                    depth -= 1
                    if depth == 0:
                        options = source[bracket + 1:cursor]
                        break
        body = brace_body(source, brace)
        nodes.append((line, body, options))
        index = brace + len(body) + 2
    return nodes


def scan_tex(path: Path) -> list[dict]:
    source = path.read_text(encoding="utf-8")
    nodes = tex_nodes(source)
    display = _display_path(path)
    units: list[Unit] = []
    for order, (line, body, options) in enumerate(nodes):
        is_title = "bfseries" in options or "Large" in options
        units.append(Unit(file=display, id=f"line {line}", field="node", text=body,
                          is_meta=is_title, ordinal=order))
    findings: list[dict] = []
    for unit in units:
        findings.extend(check_unit(unit))
    page = " ".join(body for _, body, _ in nodes)
    titles = [body for line, body, options in nodes
              if "bfseries" in options or "Large" in options]
    findings.extend(check_story(page, titles))
    return findings


# ── 基线（存量欠账：按 file+id+field+code 冻结，只减不许增）────────────────────


def baseline_key(item: dict) -> tuple:
    return (item["file"], item["id"], item["field"], item["code"])


def load_baseline(path: Path) -> dict[tuple, dict]:
    if not path.exists():
        return {}
    data = json.loads(path.read_text(encoding="utf-8"))
    entries = data.get("entries") if isinstance(data, dict) else data
    out: dict[tuple, dict] = {}
    for entry in entries or []:
        out[baseline_key(entry)] = entry
    return out


def baseline_payload(findings: list[dict], previous: dict[tuple, dict]) -> dict:
    entries = []
    for item in findings:
        key = baseline_key(item)
        entries.append({
            "file": key[0],
            "id": key[1],
            "field": key[2],
            "code": key[3],
            "axiom": item["axiom"],
            "level": item["level"],
            "note": previous.get(key, {}).get("note") or item["note"],
        })
    errors = sum(1 for item in findings if item["level"] == "error")
    warnings = len(findings) - errors
    return {
        "schema": "soko.text-lint-baseline/1",
        "note": ("文本公理门禁的存量欠账基线：键 = file+id+field+code；"
                 "只许减不许增（新条目判红，已修条目没同步收紧也判红）。"
                 "收紧用 `python3 courses/set-theory/knowledge/text_lint.py --update-baseline`。"),
        "updated": "2026-11-28",
        "errors": errors,
        "warnings": warnings,
        "entries": entries,
    }


def check_baseline(findings: list[dict], baseline: dict[tuple, dict]) -> tuple[list, list]:
    """返回 (新增条目, 已修条目)。新增 = 不在基线；已修 = 在基线但当前没有。"""
    current = {baseline_key(item): item for item in findings}
    new = [item for key, item in current.items() if key not in baseline]
    stale = [entry for key, entry in baseline.items() if key not in current]
    return new, stale


# ── 报告 / 自检 / 人工清单 ────────────────────────────────────────────────────


def counts(findings: list[dict]) -> tuple[int, int]:
    errors = sum(1 for item in findings if item["level"] == "error")
    return errors, len(findings) - errors


def print_findings(findings: list[dict], stream=sys.stdout) -> None:
    for item in findings:
        print("[%s] %s %s %s:%s:%s — %s → %s" % (
            item["level"], item["axiom"], item["code"], item["file"],
            item["id"], item["field"], item["note"], item["text"]), file=stream)


def run_selftest() -> int:
    """红/绿对照 fixtures（每条公理至少一条必红、一条必绿）+ 基线棘轮 + 端到端反例。"""
    if not FIXTURES.exists():
        print(f"SELFTEST FAIL: 找不到 {FIXTURES}")
        return 1
    fixtures = json.loads(FIXTURES.read_text(encoding="utf-8"))
    failures: list[str] = []
    passed = 0
    for axiom, cases in (fixtures.get("axioms") or {}).items():
        for case in cases.get("red") or []:
            got = _fixture_findings(case)
            hits = [item for item in got if item["axiom"] == axiom
                    and item["level"] == case.get("expect", "error")]
            if hits:
                passed += 1
            else:
                failures.append(f"{axiom} 红例没有被咬住：{case.get('name') or case}")
        for case in cases.get("green") or []:
            got = _fixture_findings(case)
            hits = [item for item in got if item["axiom"] == axiom]
            if hits:
                failures.append(f"{axiom} 绿例被误报：{case.get('name') or case} → {hits[0]['note']}")
            else:
                passed += 1
    # 基线棘轮：新条目判红、已修条目没收紧也判红。
    base = {("f", "id", "field", "X1-terminal"): {"note": "fixture"}}
    new, stale = check_baseline(
        [{"file": "f", "id": "id", "field": "field", "code": "X1-terminal", "level": "error",
          "axiom": "X1", "note": "fixture"},
         {"file": "f", "id": "id2", "field": "field", "code": "X5-long", "level": "warning",
          "axiom": "X5", "note": "fixture"}],
        base)
    if len(new) == 1 and not stale:
        passed += 1
    else:
        failures.append("基线棘轮：新条目没有被判红")
    if len(check_baseline([], base)[1]) == 1:
        passed += 1
    else:
        failures.append("基线棘轮：已修条目没有被要求收紧")
    # 端到端反例：临时副本里故意写残句 ⇒ 扫描必须出 X1 error；正常卡片 ⇒ 0 error。
    with tempfile.TemporaryDirectory() as tmp:
        broken = Path(tmp) / "broken.json"
        good = Path(tmp) / "good.json"
        broken.write_text(json.dumps({"lessons": [{"unit": "x", "goal": "目标。",
            "intuition": "费马（约 1659）：要证『不存在』，"}]}, ensure_ascii=False),
            encoding="utf-8")
        good.write_text(json.dumps({"lessons": [{"unit": "x", "goal": "目标。",
            "intuition": "费马大约在 1659 年想到无穷递降。假设有最小解，就会推出更小的解。"}]},
            ensure_ascii=False), encoding="utf-8")
        broken_errors = [item for item in scan(collect_json_units(broken), with_terms=False)
                         if item["level"] == "error"]
        good_errors = [item for item in scan(collect_json_units(good), with_terms=False)
                       if item["level"] == "error"]
        if broken_errors and not good_errors:
            passed += 1
        else:
            failures.append("端到端反例：残句没判红 / 正常句被误红")
    if failures:
        print("SELFTEST FAIL")
        for failure in failures:
            print("  -", failure)
        return 1
    print(f"SELFTEST OK ({passed} 条对照；覆盖 {', '.join(sorted((fixtures.get('axioms') or {}).keys()))})")
    return 0


def _fixture_findings(case: dict) -> list[dict]:
    if case.get("kind") == "story":
        return check_story(case.get("page") or "", case.get("titles") or [])
    if case.get("kind") == "term":
        units = [Unit(file="fixture", id=f"u{i}", field="intuition", text=text,
                      ordinal=i) for i, text in enumerate(case.get("units") or [])]
        return check_term_first_occurrence(units, [case.get("term") or ""])
    unit = Unit(
        file="fixture",
        id=case.get("id") or "fixture",
        field=case.get("field") or "intuition",
        text=case.get("text") or "",
        is_list_item=bool(case.get("is_list_item")),
        is_meta=bool(case.get("is_meta")),
        split_semicolons=bool(case.get("split_semicolons")),
    )
    return check_unit(unit)


def review(units: list[Unit]) -> None:
    """X2/X9/X10 人工评审清单（不判红；给内容作者逐条勾）。"""
    print("# 文本人工评审清单（X2 一事一段 / X9 可检验 / X10 不跳过推理）")
    print()
    print("> 机器只给证据，勾选由人做。命令：")
    print("> `python3 courses/set-theory/knowledge/text_lint.py --review`")
    print()
    # X2/X9/X10 是**页面级**（讲解段/卡片）评审项，不套在题目题干与选项上。
    groups: dict[tuple[str, str], list[Unit]] = {}
    for unit in units:
        if unit.is_meta or unit.field in ("goal",) or unit.file == F_QUESTIONS:
            continue
        groups.setdefault((unit.file, unit.id), []).append(unit)
    for (file, ident), group in groups.items():
        print(f"## {Path(file).name} · {ident}")
        for unit in group:
            text = unit.text
            parts = sentences(text)
            if not parts:
                continue
            first, last = parts[0], parts[-1]
            print(f"- [ ] **X2 一事一段** `{unit.field}`：首句是主题句？"
                  f"「{_clip(first)}」；末句给结论/下一步？「{_clip(last)}」")
        body = " ".join(unit.text for unit in group)
        for sentence in sentences(body):
            if any(mark in sentence for mark in SKIP_REASONING):
                print(f"- [ ] **X10 不跳过推理**：把「{_clip(sentence)}」改成可复述的论证")
        if group:
            print(f"- [ ] **X9 可检验**：能用一句话复述「{_clip(group[0].text)}」吗？")
        print()


def _clip(text: str, limit: int = 60) -> str:
    text = (text or "").strip()
    return text if len(text) <= limit else text[:limit] + "…"


# ── 主流程 ────────────────────────────────────────────────────────────────────


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="教学表达底线（文本公理 X1–X10）机检门禁",
        formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--json", action="append", default=[], metavar="PATH",
                        help="扫描任意 JSON 的文本字段（可重复；默认扫课程数据资产）")
    parser.add_argument("--tex", action="append", default=[], metavar="PATH",
                        help="扫描讲解图 .tex 的 \\node 文本与故事页四要素（可选）")
    parser.add_argument("--selftest", action="store_true", help="红/绿对照 fixtures")
    parser.add_argument("--review", action="store_true", help="输出 X2/X9/X10 人工评审清单")
    parser.add_argument("--json-out", action="store_true", help="机读 findings（stdout）")
    parser.add_argument("--baseline", default=str(BASELINE), help="基线清单路径")
    parser.add_argument("--no-baseline", action="store_true", help="不按基线判（全部列出）")
    parser.add_argument("--update-baseline", action="store_true", help="把当前 findings 写成新基线")
    args = parser.parse_args(argv)

    if args.selftest:
        return run_selftest()
    if args.review:
        review(collect_default_units())
        return 0

    findings: list[dict] = []
    if args.json:
        for path in args.json:
            findings.extend(scan(collect_json_units(Path(path)), with_terms=False))
    if args.tex:
        for path in args.tex:
            findings.extend(scan_tex(Path(path)))
    if not args.json and not args.tex:
        findings = scan(collect_default_units())
    findings = dedupe(findings)

    errors, warnings = counts(findings)
    baseline_path = Path(args.baseline)
    new: list[dict] = []
    stale: list[dict] = []
    if not args.no_baseline:
        if args.update_baseline:
            previous = load_baseline(baseline_path)
            baseline_path.write_text(
                json.dumps(baseline_payload(findings, previous), ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8")
            print(f"text-lint: 基线已写入 {baseline_path.relative_to(REPO) if baseline_path.is_relative_to(REPO) else baseline_path}"
                  f"（errors {errors} / warnings {warnings}）")
            return 0
        baseline = load_baseline(baseline_path)
        new, stale = check_baseline(findings, baseline)

    if args.json_out:
        print(json.dumps({"errors": errors, "warnings": warnings,
                          "findings": findings, "new": new, "stale": stale},
                         ensure_ascii=False, indent=2))
    else:
        if args.no_baseline:
            print_findings(findings)
        elif new:
            print("新增（不在存量基线里，必须修掉或收紧基线）：")
            print_findings(new)
        if stale:
            print("基线已过期（这些条目已修，跑 --update-baseline 收紧）：")
            for entry in stale:
                print(f"  {entry['file']}:{entry['id']}:{entry['field']}:{entry['code']}")
        by_code: dict[tuple[str, str], int] = {}
        for item in findings:
            by_code[(item["axiom"], item["code"])] = by_code.get((item["axiom"], item["code"]), 0) + 1
        if not args.no_baseline and not (new or stale) and findings:
            print("存量基线（只减不增）：" + "、".join(
                f"{code} {count}" for (_, code), count in sorted(by_code.items())))
        print(f"text-lint: {errors} errors, {warnings} warnings"
              + ("" if args.no_baseline else f"（基线外新增 {len(new)} 条、过期 {len(stale)} 条）"))

    if errors or (not args.no_baseline and (new or stale)):
        return 1
    if not args.no_baseline:
        # 基线存在但 error 级别条目未清零时也判红（存量 errors 清账的判据）。
        baseline = load_baseline(Path(args.baseline))
        if any(entry.get("level") == "error" for entry in baseline.values()):
            print("text-lint: 基线里仍有 error 级存量，必须先清零（errors 清零是本单验收线）")
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
