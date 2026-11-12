#!/usr/bin/env python3
"""Build the interactive HTML book for the whole analysis course (M1+M2).

The builder discovers every chapter/unit from ``courses/analysis/course.json``
and reads the matching canvases and solution files from ``courses/analysis/``.
It does not hardcode a chapter id or a unit count: adding M3 units to the
course manifest (and the unit files) makes them appear on the next build.

Usage (from the repository root):

    python3 scripts/build_book_proto.py --out dist/book/book-v2
    python3 scripts/build_book_proto.py --check --out dist/book/book-v2
    python3 scripts/build_book_proto.py --out scripts/book/preview
    python3 scripts/build_book_proto.py --check --check-preview

The generated page is a single self-contained HTML file: CSS and JavaScript are
inlined, math is Unicode plus native MathML, and there are no external http(s)
references, CDN assets, or web fonts.  Standard library only.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import re
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass, field
from html.parser import HTMLParser
from pathlib import Path
from typing import Dict, Iterable, Iterator, List, Optional, Sequence, Tuple


ROOT = Path(__file__).resolve().parent.parent
COURSE_ROOT = ROOT / "courses" / "analysis"
COURSE_JSON = COURSE_ROOT / "course.json"
BOOK_DIR = ROOT / "scripts" / "book"
STYLE_CSS = BOOK_DIR / "style.css"
APP_JS = BOOK_DIR / "app.js"
PREVIEW_HTML = BOOK_DIR / "preview" / "index.html"
STYLE_LINT = ROOT / "scripts" / "content_style" / "style_lint.py"
DEFAULT_OUT = "dist/book/book-v2"

DECL_RE = re.compile(r"^(def|theorem|axiom|inductive)\s+([^\s(:]+)")
IMPORT_RE = re.compile(r"^import\s+")
TITLE_RE = re.compile(r"^unit\d+[-_].*\.sokonanoda")
UNIT_PREFIX_RE = re.compile(
    r"^单元\s*(?:[①-⑳]|[㉑-㉟]|[㊱-㊿]|[0-9]+)\s*"
)

FORMULA_MARKERS = set("=∀∃→←↔∈∉⊆⊂≤≥≠⇒⇔⟺∧∨¬")
MATH_SYMBOLS = FORMULA_MARKERS | set(
    "ℕℤℚℝℂαβγδεζηθικλμνξοπρστυφχψωΓΔΘΛΞΠΣΦΨΩ×·−+⁰¹²³⁴⁵⁶⁷⁸⁹⁻"
)
MATH_RUN_RE = re.compile(
    r"[A-Za-z0-9_'.(){}\[\]+\-*/=<>"
    r"≤≥≠∈∉⊆⊂→←↔⇒⇔⟺∀∃∧∨¬"
    r"ℕℤℚℝℂαβγδεζηθικλμνξοπρστυφχψωΓΔΘΛΞΠΣΦΨΩ"
    r"·×−+⁰¹²³⁴⁵⁶⁷⁸⁹⁻]+"
)
MATH_SPAN_RE = re.compile(rf"(?:{MATH_RUN_RE.pattern})(?:[ \t]+(?:{MATH_RUN_RE.pattern}))*")
BULLET_RE = re.compile(r"^\s*(?:[·•]|[-–—](?=\s))")
HEADING_RE = re.compile(r"^\s*(?:[0-9]+(?:\.[0-9]+)+|[①-⑳]\.\d+)\s+")
DECOR_RE = re.compile(r"^\s*[=─━\-—_*]{4,}\s*$")
SEPARATOR_LINE_RE = re.compile(r"^\s*[=\-─]{10,}\s*$")
# A paragraph that only repeats the unit heading ("单元㉕ 有理数相等的传递性
# （ratRel_trans）") is a heading echo, not learner-facing prose.
TITLE_ECHO_RE = re.compile(
    r"^\s*单元\s*(?:[①-⑳]|[㉑-㉟]|[㊱-㊿]|[0-9]+)\s*[：:·\-—]?\s*"
)
# File-header / course-engineering notes.  They are kept (never dropped) but
# rendered in a collapsed "编写注记" panel instead of the reading flow.
COURSE_NOTE_RE = re.compile(
    r"^(?:"
    r"靶子(?:\s*[（(]教材[）)])?\s*[：:]"
    r"|大纲\s*[：:]"
    r"|依赖\s*[：:]"
    r"|本单元只允许用"
    r"|本单元\s*`?[Ii]mport`?"
    r"|本单元自带"
    r"|本文件按\s*L3"
    r"|⚠"
    r")"
)
MAX_PROSE_CHARS = 1200
SUP_RE = re.compile(r"^[⁻]?[⁰¹²³⁴⁵⁶⁷⁸⁹]+$")
SUP_TRANS = str.maketrans("⁰¹²³⁴⁵⁶⁷⁸⁹⁻", "0123456789-")
MML_TOKEN_RE = re.compile(
    r"\s+"
    r"|\([^()]*\)"
    r"|\[[^\]]*\]"
    r"|[A-Za-z_][A-Za-z0-9_'.]*"
    r"|\d+(?:\.\d+)?"
    r"|[⁻]?[⁰¹²³⁴⁵⁶⁷⁸⁹]+"
    r"|[^\s]"
)
KIND_LABELS = {
    "def": "定义",
    "theorem": "定理",
    "axiom": "公理",
    "inductive": "归纳类型",
}


# ── data model ──────────────────────────────────────────────────────────────


@dataclass
class Hint:
    label: str
    body: str


@dataclass
class Item:
    kind: str  # text | formula | list | heading | rule | hint
    text: str = ""
    lines: List[str] = field(default_factory=list)
    hint: Optional[Hint] = None


@dataclass
class Decl:
    kind: str
    name: str
    code: str
    statement: str
    body: str
    is_exercise: bool
    has_sorry: bool
    index: int = 0
    exercise_no: int = 0
    lead: List[Item] = field(default_factory=list)
    hints: List[Hint] = field(default_factory=list)
    solution: Optional["Decl"] = None
    solution_note: str = ""


@dataclass
class Unit:
    index: int
    title: str
    title_en: str
    file: str
    chapter_id: str
    chapter_title: str
    chapter_position: int
    volume_id: str
    volume_title: str
    decls: List[Decl] = field(default_factory=list)
    trailing: List[Item] = field(default_factory=list)
    intro: List[Item] = field(default_factory=list)
    notes: List[Item] = field(default_factory=list)
    imports: List[str] = field(default_factory=list)
    solution_path: Optional[Path] = None
    solution_decls: Dict[str, Decl] = field(default_factory=dict)
    extra_solutions: List[Decl] = field(default_factory=list)
    stray: List[str] = field(default_factory=list)

    @property
    def display_title(self) -> str:
        stripped = UNIT_PREFIX_RE.sub("", self.title).strip()
        return stripped or self.title

    @property
    def chapter_slug(self) -> str:
        return chapter_slug(self.chapter_id)

    @property
    def exercises(self) -> List[Decl]:
        return [decl for decl in self.decls if decl.is_exercise]

    @property
    def exercise_count(self) -> int:
        return len(self.exercises)

    @property
    def hints(self) -> List[Hint]:
        hints: List[Hint] = []
        for decl in self.decls:
            hints.extend(decl.hints)
        return hints

    @property
    def blocks(self) -> List[object]:
        return [*self.decls, *self.trailing]

    @property
    def defs(self) -> int:
        return sum(1 for decl in self.decls if decl.kind == "def")

    @property
    def theorems(self) -> int:
        return sum(1 for decl in self.decls if decl.kind == "theorem")

    @property
    def axioms(self) -> int:
        return sum(1 for decl in self.decls if decl.kind == "axiom")

    @property
    def inductives(self) -> int:
        return sum(1 for decl in self.decls if decl.kind == "inductive")


@dataclass
class Chapter:
    id: str
    title: str
    volume_id: str
    volume_title: str
    position: int
    units: List[Unit] = field(default_factory=list)

    @property
    def slug(self) -> str:
        return chapter_slug(self.id)

    @property
    def anchor(self) -> str:
        return f"chapter-{self.slug}"

    @property
    def exercises(self) -> int:
        return sum(unit.exercise_count for unit in self.units)

    @property
    def hints(self) -> int:
        return sum(len(unit.hints) for unit in self.units)

    @property
    def declarations(self) -> int:
        return sum(len(unit.decls) for unit in self.units)


@dataclass
class Book:
    title: str
    volume_titles: List[str]
    chapters: List[Chapter]
    source_fingerprint: str
    import_count: int
    unlisted_unit_files: List[str] = field(default_factory=list)
    manifest_unit_count: int = 0

    @property
    def units(self) -> List[Unit]:
        return [unit for chapter in self.chapters for unit in chapter.units]

    @property
    def chapter_span(self) -> str:
        if not self.chapters:
            return ""
        first, last = self.chapters[0].id, self.chapters[-1].id
        return first if first == last else f"{first}–{last}"

    @property
    def exercises(self) -> int:
        return sum(unit.exercise_count for unit in self.units)

    @property
    def hints(self) -> int:
        return sum(len(unit.hints) for unit in self.units)

    @property
    def declarations(self) -> int:
        return sum(len(unit.decls) for unit in self.units)

    @property
    def solutions(self) -> int:
        return sum(1 for unit in self.units for decl in unit.exercises if decl.solution)

    @property
    def extra_solutions(self) -> int:
        return sum(len(unit.extra_solutions) for unit in self.units)

    @property
    def defs(self) -> int:
        return sum(unit.defs for unit in self.units)

    @property
    def theorems(self) -> int:
        return sum(unit.theorems for unit in self.units)

    @property
    def axioms(self) -> int:
        return sum(unit.axioms for unit in self.units)

    @property
    def inductives(self) -> int:
        return sum(unit.inductives for unit in self.units)


# ── source parsing ──────────────────────────────────────────────────────────


def escape(text: str) -> str:
    return html.escape(text, quote=False)


def strip_comment(line: str) -> str:
    """Turn one `-- ...` line into its content, preserving indentation."""
    content = line[2:]
    if content.startswith(" "):
        content = content[1:]
    return content.rstrip()


def _is_cjk(ch: str) -> bool:
    code = ord(ch)
    return 0x3400 <= code <= 0x9FFF or 0xF900 <= code <= 0xFAFF


def _looks_formula(line: str) -> bool:
    text = line.strip()
    if len(text) < 2 or len(text) > 320:
        return False
    if BULLET_RE.match(line):
        return False
    if not any(ch in FORMULA_MARKERS for ch in text):
        return False
    cjk = sum(1 for ch in text if _is_cjk(ch))
    if cjk > 0:
        return False
    # Avoid treating a quoted sentence that happens to contain "=" as math.
    if text.startswith(("`", "“", "「", "『")):
        return False
    return True


def _is_codeish_formula(text: str) -> bool:
    return bool(re.search(r"(?:^|\s)(?:fun|match)\b|@[A-Za-z]|:=|=>", text))


def _is_heading(text: str) -> bool:
    return bool(HEADING_RE.match(text))


def _is_title_line(text: str, unit_title: str, filename: str) -> bool:
    stripped = text.strip()
    if not stripped:
        return False
    if stripped == unit_title:
        return True
    if stripped.startswith(unit_title) and len(stripped) <= len(unit_title) + 8:
        return True
    if stripped.startswith(filename):
        return True
    if TITLE_RE.match(stripped):
        return True
    if stripped.startswith("单元") and "解答" in stripped and len(stripped) < 80:
        return True
    if stripped.startswith("单元") and "：" in stripped:
        head = stripped.split("：", 1)[0].strip()
        if head == unit_title or head.startswith(unit_title) or unit_title.startswith(head):
            return True
    return False


def _split_paragraph_lines(lines: Sequence[str], unit_title: str, filename: str) -> List[Item]:
    """Split a run of clean comment lines into prose / formula / list items."""
    items: List[Item] = []
    text_buf: List[str] = []
    list_items: List[str] = []
    current_item: Optional[str] = None

    def flush_text() -> None:
        if not text_buf:
            return
        items.append(Item("text", text="\n".join(text_buf).rstrip()))
        text_buf.clear()

    def flush_list() -> None:
        nonlocal current_item
        if current_item is not None:
            list_items.append(current_item)
            current_item = None
        if list_items:
            items.append(Item("list", lines=list(list_items)))
            list_items.clear()

    for raw in lines:
        if _is_title_line(raw, unit_title, filename):
            continue
        if not raw.strip():
            flush_text()
            flush_list()
            continue
        if _is_heading(raw):
            flush_text()
            flush_list()
            items.append(Item("heading", text=raw.strip()))
            continue
        if _looks_formula(raw):
            flush_text()
            flush_list()
            items.append(Item("formula", text=raw.strip()))
            continue
        if BULLET_RE.match(raw):
            flush_text()
            if current_item is not None:
                list_items.append(current_item)
            current_item = raw.strip()
            continue
        if current_item is not None and raw.startswith((" ", "\t")):
            current_item += "\n" + raw.strip()
            continue
        flush_list()
        text_buf.append(raw.rstrip())

    flush_text()
    flush_list()
    return items


def _clean_items(items: Sequence[Item]) -> List[Item]:
    """Collapse decorative rules and drop leading/trailing ones."""
    cleaned: List[Item] = []
    for item in items:
        if item.kind == "rule" and (not cleaned or cleaned[-1].kind == "rule"):
            continue
        cleaned.append(item)
    while cleaned and cleaned[0].kind == "rule":
        cleaned.pop(0)
    while cleaned and cleaned[-1].kind == "rule":
        cleaned.pop()
    return cleaned


def parse_comment_group(raw_lines: Sequence[str], unit_title: str, filename: str) -> List[Item]:
    """Parse consecutive `-- ...` lines into ordered prose/hint items."""
    items: List[Item] = []
    paras: List[str] = []
    current_hint: Optional[Hint] = None

    def flush_paras() -> None:
        nonlocal paras
        if not paras:
            return
        items.extend(_split_paragraph_lines(paras, unit_title, filename))
        paras = []

    for raw in raw_lines:
        content = strip_comment(raw)
        stripped = content.strip()

        if not stripped:
            flush_paras()
            current_hint = None
            continue

        if stripped.startswith("soko:hint"):
            flush_paras()
            body = stripped[len("soko:hint"):].strip()
            label = "提示"
            rest = body
            match = re.match(r"^([^：:]{1,16})[：:]\s*(.*)$", body)
            if match:
                label = match.group(1).strip() or "提示"
                rest = match.group(2).strip()
            current_hint = Hint(label=label, body=rest)
            items.append(Item("hint", hint=current_hint))
            continue

        if current_hint is not None and content.startswith(" "):
            current_hint.body = (current_hint.body + "\n" + stripped).strip()
            continue

        if DECOR_RE.match(content):
            flush_paras()
            current_hint = None
            items.append(Item("rule"))
            continue

        current_hint = None
        paras.append(content)

    flush_paras()
    return _clean_items(items)


def _split_statement_body(code: str) -> Tuple[str, str]:
    """Split a declaration at its first top-level ``:=`` (good enough for this
    course: proof/definition bodies are indented, and the operator is at the
    end of the statement line)."""
    index = code.find(":=")
    if index < 0:
        return code.rstrip(), ""
    statement = code[:index].rstrip() + " :="
    body = code[index + 2:].strip("\n")
    return statement, body


def parse_declarations(
    lines: Sequence[str], unit_title: str, filename: str
) -> Tuple[List[Decl], List[Item], List[str], List[str]]:
    """Split a canvas/solution file into declarations, trailing prose, imports,
    and top-level lines the parser could not place (reported, never silent)."""
    decls: List[Decl] = []
    pending: List[Item] = []
    imports: List[str] = []
    stray: List[str] = []
    index = 0
    total = len(lines)

    while index < total:
        line = lines[index]
        if not line.strip():
            index += 1
            continue

        if line.startswith("--"):
            group: List[str] = []
            while index < total and lines[index].startswith("--"):
                group.append(lines[index])
                index += 1
            pending.extend(parse_comment_group(group, unit_title, filename))
            continue

        if IMPORT_RE.match(line):
            imports.append(line.strip())
            index += 1
            continue

        match = DECL_RE.match(line)
        if not match:
            stray.append(f"{filename}:{index + 1}: {line.strip()[:120]}")
            index += 1
            continue

        start = index
        index += 1
        while index < total:
            candidate = lines[index]
            if (
                candidate.startswith("--")
                or IMPORT_RE.match(candidate)
                or DECL_RE.match(candidate)
            ):
                break
            index += 1

        code = "\n".join(lines[start:index]).rstrip()
        statement, body = _split_statement_body(code)
        pending = _clean_items(pending)
        hints = [item.hint for item in pending if item.kind == "hint" and item.hint]
        decl = Decl(
            kind=match.group(1),
            name=match.group(2),
            code=code,
            statement=statement,
            body=body,
            is_exercise=("sorry" in code),
            has_sorry=("sorry" in code),
            index=len(decls) + 1,
            lead=list(pending),
            hints=[hint for hint in hints if hint is not None],
        )
        decls.append(decl)
        pending = []

    return decls, _clean_items(pending), imports, stray


def load_source(path: Path) -> List[str]:
    return path.read_text(encoding="utf-8").splitlines()


def find_solution(unit_file: Path, chapter_id: str) -> Optional[Path]:
    """Solutions follow the course's basename convention, with a short
    ``unitNN-solution`` form for most units and a longer form for a few."""
    match = re.match(r"unit(\d+)", unit_file.stem)
    if not match:
        return None
    number = match.group(1)
    solution_dir = unit_file.parent.parent / "solutions" / chapter_id
    candidates = [
        solution_dir / f"{unit_file.stem}-solution.sokonanoda",
        solution_dir / f"unit{number}-solution.sokonanoda",
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    globbed = sorted(solution_dir.glob(f"unit{number}-*-solution.sokonanoda"))
    if len(globbed) == 1:
        return globbed[0]
    return None


def _unit_title_en(unit: dict) -> str:
    return str(unit.get("title_en") or "").strip()


def parse_unit(
    entry: dict,
    chapter: dict,
    volume: dict,
    chapter_position: int,
    course_root: Path,
) -> Unit:
    unit_file = course_root / str(entry["file"])
    lines = load_source(unit_file)
    title = str(entry.get("title") or "").strip()
    title_en = _unit_title_en(entry)
    decls, trailing, imports, stray = parse_declarations(lines, title, unit_file.name)
    result = Unit(
        index=int(entry["unit"]),
        title=title,
        title_en=title_en,
        file=str(entry["file"]),
        chapter_id=str(chapter["id"]),
        chapter_title=str(chapter.get("title") or ""),
        chapter_position=chapter_position,
        volume_id=str(volume.get("id") or ""),
        volume_title=str(volume.get("title") or ""),
        decls=decls,
        trailing=trailing,
        imports=imports,
        stray=stray,
    )

    exercise_no = 0
    for decl in result.decls:
        if decl.is_exercise:
            exercise_no += 1
            decl.exercise_no = exercise_no

    solution_path = find_solution(unit_file, str(chapter["id"]))
    result.solution_path = solution_path
    if solution_path is not None:
        solution_lines = load_source(solution_path)
        solution_decls, _, _, solution_stray = parse_declarations(
            solution_lines, title, solution_path.name
        )
        result.stray.extend(
            f"解答 {item}" for item in solution_stray
        )
        result.solution_decls = {decl.name: decl for decl in solution_decls}
        for decl in result.decls:
            if not decl.is_exercise:
                continue
            solution = result.solution_decls.get(decl.name)
            if solution is None:
                continue
            decl.solution = solution
            decl.solution_note = _solution_note(decl, solution)
        canvas_names = {decl.name for decl in result.decls}
        result.extra_solutions = [
            decl for name, decl in result.solution_decls.items() if name not in canvas_names
        ]
    return result


def _normalise_text(text: str) -> str:
    return re.sub(r"\s+", "", text)


def _solution_note(exercise: Decl, solution: Decl) -> str:
    """Return solution-only prose that is not already in the canvas prompt."""
    known = {
        _normalise_text(item.text)
        for item in exercise.lead
        if item.kind in ("text", "list")
    }
    notes: List[str] = []
    for item in solution.lead:
        if item.kind == "hint":
            continue
        if item.kind in ("heading", "rule"):
            continue
        if item.kind == "list":
            text = "；".join(
                line.lstrip("·•-–— ").strip() for line in item.lines if line.strip()
            )
        else:
            text = item.text.strip()
        if not text:
            continue
        if _normalise_text(text) in known:
            continue
        notes.append(text)
    return "\n\n".join(notes)


_MARKDOWN_EMPHASIS_RE = re.compile(r"\*\*(.+?)\*\*|__(.+?)__", re.S)


def plain_text(text: str) -> str:
    """Markdown markers out of text that is displayed as plain text.

    Search results and contexts are rendered with ``textContent``, so they must
    carry no ``**`` / ``__`` / backtick markers.  The replacement is repeated
    so nested emphasis collapses too; nothing else is removed.
    """
    previous = None
    while previous != text:
        previous = text
        text = _MARKDOWN_EMPHASIS_RE.sub(
            lambda match: match.group(1) if match.group(1) is not None else match.group(2),
            text,
        )
    return re.sub(r"\s+", " ", text.replace("`", "")).strip()


def _comparison_key(text: str) -> str:
    return re.sub(
        r"[\s`*_，。：；、！？（）()\[\]{}「」『』《》·—\-–]+", "", plain_text(text)
    )


def _is_title_echo(text: str, unit: Unit) -> bool:
    """True when a paragraph merely repeats the unit heading.

    The heading is already rendered as the unit ``<h2>``; repeating it as the
    first paragraph adds noise without adding information.  This intentionally
    only matches paragraphs that start with ``单元N`` and whose remainder is the
    heading (plus an optional parenthetical gloss).
    """
    first_line = plain_text(text.splitlines()[0]) if text.strip() else ""
    if not TITLE_ECHO_RE.match(first_line):
        return False
    rest = _comparison_key(TITLE_ECHO_RE.sub("", first_line))
    title = _comparison_key(unit.display_title)
    if not rest or not title:
        return False
    if rest == title:
        return True
    if rest.startswith(title) and len(rest) <= len(title) + 40:
        return True
    return False


def _is_course_note(text: str) -> bool:
    stripped = plain_text(text)
    return bool(stripped) and bool(COURSE_NOTE_RE.match(stripped))


def _is_heading_restatement(text: str, unit: Unit) -> bool:
    """Exact ``单元N …`` heading echoes are moved out of the reading flow.

    Kept as a separate hook from course-note detection: the two removals have
    different reasons (duplicate heading vs. engineering metadata) even though
    both end up in the collapsed notes panel.
    """
    return _is_title_echo(text, unit)


def _detach_course_notes(items: Sequence[Item]) -> Tuple[List[Item], List[Item]]:
    """Split a lead/trailing sequence into reading prose and engineering notes.

    Notes are moved, never deleted: the renderer puts them into one collapsed
    ``编写注记`` panel per unit.  A note ending in ``：`` also takes the list or
    formula block that immediately follows it, so the moved note keeps its
    attachment.
    """
    kept: List[Item] = []
    notes: List[Item] = []
    index = 0
    while index < len(items):
        item = items[index]
        if item.kind == "text" and _is_course_note(item.text):
            notes.append(item)
            index += 1
            if item.text.rstrip().endswith(("：", ":")):
                while index < len(items) and items[index].kind in ("list", "formula"):
                    notes.append(items[index])
                    index += 1
            continue
        kept.append(item)
        index += 1
    return kept, notes


def _detach_heading_restatements(
    items: Sequence[Item], unit: Unit
) -> Tuple[List[Item], List[Item]]:
    kept: List[Item] = []
    moved: List[Item] = []
    for item in items:
        if item.kind == "text" and _is_heading_restatement(item.text, unit):
            moved.append(item)
        else:
            kept.append(item)
    return kept, moved


def prepare_unit_prose(unit: Unit) -> None:
    """Lift the first meaningful paragraph into ``unit.intro`` and move
    course-engineering notes into ``unit.notes``.

    The paragraph is moved, not copied or rewritten: the unit keeps exactly the
    author's words.  Notes stay retrievable in a collapsed panel.  This is a
    presentation pass over the existing course files; no course file is edited.
    """
    notes: List[Item] = []
    for decl in unit.decls:
        decl.lead, detached = _detach_heading_restatements(decl.lead, unit)
        notes.extend(detached)
        decl.lead, detached = _detach_course_notes(decl.lead)
        notes.extend(detached)
    unit.trailing, detached = _detach_heading_restatements(unit.trailing, unit)
    notes.extend(detached)
    unit.trailing, detached = _detach_course_notes(unit.trailing)
    notes.extend(detached)
    unit.notes = notes

    for decl in unit.decls:
        for index, item in enumerate(decl.lead):
            if item.kind != "text":
                continue
            text = item.text.strip()
            if not text or _is_title_echo(text, unit) or _is_course_note(text):
                continue
            take = [item]
            end = index + 1
            # A lead often opens with "…定义成" / "…如下：" and is completed by
            # the formula or the next prose paragraph.  Take at most three
            # items (and ~900 chars) so the block stays a lead, not a section.
            while end < len(decl.lead) and len(take) < 3:
                current = " ".join(
                    part.text.strip() if part.kind == "text" else ""
                    for part in take
                ).strip()
                if current and current.rstrip().endswith(("。", "！", "？")):
                    break
                nxt = decl.lead[end]
                if nxt.kind in ("hint", "heading", "rule"):
                    break
                candidate_len = len(current) + len(
                    nxt.text if nxt.kind == "text" else " ".join(nxt.lines)
                )
                if candidate_len > 900:
                    break
                take.append(nxt)
                end += 1
                if nxt.kind == "text" and nxt.text.strip().endswith(("。", "！", "？")):
                    break
            del decl.lead[index:end]
            unit.intro = take
            return


def source_fingerprint(paths: Iterable[Path]) -> str:
    digest = hashlib.sha256()
    for path in sorted(paths):
        try:
            label = path.resolve().relative_to(ROOT).as_posix()
        except ValueError:
            label = path.name
        digest.update(label.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
    return digest.hexdigest()[:12]


def chapter_slug(chapter_id: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", chapter_id.lower()).strip("-")


def _filesystem_unit_files() -> set[Path]:
    return {path.resolve() for path in COURSE_ROOT.glob("units/*/*.sokonanoda")}


def load_book() -> Book:
    course = json.loads(COURSE_JSON.read_text(encoding="utf-8"))
    volumes = course.get("volumes")
    if not isinstance(volumes, list) or not volumes:
        raise SystemExit("course.json: expected a non-empty volumes array")
    chapters: List[Chapter] = []
    manifest_paths: set[Path] = set()
    source_paths: List[Path] = [COURSE_JSON]
    manifest_unit_count = 0

    for volume in volumes:
        volume_title = str(volume.get("title") or volume.get("id") or "")
        for position, chapter in enumerate(volume.get("chapters", []), start=1):
            chapter_id = str(chapter.get("id") or "")
            if not chapter_id:
                raise SystemExit("course.json: chapter without id")
            chapter_obj = Chapter(
                id=chapter_id,
                title=str(chapter.get("title") or ""),
                volume_id=str(volume.get("id") or ""),
                volume_title=volume_title,
                position=position,
            )
            entries = chapter.get("units")
            if not isinstance(entries, list):
                raise SystemExit(f"course.json: chapter {chapter_id!r} has no units array")
            for unit_position, entry in enumerate(entries, start=1):
                file_name = str(entry.get("file") or "")
                if not file_name:
                    raise SystemExit(f"course.json: chapter {chapter_id!r} has a unit without file")
                path = COURSE_ROOT / file_name
                if not path.is_file():
                    raise SystemExit(f"course.json: unit file missing: {file_name}")
                manifest_paths.add(path.resolve())
                unit = parse_unit(entry, chapter, volume, unit_position, COURSE_ROOT)
                prepare_unit_prose(unit)
                chapter_obj.units.append(unit)
                source_paths.append(path)
                if unit.solution_path is not None:
                    source_paths.append(unit.solution_path)
                manifest_unit_count += 1
            chapters.append(chapter_obj)

    if not chapters:
        raise SystemExit("course.json: no chapters found")
    for chapter in chapters:
        if not chapter.units:
            raise SystemExit(f"course.json: chapter {chapter.id!r} has no units")

    filesystem_paths = _filesystem_unit_files()
    unlisted = sorted(
        path.relative_to(ROOT).as_posix() for path in filesystem_paths - manifest_paths
    )
    missing = sorted(
        path.relative_to(ROOT).as_posix() for path in manifest_paths - filesystem_paths
    )
    if missing:
        raise SystemExit("course.json lists unit files that are absent: " + ", ".join(missing))

    source_paths.extend([STYLE_CSS, APP_JS])
    return Book(
        title=str(course.get("title") or "analytical reader"),
        volume_titles=list(dict.fromkeys(chapter.volume_title for chapter in chapters)),
        chapters=chapters,
        source_fingerprint=source_fingerprint(source_paths),
        import_count=sum(len(unit.imports) for chapter in chapters for unit in chapter.units),
        unlisted_unit_files=unlisted,
        manifest_unit_count=manifest_unit_count,
    )


# ── math rendering (Unicode + a tiny MathML helper) ─────────────────────────


def _mml_escape(text: str) -> str:
    return html.escape(text, quote=False)


def _strip_outer_parens(text: str) -> str:
    text = text.strip()
    if not (text.startswith("(") and text.endswith(")")):
        return text
    depth = 0
    for index, ch in enumerate(text):
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0 and index != len(text) - 1:
                return text
    return text[1:-1].strip()


def _top_level_slash(text: str) -> int:
    depth = 0
    for index, ch in enumerate(text):
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth = max(0, depth - 1)
        elif ch == "/" and depth == 0:
            return index
    return -1


def _mml_sup(text: str) -> str:
    plain = text.translate(SUP_TRANS)
    parts: List[str] = []
    for ch in plain:
        if ch.isdigit():
            parts.append(f"<mn>{ch}</mn>")
        else:
            parts.append(f"<mo>{_mml_escape(ch)}</mo>")
    return "".join(parts)


def _mml_token(token: str) -> str:
    if len(token) >= 2 and token.startswith("(") and token.endswith(")"):
        return "<mrow><mo>(</mo>" + _mml_expr(token[1:-1]) + "<mo>)</mo></mrow>"
    if len(token) >= 2 and token.startswith("[") and token.endswith("]"):
        return "<mrow><mo>[</mo>" + _mml_expr(token[1:-1]) + "<mo>]</mo></mrow>"
    if SUP_RE.match(token):
        return f"<mo>{_mml_escape(token)}</mo>"
    if re.fullmatch(r"\d+(?:\.\d+)?", token):
        return f"<mn>{token}</mn>"
    if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_'.]*", token):
        variant = ' mathvariant="normal"' if len(token) > 1 else ""
        return f"<mi{variant}>{_mml_escape(token)}</mi>"
    if token == "'":
        return "<mo>′</mo>"
    if token == "*":
        return "<mo>×</mo>"
    if token == "-":
        return "<mo>−</mo>"
    if token in "≤≥≠∈∉⊆⊂→←↔⇒⇔⟺∀∃∧∨¬":
        return f"<mo>{_mml_escape(token)}</mo>"
    if any(_is_cjk(ch) for ch in token):
        return f"<mtext>{_mml_escape(token)}</mtext>"
    return f"<mo>{_mml_escape(token)}</mo>"


def _mml_expr(text: str) -> str:
    text = text.strip()
    if not text:
        return "<mrow></mrow>"

    slash = _top_level_slash(text)
    if slash > 0:
        left = text[:slash].strip()
        right = text[slash + 1:].strip()
        if left and right:
            return (
                "<mfrac>"
                + _mml_expr(_strip_outer_parens(left))
                + _mml_expr(_strip_outer_parens(right))
                + "</mfrac>"
            )

    parts: List[str] = []
    pending_sup = False
    for match in MML_TOKEN_RE.finditer(text):
        token = match.group(0)
        if token.isspace():
            if parts and parts[-1] != '<mspace width="0.22em"/>':
                parts.append('<mspace width="0.22em"/>')
            continue
        if pending_sup:
            pending_sup = False
            base = parts.pop() if parts else ""
            if base:
                parts.append(f"<msup>{base}<mrow>{_mml_expr(token)}</mrow></msup>")
                continue
        if token == "^":
            pending_sup = True
            continue
        if SUP_RE.match(token) and parts:
            base = parts.pop()
            parts.append(f"<msup>{base}<mrow>{_mml_sup(token)}</mrow></msup>")
            continue
        parts.append(_mml_token(token))

    while parts and parts[-1].startswith("<mspace"):
        parts.pop()
    return "<mrow>" + "".join(parts) + "</mrow>"


def mathml(text: str) -> Optional[str]:
    """Return a small block MathML node, or None when MathML should not own the
    formula (CJK text, code-shaped formulas, or oversized display lines)."""
    source = text.strip().replace("`", "").strip("。；，、 ")
    if not source or len(source) > 320:
        return None
    if any(_is_cjk(ch) for ch in source):
        return None
    if _is_codeish_formula(source):
        return None
    body = _mml_expr(source)
    label = html.escape(source)
    return (
        f'<div class="math-display" role="math" aria-label="{label}">'
        f'<math display="block" class="mathml">{body}</math>'
        "</div>"
    )


def render_formula(text: str) -> str:
    source = text.strip()
    if _is_codeish_formula(source):
        return (
            '<div class="math-display intuition-only">'
            f'<pre class="display-code"><code>{escape(source)}</code></pre>'
            "</div>"
        )
    rendered = mathml(source)
    if rendered is not None:
        return rendered.replace('class="math-display"', 'class="math-display intuition-only"', 1)
    label = html.escape(source)
    return (
        f'<div class="math-display intuition-only" role="math" aria-label="{label}">'
        f'<span class="math">{escape(source)}</span></div>'
    )


def _wrap_math(text: str) -> str:
    out: List[str] = []
    position = 0
    for match in MATH_SPAN_RE.finditer(text):
        span = match.group(0)
        stripped = span.strip()
        if not stripped or not any(ch in MATH_SYMBOLS for ch in stripped):
            continue
        leading = span[: len(span) - len(span.lstrip())]
        trailing = span[len(span.rstrip()):]
        out.append(escape(text[position:match.start()]))
        if leading:
            out.append(escape(leading))
        out.append(f'<span class="math">{escape(stripped)}</span>')
        if trailing:
            out.append(escape(trailing))
        position = match.end()
    out.append(escape(text[position:]))
    return "".join(out)


def _find_emphasis_close(text: str, start: int, marker: str) -> int:
    """Find a closing emphasis marker that is not inside an inline-code span."""
    position = start
    while position < len(text) - 1:
        if text[position] == "`":
            closing = text.find("`", position + 1)
            if closing < 0:
                return -1
            position = closing + 1
            continue
        if text.startswith(marker, position):
            return position
        position += 1
    return -1


def _render_inline(text: str, depth: int = 0) -> str:
    """Tiny inline parser for `code`, **bold** / __bold__, and serif math spans."""
    out: List[str] = []
    buf: List[str] = []

    def flush() -> None:
        if buf:
            out.append(_wrap_math("".join(buf)))
            buf.clear()

    position = 0
    while position < len(text):
        char = text[position]
        if char == "`":
            closing = text.find("`", position + 1)
            if closing >= 0:
                flush()
                out.append(f"<code>{escape(text[position + 1:closing])}</code>")
                position = closing + 1
                continue
        for marker in ("**", "__"):
            if not text.startswith(marker, position) or depth >= 4:
                continue
            closing = _find_emphasis_close(text, position + 2, marker)
            if closing >= 0 and closing > position + 2:
                flush()
                inner = _render_inline(text[position + 2:closing], depth + 1)
                out.append(f"<strong>{inner}</strong>")
                position = closing + 2
                break
        else:
            buf.append(char)
            position += 1
    flush()
    return "".join(out)


def render_inline(text: str) -> str:
    """Escape prose and add inline code, bold, and serif math spans."""
    return _render_inline(text)


# ── HTML rendering ──────────────────────────────────────────────────────────


CJK_PUNCT = set("，。；：！？、（）「」『』《》“”‘’…—·\"'")


def _join_prose_lines(lines: Sequence[str]) -> str:
    """Undo the source file's hard wraps while keeping CJK words joined."""
    result = ""
    for raw in lines:
        line = raw.strip()
        if not line:
            continue
        if not result:
            result = line
        elif (
            (_is_cjk(result[-1]) and _is_cjk(line[0]))
            or result[-1] in CJK_PUNCT
            or line[0] in CJK_PUNCT
        ):
            result += line
        else:
            result += " " + line
    return result


