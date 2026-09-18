#!/usr/bin/env python3
"""把最新一条 e2e 记录渲染成 Markdown（CI job summary / 本地查看）。

用法：
  scripts/e2e-summary.py                 # 读 docs/e2e/latest.json
  scripts/e2e-summary.py --ledger        # 附上台账里最近 10 条的趋势表

设计：`scripts/vscode-e2e.sh` 负责**产生**记录，这里只负责**呈现**——CI 的
`$GITHUB_STEP_SUMMARY` 不能写多行 heredoc（YAML 缩进会把 `PY` 终止符顶掉），
所以把渲染抽成脚本，本地也能用同一条命令看结果。
"""

import json
import pathlib
import sys

LATEST = pathlib.Path("docs/e2e/latest.json")
LEDGER = pathlib.Path("docs/e2e/ledger.jsonl")


def load_latest() -> dict | None:
    if not LATEST.exists():
        return None
    return json.loads(LATEST.read_text())


def render_latest(entry: dict) -> str:
    tests = entry["tests"]
    lines = [
        f"- 结果：**{tests['passed']} passed / {tests['failed']} failed**"
        f"（exit={entry['exit']}，pending={tests.get('pending', 0)}）",
        f"- VS Code：{entry['vscode']} · host：{entry['host']['system']}/{entry['host']['machine']}",
        f"- 版本：extension v{entry['version']} @ `{entry['commit'][:7]}`（dirty={entry['dirty']}）",
        f"- 服务器：{entry['server'] or 'n/a'}",
        f"- 被测 LSP sha256[0:16]：`{entry['lsp_sha256_16']}`",
        f"- 日志：`{entry['log']}`",
    ]
    return "\n".join(lines)


def render_trend(rows: list[dict], limit: int = 10) -> str:
    if not rows:
        return ""
    lines = ["", "| commit | 日期 | VS Code | 结果 | dirty | 服务器 |", "| --- | --- | --- | --- | --- | --- |"]
    for entry in rows[-limit:]:
        tests = entry["tests"]
        result = f"{tests['passed']}/{tests['passed'] + tests['failed']}"
        lines.append(
            f"| `{entry['commit'][:7]}` | {entry['date'][:10]} | {entry['vscode']} | "
            f"{result} | {entry['dirty']} | {entry['server'] or 'n/a'} |"
        )
    return "\n".join(lines)


def main() -> int:
    want_trend = "--ledger" in sys.argv
    entry = load_latest()
    if entry is None:
        print("_没有 e2e 记录_（`docs/e2e/latest.json` 不存在：脚本可能在前置/构建阶段就失败了）")
    else:
        print(render_latest(entry))
    if want_trend and LEDGER.exists():
        rows = [json.loads(line) for line in LEDGER.read_text().splitlines() if line.strip()]
        trend = render_trend(rows)
        if trend:
            print(trend)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
