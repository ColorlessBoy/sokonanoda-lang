#!/usr/bin/env python3
"""Compose one runnable exercise file from the dataset + its course sources.

Usage (repository root):

    python3 courses/exercise-dataset/compose.py <exercise-id>
    python3 courses/exercise-dataset/compose.py <exercise-id> --solution
    python3 courses/exercise-dataset/compose.py <exercise-id> --out /tmp/ex.sokonanoda

Why this exists: one dataset record stores the exercise declaration and its
solution, but a `.sokonanoda` file must also carry the unit's imports and the
declarations that precede the exercise.  Composing from the *course source*
keeps that prefix authoritative (and lets the language-service plugin hand a
real file to `soko grade` without duplicating every unit in the dataset).
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import model  # noqa: E402


def load_records(dataset_dir: Path = HERE) -> dict:
    records = {}
    for path in sorted((dataset_dir / "data").glob("*.json")):
        if path.name == "manifest.json":
            continue
        payload = json.loads(path.read_text(encoding="utf-8"))
        for exercise in payload.get("exercises", []):
            records[exercise["id"]] = exercise
    return records


def _cut_after(decls: list, line: int) -> int:
    """Number of lines to keep so the record at `line` is complete."""
    starts = [decl.start_line for decl in decls]
    after = [start for start in starts if start > line]
    if not after:
        return 0  # 0 = whole file
    return after[0] - 1


def compose(record: dict, solution: bool = False) -> str:
    source = record["solution_source"] if solution else record["source"]
    path = model.ROOT / source["file"]
    lines = path.read_text(encoding="utf-8").splitlines()
    parsed = model.parse_file(path)
    keep = _cut_after(parsed.decls, int(source["line"]))
    text = "\n".join(lines[:keep] if keep else lines).rstrip() + "\n"
    expected = record["solution"] if solution else record["template"]
    if expected not in text:
        raise SystemExit(f"compose: record text is not present in {source['file']}:{source['line']}")
    return text


def main(argv: list = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("exercise_id", help="dataset id, e.g. set-theory/I.1/unit01-sets-membership/mem_of_subset")
    parser.add_argument("--solution", action="store_true", help="compose the solution key instead of the exercise")
    parser.add_argument("--out", type=Path, help="write here instead of stdout")
    parser.add_argument("--json", action="store_true", help="print metadata instead of the file")
    args = parser.parse_args(argv)

    records = load_records()
    record = records.get(args.exercise_id)
    if record is None:
        print(f"unknown exercise id: {args.exercise_id}", file=sys.stderr)
        return 2
    text = compose(record, solution=args.solution)
    if args.out:
        args.out.write_text(text, encoding="utf-8")
    if args.json:
        print(
            json.dumps(
                {
                    "id": record["id"],
                    "kind": record["kind"],
                    "solution": args.solution,
                    "out": str(args.out) if args.out else None,
                    "lines": len(text.splitlines()),
                    "bytes": len(text.encode("utf-8")),
                    "source": record["solution_source"] if args.solution else record["source"],
                },
                ensure_ascii=False,
                indent=2,
            )
        )
    elif not args.out:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