def render_hint(hint: Hint, index: int) -> str:
    body_lines = [line for line in hint.body.splitlines() if line.strip()]
    paragraphs: List[str] = []
    text_buf: List[str] = []

    def flush_text() -> None:
        if text_buf:
            paragraphs.append(f"<p>{render_inline(_join_prose_lines(text_buf))}</p>")
            text_buf.clear()

    for line in body_lines:
        if _looks_formula(line):
            flush_text()
            paragraphs.append(render_formula(line))
        else:
            text_buf.append(line)
    flush_text()
    if not paragraphs:
        paragraphs.append("<p>（无提示正文）</p>")
    return (
        '<details class="hint intuition-only">'
        f'<summary><span class="hint-index">提示 {index}</span>'
        f'<span class="hint-label">{render_inline(hint.label)}</span></summary>'
        f'<div class="hint-body">{"".join(paragraphs)}</div>'
        "</details>"
    )


def render_hints(hints: Sequence[Hint]) -> str:
    if not hints:
        return ""
    rendered = [render_hint(hint, index + 1) for index, hint in enumerate(hints)]
    return '<div class="hints">' + "".join(rendered) + "</div>"


def render_item(item: Item) -> str:
    if item.kind == "text":
        body = _join_prose_lines(item.text.splitlines())
        if not body:
            return ""
        return f'<div class="block prose intuition-only"><p>{render_inline(body)}</p></div>'
    if item.kind == "list":
        rendered_items: List[str] = []
        for line in item.lines:
            parts = line.splitlines()
            first = parts[0].lstrip("·•-–— ")
            content = _join_prose_lines([first, *[part for part in parts[1:]]])
            rendered_items.append(f"<li>{render_inline(content)}</li>")
        items = "".join(rendered_items)
        # Clarity-first: exhaustive lists (declarations, lemma inventories,
        # proof-route enumerations) stay available but do not dominate the
        # learner's first read.  Short lists (≤4) remain inline.
        if len(item.lines) >= 5:
            return (
                '<details class="decl-index intuition-only">'
                f'<summary>细节清单（{len(item.lines)} 项，展开）</summary>'
                f'<ul class="prose-list">{items}</ul></details>'
            )
        return (
            '<div class="block prose intuition-only">'
            f'<ul class="prose-list">{items}</ul></div>'
        )
    if item.kind == "formula":
        return render_formula(item.text)
    if item.kind == "heading":
        return f'<h3 class="unit-subhead">{render_inline(item.text)}</h3>'
    if item.kind == "rule":
        return '<hr class="section-rule">'
    return ""


