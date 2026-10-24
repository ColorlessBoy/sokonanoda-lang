#!/usr/bin/env python3
"""生成/检查 README 里的 Lean Kernel Arena 可信度块。

单一事实来源 = `site/data/arena.json`（由 `scripts/arena-gate.py --write-result`
在**官方最新 master 语料通过后**写入）。`scripts/bump.py` 在写版本号前跑完 gate
后调用本脚本 `--write`；`scripts/release-preflight.sh` 用 `--check` 防止漂移。

用法：

    python3 scripts/gen-arena-summary.py --write   # 重写 README 块
    python3 scripts/gen-arena-summary.py --check   # 只比对（不一致 exit 1）
    python3 scripts/gen-arena-summary.py           # 打到 stdout
"""

from __future__ import annotations

import argparse
import json
import os
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ARENA = os.path.join(REPO_ROOT, "site", "data", "arena.json")
README = os.path.join(REPO_ROOT, "README.md")
BEGIN = "<!-- arena-summary:begin -->"
END = "<!-- arena-summary:end -->"


def load_arena() -> dict:
    try:
        with open(ARENA, encoding="utf-8") as handle:
            data = json.load(handle)
    except OSError as error:
        print(f"arena-summary: 读不到 {ARENA}：{error}", file=sys.stderr)
        raise SystemExit(3)
    except json.JSONDecodeError as error:
        print(f"arena-summary: {ARENA} 不是合法 JSON：{error}", file=sys.stderr)
        raise SystemExit(3)
    if data.get("passed") is not True:
        print(
            "arena-summary: arena.json 不是一次通过的结果（passed != true）—— 不生成可信度块",
            file=sys.stderr,
        )
        raise SystemExit(3)
    return data


def render(data: dict) -> str:
    revision = str(data.get("revision", ""))
    short = revision[:9] if revision else "?"
    date = str(data.get("created_at") or "?")[:10]
    good = data["good"]
    accepted = data["accepted"]
    bad = data["bad"]
    rejected = data["rejected"]
    declined = data["declined"]
    slow_timeouts = data.get("slow_timeouts", 0)
    run_id = data.get("run_id", "?")
    slow_note = (
        f"{slow_timeouts} performance-only timeout"
        if int(slow_timeouts) == 1
        else f"{slow_timeouts} performance-only timeouts"
    )
    return "\n".join(
        [
            BEGIN,
            f"**Latest gate:** Lean Kernel Arena `master@{short}` (run `{run_id}`, {date}) — "
            f"**{accepted}/{good}** accept ({slow_note}), "
            f"**{rejected}/{bad}** reject ({declined} known unsupported declined).",
            "Reproduce `python3 scripts/arena-gate.py`; public artifact = scored tests "
            "≤10MB (big cases need `LEAN_KERNEL_ARENA`); no official Lean toolchain.",
            END,
        ]
    )


def replace_block(text: str, block: str) -> str:
    start = text.find(BEGIN)
    end = text.find(END)
    if start == -1 or end == -1 or end < start:
        print(
            f"arena-summary: README.md 缺 {BEGIN} … {END} 标记（先加空块再 --write）",
            file=sys.stderr,
        )
        raise SystemExit(2)
    return text[:start] + block + text[end + len(END) :]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="生成/检查 README 的 LKA 可信度块")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--check", action="store_true", help="只比对，不一致 exit 1")
    group.add_argument("--write", action="store_true", help="重写 README 块")
    args = parser.parse_args(argv)

    block = render(load_arena())
    try:
        text = open(README, encoding="utf-8").read()
    except OSError as error:
        print(f"arena-summary: 读不到 {README}：{error}", file=sys.stderr)
        return 3

    if not args.check and not args.write:
        sys.stdout.write(block + "\n")
        return 0

    updated = replace_block(text, block)
    if args.check:
        if updated != text:
            print(
                "arena-summary: README 的可信度块与 site/data/arena.json 不一致 "
                "（跑 `python3 scripts/gen-arena-summary.py --write`）",
                file=sys.stderr,
            )
            return 1
        print("arena-summary: README 可信度块与 arena.json 一致")
        return 0

    if updated != text:
        with open(README, "w", encoding="utf-8") as handle:
            handle.write(updated)
        print("arena-summary: 已重写 README 可信度块")
    else:
        print("arena-summary: README 可信度块已是最新")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
