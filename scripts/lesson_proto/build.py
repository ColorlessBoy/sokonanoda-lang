#!/usr/bin/env python3
"""Build the unit ⑧ Duolingo-style lesson prototype.

The prototype is intentionally a single, self-contained HTML file: CSS and JS are
inlined, the lesson content is embedded as JSON, and there are no network
requests.  Python standard library only.

Usage:
    python3 scripts/lesson_proto/build.py --out dist/lesson/unit8
    python3 scripts/lesson_proto/build.py --check --out dist/lesson/unit8
"""

from __future__ import annotations

import argparse
import html
import json
import re
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
DEFAULT_DATA = HERE / "unit8.json"
TEMPLATE = HERE / "template.html"
STYLE = HERE / "style.css"
APP = HERE / "app.js"
DEFAULT_OUT = Path("dist/lesson/unit8")

# The five-step flow is part of the product contract, not just presentation.
EXPECTED_STEPS = [
    ("goal", "goal"),
    ("opposite-of-3", "choice"),
    ("pair-swap", "choice"),
    ("pair-sum", "choice"),
    ("conclusion", "choice"),
]
EXPECTED_ANSWERS = {
    "opposite-of-3": "minus-three",
    "pair-swap": "pair-zero-three",
    "pair-sum": "zero",
    "conclusion": "inverse",
}
EXPECTED_LABEL_FRAGMENTS = {
    "opposite-of-3": "-3",
    "pair-swap": "(0,3)",
    "pair-sum": "0",
    "conclusion": "x+(-x)=0",
}
EXPECTED_VISUALS = {
    "opposite-of-3": "numberline_quiz",
    "pair-swap": "pair_swap",
    "pair-sum": "pair_sum",
    "conclusion": "conclusion",
}
REQUIRED_SYMBOLS = {"opposite", "pair", "integer", "inverse", "zero"}
FORBIDDEN_MAIN_PATH = ("sorry", ":=", "import ", "Quot.sound", "Lean")
EXTERNAL_URL = re.compile(r"https?://", re.IGNORECASE)
PROTOCOL_RELATIVE = re.compile(
    r"""(?:\b(?:src|href)\s*=\s*["']\s*//)|(?:url\(\s*["']?\s*//)""",
    re.IGNORECASE,
)


class LessonError(Exception):
    """A lesson-data or build-output error that should fail the command."""


def fail(message: str) -> None:
    raise LessonError(message)


def require(condition: Any, message: str) -> None:
    if not condition:
        fail(message)


def require_str(mapping: dict[str, Any], key: str, where: str) -> str:
    value = mapping.get(key)
    require(
        isinstance(value, str) and value.strip(),
        f"{where}.{key} must be a non-empty string",
    )
    return value.strip()


def require_int(
    mapping: dict[str, Any],
    key: str,
    where: str,
    *,
    minimum: int = 0,
    maximum: int | None = None,
) -> int:
    value = mapping.get(key)
    require(
        isinstance(value, int) and not isinstance(value, bool),
        f"{where}.{key} must be an integer",
    )
    require(value >= minimum, f"{where}.{key} must be >= {minimum}")
    if maximum is not None:
        require(value <= maximum, f"{where}.{key} must be <= {maximum}")
    return value


def iter_strings(value: Any, trail: str = "") -> list[tuple[str, str]]:
    """Return every string value in a JSON tree, with its path."""
    found: list[tuple[str, str]] = []
    if isinstance(value, str):
        found.append((trail or "$", value))
    elif isinstance(value, dict):
        for key, child in value.items():
            found.extend(iter_strings(child, f"{trail}.{key}" if trail else key))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            found.extend(iter_strings(child, f"{trail}[{index}]"))
    return found


def scan_no_external_urls(value: Any, where: str) -> None:
    for path, text in iter_strings(value):
        require(
            EXTERNAL_URL.search(text) is None,
            f"{where}: external URL found in {path}",
        )
        require(
            PROTOCOL_RELATIVE.search(text) is None,
            f"{where}: protocol-relative URL found in {path}",
        )