def render_items(items: Sequence[Item]) -> str:
    out: List[str] = []
    for item in items:
        if item.kind == "hint":
            continue
        out.append(render_item(item))
    return "".join(part for part in out if part)


def _outside_prompt(lead: Sequence[Item]) -> List[Item]:
    return [item for item in lead if item.kind in ("heading", "rule")]


def _prompt_items(lead: Sequence[Item]) -> List[Item]:
    return [item for item in lead if item.kind in ("text", "list", "formula")]


def split_proof_steps(body: str) -> List[str]:
    lines = body.splitlines()
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    if not lines:
        return []
    if lines[0].strip() == "by" and len(lines) > 1:
        lines = lines[1:]
    nonblank = [line for line in lines if line.strip()]
    if not nonblank:
        return []
    min_indent = min(len(line) - len(line.lstrip()) for line in nonblank)
    steps: List[str] = []
    current: List[str] = []
    for line in lines:
        indent = len(line) - len(line.lstrip())
        if line.strip() and indent == min_indent and current:
            steps.append("\n".join(current).rstrip())
            current = [line]
        else:
            current.append(line)
    if current:
        steps.append("\n".join(current).rstrip())
    return [step for step in steps if step.strip()]


def render_proof(decl: Decl) -> str:
    steps = split_proof_steps(decl.body)
    if not steps:
        return ""
    line_count = len([line for line in decl.body.splitlines() if line.strip()])
    step_html = "".join(
        f"<li><pre><code>{escape(step)}</code></pre></li>" for step in steps
    )
    return (
        '<details class="proof-panel formal-only">'
        f"<summary>证明 · {len(steps)} 步 · {line_count} 行</summary>"
        f'<ol class="proof-steps">{step_html}</ol>'
        "</details>"
    )


