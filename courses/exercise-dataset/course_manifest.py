#!/usr/bin/env python3
"""Course manifests and unit/solution layout.

The three courses use two manifest shapes (v1 flat list, v2
volumes → chapters → units) and two solution layouts; both are explicit here
instead of being guessed by the builder.

Stdlib only.
"""
from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterator, Optional

from model import ROOT

COURSE_ROOTS = {
    "quick-start": ROOT / "courses" / "quick-start",
    "set-theory": ROOT / "courses" / "set-theory",
    "analysis": ROOT / "courses" / "analysis",
}
# Order is part of the dataset's stable output: prerequisites first, then the
# two volumes of the book.
COURSE_ORDER = ("quick-start", "set-theory", "analysis")


@dataclass(frozen=True)
class UnitEntry:
    course: str
    volume_id: Optional[str]
    volume_title: Optional[str]
    chapter_id: Optional[str]
    chapter_title: Optional[str]
    unit_index: int
    unit_file: str  # course-relative
    title: str
    title_en: str
    chapter_tags: tuple

    @property
    def course_dir(self) -> Path:
        return COURSE_ROOTS[self.course]

    @property
    def path(self) -> Path:
        return self.course_dir / self.unit_file

    @property
    def stem(self) -> str:
        return Path(self.unit_file).stem

    @property
    def id_prefix(self) -> str:
        if self.chapter_id:
            return f"{self.course}/{self.chapter_id}/{self.stem}"
        return f"{self.course}/{self.stem}"


def load_manifest(course: str) -> dict:
    path = COURSE_ROOTS[course] / "course.json"
    return json.loads(path.read_text(encoding="utf-8"))


def course_title(course: str) -> str:
    manifest = load_manifest(course)
    if isinstance(manifest, dict):
        return str(manifest.get("title") or course)
    return "入门课" if course == "quick-start" else course


def iter_units(course: str) -> Iterator[UnitEntry]:
    """Every course unit, in manifest order (stable output depends on it)."""
    manifest = load_manifest(course)
    if isinstance(manifest, list):  # quick-start: v1 flat manifest, no chapters
        for entry in manifest:
            yield UnitEntry(
                course=course,
                volume_id=None,
                volume_title=None,
                chapter_id=None,
                chapter_title="入门课",
                unit_index=int(entry.get("unit", 0)),
                unit_file=str(entry["file"]),
                title=str(entry.get("title") or ""),
                title_en=str(entry.get("title_en") or ""),
                chapter_tags=(),
            )
        return
    for volume in manifest.get("volumes", []):
        for chapter in volume.get("chapters", []):
            for entry in chapter.get("units", []):
                yield UnitEntry(
                    course=course,
                    volume_id=str(volume.get("id") or ""),
                    volume_title=str(volume.get("title") or ""),
                    chapter_id=str(chapter.get("id") or ""),
                    chapter_title=str(chapter.get("title") or ""),
                    unit_index=int(entry.get("unit", 0)),
                    unit_file=str(entry["file"]),
                    title=str(entry.get("title") or ""),
                    title_en=str(entry.get("title_en") or ""),
                    chapter_tags=tuple(str(tag) for tag in chapter.get("tags", ())),
                )


def find_solution_path(unit: UnitEntry) -> Optional[Path]:
    """Locate the solution key for one unit (all three layouts are explicit)."""
    stem = unit.stem
    if unit.course == "quick-start":
        path = unit.course_dir / "solutions" / f"{stem}-solution.sokonanoda"
        return path if path.is_file() else None
    directory = unit.course_dir / "units" / "solutions" / (unit.chapter_id or "")
    numbered = re.match(r"unit(\d+)", stem)
    candidates = [directory / f"{stem}-solution.sokonanoda"]
    if numbered:
        candidates.append(directory / f"unit{numbered.group(1)}-solution.sokonanoda")
        if directory.is_dir():
            candidates.extend(sorted(directory.glob(f"unit{numbered.group(1)}-*-solution.sokonanoda")))
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    return None