def validate_symbols(data: dict[str, Any]) -> None:
    symbols = data.get("symbols")
    require(isinstance(symbols, dict), "symbols must be an object")
    for symbol_id in REQUIRED_SYMBOLS:
        require(symbol_id in symbols, f"symbols.{symbol_id} is required")
    for symbol_id, symbol in symbols.items():
        where = f"symbols.{symbol_id}"
        require(isinstance(symbol, dict), f"{where} must be an object")
        require_str(symbol, "term", where)
        require_str(symbol, "reading", where)
        require_str(symbol, "plain", where)
        require_str(symbol, "example", where)
        require_str(symbol, "help", where)
        seconds = require_int(symbol, "seconds", where, minimum=1, maximum=60)
        require(seconds == 20, f"{where}.seconds must be 20 for this prototype")


def validate_help(step: dict[str, Any], where: str, symbol_ids: set[str]) -> None:
    help_ids = step.get("help")
    require(
        isinstance(help_ids, list) and help_ids,
        f"{where}.help must be a non-empty list of symbol ids",
    )
    for symbol_id in help_ids:
        require(
            isinstance(symbol_id, str) and symbol_id in symbol_ids,
            f"{where}.help references unknown symbol {symbol_id!r}",
        )


def validate_goal(step: dict[str, Any], where: str, symbol_ids: set[str]) -> None:
    require_str(step, "body", where)
    require_str(step, "primary", where)
    require(
        "x + (-x) = 0" in step["formula"],
        f"{where}.formula must contain the course goal x + (-x) = 0",
    )
    require("整数" in step.get("title", ""), f"{where}.title must state the math goal")
    visual = step.get("visual")
    require(isinstance(visual, dict), f"{where}.visual must be an object")
    require(
        visual.get("type") == "numberline",
        f"{where}.visual.type must be numberline",
    )
    require_str(visual, "caption", f"{where}.visual")
    require_str(visual, "default_note", f"{where}.visual")
    markers = visual.get("markers")
    require(
        isinstance(markers, list) and len(markers) == 2,
        f"{where}.visual.markers must contain exactly the two symmetric markers",
    )
    require(
        all(isinstance(marker, dict) for marker in markers),
        f"{where}.visual.markers must contain objects",
    )
    values = sorted(marker.get("value") for marker in markers)
    require(values == [-3, 3], f"{where}.visual.markers must be at -3 and 3")
    for index, marker in enumerate(markers):
        mwhere = f"{where}.visual.markers[{index}]"
        require(isinstance(marker, dict), f"{mwhere} must be an object")
        require_str(marker, "label", mwhere)
        require_str(marker, "tone", mwhere)
        require_str(marker, "note", mwhere)
    validate_help(step, where, symbol_ids)
    require_int(step, "xp", where, minimum=1)
    require_int(step, "first_try_bonus", where, minimum=0)
    require(
        step.get("first_try_bonus") == 0,
        f"{where}.first_try_bonus must be 0 for the ungraded goal card",
    )


