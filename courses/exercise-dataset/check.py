#!/usr/bin/env python3
"""Validate the committed course exercise dataset.

Usage (repository root):

    python3 courses/exercise-dataset/check.py            # structure + drift + source map
    python3 courses/exercise-dataset/check.py --json      # machine-readable summary
    python3 courses/exercise-dataset/check.py --selftest  # the guard must bite on bad data

The check is deliberately independent of the generator: it re-reads the course
sources, verifies every record's source map, and refuses a solution that still
contains `sorry`.  `--selftest` builds a synthetic course and injects each known
failure mode; a validator that stops rejecting one of them fails the selftest.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import course_manifest  # noqa: E402
import dataset  # noqa: E402
import model  # noqa: E402

DATASET_DIR = HERE
DATA_DIR = HERE / "data"
SECRET_PATTERNS = (
    re.compile(r"/Users/[^/\s\"']+"),
    re.compile(r"/home/[^/\s\"']+"),
    re.compile(r"~/(?:\.dsh|\.ssh|\.config)"),
    re.compile(r"\bsk-[A-Za-z0-9]{12,}"),
    re.compile(r"\bgh[ps]_[A-Za-z0-9]{20,}"),
    re.compile(r"\bgithub_pat_[A-Za-z0-9_]{20,}"),
)
ID_RE = re.compile(r"^[a-z][a-z0-9-]+/(?:[^/]+/){1,2}[A-Za-z0-9_.-]+$")


def _load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _strip_comments(code: str) -> str:
    return "\n".join(line.split("--", 1)[0] for line in code.splitlines())


def _sources_for(course: dict) -> list[Path]:
    paths = []
    for unit in course.get("units", []):
        path = Path(unit["file"])
        if not path.is_absolute():
            path = model.ROOT / path
        paths.append(path)
    for exercise in course.get("exercises", []):
        path = Path(exercise["solution_source"]["file"])
        if not path.is_absolute():
            path = model.ROOT / path
        paths.append(path)
    return paths


def validate(payload: dict) -> list[str]:
    errors: list[str] = []
    manifest = payload.get("manifest") or {}
    courses = payload.get("courses") or {}
    if manifest.get("schema") != "soko.exercise-dataset-manifest/1":
        errors.append("manifest schema is not soko.exercise-dataset-manifest/1")
    if set(courses) != {item["id"] for item in manifest.get("courses", [])}:
        errors.append("manifest course ids do not match dataset files")
    if manifest.get("generated_by") != "courses/exercise-dataset/build.py":
        errors.append("manifest generated_by is not the dataset builder")

    totals = {key: 0 for key in ("units", "units_with_exercises", "exercises", "hints", "solutions", "prompt")}
    seen_ids = set()
    for course_id, course in courses.items():
        if course.get("schema") != "soko.exercise-dataset/1":
            errors.append(f"{course_id}: schema is not soko.exercise-dataset/1")
        if course.get("course") != course_id:
            errors.append(f"{course_id}: course field mismatch")
        counts = course.get("counts") or {}
        exercises = course.get("exercises", [])
        units = course.get("units", [])
        by_unit = {}
        for unit in units:
            if unit["id"] in by_unit:
                errors.append(f"{course_id}: duplicate unit id {unit['id']}")
            by_unit.setdefault(unit["id"], 0)
            for field in ("id", "file", "title", "exercises", "hints"):
                if field not in unit:
                    errors.append(f"{course_id}: unit misses {field}")
            unit_path = model.ROOT / unit["file"]
            if not unit_path.is_file():
                errors.append(f"{course_id}: unit file missing: {unit['file']}")
        if course_id in course_manifest.COURSE_ROOTS:
            expected_units = [unit.id_prefix for unit in course_manifest.iter_units(course_id)]
            if [unit["id"] for unit in units] != expected_units:
                errors.append(f"{course_id}: unit list does not match course.json order")
        actual_units_with_exercises = len(
            {exercise.get("id", "").rsplit("/", 1)[0] for exercise in exercises}
        )
        if counts.get("units_with_exercises") != actual_units_with_exercises:
            errors.append(
                f"{course_id}: counts.units_with_exercises={counts.get('units_with_exercises')!r}, "
                f"expected {actual_units_with_exercises}"
            )
        for exercise in exercises:
            eid = exercise.get("id", "")
            if eid in seen_ids:
                errors.append(f"duplicate exercise id: {eid}")
            seen_ids.add(eid)
            if not ID_RE.match(eid):
                errors.append(f"{eid}: id does not match <course>/[<chapter>/]<unit>/<name>")
            if exercise.get("course") != course_id:
                errors.append(f"{eid}: course mismatch")
            if exercise.get("kind") not in model.DECL_KINDS:
                errors.append(f"{eid}: unknown declaration kind {exercise.get('kind')!r}")
            if not exercise.get("name"):
                errors.append(f"{eid}: empty name")
            template = exercise.get("template", "")
            solution = exercise.get("solution", "")
            if not model.SORRY_RE.search(_strip_comments(template)):
                errors.append(f"{eid}: template has no open sorry")
            if model.SORRY_RE.search(_strip_comments(solution)):
                errors.append(f"{eid}: solution still contains sorry")
            for line in exercise.get("imports", []):
                if not line.startswith("import "):
                    errors.append(f"{eid}: bad import line {line!r}")
            hints = exercise.get("hints", [])
            if hints is None:
                errors.append(f"{eid}: hints must be a list")
            for hint in hints:
                if not hint.get("label") or not hint.get("body"):
                    errors.append(f"{eid}: hint with empty label/body")
                for secret in SECRET_PATTERNS:
                    if secret.search(hint.get("label", "")) or secret.search(hint.get("body", "")):
                        errors.append(f"{eid}: hint contains a machine-local path or secret")
                        break
            prompt = exercise.get("prompt", "")
            if "soko:hint" in prompt:
                errors.append(f"{eid}: raw soko:hint marker leaked into prompt")
            for field in ("template", "solution", "prompt", "title"):
                for secret in SECRET_PATTERNS:
                    if secret.search(exercise.get(field, "")):
                        errors.append(f"{eid}: {field} contains a machine-local path or secret")
                        break
            source = exercise.get("source") or {}
            solution_source = exercise.get("solution_source") or {}
            source_path = model.ROOT / source.get("file", "")
            solution_path = model.ROOT / solution_source.get("file", "")
            if not source_path.is_file():
                errors.append(f"{eid}: source file missing: {source.get('file')}")
                continue
            if not solution_path.is_file():
                errors.append(f"{eid}: solution file missing: {solution_source.get('file')}")
                continue
            source_lines = source_path.read_text(encoding="utf-8").splitlines()
            solution_lines = solution_path.read_text(encoding="utf-8").splitlines()
            line_no = int(source.get("line", 0))
            if not (1 <= line_no <= len(source_lines)):
                errors.append(f"{eid}: source line {line_no} outside {source.get('file')}")
            else:
                line = source_lines[line_no - 1]
                if not re.match(r"^\s*" + re.escape(exercise["kind"]) + r"\b", line):
                    errors.append(f"{eid}: source line does not start with {exercise['kind']}")
                if not exercise.get("anonymous") and exercise["name"] not in line:
                    errors.append(f"{eid}: source line does not name {exercise['name']}")
            sol_line = int(solution_source.get("line", 0))
            if not (1 <= sol_line <= len(solution_lines)):
                errors.append(f"{eid}: solution line {sol_line} outside {solution_source.get('file')}")
            else:
                line = solution_lines[sol_line - 1]
                if not re.match(r"^\s*" + re.escape(exercise["kind"]) + r"\b", line):
                    errors.append(f"{eid}: solution line does not start with {exercise['kind']}")
                if not exercise.get("anonymous") and exercise["name"] not in line:
                    errors.append(f"{eid}: solution line does not name {exercise['name']}")
            if exercise["template"] not in "\n".join(source_lines):
                errors.append(f"{eid}: template text not found in its source file")
            if exercise["solution"] not in "\n".join(solution_lines):
                errors.append(f"{eid}: solution text not found in its solution file")
            unit_id = eid.rsplit("/", 1)[0]
            by_unit[unit_id] = by_unit.get(unit_id, 0) + 1
        for unit_id, count in by_unit.items():
            declared = next((u["exercises"] for u in units if u["id"] == unit_id), None)
            if declared is not None and declared != count:
                errors.append(f"{course_id}: {unit_id} declares {declared} exercises, dataset has {count}")
        for key in totals:
            totals[key] += int(counts.get(key, 0))
        expected = {
            "units": len(units),
            "units_with_exercises": actual_units_with_exercises,
            "exercises": len(exercises),
            "hints": sum(len(item.get("hints", [])) for item in exercises),
            "solutions": sum(1 for item in exercises if item.get("solution")),
            "prompt": sum(1 for item in exercises if item.get("prompt")),
        }
        for key, value in expected.items():
            if counts.get(key) != value:
                errors.append(f"{course_id}: counts.{key}={counts.get(key)!r}, expected {value}")
        try:
            fingerprint = dataset.fingerprint_files(_sources_for(course))
        except OSError as error:
            errors.append(f"{course_id}: cannot fingerprint sources: {error}")
            fingerprint = None
        if fingerprint is not None and course.get("source_fingerprint") != fingerprint:
            errors.append(f"{course_id}: source fingerprint drifted from the files on disk")
        manifest_course = next((c for c in manifest.get("courses", []) if c["id"] == course_id), {})
        if manifest_course.get("source_fingerprint") != course.get("source_fingerprint"):
            errors.append(f"{course_id}: manifest fingerprint differs from course fingerprint")
        for key in ("units", "exercises", "hints", "solutions", "prompt"):
            if manifest_course.get(key) != counts.get(key):
                errors.append(f"{course_id}: manifest {key} differs from course counts")

    if manifest.get("totals") != totals:
        errors.append("manifest totals do not equal the sum of course counts")
    return errors


def check_committed() -> list[str]:
    errors: list[str] = []
    payload = dataset.build_all()
    fresh = dataset.dataset_files(payload)
    for rel, text in fresh.items():
        path = DATASET_DIR / rel
        if not path.is_file():
            errors.append(f"missing committed file: {rel}")
        elif path.read_text(encoding="utf-8") != text:
            errors.append(f"stale committed file: {rel} (run build.py)")
    for path in sorted(DATA_DIR.glob("*.json")):
        rel = path.relative_to(DATASET_DIR).as_posix()
        if rel not in fresh:
            errors.append(f"extra data file not produced by the builder: {rel}")
    errors.extend(validate(payload))
    return errors


# ── selftest: every failure mode below must be rejected ────────────────────


FIXTURE_UNIT = """\
-- 单元① 测试单元

