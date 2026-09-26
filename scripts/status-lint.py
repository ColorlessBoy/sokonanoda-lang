#!/usr/bin/env python3
"""STATUS.md 的 lint（用户 2026-09-26 要求 ✓）—— 见 REQUIREMENTS.md §9。

**为什么** ✓：STATUS.md 曾是 **1534 行 / 67 KB** ✗，含 **163 条 "round N"** ✗ 与
**138 条瞬时状态**（"在跑/进行中/未变/判据不变/待 CI/等 CI/⏳"）✗ ——
而 **AGENTS.md §收尾义务** 本来就写着"**只保留最近 3 轮**" ✗。
"CI 在跑"这类信息**看 CI 页面就有** ✓，写进 STATUS 只有噪音 ✗，
而**每个接手 agent 都要先读那 67 KB** ✗。

**四类判据**（可判 ✓）：
  ① **总行数 ≤ 200** ✗（超出判红）；
  ② **禁词命中 = 0** ✗（`在跑`/`进行中`/`未变`/`判据不变`/`待 CI`/`等 CI`/`⏳`）；
  ③ **每段 ≤ 30 行** ✗（段 = 顶层 `##` 之间；**首段"当前快照" ≤ 40 行** ✓）；
  ④ **净增行数 ≤ 60** ✗（与 `HEAD` 版比 —— 防止一轮又灌几十行 ✗）。

退出码：0 = 通过 ✓ / 1 = 判红 ✗ / 2 = 用法或环境错。
"""
import re
import subprocess
import sys
from pathlib import Path

MAX_TOTAL = 200
MAX_SECTION = 30
MAX_SNAPSHOT = 40
MAX_GROWTH = 60
BANNED = ("在跑", "进行中", "未变", "判据不变", "待 CI", "等 CI", "⏳")
SNAPSHOT_TITLE = "当前快照"


def sections(lines):
    """按顶层 `##` 切段；返回 [(标题, 起始行号, 行数)]。"""
    out, title, start = [], "(前言)", 1
    for i, l in enumerate(lines, 1):
        if l.startswith("## "):
            out.append((title, start, i - start))
            title, start = l[3:].strip(), i
    out.append((title, start, len(lines) + 1 - start))
    return [s for s in out if s[2] > 0]


def growth(path):
    """与 HEAD 版比净增行数；无 HEAD 版（新文件）⇒ 0。"""
    try:
        old = subprocess.run(["git", "show", f"HEAD:{path}"], capture_output=True,
                             text=True, check=True).stdout
    except Exception:  # noqa: BLE001
        return 0
    return len(path and Path(path).read_text(encoding="utf-8").splitlines()) - len(old.splitlines())


def main() -> int:
    path = Path("STATUS.md")
    if not path.exists():
        print("STATUS.md 不存在 ✗", file=sys.stderr)
        return 2
    lines = path.read_text(encoding="utf-8").splitlines()
    bad = []

    # ① 总行数
    if len(lines) > MAX_TOTAL:
        bad.append(f"总行数 {len(lines)} > {MAX_TOTAL} ✗（逐轮过程应进 docs/STATUS-ARCHIVE.md ✓）")

    # ② 禁词
    for i, l in enumerate(lines, 1):
        for w in BANNED:
            if w in l:
                bad.append(f"第 {i} 行命中禁词 `{w}` ✗：{l.strip()[:60]}")

    # ③ 每段行数（首段 = 当前快照 ⇒ 40 ✓，其余 ⇒ 30 ✗）
    for title, start, n in sections(lines):
        limit = MAX_SNAPSHOT if SNAPSHOT_TITLE in title else MAX_SECTION
        if n > limit:
            bad.append(f"段「{title}」（第 {start} 行起）{n} 行 > {limit} ✗")

    # ④ 净增行数
    g = growth("STATUS.md")
    if g > MAX_GROWTH:
        bad.append(f"本次净增 {g} 行 > {MAX_GROWTH} ✗（一轮别灌几十行 ✗）")

    if bad:
        print(f"status-lint：{len(bad)} 条不通过 ✗", file=sys.stderr)
        for b in bad[:20]:
            print(f"  - {b}", file=sys.stderr)
        return 1
    print(f"status-lint ✓ 总行数 {len(lines)} ≤ {MAX_TOTAL} ✓ · 禁词 0 ✓ · "
          f"段数 {len(sections(lines))} ✓ · 净增 {g} ≤ {MAX_GROWTH} ✓")
    return 0


if __name__ == "__main__":
    sys.exit(main())