def validate_choice(step: dict[str, Any], where: str, symbol_ids: set[str]) -> None:
    require_str(step, "prompt", where)
    require_str(step, "continue_label", where)
    validate_help(step, where, symbol_ids)
    require_int(step, "xp", where, minimum=1)
    require_int(step, "first_try_bonus", where, minimum=1)

    visual = step.get("visual")
    require(isinstance(visual, dict), f"{where}.visual must be an object")
    expected_visual = EXPECTED_VISUALS[step["id"]]
    require(
        visual.get("type") == expected_visual,
        f"{where}.visual.type must be {expected_visual}",
    )

    choices = step.get("choices")
    require(isinstance(choices, list), f"{where}.choices must be a list")
    require(len(choices) == 3, f"{where}.choices must contain exactly 3 choices")
    choice_ids: list[str] = []
    for index, choice in enumerate(choices):
        cwhere = f"{where}.choices[{index}]"
        require(isinstance(choice, dict), f"{cwhere} must be an object")
        choice_id = require_str(choice, "id", cwhere)
        require(choice_id not in choice_ids, f"{where} has duplicate choice id {choice_id!r}")
        choice_ids.append(choice_id)
        require_str(choice, "label", cwhere)
        require_str(choice, "aria", cwhere)
        if "sub" in choice:
            require_str(choice, "sub", cwhere)

    expected_answer = EXPECTED_ANSWERS[step["id"]]
    answer = step.get("answer")
    require(
        answer == expected_answer,
        f"{where}.answer must be {expected_answer!r}, got {answer!r}",
    )
    require(answer in choice_ids, f"{where}.answer is not one of the choices")

    correct_choices = [choice for choice in choices if choice["id"] == answer]
    require(len(correct_choices) == 1, f"{where} must have exactly one correct choice")
    label = correct_choices[0]["label"].replace("−", "-").replace(" ", "")
    require(
        EXPECTED_LABEL_FRAGMENTS[step["id"]] in label,
        f"{where} correct choice label does not match the expected math",
    )

    feedback = step.get("feedback")
    require(isinstance(feedback, dict), f"{where}.feedback must be an object")
    correct_feedback = feedback.get("correct")
    require(isinstance(correct_feedback, dict), f"{where}.feedback.correct must be an object")
    require_str(correct_feedback, "title", f"{where}.feedback.correct")
    require_str(correct_feedback, "body", f"{where}.feedback.correct")

    wrong_feedback = feedback.get("wrong")
    require(isinstance(wrong_feedback, dict), f"{where}.feedback.wrong must be an object")
    wrong_ids = set(wrong_feedback)
    expected_wrong_ids = set(choice_ids) - {answer}
    require(
        wrong_ids == expected_wrong_ids,
        f"{where}.feedback.wrong must cover exactly the non-answer choices "
        f"(missing {sorted(expected_wrong_ids - wrong_ids)}, extra {sorted(wrong_ids - expected_wrong_ids)})",
    )
    for choice_id, message in wrong_feedback.items():
        require(
            isinstance(message, str) and message.strip(),
            f"{where}.feedback.wrong.{choice_id} must be a non-empty string",
        )

    hints = step.get("hints")
    require(isinstance(hints, list), f"{where}.hints must be a list")
    require(len(hints) >= 2, f"{where}.hints must have at least 2 tiers")
    for index, hint in enumerate(hints):
        hwhere = f"{where}.hints[{index}]"
        require(isinstance(hint, dict), f"{hwhere} must be an object")
        require_str(hint, "title", hwhere)
        require_str(hint, "body", hwhere)