def render_code_panel(decl: Decl) -> str:
    if decl.code == decl.statement:
        return ""
    line_count = len(decl.code.splitlines())
    return (
        '<details class="code-panel formal-only">'
        f"<summary>完整代码 · {line_count} 行</summary>"
        f'<pre><code>{escape(decl.code)}</code></pre>'
        "</details>"
    )


def render_decl(unit: Unit, decl: Decl) -> str:
    outside = _outside_prompt(decl.lead)
    prompt = _prompt_items(decl.lead)
    prefix = render_items(outside if decl.is_exercise else decl.lead)
    kind_label = KIND_LABELS.get(decl.kind, decl.kind)
    decl_id = f"decl-{unit.index}-{decl.index}"

    if decl.is_exercise:
        prompt_html = render_items(prompt)
        return (
            prefix
            + f'<article class="exercise" id="ex-{unit.index}-{decl.exercise_no}" '
            + f'data-exercise="{decl.exercise_no}">'
            + '<header class="exercise-head">'
            + f'<span class="exercise-badge">练习 {decl.exercise_no}</span>'
            + f'<code class="exercise-name">{escape(decl.name)}</code>'
            + f'<span class="exercise-kind">{escape(kind_label)}</span>'
            + f'<a class="exercise-appendix" href="#app-sol-{unit.index}-{decl.exercise_no}">附录解答</a>'
            + "</header>"
            + '<div class="exercise-body">'
            + (f'<div class="exercise-prompt">{prompt_html}</div>' if prompt_html else "")
            + f'<pre class="exercise-statement"><code>{escape(decl.code)}</code></pre>'
            + render_hints(decl.hints)
            + render_solution(unit, decl)
            + "</div></article>"
        )

    hints_html = render_hints(decl.hints)
    if decl.kind in ("axiom", "inductive") or not decl.body:
        body_html = (
            f'<pre class="decl-statement"><code>{escape(decl.code)}</code></pre>'
            + hints_html
        )
    elif decl.kind == "theorem":
        body_html = (
            f'<pre class="decl-statement"><code>{escape(decl.statement)}</code></pre>'
            + hints_html
            + render_proof(decl)
        )
    else:
        body_html = (
            f'<pre class="decl-statement"><code>{escape(decl.statement)}</code></pre>'
            + hints_html
            + render_code_panel(decl)
        )

    return (
        prefix
        + f'<article class="decl decl-{decl.kind}" id="{decl_id}">'
        + '<header class="decl-head">'
        + f'<span class="decl-kind kind-{decl.kind}">{escape(kind_label)}</span>'
        + f'<code class="decl-name">{escape(decl.name)}</code>'
        + f'<span class="decl-where">声明 {decl.index}</span>'
        + "</header>"
        + f'<div class="decl-body">{body_html}</div>'
        + "</article>"
    )


def render_solution(unit: Unit, decl: Decl) -> str:
    solution = decl.solution
    if solution is None:
        return (
            f'<details class="solution-panel" id="sol-{unit.index}-{decl.exercise_no}">'
            "<summary>查看解答</summary>"
            '<p class="missing-solution">按声明名没有找到对应的解答文件。</p>'
            "</details>"
        )
    line_count = len(solution.code.splitlines())
    note = ""
    if decl.solution_note:
        note = f'<p class="solution-note">{render_inline(decl.solution_note)}</p>'
    return (
        f'<details class="solution-panel" id="sol-{unit.index}-{decl.exercise_no}">'
        f'<summary>查看解答 <span class="sol-meta">{escape(KIND_LABELS.get(solution.kind, solution.kind))} · {line_count} 行</span></summary>'
        f'<div class="solution-body">{note}'
        f'<pre><code>{escape(solution.code)}</code></pre>'
        "</div></details>"
    )


def unit_meta_bits(unit: Unit) -> List[str]:
    bits: List[str] = []
    for count, label in (
        (unit.defs, "定义"),
        (unit.theorems, "定理"),
        (unit.axioms, "公理"),
        (unit.inductives, "归纳类型"),
    ):
        if count:
            bits.append(f"{count} 个{label}")
    bits.append(f"{unit.exercise_count} 道练习")
    if unit.hints:
        bits.append(f"{len(unit.hints)} 条提示")
    return bits


def render_unit_nav(book: Book, unit: Unit, position: int) -> str:
    previous = book.units[position - 1] if position > 0 else None
    following = book.units[position + 1] if position + 1 < len(book.units) else None

    def link(target: Optional[Unit], direction: str, label: str) -> str:
        if target is None:
            return (
                '<span class="unit-nav-link is-disabled">'
                f'<span class="unit-nav-label">{label}</span>'
                '<span class="unit-nav-title">没有更多单元</span>'
                "</span>"
            )
        arrow = "←" if direction == "prev" else "→"
        return (
            f'<a class="unit-nav-link" href="#unit-{target.index}" rel="{direction}">'
            f'<span class="unit-nav-label">{arrow} {label}</span>'
            f'<span class="unit-nav-title">{render_inline(target.display_title)}</span>'
            "</a>"
        )

    return (
        '<nav class="unit-nav no-print" aria-label="单元导航">'
        + link(previous, "prev", "上一单元")
        + '<a class="unit-nav-home" href="#toc">目录</a>'
        + link(following, "next", "下一单元")
        + "</nav>"
    )


def render_chapter(book: Book, chapter: Chapter) -> str:
    units = "\n".join(
        render_unit(book, unit, chapter, position)
        for position, unit in enumerate(chapter.units)
    )
    first = chapter.units[0].index
    last = chapter.units[-1].index
    meta = [
        f"{len(chapter.units)} 单元",
        f"{chapter.exercises} 道练习",
        f"{chapter.declarations} 个声明",
        f"{chapter.hints} 条提示",
    ]
    return (
        f'<section class="chapter" id="{chapter.anchor}">'
        '<header class="chapter-head">'
        f'<p class="chapter-kicker">{render_inline(chapter.volume_title)} · 第 {chapter.position} 章</p>'
        f'<h2><span class="chapter-id">{escape(chapter.id)}</span>'
        f"<span>{render_inline(chapter.title)}</span></h2>"
        f'<p class="chapter-meta">单元 {first}–{last} · {" · ".join(meta)}</p>'
        "</header>"
        f"{units}"
        "</section>"
    )


def render_unit(book: Book, unit: Unit, chapter: Chapter, position: int) -> str:
    hints = unit.hints
    meta = " · ".join(unit_meta_bits(unit))
    classes = "unit first-of-chapter" if position == 0 else "unit"
    parts = [
        f'<section class="{classes}" id="unit-{unit.index}">',
        '<header class="unit-head">',
        f'<p class="unit-kicker"><a href="#{chapter.anchor}">{escape(unit.chapter_id)}</a>'
        f' · 本章第 {position + 1} / {len(chapter.units)} 单元 · '
        f'全书第 {unit.index} / {len(book.units)} 单元</p>',
        f'<h2><span class="unit-number">{unit.index:02d}</span>'
        f"<span>{render_inline(unit.display_title)}</span></h2>",
    ]
    if unit.title_en:
        parts.append(f'<p class="unit-title-en">{escape(unit.title_en)}</p>')
    parts.extend(
        [
            f'<p class="unit-meta">{meta} · '
            f'<a href="#appendix-unit-{unit.index}">解答附录</a></p>',
            '<div class="unit-actions no-print">'
            '<button type="button" class="unit-progress-toggle" '
            f'data-progress-toggle data-unit="{unit.index}" aria-pressed="false">'
            '<span class="unit-progress-mark" aria-hidden="true"></span>'
            '<span class="unit-progress-text">标记已读</span></button>'
            '<button type="button" class="unit-copy-link" '
            f'data-copy-link="unit-{unit.index}" '
            f'aria-label="复制单元 {unit.index} 的链接">复制链接</button>'
            '<span class="unit-copy-status" role="status" aria-live="polite" '
            "data-copy-status></span>"
            "</div>",
            '<a class="back-to-toc" href="#toc">↑ 目录</a>',
            "</header>",
        ]
    )
    if unit.intro:
        parts.append(
            '<section class="unit-intro" aria-label="单元导语">'
            f'<div class="unit-intro-body">{render_items(unit.intro)}</div>'
            "</section>"
        )
    for decl in unit.decls:
        parts.append(render_decl(unit, decl))
    if unit.trailing:
        parts.append(render_items(unit.trailing))
    if unit.notes:
        parts.append(
            '<details class="unit-notes">'
            f"<summary>编写注记 · 教材对照与课程边界（{len(unit.notes)} 条）</summary>"
            f'<div class="unit-notes-body">{render_items(unit.notes)}</div>'
            "</details>"
        )
    parts.append(render_unit_nav(book, unit, book.units.index(unit)))
    parts.append("</section>")
    return "\n".join(parts)


