#!/usr/bin/env python3
"""把多份 e2e artifact 合并进 `docs/e2e/`（CI 回提交用，本地也能用）。

背景：CI 的 `e2e` job 是**矩阵**（ubuntu × VS Code 1.138.0 / 1.106.0、macOS ×
1.138.0），每个腿各自跑 `scripts/vscode-e2e.sh` 并把 `docs/e2e/` 上传成 artifact。
若让每个腿自己往仓库提交，会同时改同一个 `ledger.jsonl`（互相覆盖/冲突）；
所以由一个收尾 job 把全部 artifact 下载下来、在这里**一次合并成一条提交**。

用法：
  scripts/e2e-merge.py <artifact 目录> [<artifact 目录> ...]
      # 每个目录里找 `**/latest.json`（artifact 保留 `docs/e2e/` 结构）
  scripts/e2e-merge.py --check          # 只校验现有台账（排序/唯一/日志存在）
  scripts/e2e-merge.py --print          # 打印合并后的台账摘要

合并规则（幂等）：
* 台账按 `date` 升序保存；重复条目（commit+版本+计数+date 全同）只留一条；
* 每条记录引用的裁剪日志按记录里的 `log` 路径拷进 `docs/e2e/logs/`（同名覆盖）；
* `docs/e2e/latest.json` 写成**最新**的一条（看板/摘要读它）。
"""

from __future__ import annotations

import json
import pathlib
import shutil
import sys

E2E_DIR = pathlib.Path("docs/e2e")
LEDGER = E2E_DIR / "ledger.jsonl"
LATEST = E2E_DIR / "latest.json"
LOGS = E2E_DIR / "logs"


def key(entry: dict) -> tuple:
    tests = entry.get("tests", {})
    return (
        entry.get("commit"),
        entry.get("vscode"),
        entry.get("kind"),
        entry.get("exit"),
        tests.get("passed"),
        tests.get("failed"),
        entry.get("date"),
    )


def read_ledger() -> list[dict]:
    if not LEDGER.exists():
        return []
    return [json.loads(line) for line in LEDGER.read_text().splitlines() if line.strip()]


def write_ledger(rows: list[dict]) -> None:
    rows = sorted(rows, key=lambda entry: entry.get("date") or "")
    E2E_DIR.mkdir(parents=True, exist_ok=True)
    LEDGER.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows))
    if rows:
        LATEST.write_text(json.dumps(rows[-1], indent=2, ensure_ascii=False) + "\n")


def find_records(root: pathlib.Path) -> list[tuple[pathlib.Path, dict]]:
    """在 artifact 目录里找 `**/latest.json`，返回 (目录, 记录)。"""
    found: list[tuple[pathlib.Path, dict]] = []
    for path in sorted(root.rglob("latest.json")):
        try:
            entry = json.loads(path.read_text())
        except json.JSONDecodeError as error:  # 半截文件不算记录，直接报出来
            raise SystemExit(f"error: {path} 不是合法 JSON：{error}")
        if entry.get("schema") != "soko.e2e/1":
            continue
        found.append((path.parent, entry))
    return found


def merge(dirs: list[str]) -> int:
    rows = read_ledger()
    seen = {key(entry) for entry in rows}
    LOGS.mkdir(parents=True, exist_ok=True)
    added_entries: list[dict] = []
    for directory in dirs:
        root = pathlib.Path(directory)
        if not root.exists():
            raise SystemExit(f"error: {root} 不存在")
        for base, entry in find_records(root):
            # 裁剪日志跟着记录走（artifact 里是 `logs/xxx.log`）。
            log_name = pathlib.Path(entry.get("log", "")).name
            if log_name:
                source = base / "logs" / log_name
                if source.exists():
                    shutil.copyfile(source, LOGS / log_name)
                else:
                    print(f"warn: {entry.get('log')} 在 {base} 里找不到，记录仍保留")
            if key(entry) in seen:
                print(f"skip: {entry['commit'][:7]} / VS Code {entry['vscode']} 已在台账里")
                continue
            seen.add(key(entry))
            rows.append(entry)
            added_entries.append(entry)
    write_ledger(rows)
    print(f"e2e-merge: +{len(added_entries)} 条（台账共 {len(rows)} 条）")
    if added_entries:
        # CI 用它拼提交标题（`sed -n 's/^e2e-merge-subject: //p'`）。
        summary = " · ".join(
            f"VS Code {entry['vscode']} {entry['tests']['passed']}/{entry['tests']['passed'] + entry['tests']['failed']}"
            for entry in added_entries
        )
        print(f"e2e-merge-subject: {summary}")
    return 0


def check() -> int:
    rows = read_ledger()
    problems = []
    dates = [entry.get("date") or "" for entry in rows]
    if dates != sorted(dates):
        problems.append("台账没有按 date 升序保存")
    keys = [key(entry) for entry in rows]
    if len(keys) != len(set(keys)):
        problems.append("台账里有重复条目")
    for entry in rows:
        # 记录里的 `log` 是**仓库根相对路径**（`scripts/vscode-e2e.sh` 从仓库根跑）。
        log = pathlib.Path(entry.get("log", ""))
        if entry.get("log") and not log.exists():
            problems.append(f"记录引用的日志不存在：{entry['log']}")
    if problems:
        for problem in problems:
            print(f"error: {problem}")
        return 1
    print(f"e2e ledger: ok（{len(rows)} 条，日志齐全，按日期升序）")
    return 0


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__.strip())
        return 2
    if argv[0] == "--check":
        return check()
    if argv[0] == "--print":
        for entry in read_ledger():
            tests = entry["tests"]
            print(
                f"{entry['date'][:19]}  {entry['commit'][:7]}  VS Code {entry['vscode']:>8}  "
                f"{tests['passed']}/{tests['passed'] + tests['failed']}  exit={entry['exit']}"
            )
        return 0
    return merge(argv)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
