#!/usr/bin/env python3
"""把多份 e2e artifact 合并进 `docs/e2e/`（CI 回提交用，本地也能用）。

背景：CI 的 `e2e` job 是**矩阵**（ubuntu × VS Code 1.138.0 / 1.106.0、macOS ×
1.138.0），每个腿各自跑 `scripts/vscode-e2e.sh` 并把 `docs/e2e/` 上传成 artifact。
若让每个腿自己往仓库提交，会同时改同一个 `ledger.jsonl`（互相覆盖/冲突）；
所以由一个收尾 job 把全部 artifact 下载下来、在这里**一次合并成一条提交**。

用法：
  scripts/e2e-merge.py <artifact 目录> [<artifact 目录> ...]
      # 每个目录里找 `**/latest.json`（artifact 保留 `docs/e2e/` 结构）
  scripts/e2e-merge.py --check          # 只校验现有台账（排序/唯一/日志存在**或已记账裁剪**）
  scripts/e2e-merge.py --print          # 打印合并后的台账摘要

合并规则（幂等）：
* 台账按 `date` 升序保存；重复条目（commit+版本+计数+date 全同）只留一条；
* **只保留最近 `LEDGER_KEEP` 条**；更早的折成一行 `kind:"pruned"` tombstone
  （count / 日期范围 / `log_pruned` 理由；完整历史见 git）——日志本来就不进仓库；
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
# **保留最近 N 条**（2026-10-23）：`docs/e2e/ledger.jsonl` 每轮 CI 追加 3 条（三腿），
# 而 `docs-lint` ① 的「other」类已贴顶 ⇒ 每轮都会把下一次 push 顶红。日志本来就不进仓库
# （`log_pruned`），这里再把**旧条目**折成一行 tombstone：保留最近 N 条完整记录，
# 更早的只留 count / 日期范围 / `log_pruned` 理由（完整历史在 git）。
LEDGER_KEEP = 20
PRUNED_KIND = "pruned"
PRUNED_NOTE = (
    "只保留最近 20 条；更早条目已裁剪（日志本来就不进仓库，见 log_pruned；"
    "完整历史见 git）"
)


def key(entry: dict) -> tuple:
    """幂等合并的去重键。

    含 host.system/machine：同一个 commit + 同一个 VS Code 版本会在 **两个 os 腿**
    （ubuntu 与 macos 各一条 1.138.0）上各记一条——如果两条腿的 `date` 恰好同秒
    （矩阵并行时不罕见），不含宿主就会把其中一条当重复丢掉，台账静默少一条腿。
    """
    tests = entry.get("tests", {})
    host = entry.get("host") or {}
    return (
        entry.get("commit"),
        entry.get("vscode"),
        host.get("system"),
        host.get("machine"),
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


def is_pruned(entry: dict) -> bool:
    return entry.get("kind") == PRUNED_KIND


def split_pruned(rows: list[dict]) -> tuple[list[dict], list[dict]]:
    """拆出真实条目 / tombstone（tombstone 不是一次 e2e 跑，不参与排序/去重）。"""
    pruned = [r for r in rows if is_pruned(r)]
    return [r for r in rows if not is_pruned(r)], pruned


def write_ledger(rows: list[dict], pruned_count: int = 0,
                 pruned_first: str | None = None,
                 pruned_last: str | None = None) -> tuple[int, int]:
    rows = sorted(rows, key=lambda entry: entry.get("date") or "")
    if len(rows) > LEDGER_KEEP:
        dropped = rows[:-LEDGER_KEEP]
        rows = rows[-LEDGER_KEEP:]
        pruned_count += len(dropped)
        first = dropped[0].get("date")
        last = dropped[-1].get("date")
        if first:
            pruned_first = min(pruned_first, first) if pruned_first else first
        if last:
            pruned_last = max(pruned_last, last) if pruned_last else last
    E2E_DIR.mkdir(parents=True, exist_ok=True)
    lines: list[str] = []
    if pruned_count:
        lines.append(json.dumps({
            "schema": "soko.e2e/1",
            "kind": PRUNED_KIND,
            "count": pruned_count,
            "first_date": pruned_first,
            "last_date": pruned_last,
            "log_pruned": PRUNED_NOTE,
        }, ensure_ascii=False))
    lines.extend(json.dumps(r, ensure_ascii=False) for r in rows)
    LEDGER.write_text("\n".join(lines) + ("\n" if lines else ""))
    if rows:
        LATEST.write_text(json.dumps(rows[-1], indent=2, ensure_ascii=False) + "\n")
    return len(rows), pruned_count


def find_records(root: pathlib.Path) -> list[tuple[pathlib.Path, dict]]:
    """在 artifact 目录里找 `**/latest.json`，返回 (目录, 记录)。"""
    found: list[tuple[pathlib.Path, dict]] = []
    # **只收文件**（审计 #10 ✓）：同名**目录**会让 `read_text()` 抛
    # `IsADirectoryError` —— 而下面的 `except` 只接 `JSONDecodeError` ✗
    # ⇒ 那是 **traceback 而不是 `SystemExit`**（"崩了却不判"的原形 ✓）。
    for path in sorted(p for p in root.rglob("latest.json") if p.is_file()):
        try:
            entry = json.loads(path.read_text())
        except (json.JSONDecodeError, OSError) as error:  # 半截文件/读不了都不算记录，直接报出来
            raise SystemExit(f"error: {path} 不是合法 JSON：{error}")
        if entry.get("schema") != "soko.e2e/1":
            continue
        found.append((path.parent, entry))
    return found


def merge(dirs: list[str]) -> int:
    raw = read_ledger()
    rows, pruned = split_pruned(raw)
    pruned_count = sum(int(t.get("count") or 0) for t in pruned)
    pruned_first = min((t.get("first_date") for t in pruned if t.get("first_date")),
                       default=None)
    pruned_last = max((t.get("last_date") for t in pruned if t.get("last_date")),
                      default=None)
    seen = {key(entry) for entry in rows}
    LOGS.mkdir(parents=True, exist_ok=True)
    added_entries: list[dict] = []
    for directory in dirs:
        root = pathlib.Path(directory)
        if not root.exists():
            raise SystemExit(f"error: {root} 不存在")
        for base, entry in find_records(root):
            # **2026-10-20（docs-lint ① 连续两轮顶格）**：日志**不再**拷进仓库 ——
            # 它是 CI artifact 的产物，而 `docs/e2e/logs/` 计进活文档预算的「other」类：
            # 每次 e2e 台账提交回 main 就 +80 KB，把下一次 push 的 pre-push / CI 直接顶红
            # （实测：`1a3d7bbb` 提交 2 份日志 ⇒ `other` 4.19 → 4.26 > 上限 4.20 ✗）。
            # 改为**行内记 `log_pruned`**：`--check` 认这个标记（咬住"引用了不存在的日志
            # 又没记标记"这条），权威记录 = 本行计数 + CI artifact ✓。
            log_name = pathlib.Path(entry.get("log", "")).name
            if log_name:
                source = base / "logs" / log_name
                if not source.exists():
                    print(f"warn: {entry.get('log')} 在 {base} 里找不到，记录仍保留")
                entry["log_pruned"] = (
                    "CI artifact 即权威（`docs-lint` ①：日志进仓库每轮约 +80 KB，会顶格）"
                )
            if key(entry) in seen:
                print(f"skip: {entry['commit'][:7]} / VS Code {entry['vscode']} 已在台账里")
                continue
            seen.add(key(entry))
            rows.append(entry)
            added_entries.append(entry)
    kept, pruned_total = write_ledger(rows, pruned_count, pruned_first, pruned_last)
    print(f"e2e-merge: +{len(added_entries)} 条（台账共 {kept} 条；更早 {pruned_total} 条已裁剪）")
    if added_entries:
        # CI 用它拼提交标题（`sed -n 's/^e2e-merge-subject: //p'`）。
        summary = " · ".join(
            f"VS Code {entry['vscode']} {entry['tests']['passed']}/{entry['tests']['passed'] + entry['tests']['failed']}"
            for entry in added_entries
        )
        print(f"e2e-merge-subject: {summary}")
    return 0


def check() -> int:
    raw = read_ledger()
    rows, pruned = split_pruned(raw)
    problems = []
    if len(pruned) > 1:
        problems.append(f"台账里有 {len(pruned)} 条 prune tombstone（最多 1 条）")
    for tomb in pruned:
        if not tomb.get("log_pruned"):
            problems.append("prune tombstone 缺 `log_pruned` 理由")
        if not isinstance(tomb.get("count"), int) or tomb.get("count", 0) < 1:
            problems.append("prune tombstone 的 `count` 不是正整数")
    dates = [entry.get("date") or "" for entry in rows]
    if dates != sorted(dates):
        problems.append("台账没有按 date 升序保存")
    keys = [key(entry) for entry in rows]
    if len(keys) != len(set(keys)):
        problems.append("台账里有重复条目")
    for entry in rows:
        # 记录里的 `log` 是**仓库根相对路径**（`scripts/vscode-e2e.sh` 从仓库根跑）。
        #
        # ⚠ **历史行允许已裁剪，但必须记账**（2026-10-20 ✗→✓）：
        # `docs/e2e/logs/` 是 CI 产物，跟着**活文档预算**（`docs-lint` ①）走
        # —— 预算顶到上限时只能删源文件，那时台账里的 `log` 必然断链 ✗
        # （实测 run 39341179027：`docs/e2e/logs/` **一个文件都没有** ⇒ 48 行全断链
        # ⇒ 本 job 每轮必红 ✗）。删的时候**必须**在同一行记 `log_pruned`
        # （值 = 删它的提交 + 理由）⇒ 断链变成**有记账的裁剪**，不是悄悄丢证据 ✓。
        # **判据没有放宽**：新行（`log` 有、`log_pruned` 没有）照样判红 ✓。
        log = pathlib.Path(entry.get("log", ""))
        if entry.get("log") and not log.exists() and not entry.get("log_pruned"):
            problems.append(
                f"记录引用的日志不存在、也没记 `log_pruned`：{entry['log']}"
            )
    if problems:
        for problem in problems:
            print(f"error: {problem}")
        return 1
    print(f"e2e ledger: ok（{len(rows)} 条，日志齐全或已记账裁剪，按日期升序）")
    return 0


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__.strip())
        return 2
    if argv[0] == "--check":
        return check()
    if argv[0] == "--print":
        rows, pruned = split_pruned(read_ledger())
        for tomb in pruned:
            print(f"pruned: {tomb.get('count')} 条（{tomb.get('first_date')} … {tomb.get('last_date')}）")
        for entry in rows:
            tests = entry["tests"]
            print(
                f"{entry['date'][:19]}  {entry['commit'][:7]}  VS Code {entry['vscode']:>8}  "
                f"{tests['passed']}/{tests['passed'] + tests['failed']}  exit={entry['exit']}"
            )
        return 0
    return merge(argv)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