def render_toc(book: Book) -> str:
    volume_blocks: List[str] = []
    for volume_title in book.volume_titles:
        chapters = [chapter for chapter in book.chapters if chapter.volume_title == volume_title]
        if not chapters:
            continue
        chapter_blocks: List[str] = []
        for chapter in chapters:
            unit_items: List[str] = []
            for unit in chapter.units:
                unit_items.append(
                    "<li>"
                    f'<a href="#unit-{unit.index}">'
                    f'<span class="toc-num">{unit.index:02d}</span>'
                    f'<span class="toc-title">{render_inline(unit.display_title)}</span>'
                    "</a>"
                    f'<span class="toc-meta">{unit.exercise_count} 道练习 · '
                    f"{len(unit.hints)} 条提示</span>"
                    "</li>"
                )
            chapter_blocks.append(
                '<section class="toc-chapter">'
                '<div class="toc-chapter-head">'
                f'<h4><a href="#{chapter.anchor}">'
                f'<span class="toc-chapter-id">{escape(chapter.id)}</span>'
                f"<span>{render_inline(chapter.title)}</span></a></h4>"
                f'<p class="toc-chapter-meta">{len(chapter.units)} 单元 · '
                f"{chapter.exercises} 道练习 · {chapter.declarations} 个声明</p>"
                "</div>"
                f'<ol class="toc-units">{"".join(unit_items)}</ol>'
                "</section>"
            )
        volume_blocks.append(
            '<div class="toc-volume">'
            f'<h3 class="toc-volume-title">{render_inline(volume_title)}</h3>'
            f'{"".join(chapter_blocks)}'
            "</div>"
        )
    return (
        '<nav class="toc" id="toc" aria-label="全书目录">'
        "<h2>目录</h2>"
        f'<p class="toc-lede">{len(book.chapters)} 章 · {len(book.units)} 个单元 · '
        f"{book.exercises} 道练习 · {book.hints} 条提示 · "
        "单元内可折叠证明、提示与解答。</p>"
        + render_progress_settings(book)
        + "".join(volume_blocks)
        + '<p class="toc-footer"><a href="#appendix-solutions">→ 解答附录</a>'
        ' · <a href="#top">回到封面</a></p>'
        "</nav>"
    )


def render_appendix(book: Book) -> str:
    sections: List[str] = []
    for unit in book.units:
        exercise_rows: List[str] = []
        for decl in unit.exercises:
            solution = decl.solution
            summary_kind = (
                escape(KIND_LABELS.get(solution.kind, solution.kind))
                if solution is not None
                else "缺失"
            )
            if solution is None:
                body = '<p class="missing-solution">未找到对应解答。</p>'
            else:
                note = ""
                if decl.solution_note:
                    note = f'<p class="solution-note">{render_inline(decl.solution_note)}</p>'
                body = f"{note}<pre><code>{escape(solution.code)}</code></pre>"
            exercise_rows.append(
                f'<details class="appendix-solution" id="app-sol-{unit.index}-{decl.exercise_no}">'
                f"<summary>练习 {decl.exercise_no} · {escape(decl.name)} "
                f'<span class="sol-meta">{summary_kind}</span></summary>'
                f"{body}</details>"
            )

        extras: List[str] = []
        for position, decl in enumerate(unit.extra_solutions, start=1):
            extras.append(
                f'<details class="appendix-solution" id="app-extra-{unit.index}-{position}">'
                f"<summary>补充 · {escape(decl.name)} "
                f'<span class="sol-meta">{escape(KIND_LABELS.get(decl.kind, decl.kind))}</span></summary>'
                f"<pre><code>{escape(decl.code)}</code></pre></details>"
            )

        meta = (
            f"{len(unit.exercises)} 道练习 · "
            f"{sum(1 for decl in unit.exercises if decl.solution)} 份按名匹配的解答"
        )
        if unit.extra_solutions:
            meta += f" · {len(unit.extra_solutions)} 个解答独有声明"
        sections.append(
            f'<section class="appendix-unit" id="appendix-unit-{unit.index}">'
            f'<h3><a href="#unit-{unit.index}">{render_inline(unit.display_title)}</a></h3>'
            f'<p class="appendix-meta">{meta}</p>'
            + "".join(exercise_rows)
            + ("".join(extras) if extras else "")
            + "</section>"
        )
    return (
        '<section class="appendix" id="appendix-solutions">'
        "<h2>解答附录</h2>"
        f'<p class="appendix-lede">按单元汇总 {book.solutions} 份练习解答'
        f"（另有 {book.extra_solutions} 个只存在于解答文件的声明）。"
        "正文练习卡片里的「查看解答」与这里指向同一份代码，便于顺序复习与打印。</p>"
        + "".join(sections)
        + "</section>"
    )


def render_cover(book: Book) -> str:
    volume_line = " / ".join(book.volume_titles)
    return (
        '<header class="cover" id="top">'
        '<p class="cover-kicker">sokonanoda · analysis · interactive book · v2</p>'
        f"<h1>{render_inline(book.title)}</h1>"
        f'<p class="cover-en">{escape(volume_line)} · 第 {escape(book.chapter_span)} 章 · '
        f"{len(book.chapters)} 章 {len(book.units)} 单元 · M1+M2 全书</p>"
        '<p class="cover-lede">把「直觉」与「形式」放在同一页：先读中文讲解、动机与提示，'
        "再展开声明、证明步骤、完整代码和解答。所有内容来自课程画布与解答文件；"
        "这不是 II.1 的样章，而是当前课程清单里全部章节的连续读本。</p>"
        '<ul class="cover-stats">'
        f"<li><strong>{len(book.chapters)}</strong><span>章</span></li>"
        f"<li><strong>{len(book.units)}</strong><span>单元</span></li>"
        f"<li><strong>{book.declarations}</strong><span>声明</span></li>"
        f"<li><strong>{book.exercises}</strong><span>练习</span></li>"
        f"<li><strong>{book.hints}</strong><span>提示</span></li>"
        f"<li><strong>{book.solutions}</strong><span>解答</span></li>"
        "</ul>"
        '<div class="reading-guide" aria-label="阅读方式">'
        '<div class="guide-card"><span class="guide-name">直觉</span>'
        "<p>中文讲解、动机与 <code>soko:hint</code> 提示；先建立图景。</p></div>"
        '<div class="guide-card"><span class="guide-name">形式</span>'
        "<p>声明签名、证明步骤与完整代码；核对每一步。</p></div>"
        '<div class="guide-card"><span class="guide-name">并读</span>'
        "<p>两层同页对照；工具栏切换，浏览器会记住你的选择。</p></div>"
        "</div>"
        f'<p class="cover-note">范围：{escape(volume_line)} 的 '
        f"{len(book.chapters)} 章 {len(book.units)} 个单元（{escape(book.chapter_span)}）。"
        "M3 及以后单元加入 <code>course.json</code> 后会在下一次构建自动出现。"
        "工具栏可搜索全书、记录阅读进度；每个单元标题下可「标记已读」与「复制链接」。"
        "数学 = Unicode 衬线体 + 原生 MathML（简单分式/上标结构化，复杂或代码式公式回退为"
        " Unicode / 等宽，不丢内容）；无 CDN、无 Web 字体。证明、提示、解答均可折叠；"
        "打印按 A4 分页，每个单元另起一页。</p>"
        "</header>"
    )


def search_entries(book: Book) -> List[Dict[str, object]]:
    """Build the static in-page search index.

    Every anchor is one that ``--check`` can resolve: chapter/unit section ids,
    declaration and exercise article ids, and unit anchors for trailing prose
    headings.  Prose headings are included so Chinese phrases that only appear
    in a section title (e.g. 极限) remain findable; prose paragraphs are not
    indexed, which keeps the payload small and the result targets stable.
    """
    entries: List[Dict[str, object]] = []
    for chapter in book.chapters:
        entries.append(
            {"t": f"{chapter.id} {plain_text(chapter.title)}", "a": chapter.anchor, "k": "chapter"}
        )
    for unit in book.units:
        entries.append(
            {
                "t": f"{unit.index:02d} {plain_text(unit.display_title)}",
                "a": f"unit-{unit.index}",
                "k": "unit",
                "u": unit.index,
            }
        )
        for decl in unit.decls:
            anchor = (
                f"ex-{unit.index}-{decl.exercise_no}"
                if decl.is_exercise
                else f"decl-{unit.index}-{decl.index}"
            )
            entries.append(
                {
                    "t": plain_text(decl.name),
                    "a": anchor,
                    "k": "exercise" if decl.is_exercise else decl.kind,
                    "u": unit.index,
                    "h": plain_text(unit.display_title),
                }
            )
            for item in decl.lead:
                if item.kind != "heading":
                    continue
                entries.append(
                    {
                        "t": plain_text(item.text),
                        "a": anchor,
                        "k": "section",
                        "u": unit.index,
                        "h": plain_text(unit.display_title),
                    }
                )
        for item in unit.trailing:
            if item.kind != "heading":
                continue
            entries.append(
                {
                    "t": plain_text(item.text),
                    "a": f"unit-{unit.index}",
                    "k": "section",
                    "u": unit.index,
                    "h": plain_text(unit.display_title),
                }
            )
        entries.append(
            {
                "t": f"解答：{plain_text(unit.display_title)}",
                "a": f"appendix-unit-{unit.index}",
                "k": "appendix",
                "u": unit.index,
            }
        )
        for position, decl in enumerate(unit.extra_solutions, start=1):
            entries.append(
                {
                    "t": plain_text(decl.name),
                    "a": f"app-extra-{unit.index}-{position}",
                    "k": "solution",
                    "u": unit.index,
                    "h": f"解答附录 · {plain_text(unit.display_title)}",
                }
            )
    return entries


def render_search_index(book: Book) -> str:
    payload = json.dumps(
        search_entries(book), ensure_ascii=False, separators=(",", ":")
    )
    # A JSON script block must not contain "</script" or "<!--"; escaping "<"
    # with a JSON unicode escape keeps the payload parsed by browsers as data.
    payload = payload.replace("<", "\\u003c")
    return (
        '<script type="application/json" id="book-search-index">'
        f"{payload}</script>"
    )


def render_search() -> str:
    """Topbar search box; the index itself is embedded as JSON below <main>."""
    return (
        '<div class="search" role="search" data-search>'
        '<label class="search-label" for="book-search">搜索</label>'
        '<input id="book-search" class="search-input" type="search" '
        'placeholder="搜索单元 / 定义 / 定理" autocomplete="off" spellcheck="false" '
        'role="combobox" aria-autocomplete="list" aria-expanded="false" '
        'aria-controls="book-search-results" aria-describedby="book-search-status">'
        '<div class="search-panel" id="book-search-panel" hidden>'
        '<p class="search-status" id="book-search-status" role="status" aria-live="polite"></p>'
        '<ul class="search-results" id="book-search-results" role="listbox" '
        'aria-label="搜索结果"></ul>'
        "</div>"
        "</div>"
    )


def render_progress_widget(book: Book) -> str:
    total = len(book.units)
    return (
        '<div class="reading-progress" data-progress-widget>'
        f'<span class="progress-count" data-progress-count>已读 0 / {total}</span>'
        f'<span class="progress-track" role="progressbar" aria-label="阅读进度" '
        f'aria-valuemin="0" aria-valuemax="{total}" aria-valuenow="0" '
        f'aria-valuetext="已读 0 / {total} 单元" data-progressbar>'
        '<span class="progress-fill" data-progress-fill></span></span>'
        "</div>"
    )


def render_progress_settings(book: Book) -> str:
    total = len(book.units)
    return (
        '<div class="toc-progress no-print" data-progress-settings>'
        '<div class="toc-progress-head">'
        '<span class="toc-progress-label">阅读进度</span>'
        f'<strong data-progress-count>已读 0 / {total}</strong>'
        "</div>"
        f'<span class="progress-track" role="progressbar" aria-label="阅读进度" '
        f'aria-valuemin="0" aria-valuemax="{total}" aria-valuenow="0" '
        f'aria-valuetext="已读 0 / {total} 单元" data-progressbar>'
        '<span class="progress-fill" data-progress-fill></span></span>'
        '<div class="toc-progress-actions">'
        '<button type="button" class="plain" data-action="reset-progress" '
        'aria-expanded="false">重置进度</button>'
        '<span class="reset-confirm" data-reset-confirm hidden>'
        "<span>确定清除全部已读标记？</span>"
        '<button type="button" class="plain danger" '
        'data-action="reset-progress-confirm">确认重置</button>'
        '<button type="button" class="plain" '
        'data-action="reset-progress-cancel">取消</button>'
        "</span>"
        '<span class="reset-status" role="status" aria-live="polite" '
        'data-reset-status></span>'
        "</div>"
        '<p class="toc-progress-note">进度只保存在本机浏览器'
        "（localStorage），不做账号同步，也不按停留时间自动标记；"
        "点单元标题下的「标记已读」才会记入。</p>"
        "</div>"
    )


def render_toolbar(book: Book) -> str:
    options = "".join(
        f'<option value="#{chapter.anchor}">{escape(chapter.id)} '
        f"{escape(chapter.title)}</option>"
        for chapter in book.chapters
    )
    return (
        '<nav class="toolbar no-print" aria-label="阅读控制">'
        '<span class="toolbar-label">阅读层</span>'
        '<div class="seg" role="group" aria-label="直觉与形式">'
        '<button type="button" data-view="both" aria-pressed="true">并读</button>'
        '<button type="button" data-view="intuition" aria-pressed="false">直觉</button>'
        '<button type="button" data-view="formal" aria-pressed="false">形式</button>'
        "</div>"
        + render_search()
        + render_progress_widget(book)
        + '<label class="chapter-jump"><span>章节</span>'
        f'<select data-action="chapter-jump" aria-label="跳转到章">{options}</select>'
        "</label>"
        '<button type="button" class="plain" data-action="expand-all">展开全部</button>'
        '<button type="button" class="plain" data-action="collapse-all">折叠全部</button>'
        '<span class="toolbar-spacer"></span>'
        '<a class="plain" href="#top">封面</a>'
        '<a class="plain" href="#toc">目录</a>'
        '<button type="button" class="plain" data-action="print">打印 / PDF</button>'
        "</nav>"
    )


