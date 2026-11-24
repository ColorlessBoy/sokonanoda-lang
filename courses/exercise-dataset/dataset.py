#!/usr/bin/env python3
"""Assemble the derived exercise dataset from course manifests + canvases.

`build.py` is the only writer; `check.py` re-runs this module against the
committed files. Records carry the exercise declaration, its solution key, the
prompt/hints and a source map; the unit-level prefix is not duplicated here
(`compose.py` reads it from the course source when a runnable file is needed).

Stdlib only.
"""
from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from typing import Optional

import course_manifest
import model


@dataclass
class Exercise:
    unit: course_manifest.UnitEntry
    kind: str
    name: str
    anonymous: bool
    exercise_no: int
    title: str
    prompt: str
    hints: list
    imports: list
    template: str
    solution: str
    source_line: int
    solution_line: int
    solution_file: str

    @property
    def id(self) -> str:
        return f"{self.unit.id_prefix}/{self.name}"

    def source(self) -> dict:
        return {"file": model.repo_relative(self.unit.path), "line": self.source_line}

    def solution_source(self) -> dict:
        path = self.unit.course_dir / self.solution_file
        return {"file": model.repo_relative(path), "line": self.solution_line}

    def record(self) -> dict:
        return {
            "id": self.id,
            "course": self.unit.course,
            "chapter": self.unit.chapter_id,
            "chapter_title": self.unit.chapter_title,
            "unit": self.unit.title,
            "unit_index": self.unit.unit_index,
            "kind": self.kind,
            "name": self.name,
            "anonymous": self.anonymous,
            "exercise_no": self.exercise_no,
            "title": self.title,
            "prompt": self.prompt,
            "hints": [hint.record() for hint in self.hints],
            "imports": self.imports,
            "template": self.template,
            "solution": self.solution,
            "source": self.source(),
            "solution_source": self.solution_source(),
        }


@dataclass
class CourseDataset:
    course: str
    title: str
    units: list
    exercises: list
    source_fingerprint: str

    def counts(self) -> dict:
        return {
            "units": len(self.units),
            "units_with_exercises": len({item.unit.id_prefix for item in self.exercises}),
            "exercises": len(self.exercises),
            "hints": sum(len(item.hints) for item in self.exercises),
            "solutions": sum(1 for item in self.exercises if item.solution),
            "prompt": sum(1 for item in self.exercises if item.prompt),
        }

    def payload(self) -> dict:
        units = []
        for unit in self.units:
            exercises = [item for item in self.exercises if item.unit is unit]
            units.append(
                {
                    "id": unit.id_prefix,
                    "chapter": unit.chapter_id,
                    "chapter_title": unit.chapter_title,
                    "unit": unit.unit_index,
                    "title": unit.title,
                    "title_en": unit.title_en,
                    "file": model.repo_relative(unit.path),
                    "exercises": len(exercises),
                    "hints": sum(len(item.hints) for item in exercises),
                }
            )
        return {
            "schema": "soko.exercise-dataset/1",
            "course": self.course,
            "title": self.title,
            "generated_by": "courses/exercise-dataset/build.py",
            "source_fingerprint": self.source_fingerprint,
            "counts": self.counts(),
            "units": units,
            "exercises": [item.record() for item in self.exercises],
        }


def _decl_name(decl: model.Decl) -> str:
    if decl.name:
        return decl.name
    if decl.kind == "example":
        return f"example-{decl.example_ordinal}"
    return f"anonymous-{decl.start_line}"


def _match_solution(decl: model.Decl, solutions: dict, examples: list) -> Optional[model.Decl]:
    if decl.kind == "example":
        ordinal = decl.example_ordinal
        return examples[ordinal - 1] if 0 < ordinal <= len(examples) else None
    return solutions.get(decl.name or "")


def fingerprint_files(paths: list) -> str:
    digest = hashlib.sha256()
    for path in sorted({p.resolve() for p in paths}):
        rel = model.repo_relative(path)
        digest.update(rel.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return "sha256:" + digest.hexdigest()


def collect_course(course: str) -> CourseDataset:
    units = list(course_manifest.iter_units(course))
    source_paths = [unit.path for unit in units]
    exercises: list = []
    unknown_solutions: list = []
    for unit in units:
        canvas = model.parse_file(unit.path)
        solution_path = course_manifest.find_solution_path(unit)
        solution_decls: list = []
        if solution_path is not None:
            source_paths.append(solution_path)
            solution_decls = model.parse_file(solution_path).decls
        by_name = {decl.name: decl for decl in solution_decls if decl.name}
        examples = [decl for decl in solution_decls if decl.kind == "example"]
        exercise_no = 0
        for decl in canvas.decls:
            if not decl.is_exercise:
                continue
            exercise_no += 1
            solution = _match_solution(decl, by_name, examples)
            if solution is None:
                unknown_solutions.append(f"{unit.unit_file}:{decl.start_line} {_decl_name(decl)}")
                continue
            if model.SORRY_RE.search(model.strip_comments(solution.code)):
                raise SystemExit(
                    f"solution still contains sorry: {unit.solution_file} {_decl_name(solution)}"
                )
            prompt = model.exercise_prompt(decl.leads, unit.title)
            exercises.append(
                Exercise(
                    unit=unit,
                    kind=decl.kind,
                    name=_decl_name(decl),
                    anonymous=decl.name is None,
                    exercise_no=exercise_no,
                    title=model.exercise_title(prompt, _decl_name(decl)),
                    prompt=prompt,
                    hints=decl.hints,
                    imports=canvas.imports,
                    template=decl.code,
                    solution=solution.code,
                    source_line=decl.start_line,
                    solution_line=solution.start_line,
                    solution_file=solution_path.relative_to(unit.course_dir).as_posix()
                    if solution_path
                    else "",
                )
            )
    if unknown_solutions:
        raise SystemExit(
            "exercises without a matching solution declaration:\n  "
            + "\n  ".join(unknown_solutions[:20])
        )
    return CourseDataset(
        course=course,
        title=course_manifest.course_title(course),
        units=units,
        exercises=exercises,
        source_fingerprint=fingerprint_files(source_paths),
    )


def build_all() -> dict:
    courses = [collect_course(course) for course in course_manifest.COURSE_ORDER]
    keys = ("units", "units_with_exercises", "exercises", "hints", "solutions", "prompt")
    totals = {key: sum(course.counts()[key] for course in courses) for key in keys}
    manifest = {
        "schema": "soko.exercise-dataset-manifest/1",
        "generated_by": "courses/exercise-dataset/build.py",
        "courses": [
            {
                "id": course.course,
                "title": course.title,
                "file": f"data/{course.course}.json",
                "source_fingerprint": course.source_fingerprint,
                **course.counts(),
            }
            for course in courses
        ],
        "totals": totals,
    }
    return {"manifest": manifest, "courses": {course.course: course.payload() for course in courses}}


def dumps(payload: dict) -> str:
    return json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=False) + "\n"


def dataset_files(payload: dict) -> dict:
    files = {"data/manifest.json": dumps(payload["manifest"])}
    for course, body in payload["courses"].items():
        files[f"data/{course}.json"] = dumps(body)
    return files
