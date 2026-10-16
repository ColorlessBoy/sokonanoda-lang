#!/usr/bin/env python3
"""e2e 台账**轮换**（记录 + 日志**一起**搬进 `docs/archive/`）。

## 为什么需要它（2026-10-16 用户拍板）

`docs-lint` 判据 ①（活文档 ≤ 8.50 MB）被 `docs/e2e/logs/` 撑破 ⇒ **pre-push 判红 ⇒ 所有推送被挡** ✗。
而**每一个**日志都被 `docs/e2e/ledger.jsonl` 的记录 `log` 字段引用（实测孤儿 0 个）
⇒ **单独归档日志会让 `e2e ledger` job 报「记录引用的日志不存在」** ✗（2026-10-16 实测撞过）。

⇒ 唯一正确的形状 = **台账记录与日志一起轮换**（照 `docs/archive/e2e-2026-10-15/` 的先例 `71f017b2`）。
⚠ 而它是一条**跑步机**：每跑一次 e2e 加约 2 个日志（~80 KB）⇒ 手工腾空间**一轮就被吃回去** ✗
⇒ 所以这件事必须**自动**（本脚本 = 那件事；用户 2026-10-16 拍板「可以轮换，而且把它自动化」✓）。

## 用法

```bash
python3 scripts/rotate-e2e-ledger.py            # 干跑：印将要发生什么（默认 ✓）
python3 scripts/rotate-e2e-ledger.py --apply    # 真做
python3 scripts/rotate-e2e-ledger.py --check    # 判据：台账引用的日志都存在吗（0/1）
```

## 不变量（本脚本必须守住）

1. **活台账里的每一条记录，其 `log` 指向的文件必须存在**（否则 `e2e ledger` job 判红 ✗）；
2. **归档的记录仍写原路径**（`docs/e2e/logs/<名字>.log`）—— 照 2026-10-15 先例，
   到 `docs/archive/<批次>/` 取同名 `.gz` ✓；
3. **只搬「只被归档记录引用」的日志** —— 还被活记录引用的**一个都不许动** ✗；
4. **幂等**：重复跑不改变结果 ✓。
"""
from __future__ import annotations

import argparse
import gzip
import json
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / "docs" / "e2e" / "ledger.jsonl"
LOGS = ROOT / "docs" / "e2e" / "logs"
ARCHIVE = ROOT / "docs" / "archive"
DEFAULT_KEEP = 60


def load(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        s = line.strip()
        if s:
            rows.append(json.loads(s))
    return rows


def log_name(rec: dict) -> str | None:
    lg = rec.get("log")
    if not isinstance(lg, str) or not lg:
        return None
    return Path(lg).name


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--apply", action="store_true", help="真做（默认干跑）")
    ap.add_argument("--check", action="store_true", help="判据：活记录的日志都在吗")
    ap.add_argument("--keep", type=int, default=DEFAULT_KEEP, help=f"活台账保留多少条（默认 {DEFAULT_KEEP}）")
    ap.add_argument("--batch", default=None, help="归档批次名（默认按最老被归档记录的日期）")
    a = ap.parse_args()

    rows = load(LEDGER)

    if a.check:
        missing = [n for r in rows if (n := log_name(r)) and not (LOGS / n).exists()]
        if missing:
            print(f"✗ 活台账引用的日志不存在：{len(missing)} 个 ✗")
            for m in missing[:5]:
                print(f"   · docs/e2e/logs/{m}")
            return 1
        print(f"✓ 活台账 {len(rows)} 条 · 引用的日志全部存在 ✓")
        return 0

    if len(rows) <= a.keep:
        print(f"✓ 台账 {len(rows)} 条 ≤ 保留 {a.keep} ⇒ 无需轮换 ✓")
        return 0

    keep, old = rows[-a.keep :], rows[: -a.keep]

    keep_logs = {n for r in keep if (n := log_name(r))}
    old_logs = {n for r in old if (n := log_name(r))}
    movable = sorted(old_logs - keep_logs)  # 不变量 ③：还被活记录引用的一个都不动 ✗
    kept_shared = sorted(old_logs & keep_logs)

    oldest = str(old[0].get("date") or "")[:10] or "unknown"
    batch = a.batch or f"e2e-{oldest}"
    dest = ARCHIVE / batch

    print(f"台账 {len(rows)} 条 ⇒ 活 {len(keep)} · 归档 {len(old)}")
    print(f"归档批次：docs/archive/{batch}/")
    print(f"  记录 ⇒ ledger-records-older.jsonl.gz（{len(old)} 条）")
    print(f"  日志 ⇒ {len(movable)} 个搬走（.gz）")
    if kept_shared:
        print(f"  ⚠ {len(kept_shared)} 个日志被活记录**也**引用 ⇒ **原地不动** ✓")
    for n in movable[:5]:
        print(f"   · {n}")

    if not a.apply:
        print("（干跑；加 --apply 真做）")
        return 0

    dest.mkdir(parents=True, exist_ok=True)

    # ① 归档记录（原文逐字，只 gzip）
    raw = "\n".join(json.dumps(r, ensure_ascii=False) for r in old) + "\n"
    with gzip.open(dest / "ledger-records-older.jsonl.gz", "wt", encoding="utf-8") as f:
        f.write(raw)

    # ② 归档日志（只搬 movable ✓）
    moved = 0
    for n in movable:
        src = LOGS / n
        if not src.exists():
            continue
        with open(src, "rb") as fi, gzip.open(dest / f"{n}.gz", "wb") as fo:
            shutil.copyfileobj(fi, fo)
        src.unlink()
        moved += 1

    # ③ 活台账 = 保留的那些（逐字）
    LEDGER.write_text("\n".join(json.dumps(r, ensure_ascii=False) for r in keep) + "\n", encoding="utf-8")

    print(f"✓ 归档 {len(old)} 条记录 · 搬走 {moved} 个日志 ⇒ 活台账 {len(keep)} 条")
    print(f"⚠ 别忘了在 docs/archive/README.md 里**点名** {batch}/（判据 ⑥：归档≠销毁）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
