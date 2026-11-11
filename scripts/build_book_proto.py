#!/usr/bin/env python3
"""Build the interactive HTML prototype for course II.1 (19 units).

This is deliberately a *prototype*: it reads the existing course canvas and
solution files, renders them as a static book page, and implements a small
check mode.  It does not judge proofs, call the kernel, or edit course files.

Usage (from the repository root):

    python3 scripts/build_book_proto.py --out dist/book/proto-II.1
    python3 scripts/build_book_proto.py --check --out dist/book/proto-II.1

The generated page has no external assets: CSS and JS are inlined, math is
Unicode plus native MathML, and the few simple fractions/superscripts are
converted by the tiny ``mathml()`` helper below.  Standard library only.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from html.parser import HTMLParser
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Sequence, Tuple


ROOT = Path(__file__).resolve().parent.parent
COURSE_JSON = ROOT / "courses" / "analysis" / "course.json"
BOOK_DIR = ROOT / "scripts" / "book"
STYLE_CSS = BOOK_DIR / "style.css"
APP_JS = BOOK_DIR / "app.js"
CHAPTER_ID = "II.1"
EXPECTED_UNITS = 19

DECL_RE = re.compile(r"^(def|theorem|axiom|inductive)\s+([^\s(:]+)")
IMPORT_RE = re.compile(r"^import\s+")
TITLE_RE = re.compile(r"^unit\d+[-_].*\.sokonanoda")
CIRCLED = "①②③④⑤⑥⑦⑧⑨⑩⑪⑫⑬⑭⑮⑯⑰⑱⑲⑳"

FORMULA_MARKERS = set("=∀∃→←↔∈∉⊆⊂≤≥≠⇒⇔⟺∧∨¬")
MATH_SYMBOLS = FORMULA_MARKERS | set("ℕℤℚℝℂαβγδεζηθικλμνξοπρστυφχψωΓΔΘΛΞΠΣΦΨΩ×·−+⁰¹²³⁴⁵⁶⁷⁸⁹⁻")
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
    decls: List[Decl] = field(default_factory=list)
    trailing: List[Item] = field(default_factory=list)
    imports: List[str] = field(default_factory=list)
    solution_path: Optional[Path] = None
    solution_decls: Dict[str, Decl] = field(default_factory=dict)
    extra_solutions: List[Decl] = field(default_factory=list)

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
class Book:
    chapter_title: str
    units: List[Unit]
    source_fingerprint: str
    import_count: int
    html: str = ""

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
    if len(text) < 2 or len(text) > 220:
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
) -> Tuple[List[Decl], List[Item], List[str]]:
    """Split a canvas/solution file into declarations, trailing prose, imports."""
    decls: List[Decl] = []
    pending: List[Item] = []
    imports: List[str] = []
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
            # Defensive: an unexpected top-level line belongs to the current
            # declaration start, not to prose.  The corpus does not hit this.
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

    return decls, _clean_items(pending), imports


def load_source(path: Path) -> List[str]:
    return path.read_text(encoding="utf-8").splitlines()


def find_chapter(course: dict, chapter_id: str) -> dict:
    for volume in course.get("volumes", []):
        for chapter in volume.get("chapters", []):
            if chapter.get("id") == chapter_id:
                return chapter
    raise SystemExit(f"course.json: chapter {chapter_id!r} not found")


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


def parse_unit(unit: dict, chapter: dict, course_root: Path) -> Unit:
    unit_file = course_root / unit["file"]
    lines = load_source(unit_file)
    title = str(unit.get("title") or "").strip()
    title_en = _unit_title_en(unit)
    decls, trailing, imports = parse_declarations(lines, title, unit_file.name)
    result = Unit(
        index=int(unit["unit"]),
        title=title,
        title_en=title_en,
        file=unit["file"],
        decls=decls,
        trailing=trailing,
        imports=imports,
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
        solution_decls, _, _ = parse_declarations(
            solution_lines, title, solution_path.name
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
        text = item.text.strip()
        if not text:
            continue
        if _normalise_text(text) in known:
            continue
        notes.append(text)
    return "\n\n".join(notes[:2])


def source_fingerprint(paths: Iterable[Path]) -> str:
    digest = hashlib.sha256()
    for path in sorted(paths):
        digest.update(path.name.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
    return digest.hexdigest()[:12]


def load_book() -> Book:
    course = json.loads(COURSE_JSON.read_text(encoding="utf-8"))
    chapter = find_chapter(course, CHAPTER_ID)
    course_root = COURSE_JSON.parent
    units = [parse_unit(unit, chapter, course_root) for unit in chapter.get("units", [])]
    source_paths: List[Path] = []
    for unit in units:
        source_paths.append(course_root / unit.file)
        if unit.solution_path is not None:
            source_paths.append(unit.solution_path)
    source_paths.extend([STYLE_CSS, APP_JS])
    return Book(
        chapter_title=str(chapter.get("title") or ""),
        units=units,
        source_fingerprint=source_fingerprint(source_paths),
        import_count=sum(len(unit.imports) for unit in units),
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
    """Return a small block MathML node, or None for text MathML should not own."""
    source = text.strip().replace("`", "").strip("。；，、 ")
    if not source or len(source) > 220:
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
        trailing = span[len(span.rstrip()) :]
        out.append(escape(text[position:match.start()]))
        if leading:
            out.append(escape(leading))
        out.append(f'<span class="math">{escape(stripped)}</span>')
        if trailing:
            out.append(escape(trailing))
        position = match.end()
    out.append(escape(text[position:]))
    return "".join(out)


def _find_bold_close(text: str, start: int) -> int:
    """Find a closing ``**`` that is not inside an inline-code span."""
    position = start
    while position < len(text) - 1:
        if text[position] == "`":
            closing = text.find("`", position + 1)
            if closing < 0:
                return -1
            position = closing + 1
            continue
        if text.startswith("**", position):
            return position
        position += 1
    return -1


def _render_inline(text: str, depth: int = 0) -> str:
    """Tiny inline parser for `code`, **bold**, and serif math spans."""
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
        if text.startswith("**", position) and depth < 4:
            closing = _find_bold_close(text, position + 2)
            if closing >= 0 and closing > position + 2:
                flush()
                inner = _render_inline(text[position + 2:closing], depth + 1)
                out.append(f"<strong>{inner}</strong>")
                position = closing + 2
                continue
        buf.append(char)
        position += 1
    flush()
    return "".join(out)


def render_inline(text: str) -> str:
    """Escape prose and add inline code, bold, and serif math spans."""
    return _render_inline(text)

# ── HTML rendering ──────────────────────────────────────────────────────────


CJK_PUNCT = set("，。；：！？、（）「」『』《》“”‘’…—·\"\'")


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
        f'<span class="hint-label">{escape(hint.label)}</span></summary>'
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


def render_items(items: Sequence[Item], *, include_hints: bool = False) -> str:
    out: List[str] = []
    for item in items:
        if item.kind == "hint":
            if include_hints and item.hint is not None:
                out.append(render_hint(item.hint, 1))
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
    open_attr = " open" if len(steps) <= 2 and line_count <= 10 else ""
    step_html = "".join(
        f"<li><pre><code>{escape(step)}</code></pre></li>" for step in steps
    )
    return (
        f'<details class="proof-panel formal-only"{open_attr}>'
        f"<summary>证明 · {len(steps)} 步 · {line_count} 行</summary>"
        f'<ol class="proof-steps">{step_html}</ol>'
        "</details>"
    )


def render_code_panel(decl: Decl) -> str:
    if decl.code == decl.statement:
        return ""
    line_count = len(decl.code.splitlines())
    open_attr = " open" if line_count <= 6 else ""
    return (
        f'<details class="code-panel formal-only"{open_attr}>'
        f"<summary>完整代码 · {line_count} 行</summary>"
        f'<pre><code>{escape(decl.code)}</code></pre>'
        "</details>"
    )


def render_decl(unit: Unit, decl: Decl) -> str:
    outside = _outside_prompt(decl.lead)
    prompt = _prompt_items(decl.lead)
    # Exercises keep their prompt inside the exercise card; every other
    # declaration renders all of its lead prose before the card.  (Hints are
    # rendered separately, inside the card.)
    prefix = render_items(outside if decl.is_exercise else decl.lead)
    kind_label = KIND_LABELS.get(decl.kind, decl.kind)
    decl_id = f"decl-{unit.index}-{decl.index}"

    if decl.is_exercise:
        prompt_html = render_items(prompt)
        return (
            prefix
            + f'<article class="exercise" id="ex-{unit.index}-{decl.exercise_no}">'
            + '<header class="exercise-head">'
            + f'<span class="exercise-badge">练习 {decl.exercise_no}</span>'
            + f'<code class="exercise-name">{escape(decl.name)}</code>'
            + f'<span class="exercise-kind">{escape(kind_label)}</span>'
            + "</header>"
            + '<div class="exercise-body">'
            + (f'<div class="exercise-prompt">{prompt_html}</div>' if prompt_html else "")
            + f'<pre class="exercise-statement"><code>{escape(decl.statement)}</code></pre>'
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


def render_unit(unit: Unit) -> str:
    hints = unit.hints
    meta_bits = [
        f"{unit.defs} 个定义",
        f"{unit.theorems} 个定理",
        f"{len(unit.exercises)} 道练习",
        f"{len(hints)} 条提示",
    ]
    if unit.inductives:
        meta_bits.insert(1, f"{unit.inductives} 个归纳类型")
    if unit.axioms:
        meta_bits.insert(1, f"{unit.axioms} 个公理")
    parts = [
        f'<section class="unit" id="unit-{unit.index}">',
        '<header class="unit-head">',
        f'<p class="unit-kicker">II.1 · 单元 {unit.index} / {EXPECTED_UNITS}</p>',
        f'<h2><span class="unit-circled">{CIRCLED[unit.index - 1]}</span>'
        f"<span>{render_inline(unit.title.removeprefix('单元' + CIRCLED[unit.index - 1]).strip())}</span></h2>",
        f'<p class="unit-title-en">{escape(unit.title_en)}</p>',
        f'<p class="unit-meta">{" · ".join(meta_bits)} · '
        f'<a href="#appendix-unit-{unit.index}">解答附录</a></p>',
        '<a class="back-to-toc" href="#toc">↑ 目录</a>',
        "</header>",
    ]
    for decl in unit.decls:
        parts.append(render_decl(unit, decl))
    if unit.trailing:
        parts.append(render_items(unit.trailing))
    parts.append("</section>")
    return "\n".join(parts)


def render_toc(book: Book) -> str:
    items: List[str] = []
    for unit in book.units:
        items.append(
            "<li>"
            f'<a href="#unit-{unit.index}">'
            f'<span class="toc-num">{unit.index:02d}</span>'
            f'<span class="toc-title">{render_inline(unit.title)}</span>'
            "</a>"
            f'<span class="toc-meta">{len(unit.exercises)} 道练习 · {len(unit.hints)} 条提示</span>'
            "</li>"
        )
    return (
        '<nav class="toc" id="toc" aria-label="章节目录">'
        "<h2>目录</h2>"
        f'<p class="toc-lede">19 个单元 · {book.exercises} 道练习 · '
        f"{book.hints} 条提示 · 单元内可折叠证明与提示。</p>"
        f'<ol>{"".join(items)}</ol>'
        '<p class="toc-footer"><a href="#appendix-solutions">→ 解答附录</a>'
        " · <a href=\"#top\">回到封面</a></p>"
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
                body = (
                    f"{note}<pre><code>{escape(solution.code)}</code></pre>"
                )
            exercise_rows.append(
                f'<details class="appendix-solution" id="app-sol-{unit.index}-{decl.exercise_no}">'
                f"<summary>练习 {decl.exercise_no} · {escape(decl.name)} "
                f'<span class="sol-meta">{summary_kind}</span></summary>'
                f"{body}</details>"
            )

        extras: List[str] = []
        for decl in unit.extra_solutions:
            extras.append(
                '<details class="appendix-solution">'
                f"<summary>补充 · {escape(decl.name)} "
                f'<span class="sol-meta">{escape(KIND_LABELS.get(decl.kind, decl.kind))}</span></summary>'
                f'<pre><code>{escape(decl.code)}</code></pre></details>'
            )

        meta = f"{len(unit.exercises)} 道练习 · {sum(1 for d in unit.exercises if d.solution)} 份按名匹配的解答"
        sections.append(
            f'<section class="appendix-unit" id="appendix-unit-{unit.index}">'
            f'<h3><a href="#unit-{unit.index}">{render_inline(unit.title)}</a></h3>'
            f'<p class="appendix-meta">{meta}</p>'
            + "".join(exercise_rows)
            + ("".join(extras) if extras else "")
            + "</section>"
        )
    return (
        '<section class="appendix" id="appendix-solutions">'
        "<h2>解答附录</h2>"
        f'<p class="appendix-lede">按单元汇总 {book.solutions} 份练习解答。'
        "正文里的「查看解答」与这里指向同一份解答代码，便于顺序复习与打印。</p>"
        + "".join(sections)
        + "</section>"
    )


def render_cover(book: Book) -> str:
    return (
        '<header class="cover" id="top">'
        '<p class="cover-kicker">sokonanoda · analysis · interactive book prototype</p>'
        f"<h1>{render_inline(book.chapter_title)}</h1>"
        '<p class="cover-en">Chapter II.1 · Integers: from ℕ to ℤ</p>'
        '<p class="cover-lede">把「直觉」与「形式」放在同一页：先读中文讲解与提示，'
        "再展开声明、证明步骤和解答。所有内容来自课程画布与解答文件，"
        "页面本身不判卷、不改课程，也不依赖任何在线资源。</p>"
        '<ul class="cover-stats">'
        f"<li><strong>{len(book.units)}</strong><span>单元</span></li>"
        f"<li><strong>{book.declarations}</strong><span>声明</span></li>"
        f"<li><strong>{book.exercises}</strong><span>练习</span></li>"
        f"<li><strong>{book.hints}</strong><span>提示</span></li>"
        f"<li><strong>{book.solutions}</strong><span>解答</span></li>"
        "</ul>"
        '<p class="cover-note">原型范围：卷 II 第 II.1 章（19 单元）。'
        "数学排版 = Unicode 衬线体 + 原生 MathML（简单分式/上标），无 CDN、无前端数学库。"
        "工具栏可切换「直觉 / 形式 / 并读」；证明、提示、解答均可折叠；"
        "打印按 A4 分页，单元另起一页。</p>"
        "</header>"
    )


def render_toolbar() -> str:
    return (
        '<nav class="toolbar no-print" aria-label="阅读控制">'
        '<span class="toolbar-label">阅读层</span>'
        '<div class="seg" role="group" aria-label="直觉与形式">'
        '<button type="button" data-view="both" aria-pressed="true">并读</button>'
        '<button type="button" data-view="intuition" aria-pressed="false">直觉</button>'
        '<button type="button" data-view="formal" aria-pressed="false">形式</button>'
        "</div>"
        '<button type="button" class="plain" data-action="expand-all">展开全部</button>'
        '<button type="button" class="plain" data-action="collapse-all">折叠全部</button>'
        '<span class="toolbar-spacer"></span>'
        '<a class="plain" href="#toc">目录</a>'
        '<button type="button" class="plain" data-action="print">打印 / PDF</button>'
        "</nav>"
    )


def render_book(book: Book) -> str:
    style = STYLE_CSS.read_text(encoding="utf-8")
    script = APP_JS.read_text(encoding="utf-8")
    units = "\n".join(render_unit(unit) for unit in book.units)
    return (
        "<!DOCTYPE html>\n"
        '<html lang="zh-CN">\n<head>\n<meta charset="utf-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        f"<title>{escape(book.chapter_title)} · 交互式 HTML 书原型</title>\n"
        f"<style>\n{style}\n</style>\n</head>\n<body data-view=\"both\">\n"
        '<a class="skip-link" href="#toc">跳到目录</a>\n'
        f"{render_cover(book)}\n"
        f"{render_toolbar()}\n"
        "<main>\n"
        f"{render_toc(book)}\n"
        f"{units}\n"
        f"{render_appendix(book)}\n"
        "</main>\n"
        '<footer class="book-footer">'
        f"<p>本页由 <code>scripts/build_book_proto.py</code> 从 "
        f"<code>courses/analysis/</code> 的 II.1 画布与解答生成；"
        f"源文件指纹 <code>{book.source_fingerprint}</code>。</p>"
        "<p>静态原型，不替代课程门禁；课程内容、判定与前端代码均未改动。"
        "无外部 http(s) 资源；数学公式使用浏览器原生 MathML。</p>"
        "</footer>\n"
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

    def handle_starttag(self, tag: str, attrs: Sequence[Tuple[str, Optional[str]]]) -> None:
        data = {key: value for key, value in attrs}
        if data.get("id"):
            self.ids.append(str(data["id"]))
        if data.get("href"):
            self.hrefs.append(str(data["href"]))
        if tag == "section" and data.get("class"):
            self.sections.append(str(data["class"]))
        for name in str(data.get("class") or "").split():
            self.classes[name] += 1

    handle_startendtag = handle_starttag


def math_helper_selftest() -> List[str]:
    """The course comments are mostly Unicode only; keep the tiny fraction /
    superscript helper honest with a direct assertion."""
    errors: List[str] = []
    fraction = mathml("(a+b)/(c+d)")
    if fraction is None or "<mfrac>" not in fraction:
        errors.append("MathML helper: simple fraction did not produce <mfrac>")
    power = mathml("x² + y²")
    if power is None or "<msup>" not in power:
        errors.append("MathML helper: Unicode superscript did not produce <msup>")
    ascii_power = mathml("x^2")
    if ascii_power is None or "<msup>" not in ascii_power:
        errors.append("MathML helper: ASCII ^ superscript did not produce <msup>")
    return errors


def validate(book: Book, html_text: str) -> Tuple[List[str], Dict[str, object]]:
    errors: List[str] = []
    stats: Dict[str, object] = {}

    if len(book.units) != EXPECTED_UNITS:
        errors.append(f"expected {EXPECTED_UNITS} units, found {len(book.units)}")
    stats["units"] = len(book.units)
    stats["declarations"] = book.declarations
    stats["exercises"] = book.exercises
    stats["hints"] = book.hints
    stats["solutions"] = book.solutions

    for unit in book.units:
        if not unit.title:
            errors.append(f"unit {unit.index}: empty title")
        if not unit.title_en:
            errors.append(f"unit {unit.index}: empty title_en")
        if not unit.blocks:
            errors.append(f"unit {unit.index}: no blocks")
        if not unit.exercises:
            errors.append(f"unit {unit.index}: no exercises")
        for decl in unit.exercises:
            if decl.solution is None:
                errors.append(f"unit {unit.index}: exercise {decl.name} has no matching solution")
        if render_inline(unit.title) not in html_text:
            errors.append(f"unit {unit.index}: title missing from HTML")
        if unit.title_en and escape(unit.title_en) not in html_text:
            errors.append(f"unit {unit.index}: title_en missing from HTML")

    parser = AuditParser()
    parser.feed(html_text)
    stats["ids"] = len(parser.ids)
    stats["toc_links"] = sum(1 for href in parser.hrefs if href.startswith("#"))

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
    if len([href for href in parser.hrefs if href.startswith("#unit-")]) < EXPECTED_UNITS:
        errors.append("TOC does not link every unit")

    unit_ids = {item for item in ids if re.fullmatch(r"unit-\d+", item)}
    expected_ids = {f"unit-{index}" for index in range(1, EXPECTED_UNITS + 1)}
    if unit_ids != expected_ids:
        errors.append(f"unit section ids mismatch: {sorted(unit_ids)}")

    for unit in book.units:
        match = re.search(
            rf'<section class="unit" id="unit-{unit.index}">(.*?)(?=<section class="unit" id="unit-|\Z)',
            html_text,
            re.S,
        )
        if not match:
            errors.append(f"unit {unit.index}: rendered section not found")
            continue
        segment = match.group(1)
        if not any(
            token in segment
            for token in ('class="block prose', 'class="decl ', 'class="exercise"')
        ):
            errors.append(f"unit {unit.index}: rendered section has no content blocks")

    if re.search(r"https?://", html_text):
        errors.append("generated HTML contains an external http(s) reference")
    if re.search(r"<link\b", html_text, re.I):
        errors.append("generated HTML contains a <link> tag")
    if re.search(r"<script[^>]+\bsrc\s*=", html_text, re.I):
        errors.append("generated HTML contains an external <script src>")

    if "@page" not in html_text or "break-before: page" not in html_text:
        errors.append("print CSS (@page / break-before) missing")
    if 'data-view="both"' not in html_text:
        errors.append("layer toggle default state missing")

    exercise_cards = parser.classes.get("exercise", 0)
    if exercise_cards != book.exercises:
        errors.append(f"exercise cards: expected {book.exercises}, found {exercise_cards}")
    stats["exercise_cards"] = exercise_cards
    stats["external_refs"] = len(re.findall(r"https?://", html_text))
    stats["bytes"] = len(html_text.encode("utf-8"))
    return errors, stats


def output_path(out_dir: Path) -> Path:
    return out_dir / "index.html"


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--out",
        default="dist/book/proto-II.1",
        help="output directory (default: dist/book/proto-II.1)",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="validate the already-generated output (does not write)",
    )
    args = parser.parse_args(argv)

    book = load_book()
    html_text = render_book(book)
    out_dir = Path(args.out)
    if not out_dir.is_absolute():
        out_dir = (ROOT / out_dir).resolve()
    target = output_path(out_dir)

    if args.check:
        if not target.is_file():
            print(f"FAIL: {target} does not exist; run the build first", file=sys.stderr)
            return 1
        written = target.read_text(encoding="utf-8")
        errors, stats = validate(book, written)
        errors.extend(math_helper_selftest())
        if written != html_text:
            errors.append("output is stale; re-run the build")
        if errors:
            print("book prototype check: FAIL", file=sys.stderr)
            for error in errors:
                print(f"  - {error}", file=sys.stderr)
            return 1
        print("book prototype check: PASS")
        print(f"  units: {stats['units']} / {EXPECTED_UNITS}")
        print(
            f"  blocks: {stats['declarations']} declarations · "
            f"{stats['exercises']} exercises · {stats['hints']} hints · "
            f"{stats['solutions']} solutions"
        )
        print(
            f"  toc links: {stats['toc_links']} · ids: {stats['ids']} · "
            f"external refs: {stats['external_refs']}"
        )
        print("  math helper: fraction + superscript self-test OK")
        print(f"  output: {target} ({stats['bytes']:,} bytes)")
        return 0

    out_dir.mkdir(parents=True, exist_ok=True)
    target.write_text(html_text, encoding="utf-8")
    print(f"wrote {target} ({len(html_text.encode('utf-8')):,} bytes)")
    print(
        f"  units: {len(book.units)} · declarations: {book.declarations} · "
        f"exercises: {book.exercises} · hints: {book.hints} · "
        f"solutions: {book.solutions}"
    )
    print(f"  source fingerprint: {book.source_fingerprint}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