def validate_formal_proof(data: dict[str, Any]) -> None:
    proof = data.get("formal_proof")
    require(isinstance(proof, dict), "formal_proof must be an object")
    require_str(proof, "summary", "formal_proof")
    require_str(proof, "theorem_lead", "formal_proof")
    require_str(proof, "note", "formal_proof")
    evidence = require_str(proof, "evidence", "formal_proof")
    require("int_add_neg" in evidence, "formal_proof.evidence must point to int_add_neg")

    theorems = proof.get("theorems")
    require(isinstance(theorems, list), "formal_proof.theorems must be a list")
    require(len(theorems) >= 5, "formal_proof.theorems must contain at least 5 entries")
    names: list[str] = []
    for index, theorem in enumerate(theorems):
        where = f"formal_proof.theorems[{index}]"
        require(isinstance(theorem, dict), f"{where} must be an object")
        name = require_str(theorem, "name", where)
        require(name not in names, f"formal_proof.theorems has duplicate name {name!r}")
        names.append(name)
        require_str(theorem, "kind", where)
        require_str(theorem, "plain", where)
    require("int_add_neg" in names, "formal_proof.theorems must include int_add_neg")

    serialized = json.dumps(proof, ensure_ascii=False)
    for forbidden in ("sorry", ":=", "import "):
        require(
            forbidden not in serialized,
            f"formal_proof must not embed full proof source (found {forbidden!r})",
        )

    # The theorem names are the contract with the real unit.  When the course
    # checkout is present, verify every hidden name still exists there.
    repo_root = HERE.parents[1]
    source = data.get("source", {})
    unit_path = repo_root / str(source.get("unit_file", ""))
    solution_path = repo_root / str(source.get("solution_file", ""))
    require(unit_path.is_file(), f"source.unit_file does not exist: {unit_path}")
    require(solution_path.is_file(), f"source.solution_file does not exist: {solution_path}")
    source_text = unit_path.read_text(encoding="utf-8")
    for name in names:
        require(
            re.search(rf"\b(?:theorem|def)\s+{re.escape(name)}\b", source_text)
            is not None,
            f"formal_proof theorem {name!r} is not declared in {unit_path}",
        )


def validate_lesson(data: Any) -> dict[str, int]:
    """Validate the lesson content model and return a few summary counts."""
    require(isinstance(data, dict), "lesson data must be a JSON object")
    require(data.get("schema") == "soko.lesson/1", "schema must be soko.lesson/1")
    for key in (
        "id",
        "title",
        "title_en",
        "course_title",
        "course_title_en",
        "unit_title",
        "unit_title_en",
    ):
        require_str(data, key, "lesson")
    require_int(data, "unit", "lesson", minimum=1)
    require_int(data, "estimated_minutes", "lesson", minimum=1, maximum=15)
    require_int(data, "xp_total", "lesson", minimum=1)

    source = data.get("source")
    require(isinstance(source, dict), "lesson.source must be an object")
    require_str(source, "unit_file", "lesson.source")
    require_str(source, "solution_file", "lesson.source")

    badge = data.get("badge")
    require(isinstance(badge, dict), "lesson.badge must be an object")
    require_str(badge, "icon", "lesson.badge")
    badge_title = require_str(badge, "title", "lesson.badge")
    require(
        badge_title == "整数加法有逆元",
        "lesson.badge.title must be the unit ⑧ competency badge",
    )
    require_str(badge, "subtitle", "lesson.badge")
    require_str(badge, "message", "lesson.badge")

    validate_symbols(data)
    symbol_ids = set(data["symbols"])

    steps = data.get("steps")
    require(isinstance(steps, list), "lesson.steps must be a list")
    require(
        len(steps) == len(EXPECTED_STEPS),
        f"lesson.steps must contain exactly {len(EXPECTED_STEPS)} steps, got {len(steps)}",
    )
    for index, (expected_id, expected_type) in enumerate(EXPECTED_STEPS):
        step = steps[index]
        where = f"lesson.steps[{index}]"
        require(isinstance(step, dict), f"{where} must be an object")
        require(
            step.get("id") == expected_id,
            f"{where}.id must be {expected_id!r}, got {step.get('id')!r}",
        )
        require(
            step.get("type") == expected_type,
            f"{where}.type must be {expected_type!r}, got {step.get('type')!r}",
        )
        require_str(step, "short", where)
        require_str(step, "kicker", where)
        require_str(step, "title", where)
        require_str(step, "formula", where)
        if expected_type == "goal":
            validate_goal(step, where, symbol_ids)
        else:
            validate_choice(step, where, symbol_ids)

    completion = data.get("completion")
    require(isinstance(completion, dict), "lesson.completion must be an object")
    require_int(completion, "xp", "lesson.completion", minimum=1)
    for key in ("eyebrow", "title", "body", "replay", "next"):
        require_str(completion, key, "lesson.completion")
    require(
        completion["title"] == badge_title,
        "lesson.completion.title must match lesson.badge.title",
    )

    computed_xp_total = (
        sum(step["xp"] for step in steps)
        + sum(step["first_try_bonus"] for step in steps)
        + completion["xp"]
    )
    require(
        data["xp_total"] == computed_xp_total,
        f"lesson.xp_total must equal {computed_xp_total} (steps + bonuses + completion)",
    )

    validate_formal_proof(data)

    # The main learning path must never expose proof source or Lean syntax.
    main_path = {"steps": steps, "completion": completion, "symbols": data["symbols"]}
    for path, text in iter_strings(main_path):
        for forbidden in FORBIDDEN_MAIN_PATH:
            require(
                forbidden not in text,
                f"main path leaks formal source/tooling in {path} ({forbidden!r})",
            )

    scan_no_external_urls(data, "lesson data")

    return {
        "steps": len(steps),
        "graded": sum(1 for step in steps if step["type"] == "choice"),
        "symbols": len(data["symbols"]),
        "theorems": len(data["formal_proof"]["theorems"]),
        "xp": data["xp_total"],
        "hints": sum(len(step.get("hints", [])) for step in steps),
    }