def render_footer(book: Book) -> str:
    return (
        '<footer class="book-footer">'
        f"<p>本页由 <code>scripts/build_book_proto.py</code> 从 "
        f"<code>courses/analysis/</code> 的 {len(book.chapters)} 章 "
        f"{len(book.units)} 个单元画布与解答生成；源文件指纹 "
        f"<code>{book.source_fingerprint}</code>。</p>"
        "<p>静态读本，不替代课程门禁：页面不判卷、不改课程。"
        "无外部 http(s) 资源；数学公式使用浏览器原生 MathML，"
        "不能结构化的公式回退为 Unicode 文本。"
        "单元与附录锚点稳定，可直接用 <code>#unit-N</code> 深链；"
        "搜索索引与阅读进度都在本机运行，进度存于 localStorage。</p>"
        "</footer>"
    )


def render_book(book: Book) -> str:
    style = STYLE_CSS.read_text(encoding="utf-8")
    script = APP_JS.read_text(encoding="utf-8")
    chapters = "\n".join(render_chapter(book, chapter) for chapter in book.chapters)
    title = f"{book.title} · 卷 II 全书"
    return (
        "<!DOCTYPE html>\n"
        '<html lang="zh-CN">\n<head>\n<meta charset="utf-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        f"<title>{escape(title)}</title>\n"
        '<meta name="description" content="从集合论到实分析：卷 II 数系与实数的交互式 HTML 全书">\n'
        f"<style>\n{style}\n</style>\n</head>\n<body data-view=\"both\">\n"
        '<a class="skip-link" href="#toc">跳到目录</a>\n'
        f"{render_cover(book)}\n"
        f"{render_toolbar(book)}\n"
        '<noscript><p class="noscript-note">JavaScript 已关闭：正文与原生折叠面板仍可阅读；'
        "图层切换、章节跳转、搜索、阅读进度与「展开全部」不可用。</p></noscript>\n"
        "<main>\n"
        f"{render_toc(book)}\n"
        f"{chapters}\n"
        f"{render_appendix(book)}\n"
        "</main>\n"
        f"{render_footer(book)}\n"
        f"{render_search_index(book)}\n"
        f"<script>\n{script}\n</script>\n</body>\n</html>\n"
    )


# ── check mode ──────────────────────────────────────────────────────────────


class AuditParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: List[str] = []
        self.hrefs: List[str] = []
        self.sections: List[str] = []
        self.classes: Counter[str] = Counter()
        self.tags: Counter[str] = Counter()

    def handle_starttag(self, tag: str, attrs: Sequence[Tuple[str, Optional[str]]]) -> None:
        data = {key: value for key, value in attrs}
        self.tags[tag] += 1
        if data.get("id"):
            self.ids.append(str(data["id"]))
        if data.get("href"):
            self.hrefs.append(str(data["href"]))
        if tag == "section" and data.get("class"):
            self.sections.append(str(data["class"]))
        for name in str(data.get("class") or "").split():
            self.classes[name] += 1

    handle_startendtag = handle_starttag


class ReadingParser(HTMLParser):
    """Collect the text a reader can actually see in each unit.

    ``pre``/``code`` are excluded from the marker and separator audits (code is
    content, not markdown), but paragraphs of prose are collected in document
    order so the checker can detect adjacent duplicates.  Search widgets, the
    cover, the TOC, the appendix and scripts/styles are outside the unit set.
    """

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.skip = 0
        self.current_unit: Optional[str] = None
        self.current_para: Optional[List[str]] = None
        self.unit_text: Dict[str, List[str]] = {}
        self.paragraphs: List[Tuple[str, str]] = []
        self.intro_units: List[str] = []

    def handle_starttag(self, tag: str, attrs: Sequence[Tuple[str, Optional[str]]]) -> None:
        data = {key: value for key, value in attrs}
        if tag == "section":
            classes = str(data.get("class") or "").split()
            if "unit" in classes and data.get("id"):
                self.current_unit = str(data["id"])
            elif "appendix" in classes:
                self.current_unit = None
            elif "unit-intro" in classes and self.current_unit:
                self.intro_units.append(self.current_unit)
        if tag in ("script", "style", "pre", "code"):
            self.skip += 1
            return
        if tag == "p" and self.skip == 0:
            self.current_para = []

    def handle_endtag(self, tag: str) -> None:
        if tag in ("script", "style", "pre", "code"):
            self.skip = max(0, self.skip - 1)
            return
        if tag == "p" and self.current_para is not None:
            text = "".join(self.current_para).strip()
            if text and self.current_unit:
                self.paragraphs.append((self.current_unit, text))
            self.current_para = None

    def handle_data(self, data: str) -> None:
        if self.skip:
            return
        if self.current_unit:
            self.unit_text.setdefault(self.current_unit, []).append(data)
        if self.current_para is not None:
            self.current_para.append(data)


def _prose_normal(text: str) -> str:
    return re.sub(r"\s+", "", plain_text(text))


def readability_checks(
    book: Book, html_text: str, parser: ReadingParser
) -> Tuple[List[str], Dict[str, object]]:
    """Human-facing assertions: no parser junk, a lead paragraph, sane
    paragraph sizes, and a readable default layer.  These complement the
    structural checks; they never rewrite or judge course prose."""
    errors: List[str] = []
    stats: Dict[str, object] = {}
    units = book.units
    expected_units = {f"unit-{unit.index}": unit for unit in units}

    stats["unit_intros"] = len(parser.intro_units)
    if len(parser.intro_units) != len(units) or set(parser.intro_units) != set(expected_units):
        errors.append(
            f"unit intro elements: expected {len(units)}, found {len(parser.intro_units)}"
        )
    for unit in units:
        if not unit.intro:
            errors.append(f"unit {unit.index}: no lead/intro paragraph was found")
        if f"unit-{unit.index}" not in parser.intro_units:
            errors.append(f"unit {unit.index}: intro element missing from HTML")

    literal = Counter()
    separator = []
    for unit_id, chunks in parser.unit_text.items():
        for chunk in chunks:
            for marker in ("**", "__"):
                if marker in chunk:
                    literal[marker] += 1
            for line in chunk.splitlines():
                if SEPARATOR_LINE_RE.match(line):
                    separator.append(f"{unit_id}: {line.strip()[:24]}")
    stats["literal_markers"] = sum(literal.values())
    stats["separator_lines"] = len(separator)
    if literal:
        errors.append(
            "visible markdown markers leaked into unit text: "
            + ", ".join(f"{marker}×{count}" for marker, count in literal.items())
        )
    if separator:
        errors.append("separator-only prose lines are visible: " + "; ".join(separator[:4]))

    hint_markers = [
        unit_id
        for unit_id, chunks in parser.unit_text.items()
        if any("soko:hint" in chunk for chunk in chunks)
    ]
    if hint_markers:
        errors.append(
            "raw soko:hint markers are visible in units: " + ", ".join(hint_markers[:6])
        )

    # Adjacent duplicate prose paragraphs, after whitespace/markdown normalisation.
    # Excluded on purpose: mathematical statements repeated in separate
    # declaration cards or in prompt vs. solution panels (intentional repetition,
    # not adjacent prose), and code/proof text inside <pre> (never parsed here).
    duplicates = []
    by_unit: Dict[str, List[str]] = {}
    for unit_id, text in parser.paragraphs:
        by_unit.setdefault(unit_id, []).append(text)
    for unit_id, paragraphs in by_unit.items():
        for left, right in zip(paragraphs, paragraphs[1:]):
            if _prose_normal(left) and _prose_normal(left) == _prose_normal(right):
                duplicates.append(f"{unit_id}: {plain_text(left)[:60]}")
    stats["duplicate_paragraphs"] = len(duplicates)
    if duplicates:
        errors.append(
            "adjacent duplicate prose paragraphs: " + "; ".join(duplicates[:4])
        )

    long_paragraphs = []
    for unit_id, text in parser.paragraphs:
        clean = plain_text(text)
        if len(clean) <= MAX_PROSE_CHARS:
            continue
        # Long mathematical definitions legitimately mix prose and formulas;
        # the cap targets prose walls, not displayed mathematics.
        if any(ch in MATH_SYMBOLS for ch in clean):
            continue
        long_paragraphs.append(f"{unit_id}: {len(clean)} chars")
    stats["long_paragraphs"] = len(long_paragraphs)
    stats["max_prose_chars"] = max(
        (len(plain_text(text)) for _, text in parser.paragraphs), default=0
    )
    if long_paragraphs:
        errors.append(
            f"prose paragraph longer than {MAX_PROSE_CHARS} chars: "
            + "; ".join(long_paragraphs[:4])
        )

    # Code/proof panels must start collapsed: the first read is prose.
    if re.search(r'<details class="(?:proof|code)-panel[^"]*"\s+open', html_text):
        errors.append("proof/code panel is open by default")
    if not re.search(
        r'<button type="button" data-view="both" aria-pressed="true">', html_text
    ):
        errors.append("default reading layer is not 并读 (both)")
    if 'body data-view="both"' not in html_text:
        errors.append("body is not initialised to the 并读 layer")

    stats["unit_notes"] = sum(1 for unit in units if unit.notes)
    return errors, stats


def math_helper_selftest() -> List[str]:
    """The course comments are mostly Unicode only; keep the tiny fraction /
    superscript helper honest with direct assertions, including the fallback
    shapes used by M2."""
    errors: List[str] = []
    cases = [
        ("(a+b)/(c+d)", "<mfrac>", "simple fraction"),
        ("x² + y²", "<msup>", "Unicode superscript"),
        ("x^2", "<msup>", "ASCII caret superscript"),
        ("f(x) ≤ g(y)", "<mo>≤</mo>", "relation"),
        ("a → b", "<mo>→</mo>", "arrow"),
    ]
    for source, needle, label in cases:
        rendered = mathml(source)
        if rendered is None or needle not in rendered:
            errors.append(f"MathML helper: {label} did not produce {needle}")
    if mathml("中文公式 x = y") is not None:
        errors.append("MathML helper: CJK text should fall back, not become MathML")
    if mathml("fun x => x") is not None:
        errors.append("MathML helper: code-shaped formula should fall back")
    fallback = render_formula("中文公式 x = y")
    if "math-display" not in fallback or "中文公式" not in fallback:
        errors.append("formula fallback dropped content")
    return errors


def _unit_segments(html_text: str) -> Dict[str, str]:
    starts = list(
        re.finditer(r'<section class="unit[^"]*" id="(unit-\d+)">', html_text)
    )
    segments: Dict[str, str] = {}
    for index, match in enumerate(starts):
        end = starts[index + 1].start() if index + 1 < len(starts) else html_text.find(
            '<section class="appendix"', match.end()
        )
        if end < 0:
            end = len(html_text)
        segments[match.group(1)] = html_text[match.start():end]
    return segments


def _chapter_segments(html_text: str) -> Dict[str, str]:
    starts = list(
        re.finditer(r'<section class="chapter" id="(chapter-[^"]+)">', html_text)
    )
    segments: Dict[str, str] = {}
    for index, match in enumerate(starts):
        end = starts[index + 1].start() if index + 1 < len(starts) else html_text.find(
            '<section class="appendix"', match.end()
        )
        if end < 0:
            end = len(html_text)
        segments[match.group(1)] = html_text[match.start():end]
    return segments


def formula_selfcheck(book: Book) -> Tuple[int, int, List[str]]:
    """Render every formula item once.  Return (mathml, fallback, errors)."""
    errors: List[str] = []
    mathml_count = 0
    fallback_count = 0

    def check_item(item: Item, where: str) -> None:
        nonlocal mathml_count, fallback_count
        if item.kind != "formula":
            return
        source = item.text.strip()
        rendered = render_formula(source)
        if not rendered:
            errors.append(f"{where}: formula rendered as empty")
            return
        structured = mathml(source) is not None
        if structured:
            mathml_count += 1
        else:
            fallback_count += 1
            needle = escape(source.replace("`", "").strip("。；，、 "))
            if needle not in rendered:
                errors.append(f"{where}: fallback formula dropped its text")

    for unit in book.units:
        for item in unit.intro:
            check_item(item, f"unit {unit.index} intro")
        for item in unit.notes:
            check_item(item, f"unit {unit.index} notes")
        for decl in unit.decls:
            for item in decl.lead:
                check_item(item, f"unit {unit.index} {decl.name}")
            for hint in decl.hints:
                for line in hint.body.splitlines():
                    if _looks_formula(line):
                        fake = Item("formula", text=line.strip())
                        check_item(fake, f"unit {unit.index} hint")
        for item in unit.trailing:
            check_item(item, f"unit {unit.index} trailing")
    return mathml_count, fallback_count, errors


