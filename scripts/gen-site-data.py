#!/usr/bin/env python3
"""Generate site/data/site.json — the single machine-readable source of truth
for the static GitHub Pages site.

Design: docs/design/site.md §3. No third-party dependencies — python3 stdlib only.

The site must never hand-write three things: version, unit count, and progress.
This script pulls each from its real source so the numbers cannot drift:

  version      <- Cargo.toml   [workspace.package] version
  units        <- course/course.json  (each unit flags its English mirror if present)
  round        <- STATUS.md    latest "## 本轮进度（日期，第N轮：标题）" header
  round_date   <-   (same header)
  round_title  <-   (same header)
  examples     <- examples/*.sokonanoda  filenames

Every field is parsed; if a source is missing or unparseable the field is left
out rather than fabricated. Output is deterministic (sorted keys, stable order).
"""

import glob
import json
import os
import re

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def _read(rel_path):
    path = os.path.join(REPO_ROOT, rel_path)
    try:
        with open(path, encoding="utf-8") as fh:
            return fh.read()
    except FileNotFoundError:
        return ""


def get_version():
    """Return [workspace.package].version from Cargo.toml, or None."""
    text = _read("Cargo.toml")
    if not text:
        return None
    # Isolate the [workspace.package] section so a top-level `version` (if any)
    # does not shadow the workspace package version.
    m = re.search(r"\[workspace\.package\](.*?)(?:\n\[|\Z)", text, re.S)
    block = m.group(1) if m else text
    mm = re.search(r'^\s*version\s*=\s*"([^"]+)"', block, re.M)
    return mm.group(1) if mm else None


def get_units():
    """Return the ordered unit list from course/course.json.

    Each item carries file/title/title_en/unit, and an extra `en_file` key when
    an English mirror exists under course/en/.
    """
    text = _read("course/course.json")
    if not text:
        return []
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return []
    if not isinstance(data, list):
        return []

    units = []
    for entry in data:
        if not isinstance(entry, dict):
            continue
        item = {
            "file": entry.get("file", ""),
            "title": entry.get("title", ""),
            "title_en": entry.get("title_en", ""),
            "unit": entry.get("unit"),
        }
        en_path = os.path.join("course", "en", entry.get("file", ""))
        if entry.get("file") and os.path.exists(os.path.join(REPO_ROOT, en_path)):
            item["en_file"] = en_path
        units.append(item)

    # Stable ordering by unit number (entries without a numeric unit sort last).
    units.sort(
        key=lambda u: (u.get("unit") is None, u.get("unit"))
        if isinstance(u.get("unit"), int)
        else (True, 0)
    )
    return units


_CN_DIGITS = {
    "零": 0, "一": 1, "二": 2, "两": 2, "三": 3, "四": 4,
    "五": 5, "六": 6, "七": 7, "八": 8, "九": 9,
}


def _cn_to_int(token):
    """Convert a Chinese/Arabic numeral token to int (handles up to 万).

    STATUS.md writes rounds as Chinese numerals (e.g. 第三十九轮), so a plain
    \\d+ would never match. Arabic digits are also accepted as a fallback.
    """
    total = 0
    section = 0
    num = 0
    for ch in token:
        if ch in _CN_DIGITS:
            num = _CN_DIGITS[ch]
        elif ch == "十":
            section += (num or 1) * 10
            num = 0
        elif ch == "百":
            section += (num or 1) * 100
            num = 0
        elif ch == "千":
            section += (num or 1) * 1000
            num = 0
        elif ch == "万":
            total += (section + (num or 0)) * 10000
            section = 0
            num = 0
        elif ch.isdigit():
            num = num * 10 + int(ch)
    value = total + section + num
    return value or None


def get_round():
    """Parse the most recent '## 本轮进度（日期，第N轮：标题）' header.

    STATUS.md orders rounds newest-first, so the first match in the file is the
    current round.
    """
    text = _read("STATUS.md")
    if not text:
        return {}
    m = re.search(
        r"^##\s*本轮进度（(\d{4}-\d{2}-\d{2})，第([^轮]+)轮：([^）]*)）\s*$",
        text,
        re.M,
    )
    if not m:
        return {}
    round_num = _cn_to_int(m.group(2))
    return {
        "round_date": m.group(1),
        "round": round_num,
        "round_title": m.group(3).replace("`", "").strip(),
    }


def get_examples():
    """List examples/*.sokonanoda filenames (sorted)."""
    pattern = os.path.join(REPO_ROOT, "examples", "*.sokonanoda")
    return sorted(os.path.basename(p) for p in glob.glob(pattern))


def main():
    data = {}

    version = get_version()
    if version:
        data["version"] = version

    units = get_units()
    if units:
        data["units"] = units

    data.update(get_round())  # round / round_date / round_title (only if found)

    examples = get_examples()
    if examples:
        data["examples"] = examples

    # tests_total is intentionally omitted: TESTING.md only carries it inside
    # prose-style per-round totals that are not a single stable field. We do not
    # fabricate a number (docs/design/site.md §3).

    out_dir = os.path.join(REPO_ROOT, "site", "data")
    os.makedirs(out_dir, exist_ok=True)
    out_path = os.path.join(out_dir, "site.json")
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(data, fh, ensure_ascii=False, indent=2, sort_keys=True)
        fh.write("\n")

    print("wrote", os.path.relpath(out_path, REPO_ROOT))


if __name__ == "__main__":
    main()