def read_asset(path: Path, label: str) -> str:
    require(path.is_file(), f"{label} not found: {path}")
    text = path.read_text(encoding="utf-8")
    require(text.strip(), f"{label} is empty: {path}")
    return text


def inline_assets() -> dict[str, str]:
    template = read_asset(TEMPLATE, "template")
    style = read_asset(STYLE, "style")
    app = read_asset(APP, "app")
    require("</style" not in style.lower(), "style.css must not contain </style")
    require("</script" not in app.lower(), "app.js must not contain </script")
    for placeholder in (
        "{{TITLE}}",
        "{{DESCRIPTION}}",
        "{{LESSON_ID}}",
        "{{UNIT_LABEL}}",
        "{{STYLE_CSS}}",
        "{{APP_JS}}",
        "{{LESSON_JSON}}",
    ):
        require(placeholder in template, f"template is missing {placeholder}")
    return {"template": template, "style": style, "app": app}


def render_lesson(data: dict[str, Any], assets: dict[str, str]) -> str:
    title = f'{data["title"]} · 单元⑧ | 数学小课'
    description = (
        "Duolingo 式数学小课原型：5 步互动，从 3 的相反数走到整数加法逆元。"
        " 单文件、离线可开，形式证明默认折叠。"
    )
    unit_label = f'{data["course_title"]} · Unit ⑧'
    lesson_json = json.dumps(data, ensure_ascii=False, separators=(",", ":"))
    # Keep the embedded JSON inert inside a <script> element.
    lesson_json = lesson_json.replace("<", "\\u003c").replace(">", "\\u003e")

    rendered = assets["template"]
    replacements = {
        "{{TITLE}}": html.escape(title, quote=True),
        "{{DESCRIPTION}}": html.escape(description, quote=True),
        "{{LESSON_ID}}": html.escape(data["id"], quote=True),
        "{{UNIT_LABEL}}": html.escape(unit_label, quote=True),
        "{{STYLE_CSS}}": assets["style"],
        "{{APP_JS}}": assets["app"],
        "{{LESSON_JSON}}": lesson_json,
    }
    for placeholder, value in replacements.items():
        rendered = rendered.replace(placeholder, value)
    require("{{" not in rendered, "template still contains an unresolved placeholder")
    return rendered


def validate_output(output: Path, data: dict[str, Any]) -> None:
    require(output.is_file(), f"generated output not found: {output}")
    text = output.read_text(encoding="utf-8")
    validate_output_text(text, data)