def stale_error(written: str, fresh: str) -> Optional[str]:
    if written == fresh:
        return None
    written_lines = written.splitlines()
    fresh_lines = fresh.splitlines()
    for index, (left, right) in enumerate(zip(written_lines, fresh_lines), start=1):
        if left != right:
            return (
                f"output is stale; first difference at line {index} "
                "(re-run the build)"
            )
    return "output is stale; file length differs (re-run the build)"


def validate(book: Book, html_text: str) -> Tuple[List[str], Dict[str, object]]:
    errors: List[str] = []
    stats: Dict[str, object] = {}
    units = book.units
    expected_units = sum(len(chapter.units) for chapter in book.chapters)

    stats["units"] = len(units)
    stats["manifest_units"] = book.manifest_unit_count
    stats["chapters"] = len(book.chapters)
    stats["declarations"] = book.declarations
    stats["exercises"] = book.exercises
    stats["hints"] = book.hints
    stats["solutions"] = book.solutions
    stats["extra_solutions"] = book.extra_solutions
    stats["chapter_units"] = {chapter.id: len(chapter.units) for chapter in book.chapters}
    stats["chapter_exercises"] = {
        chapter.id: chapter.exercises for chapter in book.chapters
    }

    if not units:
        errors.append("book has no units")
    if len(units) != expected_units:
        errors.append(f"loaded unit count {len(units)} != manifest sum {expected_units}")
    if len(units) != book.manifest_unit_count:
        errors.append(
            f"loaded unit count {len(units)} != course.json unit count {book.manifest_unit_count}"
        )
    if not book.chapters:
        errors.append("book has no chapters")
    if book.declarations <= 0:
        errors.append("declaration count is zero")
    if book.exercises <= 0:
        errors.append("exercise count is zero")
    if book.hints <= 0:
        errors.append("hint count is zero")
    if book.solutions <= 0:
        errors.append("solution count is zero")
    if book.solutions != book.exercises:
        errors.append(
            f"solution count {book.solutions} != exercise count {book.exercises}"
        )
    if book.unlisted_unit_files:
        errors.append(
            "unit files exist under courses/analysis/units but are absent from course.json: "
            + ", ".join(book.unlisted_unit_files[:8])
        )

    numbers = [unit.index for unit in units]
    if len(set(numbers)) != len(numbers):
        errors.append("duplicate unit numbers")
    if numbers != sorted(numbers):
        errors.append("unit numbers are not in ascending order")
    for chapter in book.chapters:
        if not chapter.units:
            errors.append(f"chapter {chapter.id}: no units")
        chapter_ids = [unit.index for unit in chapter.units]
        if chapter_ids != list(range(chapter_ids[0], chapter_ids[0] + len(chapter_ids))):
            errors.append(f"chapter {chapter.id}: unit numbers are not contiguous")

    for unit in units:
        if not unit.title:
            errors.append(f"unit {unit.index}: empty title")
        if not unit.display_title:
            errors.append(f"unit {unit.index}: empty display title")
        if not unit.blocks:
            errors.append(f"unit {unit.index}: no blocks")
        if not unit.decls:
            errors.append(f"unit {unit.index}: no declarations")
        if not unit.exercises:
            errors.append(f"unit {unit.index}: no exercises")
        if unit.stray:
            errors.append(
                f"unit {unit.index}: unparsed source lines: " + "; ".join(unit.stray[:4])
            )
        for decl in unit.decls:
            if decl.solution is None and decl.is_exercise:
                errors.append(
                    f"unit {unit.index}: exercise {decl.name} has no matching solution"
                )
        if render_inline(unit.display_title) not in html_text:
            errors.append(f"unit {unit.index}: display title missing from HTML")
        if unit.title_en and escape(unit.title_en) not in html_text:
            errors.append(f"unit {unit.index}: title_en missing from HTML")

    parser = AuditParser()
    parser.feed(html_text)
    reading = ReadingParser()
    reading.feed(html_text)
    readability_errors, readability_stats = readability_checks(book, html_text, reading)
    errors.extend(readability_errors)
    stats.update(readability_stats)
    stats["ids"] = len(parser.ids)
    stats["toc_links"] = sum(1 for href in parser.hrefs if href.startswith("#"))
    stats["details"] = parser.tags.get("details", 0)
    stats["mathml"] = parser.classes.get("mathml", 0)
    stats["math_displays"] = parser.classes.get("math-display", 0)
    stats["exercise_cards"] = parser.classes.get("exercise", 0)
    stats["hint_blocks"] = parser.classes.get("hint", 0)
    stats["solution_panels"] = parser.classes.get("solution-panel", 0)
    stats["appendix_solutions"] = parser.classes.get("appendix-solution", 0)

    duplicates = sorted({item for item in parser.ids if parser.ids.count(item) > 1})
    if duplicates:
        errors.append("duplicate ids: " + ", ".join(duplicates[:8]))

    ids = set(parser.ids)
    for href in parser.hrefs:
        if not href.startswith("#"):
            continue
        target = href[1:]
        if target and target not in ids:
            errors.append(f"internal link does not resolve: {href}")

    if "top" not in ids:
        errors.append("cover anchor #top missing")
    if "toc" not in ids:
        errors.append("TOC anchor #toc missing")
    if 'data-view="both"' not in html_text:
        errors.append("layer toggle default state missing")
    for action in ("expand-all", "collapse-all", "print", "chapter-jump"):
        if f'data-action="{action}"' not in html_text:
            errors.append(f"reading control missing: {action}")
    if "localStorage" not in html_text:
        errors.append("localStorage view memory missing")
    if "beforeprint" not in html_text:
        errors.append("beforeprint handling missing")

    # ── search ────────────────────────────────────────────────────────────
    if 'id="book-search"' not in html_text:
        errors.append("search input missing")
    if 'placeholder="搜索单元 / 定义 / 定理"' not in html_text:
        errors.append("search placeholder missing")
    if 'role="combobox"' not in html_text or 'role="listbox"' not in html_text:
        errors.append("search combobox/listbox semantics missing")
    if 'aria-live="polite"' not in html_text:
        errors.append("search result live region missing")
    search_match = re.search(
        r'<script type="application/json" id="book-search-index">(.*?)</script>',
        html_text,
        re.DOTALL,
    )
    search_payload: object = []
    if search_match is None:
        errors.append("search index script missing")
    else:
        try:
            search_payload = json.loads(search_match.group(1))
        except json.JSONDecodeError as exc:
            errors.append(f"search index is not valid JSON: {exc}")
    expected_search = search_entries(book)
    if search_payload != expected_search:
        errors.append("search index does not match the live book model")
    if isinstance(search_payload, list):
        stats["search_entries"] = len(search_payload)
        search_markdown = [
            entry
            for entry in search_payload
            if isinstance(entry, dict)
            and any(
                isinstance(entry.get(key), str)
                and re.search(r"\*\*|__|`", str(entry.get(key)))
                for key in ("t", "h")
            )
        ]
        stats["search_markdown"] = len(search_markdown)
        if search_markdown:
            errors.append(
                "search index display text still contains markdown markers: "
                + "; ".join(str(entry.get("t"))[:60] for entry in search_markdown[:4])
            )
        stats["search_sections"] = sum(
            1
            for entry in search_payload
            if isinstance(entry, dict) and entry.get("k") == "section"
        )
        stats["search_declarations"] = sum(
            1
            for entry in search_payload
            if isinstance(entry, dict)
            and entry.get("k") in ("def", "theorem", "axiom", "inductive")
        )
        stats["search_appendix"] = sum(
            1
            for entry in search_payload
            if isinstance(entry, dict)
            and entry.get("k") in ("appendix", "solution")
        )
        broken = [
            entry.get("a")
            for entry in search_payload
            if isinstance(entry, dict) and entry.get("a") not in ids
        ]
        if broken:
            errors.append(
                "search index anchors do not resolve: "
                + ", ".join(str(anchor) for anchor in broken[:6])
            )
    else:
        stats["search_entries"] = 0
        stats["search_sections"] = 0
        stats["search_declarations"] = 0
        stats["search_appendix"] = 0

    # ── reading progress / per-unit links ─────────────────────────────────
    progress_toggles = len(
        re.findall(r'data-progress-toggle data-unit="\d+"', html_text)
    )
    copy_links = len(re.findall(r'data-copy-link="unit-\d+"', html_text))
    stats["progress_toggles"] = progress_toggles
    stats["copy_links"] = copy_links
    if progress_toggles != len(units):
        errors.append(
            f"progress toggles: expected {len(units)}, found {progress_toggles}"
        )
    if copy_links != len(units):
        errors.append(f"copy-link controls: expected {len(units)}, found {copy_links}")
    if "soko-book-progress-v1" not in html_text:
        errors.append("reading-progress localStorage key missing")
    if 'role="progressbar"' not in html_text:
        errors.append("reading-progress bar (role=progressbar) missing")
    if 'data-progress-count' not in html_text:
        errors.append("reading-progress count missing")
    for action in ("reset-progress", "reset-progress-confirm", "reset-progress-cancel"):
        if f'data-action="{action}"' not in html_text:
            errors.append(f"progress reset control missing: {action}")
    if "pushState" not in html_text:
        errors.append("pushState navigation missing")
    if "hashchange" not in html_text:
        errors.append("hashchange handling missing")
    if "data-copy-status" not in html_text:
        errors.append("copy-link visible status missing")

    toc_start = html_text.find('<nav class="toc" id="toc"')
    toc_end = html_text.find("</nav>", toc_start)
    toc_segment = html_text[toc_start:toc_end] if toc_start >= 0 and toc_end > toc_start else ""
    for chapter in book.chapters:
        if f'href="#{chapter.anchor}"' not in toc_segment:
            errors.append(f"TOC does not link chapter {chapter.id}")
        if f'id="{chapter.anchor}"' not in html_text:
            errors.append(f"chapter anchor missing: {chapter.anchor}")
        for unit in chapter.units:
            if f'href="#unit-{unit.index}"' not in toc_segment:
                errors.append(f"TOC does not link unit {unit.index}")

    chapter_segments = _chapter_segments(html_text)
    for chapter in book.chapters:
        if chapter.anchor not in chapter_segments:
            errors.append(f"chapter {chapter.id}: rendered section not found")
            continue
        segment = chapter_segments[chapter.anchor]
        rendered_units = len(
            re.findall(r'<section class="unit[^"]*" id="unit-\d+">', segment)
        )
        if rendered_units != len(chapter.units):
            errors.append(
                f"chapter {chapter.id}: expected {len(chapter.units)} unit sections, "
                f"found {rendered_units}"
            )
        for unit in chapter.units:
            if f'id="unit-{unit.index}"' not in segment:
                errors.append(f"chapter {chapter.id}: unit {unit.index} missing from section")

    unit_segments = _unit_segments(html_text)
    expected_unit_ids = {f"unit-{unit.index}" for unit in units}
    if set(unit_segments) != expected_unit_ids:
        errors.append(
            "unit section ids mismatch: "
            f"missing={sorted(expected_unit_ids - set(unit_segments))[:6]} "
            f"extra={sorted(set(unit_segments) - expected_unit_ids)[:6]}"
        )

    for unit in units:
        segment = unit_segments.get(f"unit-{unit.index}")
        if segment is None:
            errors.append(f"unit {unit.index}: rendered section not found")
            continue
        if not any(
            token in segment
            for token in ('class="block prose', 'class="decl ', 'class="exercise"')
        ):
            errors.append(f"unit {unit.index}: rendered section has no content blocks")
        for decl in unit.decls:
            if escape(decl.name) not in segment:
                errors.append(f"unit {unit.index}: declaration {decl.name} missing from HTML")
        for decl in unit.exercises:
            if f'id="ex-{unit.index}-{decl.exercise_no}"' not in segment:
                errors.append(f"unit {unit.index}: exercise card {decl.exercise_no} missing")
            if f'id="sol-{unit.index}-{decl.exercise_no}"' not in segment:
                errors.append(f"unit {unit.index}: solution toggle {decl.exercise_no} missing")
            if f'id="app-sol-{unit.index}-{decl.exercise_no}"' not in html_text:
                errors.append(f"unit {unit.index}: appendix solution {decl.exercise_no} missing")
        position = units.index(unit)
        if position > 0:
            previous = units[position - 1]
            if f'href="#unit-{previous.index}"' not in segment:
                errors.append(f"unit {unit.index}: missing previous-unit link")
        if position + 1 < len(units):
            following = units[position + 1]
            if f'href="#unit-{following.index}"' not in segment:
                errors.append(f"unit {unit.index}: missing next-unit link")
        if f'id="appendix-unit-{unit.index}"' not in html_text:
            errors.append(f"unit {unit.index}: appendix unit anchor missing")

    if parser.classes.get("exercise", 0) != book.exercises:
        errors.append(
            f"exercise cards: expected {book.exercises}, "
            f"found {parser.classes.get('exercise', 0)}"
        )
    if parser.classes.get("hint", 0) != book.hints:
        errors.append(
            f"hint blocks: expected {book.hints}, found {parser.classes.get('hint', 0)}"
        )
    if parser.classes.get("solution-panel", 0) != book.exercises:
        errors.append(
            f"solution toggles: expected {book.exercises}, "
            f"found {parser.classes.get('solution-panel', 0)}"
        )
    expected_appendix = book.exercises + book.extra_solutions
    if parser.classes.get("appendix-solution", 0) != expected_appendix:
        errors.append(
            f"appendix solutions: expected {expected_appendix}, "
            f"found {parser.classes.get('appendix-solution', 0)}"
        )
    if parser.classes.get("math-display", 0) <= 0:
        errors.append("no display formulas rendered")
    if parser.classes.get("mathml", 0) <= 0:
        errors.append("no native MathML rendered")

    mathml_count, fallback_count, formula_errors = formula_selfcheck(book)
    errors.extend(formula_errors)
    stats["formula_mathml"] = mathml_count
    stats["formula_fallbacks"] = fallback_count

    if re.search(r"https?://", html_text, re.I):
        errors.append("generated HTML contains an external http(s) reference")
    if re.search(r"<link\b", html_text, re.I):
        errors.append("generated HTML contains a <link> tag")
    if re.search(r"<script[^>]+\bsrc\s*=", html_text, re.I):
        errors.append("generated HTML contains an external <script src>")
    if re.search(r"(?:src|href)\s*=\s*[\"']//", html_text, re.I):
        errors.append("generated HTML contains a protocol-relative reference")
    if re.search(r"@import\s+(?:url\(|[\"'])", html_text, re.I):
        errors.append("generated HTML contains a CSS @import")

    if "@page" not in html_text or "break-before: page" not in html_text:
        errors.append("print CSS (@page / break-before) missing")
    if "size: A4" not in html_text:
        errors.append("print CSS is not A4")
    if 'lang="zh-CN"' not in html_text:
        errors.append("document language is not zh-CN")

    stats["external_refs"] = len(re.findall(r"https?://", html_text))
    stats["bytes"] = len(html_text.encode("utf-8"))
    return errors, stats


