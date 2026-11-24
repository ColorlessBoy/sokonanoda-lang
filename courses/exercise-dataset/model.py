#!/usr/bin/env python3
"""Canvas/solution parser shared by build, check and compose.

This module only knows how a `.sokonanoda` file is shaped: declarations,
leading prose, `soko:hint` steps and the stable source line of each declaration.
Course manifests and dataset assembly live in `course_manifest.py` /
`dataset.py` so every file stays small and testable.

Stdlib only.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

ROOT = Path(__file__).resolve().parents[2]

DECL_KINDS = (
    "def",
    "theorem",
    "lemma",
    "example",
    "axiom",
    "inductive",
    "structure",
    "instance",
)
DECL_RE = re.compile(r"^(" + "|".join(DECL_KINDS) + r")\b(.*)$")
NAME_RE = re.compile(r"\s+([^\s(:{]+)")
IMPORT_RE = re.compile(r"^import\s+\S")
TOP_LEVEL_RE = re.compile(
    r"^(?:#|namespace\b|open\b|section\b|end\b|set_option\b|attribute\b|@\[)"
)
RULE_RE = re.compile(r"^[=─━\-—_*]{4,}$")
EXERCISE_LABEL_RE = re.compile(
    r"^练习\s*(?:\d+|[①-⑳]|[㉑-㉟]|[㊱-㊿])?\s*[（(]([^）)]*)[）)]\s*[：:]\s*(.*)$"
)
NUMBERED_HEADING_RE = re.compile(
    r"^(?:\d+(?:\.\d+)+|单元\s*[①-⑳㉑-㉟㊱-㊿\d]+)\b"
)
SORRY_RE = re.compile(r"\bsorry\b")
PROMPT_MAX_CHARS = 1200
TITLE_MAX_CHARS = 120
HINT_HARD_LIMIT = 12


@dataclass
class Hint:
    label: str
    body: str

    def record(self) -> dict:
        return {"label": self.label, "body": self.body}


@dataclass
class Decl:
    kind: str
    name: Optional[str]
    code: str
    statement: str
    body: str
    start_line: int
    is_exercise: bool
    example_ordinal: int = 0
    leads: list = field(default_factory=list)
    hints: list = field(default_factory=list)


@dataclass
class ParsedFile:
    path: Path
    imports: list
    decls: list
    stray: list


def repo_relative(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def _is_rule(text: str) -> bool:
    return bool(RULE_RE.match(text.strip()))


def _parse_comment_group(raw_lines: list) -> list:
    """Turn consecutive `-- …` lines into ordered prose / hint / rule items."""
    items: list = []
    paragraphs: list = []
    current: Optional[Hint] = None

    def flush() -> None:
        nonlocal paragraphs
        if paragraphs:
            items.append({"kind": "text", "text": "\n".join(paragraphs).strip()})
            paragraphs = []

    for raw in raw_lines:
        content = raw[2:] if raw.startswith("--") else raw
        stripped = content.strip()
        if not stripped:
            flush()
            current = None
            continue
        if stripped.startswith("soko:hint"):
            flush()
            body = stripped[len("soko:hint"):].strip()
            label, rest = "提示", body
            match = re.match(r"^([^：:]{1,16})[：:]\s*(.*)$", body)
            if match:
                label = match.group(1).strip() or "提示"
                rest = match.group(2).strip()
            current = Hint(label=label, body=rest)
            items.append({"kind": "hint", "hint": current})
            continue
        if current is not None and content.startswith(" "):
            current.body = (current.body + "\n" + stripped).strip()
            continue
        if stripped.startswith("soko:"):
            # Non-hint directives (`soko:notation-ok` etc.) are bookkeeping.
            continue
        if _is_rule(stripped):
            flush()
            current = None
            items.append({"kind": "rule"})
            continue
        current = None
        paragraphs.append(content.strip())
    flush()
    return items


def _split_statement(code: str) -> tuple[str, str]:
    index = code.find(":=")
    if index < 0:
        return code.rstrip(), ""
    return code[:index].rstrip(), code[index + 2:].strip("\n")


def strip_comments(code: str) -> str:
    return "\n".join(line.split("--", 1)[0] for line in code.splitlines())


def parse_file(path: Path) -> ParsedFile:
    """Split a canvas/solution file into declarations + comment context."""
    lines = path.read_text(encoding="utf-8").splitlines()
    imports: list = []
    decls: list = []
    stray: list = []
    pending: list = []
    example_ordinal = 0
    index = 0
    total = len(lines)

    while index < total:
        line = lines[index]
        if not line.strip():
            index += 1
            continue
        if line.startswith("--"):
            group = []
            while index < total and lines[index].startswith("--"):
                group.append(lines[index])
                index += 1
            pending.extend(_parse_comment_group(group))
            continue
        if IMPORT_RE.match(line):
            imports.append(line.strip())
            index += 1
            continue
        match = DECL_RE.match(line)
        if not match:
            stray.append(f"{path.name}:{index + 1}: {line.strip()[:120]}")
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
                or (candidate and not candidate[0].isspace() and TOP_LEVEL_RE.match(candidate))
            ):
                break
            index += 1
        code = "\n".join(lines[start:index]).rstrip()
        statement, body = _split_statement(code)
        kind = match.group(1)
        name_match = NAME_RE.match(match.group(2))
        name = name_match.group(1) if name_match else None
        if kind == "example":
            example_ordinal += 1
        hints = [item["hint"] for item in pending if item["kind"] == "hint"]
        decls.append(
            Decl(
                kind=kind,
                name=name,
                code=code,
                statement=statement,
                body=body,
                start_line=start + 1,
                is_exercise=bool(SORRY_RE.search(strip_comments(code))),
                example_ordinal=example_ordinal if kind == "example" else 0,
                leads=[item for item in pending if item["kind"] in ("text", "rule")],
                hints=hints[:HINT_HARD_LIMIT],
            )
        )
        pending = []
    return ParsedFile(path=path, imports=imports, decls=decls, stray=stray)


def _normalise(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def exercise_title(prompt: str, fallback: str) -> str:
    first = prompt.splitlines()[0].strip() if prompt else fallback
    match = EXERCISE_LABEL_RE.match(first)
    if match:
        first = match.group(2).strip()
    if not first:
        first = fallback
    return _normalise(first)[:TITLE_MAX_CHARS]


def exercise_prompt(leads: list, fallback: str) -> str:
    """The prose that introduces this declaration, not the whole unit header."""
    texts = [_normalise(item["text"]) for item in leads if item["kind"] == "text" and item.get("text")]
    texts = [text for text in texts if text]
    if not texts:
        return ""
    # Prefer the nearest exercise label / numbered section heading; the file
    # header and long unit introductions before it are context, not this task.
    start = None
    for i in range(len(texts) - 1, -1, -1):
        if EXERCISE_LABEL_RE.match(texts[i]) or NUMBERED_HEADING_RE.match(texts[i]):
            start = i
            break
    prompt = "\n\n".join(texts[start:]) if start is not None else texts[-1]
    if len(prompt) > PROMPT_MAX_CHARS:
        prompt = prompt[:PROMPT_MAX_CHARS].rsplit("\n\n", 1)[0] or prompt[:PROMPT_MAX_CHARS]
    return prompt