def build(data_path: Path, out_dir: Path, *, check_only: bool) -> int:
    try:
        require(data_path.is_file(), f"lesson data not found: {data_path}")
        data = json.loads(data_path.read_text(encoding="utf-8"))
        counts = validate_lesson(data)
        assets = inline_assets()
        output = out_dir / "index.html"

        if check_only:
            print(
                "check: lesson data OK "
                f"({counts['steps']} steps, {counts['graded']} graded, "
                f"xp {counts['xp']}, {counts['symbols']} symbol cards, "
                f"{counts['theorems']} hidden theorem notes)"
            )
            if output.is_file():
                validate_output(output, data)
                size_kb = output.stat().st_size / 1024
                print(
                    f"check: generated output OK ({output}, {size_kb:.1f} KiB, "
                    "5 steps embedded, 0 external asset references)"
                )
            else:
                print(
                    f"check: output not built at {output}; source-only check "
                    "(run without --check to build)"
                )
            return 0

        rendered = render_lesson(data, assets)
        validate_output_text(rendered, data)
        out_dir.mkdir(parents=True, exist_ok=True)
        tmp = out_dir / ".index.html.tmp"
        try:
            tmp.write_text(rendered, encoding="utf-8")
            tmp.replace(output)
        finally:
            if tmp.exists():
                tmp.unlink()
        size_kb = output.stat().st_size / 1024
        print(
            f"built {output} ({size_kb:.1f} KiB) · "
            f"{counts['steps']} steps · xp {counts['xp']} · "
            f"{counts['hints']} hint tiers · 0 external asset references"
        )
        return 0
    except LessonError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1


def validate_output_text(rendered: str, data: dict[str, Any]) -> None:
    """Validate a rendered HTML document, whether on disk or still in memory."""
    require(rendered.strip(), "generated output is empty")
    require(EXTERNAL_URL.search(rendered) is None, "generated HTML references an external URL")
    require(
        PROTOCOL_RELATIVE.search(rendered) is None,
        "generated HTML references a protocol-relative URL",
    )
    require("{{" not in rendered, "generated HTML still contains a template placeholder")
    require('id="lesson-data"' in rendered, "generated HTML is missing the embedded lesson JSON")
    require("window.SokoLesson" in rendered, "generated HTML is missing the host integration hook")
    require('lang="zh-CN"' in rendered, "generated HTML is missing the zh-CN locale")
    require(
        'name="viewport"' in rendered and "width=device-width" in rendered,
        "generated HTML is missing a mobile viewport",
    )

    for attr, value in re.findall(r'\b(src|href)\s*=\s*"([^"]*)"', rendered, re.IGNORECASE):
        require(
            value.startswith("#") or value.startswith("data:"),
            f"generated HTML has a non-inline {attr}: {value!r}",
        )

    match = re.search(
        r'<script\s+id="lesson-data"\s+type="application/json">(.*?)</script>',
        rendered,
        re.DOTALL,
    )
    require(match is not None, "could not extract the embedded lesson JSON")
    embedded_raw = match.group(1)
    try:
        embedded = json.loads(embedded_raw)
    except json.JSONDecodeError as exc:
        fail(f"embedded lesson JSON does not parse: {exc}")
    require(embedded == data, "embedded lesson JSON is stale; re-run the build")
    require(
        len(embedded.get("steps", [])) == 5,
        "generated HTML must embed exactly 5 lesson steps",
    )
    for step in data["steps"]:
        require(
            step["id"] in embedded_raw,
            f"generated HTML is missing step id {step['id']!r}",
        )
        require(
            step["title"] in rendered,
            f"generated HTML is missing step title {step['title']!r}",
        )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--out",
        default=str(DEFAULT_OUT),
        help=f"output directory (default: {DEFAULT_OUT})",
    )
    parser.add_argument(
        "--data",
        default=str(DEFAULT_DATA),
        help=f"lesson JSON path (default: {DEFAULT_DATA})",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="validate the lesson and, if present, the generated HTML; write nothing",
    )
    args = parser.parse_args(argv)
    return build(Path(args.data).resolve(), Path(args.out).resolve(), check_only=args.check)


if __name__ == "__main__":
    raise SystemExit(main())