def negative_selftest(book: Book, fresh: str) -> List[str]:
    """Prove the checker bites: external URL, broken anchor, duplicate id,
    stale output must each be rejected."""
    errors: List[str] = []
    cases: Dict[str, str] = {}
    cases["external URL"] = fresh.replace(
        "</main>", '<a href="https://example.com/injected">bad</a></main>', 1
    )
    first_unit = f'id="unit-{book.units[0].index}"'
    cases["broken unit anchor"] = fresh.replace(
        first_unit, f'id="unit-{book.units[0].index}-broken"', 1
    )
    cases["duplicate toc id"] = fresh.replace(
        "</main>", '<span id="toc"></span></main>', 1
    )
    cases["search index removed"] = fresh.replace(
        'id="book-search-index"', 'id="book-search-index-gone"', 1
    )
    cases["copy-link control removed"] = fresh.replace(
        'data-copy-link="unit-1"', 'data-copy-link-gone="unit-1"', 1
    )
    intro_marker = '<section class="unit-intro" aria-label="单元导语">'
    intro_label = intro_marker
    cases["unit intro removed"] = fresh.replace(intro_marker, "<div>", 1)
    cases["visible ** markdown"] = fresh.replace(
        intro_label, intro_label + "<p>**junk**</p>", 1
    )
    cases["separator-only prose line"] = fresh.replace(
        intro_label, intro_label + "<p>============</p>", 1
    )
    cases["adjacent duplicate prose"] = fresh.replace(
        intro_label, intro_label + "<p>相邻重复的导语。</p><p>相邻重复的导语。</p>", 1
    )
    long_paragraph = "长" * (MAX_PROSE_CHARS + 50)
    cases["over-long prose paragraph"] = fresh.replace(
        intro_label, intro_label + f"<p>{long_paragraph}</p>", 1
    )
    for label, tampered in cases.items():
        found, _ = validate(book, tampered)
        if not found:
            errors.append(f"negative selftest: {label} was not rejected")
    if stale_error(fresh + "\n", fresh) is None:
        errors.append("negative selftest: stale output was not rejected")
    return errors


def output_path(out_dir: Path) -> Path:
    return out_dir / "index.html"


def format_stats(stats: Dict[str, object]) -> List[str]:
    chapter_units = stats.get("chapter_units", {})
    chapter_exercises = stats.get("chapter_exercises", {})
    return [
        f"  units: {stats['units']} (course.json {stats['manifest_units']}) · "
        f"chapters: {stats['chapters']}",
        "  per chapter: " + " · ".join(
            f"{key} {count}u/{chapter_exercises.get(key, 0)}ex" for key, count in chapter_units.items()
        ),
        f"  blocks: {stats['declarations']} declarations · "
        f"{stats['exercises']} exercises · {stats['hints']} hints · "
        f"{stats['solutions']} solutions · {stats['extra_solutions']} extras",
        f"  rendered: {stats['exercise_cards']} exercise cards · "
        f"{stats['solution_panels']} solution toggles · {stats['details']} details · "
        f"{stats['mathml']} MathML blocks · {stats['formula_fallbacks']} formula fallbacks",
        f"  search: {stats['search_entries']} entries "
        f"({stats['search_declarations']} declarations · "
        f"{stats['search_sections']} section headings · "
        f"{stats['search_appendix']} appendix entries · "
        f"{stats.get('search_markdown', 0)} markdown leftovers) · "
        f"{stats['progress_toggles']} progress toggles · {stats['copy_links']} copy links",
        f"  readability: {stats['unit_intros']} unit intros · "
        f"{stats.get('unit_notes', 0)} units with collapsed notes · "
        f"max prose paragraph {stats.get('max_prose_chars', 0)} chars · "
        f"{stats.get('literal_markers', 0)} literal markdown markers · "
        f"{stats.get('separator_lines', 0)} separator lines · "
        f"{stats.get('duplicate_paragraphs', 0)} duplicate paragraphs",
        f"  toc links: {stats['toc_links']} · ids: {stats['ids']} · "
        f"external refs: {stats['external_refs']}",
    ]


def style_lint_report(html_path: Optional[Path] = None) -> Tuple[List[str], Dict[str, object]]:
    """Run the copy-editor hard gate over source prose and the generated page.

    BLOCK-class findings (AI-writing tells, internal-jargon leaks, separator
    noise, template boilerplate, over-formatting in source) are hard failures.
    The HTML pass checks visible text only; source-only structural rules are not
    applied to the rendered DOM.  Counts are reported for the readability
    sprint but only hard failures fail this check.
    """
    errors: List[str] = []
    stats: Dict[str, object] = {}
    if not STYLE_LINT.is_file():
        return ["style linter missing: scripts/content_style/style_lint.py"], stats
    cmd = [
        sys.executable,
        str(STYLE_LINT),
        "--root",
        str(COURSE_ROOT / "units"),
        "--json",
    ]
    if html_path is not None and html_path.is_file():
        cmd.extend(["--html", str(html_path)])
    proc = subprocess.run(cmd, capture_output=True, text=True)
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError:
        errors.append(
            f"style linter produced non-JSON output (rc={proc.returncode}): {proc.stderr.strip()[:300]}"
        )
        return errors, stats
    reports = data.get("reports", []) if isinstance(data, dict) else []
    source = next((r for r in reports if r.get("mode") == "source"), {})
    html = next((r for r in reports if r.get("mode") == "html"), {})
    stats = {
        "source_files": source.get("files", 0),
        "source_errors": len(source.get("errors", [])),
        "html_errors": len(html.get("errors", [])),
        "ok": bool(data.get("ok")),
    }
    if proc.returncode != 0 or not data.get("ok"):
        if source.get("errors"):
            errors.append(f"style lint (source): {len(source['errors'])} hard finding(s)")
        if html.get("errors"):
            errors.append(f"style lint (html): {len(html['errors'])} hard finding(s)")
        if not errors:
            errors.append(f"style lint failed (rc={proc.returncode})")
    return errors, stats

def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--out",
        default=DEFAULT_OUT,
        help=f"output directory (default: {DEFAULT_OUT})",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="validate the already-generated output (does not write)",
    )
    parser.add_argument(
        "--check-preview",
        action="store_true",
        help="also validate the committed scripts/book/preview/index.html",
    )
    parser.add_argument(
        "--selftest",
        action="store_true",
        help="run negative checker tests and exit (does not write)",
    )
    args = parser.parse_args(argv)

    book = load_book()
    fresh = render_book(book)
    out_dir = Path(args.out)
    if not out_dir.is_absolute():
        out_dir = (ROOT / out_dir).resolve()
    target = output_path(out_dir)

    if args.selftest:
        errors = math_helper_selftest()
        errors.extend(negative_selftest(book, fresh))
        if errors:
            print("book selftest: FAIL", file=sys.stderr)
            for error in errors:
                print(f"  - {error}", file=sys.stderr)
            return 1
        print("book selftest: PASS")
        print(
            "  external URL / broken anchor / duplicate id / missing search index / "
            "missing copy-link / missing unit intro / visible ** / separator line / "
            "duplicate prose / over-long prose / stale output: all rejected"
        )
        return 0

    fresh_errors, fresh_stats = validate(book, fresh)
    if fresh_errors:
        print("book build: FAIL (fresh render did not pass its own contract)", file=sys.stderr)
        for error in fresh_errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    if args.check or args.check_preview:
        errors: List[str] = []
        stats: Dict[str, object] = {}
        if args.check:
            if not target.is_file():
                print(f"FAIL: {target} does not exist; run the build first", file=sys.stderr)
                return 1
            written = target.read_text(encoding="utf-8")
            errors, stats = validate(book, written)
            errors.extend(math_helper_selftest())
            stale = stale_error(written, fresh)
            if stale:
                errors.append(stale)
            errors.extend(negative_selftest(book, fresh))
            style_errors, style_stats = style_lint_report(target)
            errors.extend(style_errors)
            if errors:
                print("book v2 check: FAIL", file=sys.stderr)
                for error in errors:
                    print(f"  - {error}", file=sys.stderr)
                return 1
            print("book v2 check: PASS")
            for line in format_stats(stats):
                print(line)
            print("  math helper: fraction + superscript + fallback self-test OK")
            print(
                "  negative selftest: external URL / broken anchor / duplicate id / "
                "missing search index / missing copy-link / missing unit intro / "
                "visible ** / separator line / duplicate prose / over-long prose / "
                "stale output rejected"
            )
            print(
                "  style lint: "
                f"{style_stats.get('source_files', 0)} source files · "
                f"{style_stats.get('source_errors', 0)} source hard · "
                f"{style_stats.get('html_errors', 0)} html hard · "
                f"ok={style_stats.get('ok', False)}"
            )
            print(f"  output: {target} ({stats['bytes']:,} bytes)")

        if args.check_preview:
            if not PREVIEW_HTML.is_file():
                print(f"FAIL: preview missing: {PREVIEW_HTML}", file=sys.stderr)
                return 1
            preview = PREVIEW_HTML.read_text(encoding="utf-8")
            preview_errors, preview_stats = validate(book, preview)
            preview_errors.extend(math_helper_selftest())
            stale = stale_error(preview, fresh)
            if stale:
                preview_errors.append("preview " + stale)
            if preview_errors:
                print("book v2 preview check: FAIL", file=sys.stderr)
                for error in preview_errors:
                    print(f"  - {error}", file=sys.stderr)
                return 1
            print("book v2 preview check: PASS")
            print(
                f"  preview: {PREVIEW_HTML} ({preview_stats['bytes']:,} bytes, "
                f"{preview_stats['external_refs']} external refs)"
            )
        return 0

    out_dir.mkdir(parents=True, exist_ok=True)
    target.write_text(fresh, encoding="utf-8")
    print(f"wrote {target} ({len(fresh.encode('utf-8')):,} bytes)")
    for line in format_stats(fresh_stats):
        print(line)
    print(f"  source fingerprint: {book.source_fingerprint}")
    print(
        "  preview: python3 scripts/build_book_proto.py --out scripts/book/preview"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
