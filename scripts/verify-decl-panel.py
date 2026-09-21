#!/usr/bin/env python3
"""**声明栏量具**：逐个文件量"Infoview 的声明栏有没有内容"，并说清为什么。

为什么需要它：用户报「声明栏经常失效，有些文件有声明，有些没有」——而这类
"有些…有些…"的抱怨**必须逐个文件量过**才能定范围（计划 T-B01/T-B02）。
本脚本把三件事并排放在一张表里：

  * **单文件 parse**：这份文件**自己**能不能解析。判据用 CLI `query goals` 的
    `not-parsable`——它正是 `QueryDoc::parsable()`（`parse_error.is_some()`），
    而"用了 import 来的记法"时它必然是"否"。
    （**不要**用 `grade --no-project` 量这个：`--no-project` 把模块根换成入口目录，
    `import lib.Set` 会变成 `import-not-found`，量到的是另一件事——实测踩过。）
  * **CLI goals**：CLI `query goals` 的条数（**会如实报 `not-parsable`**）；
  * **LSP goals**：`soko/goals` 的条数 = **声明栏真正拿到的数字**（LSP 把错误吞成 `[]`）；
  * **symbols**：`textDocument/documentSymbol` 的条数（走 `report`，G-20 修过）。

判读：**LSP goals == 0 而 symbols > 0** ⇒ **声明栏空、但报告是好的** —— 这正是 G-22
（`parsable()` 漏打补丁，LSP 侧 `.unwrap_or_default()` 把错误吞成空数组）。
`LSP goals == symbols` ⇒ 正常。

用法：

    python3 scripts/verify-decl-panel.py                  # 卷 I + 入门课 + playground
    python3 scripts/verify-decl-panel.py --root courses/set-theory/units
    python3 scripts/verify-decl-panel.py --json
    python3 scripts/verify-decl-panel.py --lsp            # 用 LSP 量（默认，最忠实）
    python3 scripts/verify-decl-panel.py --cli            # 只用 CLI（快，量不到 symbols）

退出码：0 = 量完；1 = 发现"声明栏空但报告好"（= G-22 仍在）；2 = 环境问题。
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOKO = ROOT / "scripts" / "soko"
NODE = shutil.which("node")

DEFAULT_ROOTS = [
    "courses/set-theory/lib",
    "courses/set-theory/units",
    "courses/set-theory/units/solutions",
    "course",
    "playground.sokonanoda",
]


def collect(roots: list[str]) -> list[pathlib.Path]:
    files: list[pathlib.Path] = []
    for root in roots:
        path = ROOT / root
        if path.is_file():
            files.append(path)
        elif path.is_dir():
            files.extend(sorted(p for p in path.rglob("*.sokonanoda") if p.is_file()))
    # 去重 + 稳定顺序
    seen, out = set(), []
    for f in files:
        if f not in seen:
            seen.add(f)
            out.append(f)
    return out


def run(args: list[str]) -> subprocess.CompletedProcess:
    env = dict(__import__("os").environ)
    for key in ("SOKONANODA_BIN", "SOKONANODA_LSP_BIN"):
        env.pop(key, None)
    return subprocess.run(
        [NODE, str(SOKO), *args], capture_output=True, text=True, env=env, cwd=str(ROOT)
    )


def cli_goals(path: pathlib.Path) -> int | str:
    proc = run(["query", "goals", "--file", str(path), "--compact"])
    try:
        payload = json.loads(proc.stdout)
    except Exception:  # noqa: BLE001
        return "?"
    if not payload.get("ok"):
        return "not-parsable"
    return len(payload.get("data") or [])


class Lsp:
    """一个 LSP 进程，逐个文件问 `soko/goals` 与 `documentSymbol`。"""

    def __init__(self) -> None:
        self.child = subprocess.Popen(
            [NODE, str(SOKO), "lsp"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            cwd=str(ROOT),
        )
        self.buf = b""
        self.queue: list[dict] = []
        self.next_id = 1

    def send(self, message: dict) -> None:
        body = json.dumps(message).encode()
        self.child.stdin.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
        self.child.stdin.flush()

    def _pump(self) -> None:
        chunk = self.child.stdout.read1(65536)
        if not chunk:
            raise RuntimeError("LSP 退出")
        self.buf += chunk
        while True:
            sep = self.buf.find(b"\r\n\r\n")
            if sep < 0:
                return
            m = re.search(rb"Content-Length: (\d+)", self.buf[:sep], re.I)
            if not m:
                return
            length = int(m.group(1))
            if len(self.buf) < sep + 4 + length:
                return
            body = self.buf[sep + 4 : sep + 4 + length]
            self.buf = self.buf[sep + 4 + length :]
            self.queue.append(json.loads(body))

    def wait(self, predicate) -> dict:
        for message in self.queue:
            if predicate(message):
                self.queue.remove(message)
                return message
        while True:
            self._pump()
            for message in self.queue:
                if predicate(message):
                    self.queue.remove(message)
                    return message

    def request(self, method: str, params: dict) -> dict:
        rid = self.next_id
        self.next_id += 1
        self.send({"jsonrpc": "2.0", "id": rid, "method": method, "params": params})
        return self.wait(lambda m: m.get("id") == rid).get("result")

    def notify(self, method: str, params: dict) -> None:
        self.send({"jsonrpc": "2.0", "method": method, "params": params})

    def stop(self) -> None:
        try:
            self.child.kill()
        except Exception:  # noqa: BLE001
            pass


def measure_with_lsp(files: list[pathlib.Path], ops: bool = False) -> dict[pathlib.Path, dict]:
    lsp = Lsp()
    lsp.request("initialize", {
        "processId": None, "rootUri": ROOT.as_uri(), "capabilities": {},
        "workspaceFolders": [{"uri": ROOT.as_uri(), "name": "soko"}],
    })
    lsp.notify("initialized", {})
    out: dict[pathlib.Path, dict] = {}
    try:
        for path in files:
            uri = path.as_uri()
            text = path.read_text(encoding="utf-8")
            lsp.notify("textDocument/didOpen", {
                "textDocument": {"uri": uri, "languageId": "sokonanoda", "version": 1, "text": text},
            })
            lsp.wait(lambda m, u=uri: m.get("method") == "textDocument/publishDiagnostics"
                     and m.get("params", {}).get("uri") == u)
            goals = lsp.request("soko/goals", {"textDocument": {"uri": uri}}) or {}
            symbols = lsp.request("textDocument/documentSymbol", {"textDocument": {"uri": uri}})
            row = {
                "lsp_goals": len(goals.get("decls") or []),
                "symbols": None if symbols is None else len(symbols),
            }
            if ops:
                # `soko/nextHole`：洞导航（与 `goals` 同一条 `parsable()` 判据）。
                # 参数是 `position` + 可选 `forward`（`protocol.rs:119-126`）——
                # **不是** `offset`/`direction`（写错过一次，服务端直接反序列化失败）。
                row["next_hole"] = lsp.request("soko/nextHole", {
                    "textDocument": {"uri": uri},
                    "position": {"line": 0, "character": 0},
                    "forward": True,
                })
                # `soko/stateAt`：目标栏（读 `report`，不经 `parsable()`）。
                line = next(
                    (i for i, text_line in enumerate(text.splitlines())
                     if text_line.strip() == "sorry"),
                    None,
                )
                if line is None:
                    row["state_at"] = "（没有独立 sorry 行）"
                else:
                    state = lsp.request("soko/stateAt", {
                        "textDocument": {"uri": uri},
                        "position": {"line": line, "character": 2},
                    }) or {}
                    decl = (state.get("decl") or {}).get("name")
                    row["state_at"] = str(decl) if decl else "空"
            out[path] = row
            lsp.notify("textDocument/didClose", {"textDocument": {"uri": uri}})
    finally:
        lsp.stop()
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description="声明栏量具（计划 T-B01）")
    parser.add_argument("--root", action="append", default=None,
                        help=f"要量的路径（可重复；默认 {' '.join(DEFAULT_ROOTS)}）")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--cli", action="store_true", help="只用 CLI（量不到 documentSymbol）")
    parser.add_argument("--ops", action="store_true",
                        help="额外量 nextHole / stateAt / hints（T-B02；会慢一些）")
    args = parser.parse_args()

    if NODE is None:
        print("需要 node（scripts/soko 是 Node 启动器）", file=sys.stderr)
        return 2

    files = collect(args.root or DEFAULT_ROOTS)
    if not files:
        print("没有找到任何 .sokonanoda 文件", file=sys.stderr)
        return 2

    lsp_data = {} if args.cli else measure_with_lsp(files, ops=args.ops)

    rows = []
    for path in files:
        rel = str(path.relative_to(ROOT))
        cli = cli_goals(path)
        parses_alone = cli != "not-parsable"
        lsp_goals = lsp_data.get(path, {}).get("lsp_goals")
        symbols = lsp_data.get(path, {}).get("symbols")
        if lsp_goals is not None and symbols is not None and lsp_goals == 0 and symbols > 0:
            verdict = "**声明栏空**（报告好）"      # G-22
        elif lsp_goals is not None and symbols is not None and lsp_goals != symbols:
            verdict = "LSP goals 与 symbols 不一致"
        elif lsp_goals == 0 and symbols in (0, None):
            verdict = "真的没有声明"
        elif lsp_goals is None:
            verdict = "（只用 CLI，量不到 LSP 侧）"
        else:
            verdict = "正常"
        row = {"file": rel, "parses_alone": parses_alone, "cli_goals": cli,
               "lsp_goals": lsp_goals, "symbols": symbols, "verdict": verdict}
        if args.ops:
            data = lsp_data.get(path, {})
            row["next_hole"] = data.get("next_hole")
            row["state_at"] = data.get("state_at")
        rows.append(row)

    if args.json:
        print(json.dumps(rows, ensure_ascii=False, indent=2))
    else:
        print(f"{'文件':52s} {'单文件parse':>10s} {'CLI goals':>10s} {'LSP goals':>10s} {'symbols':>8s}  判定")
        print("-" * 122)
        for row in rows:
            lsp_goals = "-" if row["lsp_goals"] is None else str(row["lsp_goals"])
            symbols = "-" if row["symbols"] is None else str(row["symbols"])
            extra = ""
            if args.ops:
                hole = row.get("next_hole")
                hole = "有洞" if hole else "**无洞**"
                extra = f"  nextHole={hole} stateAt={row.get('state_at')}"
            print(
                f"{row['file']:52s} {'是' if row['parses_alone'] else '否':>10s} "
                f"{str(row['cli_goals']):>10s} {lsp_goals:>10s} {symbols:>8s}  {row['verdict']}{extra}"
            )
        broken = [r for r in rows if "声明栏空" in r["verdict"]]
        print()
        print(f"共 {len(rows)} 个文件；「声明栏空但报告好」{len(broken)} 个。")
        if broken:
            print("（这些就是 G-22 的现场：`parsable()` 把「单独 parse 失败但闭包好」判死了。）")

    return 1 if any("声明栏空" in r["verdict"] for r in rows) else 0


if __name__ == "__main__":
    raise SystemExit(main())
