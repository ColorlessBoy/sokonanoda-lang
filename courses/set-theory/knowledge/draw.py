#!/usr/bin/env python3
"""从题库抽题（内容固化在 data/questions.json）。用法：
  python3 draw.py --chapter I.2 --level 2 --count 1
  python3 draw.py --list-chapters
输出 JSON；--seed 可复现。
"""
import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DATA = ROOT / "data" / "questions.json"

def main() -> int:
    parser = argparse.ArgumentParser(description="抽选择题")
    parser.add_argument("--chapter")
    parser.add_argument("--level", type=int)
    parser.add_argument("--unit")
    parser.add_argument("--count", type=int, default=1)
    parser.add_argument("--seed", type=int)
    parser.add_argument("--list-chapters", action="store_true")
    args = parser.parse_args()
    data = json.loads(DATA.read_text(encoding="utf-8"))
    if args.list_chapters:
        for chapter, title in data["chapters"].items():
            total = sum(1 for q in data["questions"] if q["chapter"] == chapter)
            print(chapter, "·", title, "·", total, "题")
        return 0
    pool = [q for q in data["questions"]
            if (args.chapter is None or q["chapter"] == args.chapter)
            and (args.level is None or q["level"] == args.level)
            and (args.unit is None or args.unit in q["unit"])]
    if not pool:
        print("no question matches", file=sys.stderr)
        return 1
    seed = args.seed if args.seed is not None else 0
    state = seed & 0xFFFFFFFF
    def rand():
        nonlocal state
        state = (state * 1664525 + 1013904223) & 0xFFFFFFFF
        return state / 2 ** 32
    for i in range(len(pool) - 1, 0, -1):
        j = int(rand() * (i + 1))
        pool[i], pool[j] = pool[j], pool[i]
    print(json.dumps(pool[: max(1, args.count)], ensure_ascii=False, indent=2))
    return 0

if __name__ == "__main__":
    sys.exit(main())