import lib.Logic

-- 练习 1（T 类）：True 成立。
-- soko:hint 思路：直接用构造子。
-- soko:hint 目标形态：True。
-- soko:hint 关键件：True.intro。
theorem test_true : True := by
  sorry

example : True := by
  sorry
"""

FIXTURE_SOLUTION = """\
import lib.Logic

theorem test_true : True := by
  exact True.intro

example : True := by
  exact True.intro
"""


def _write_fixture(root: Path) -> None:
    course = root / "fixture"
    unit = course / "units" / "F.1"
    solution = course / "units" / "solutions" / "F.1"
    unit.mkdir(parents=True)
    solution.mkdir(parents=True)
    (course / "course.json").write_text(
        json.dumps(
            {
                "schema": "soko.course/2",
                "name": "fixture",
                "title": "夹具课",
                "volumes": [
                    {
                        "id": "F",
                        "title": "卷 F",
                        "chapters": [
                            {
                                "id": "F.1",
                                "title": "夹具章",
                                "units": [
                                    {
                                        "file": "units/F.1/unit01.sokonanoda",
                                        "title": "单元① 测试单元",
                                        "unit": 1,
                                    }
                                ],
                            }
                        ],
                    }
                ],
            },
            ensure_ascii=False,
        ),
        encoding="utf-8",
    )
    (unit / "unit01.sokonanoda").write_text(FIXTURE_UNIT, encoding="utf-8")
    (solution / "unit01-solution.sokonanoda").write_text(FIXTURE_SOLUTION, encoding="utf-8")


def selftest() -> int:
    """The validator must reject every injected failure; report each verdict."""
    saved_roots = dict(course_manifest.COURSE_ROOTS)
    saved_order = course_manifest.COURSE_ORDER
    failures = 0
    try:
        with tempfile.TemporaryDirectory(prefix=".selftest-", dir=str(HERE)) as tmp:
            root = Path(tmp)
            _write_fixture(root)
            course_manifest.COURSE_ROOTS = {"fixture": root / "fixture"}
            course_manifest.COURSE_ORDER = ("fixture",)
            payload = dataset.build_all()
            baseline = validate(payload)
            if baseline:
                failures += 1
                print("selftest: fixture baseline should be valid, got:", *baseline, sep="\n  ")
            course = payload["courses"]["fixture"]
            if course["counts"]["exercises"] != 2:
                failures += 1
                print("selftest: fixture should contain 2 exercises")
            if course["exercises"][1]["name"] != "example-1":
                failures += 1
                print("selftest: anonymous example should get a stable example-N name")

            cases = []
            duplicate = json.loads(json.dumps(payload))
            duplicate["courses"]["fixture"]["exercises"][1]["id"] = duplicate["courses"]["fixture"]["exercises"][0]["id"]
            cases.append(("duplicate id", duplicate))

            open_solution = json.loads(json.dumps(payload))
            open_solution["courses"]["fixture"]["exercises"][0]["solution"] = "theorem test_true : True := by\n  sorry"
            cases.append(("solution with sorry", open_solution))

            leaked_hint = json.loads(json.dumps(payload))
            leaked_hint["courses"]["fixture"]["exercises"][0]["prompt"] += "\n-- soko:hint 关键件：x"
            cases.append(("raw hint marker in prompt", leaked_hint))

            bad_source = json.loads(json.dumps(payload))
            bad_source["courses"]["fixture"]["exercises"][0]["template"] += "\n-- edited"
            cases.append(("template not found in source", bad_source))

            bad_line = json.loads(json.dumps(payload))
            bad_line["courses"]["fixture"]["exercises"][0]["source"]["line"] = 999
            cases.append(("source line outside file", bad_line))

            bad_count = json.loads(json.dumps(payload))
            bad_count["courses"]["fixture"]["counts"]["exercises"] = 99
            cases.append(("counts drift", bad_count))

            for label, case in cases:
                if not validate(case):
                    failures += 1
                    print(f"selftest: validator accepted bad input: {label}")

            # A changed source file must break the fingerprint / source map.
            unit_path = root / "fixture" / "units" / "F.1" / "unit01.sokonanoda"
            unit_path.write_text(FIXTURE_UNIT.replace("True.intro。", "True.intro。\n", 1), encoding="utf-8")
            if not validate(payload):
                failures += 1
                print("selftest: validator accepted a stale source fingerprint")
            for rel, text in dataset.dataset_files(payload).items():
                (root / rel).parent.mkdir(parents=True, exist_ok=True)
                (root / rel).write_text(text, encoding="utf-8")
            fresh = dataset.dataset_files(dataset.build_all())
            if not any("stale committed" in item for item in _freshness_errors(root, fresh)):
                failures += 1
                print("selftest: freshness guard accepted a changed source file")
    finally:
        course_manifest.COURSE_ROOTS = saved_roots
        course_manifest.COURSE_ORDER = saved_order

    if failures:
        print(f"exercise-dataset selftest: FAIL ({failures})", file=sys.stderr)
        return 1
    print("exercise-dataset selftest: PASS (6 injected failures rejected + fingerprint/freshness bite)")
    return 0


def _freshness_errors(out_dir: Path, fresh: dict) -> list[str]:
    errors = []
    for rel, text in fresh.items():
        path = out_dir / rel
        if not path.is_file() or path.read_text(encoding="utf-8") != text:
            errors.append(f"stale committed file: {rel}")
    return errors


def main(argv: list = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true", help="inject bad data and require rejection")
    parser.add_argument("--json", action="store_true", help="machine-readable summary")
    args = parser.parse_args(argv)
    if args.selftest:
        return selftest()

    errors = check_committed()
    payload = dataset.build_all()
    if args.json:
        print(json.dumps({"ok": not errors, "counts": payload["manifest"]["totals"], "errors": errors}, ensure_ascii=False, indent=2))
    else:
        if errors:
            print("exercise dataset: FAIL", file=sys.stderr)
            for error in errors:
                print(f"  - {error}", file=sys.stderr)
        else:
            total = payload["manifest"]["totals"]
            print(
                "exercise dataset: OK · "
                f"{total['exercises']} exercises · {total['units']} units · "
                f"{total['solutions']} solutions · {total['hints']} hints"
            )
            for course in payload["manifest"]["courses"]:
                print(
                    f"  {course['id']}: {course['exercises']}/{course['units']} units · "
                    f"{course['hints']} hints · source {course['source_fingerprint'][:19]}…"
                )
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
