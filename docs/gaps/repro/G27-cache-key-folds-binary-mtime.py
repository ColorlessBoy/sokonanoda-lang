#!/usr/bin/env python3
"""G-27 自断言复现：**编译缓存的键折进了「可执行文件的 mtime」**。

背景（调研发现，待本脚本判定）：`crates/front/src/compile/cache.rs` 的键里含
`build_stamp()` = **`current_exe()` 的 mtime（秒级）**。而 CLI（`sokonanoda`）与
LSP（`sokonanoda-lsp`）是**两个不同的可执行文件** ⇒ `sokonanoda build` 预热出来的
条目，能否被 LSP 命中，取决于两个二进制的 mtime 是否落在**同一秒**。

仓库里 staged 的那对恰好同秒，但那是"同一次 cargo build + 同一次 stage"的巧合，
**不是设计保证**（release 走 upload/download-artifact + 两份独立 tarball）。

本脚本把这件事变成机械判据：**同一份二进制内容、只有 mtime 不同**的两个副本，
用同一个缓存目录跑 `build` —— 第二次必须仍然命中。

退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
  0 = 缺口仍在（mtime 变了就 miss） · 1 = 已修（键与 mtime 无关） · 2 = 环境异常
"""

from __future__ import annotations

import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
SOKO = ROOT / "scripts" / "soko"
NODE = shutil.which("node")

LIB = "\n".join(
    [
        "def Set (α : Type) : Type := α -> Prop",
        "def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a",
        'infix:50 " ∈ " => Set.mem',
        "",
    ]
)
CANVAS = "\n".join(
    [
        "import SetLib",
        "",
        "theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h",
        "",
    ]
)
BUILD_SUMMARY = re.compile(r"(\d+)\s+hit,\s*(\d+)\s+compiled,\s*(\d+)\s+failed")


def soko(args: list[str], env_extra: dict[str, str]) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    for key in ("SOKONANODA_NO_CACHE", "SOKONANODA_BIN", "SOKONANODA_LSP_BIN"):
        env.pop(key, None)
    env.update(env_extra)
    return subprocess.run(
        [NODE, str(SOKO), *args], capture_output=True, text=True, env=env, cwd=str(ROOT)
    )


def main() -> int:
    if NODE is None:
        print("   → 需要 node（scripts/soko 是 Node 启动器）", file=sys.stderr)
        return 2

    resolved = soko(["version", "--json"], {})
    try:
        cli_path = pathlib.Path(json.loads(resolved.stdout)["cli"]["path"])
    except Exception as error:  # noqa: BLE001 - 形状不对就是环境问题
        print(f"   → 解析不出 CLI 路径：{error}；输出 = {resolved.stdout[:200]!r}", file=sys.stderr)
        return 2
    if not cli_path.exists():
        print(f"   → CLI 不存在：{cli_path}", file=sys.stderr)
        return 2

    workdir = pathlib.Path(tempfile.mkdtemp(prefix="g27-"))
    try:
        fixture = workdir / "proj"
        fixture.mkdir()
        (fixture / "SetLib.sokonanoda").write_text(LIB, encoding="utf-8")
        entry = fixture / "Canvas.sokonanoda"
        entry.write_text(CANVAS, encoding="utf-8")
        cache_dir = workdir / "cache"

        # 同一份二进制内容的两个副本，**只有 mtime 不同**。
        older = workdir / "sokonanoda-old"
        newer = workdir / "sokonanoda-new"
        shutil.copy2(cli_path, older)
        shutil.copy2(cli_path, newer)
        os.utime(older, (1_000_000_000, 1_000_000_000))  # 2001-09-09
        os.utime(newer, (1_700_000_000, 1_700_000_000))  # 2023-11-14
        for path in (older, newer):
            path.chmod(0o755)

        common = {"SOKONANODA_CACHE_DIR": str(cache_dir)}
        first = soko(["build", str(entry)], {**common, "SOKONANODA_BIN": str(older)})
        second = soko(["build", str(entry)], {**common, "SOKONANODA_BIN": str(newer)})

        m1 = BUILD_SUMMARY.search(first.stdout or "")
        m2 = BUILD_SUMMARY.search(second.stdout or "")
        if m1 is None or m2 is None:
            print(
                f"   → 读不出 build 摘要：{(first.stdout or '').strip()!r} / {(second.stdout or '').strip()!r}",
                file=sys.stderr,
            )
            return 2
        hits1 = int(m1.group(1))
        hits2, compiled2 = int(m2.group(1)), int(m2.group(2))

        print("== 同一份二进制内容、只有 mtime 不同：第二次 build 是否命中 ==")
        print(f"   mtime=2001 的副本（预热）= {hits1} hit")
        print(f"   mtime=2023 的副本（再跑）= {hits2} hit, {compiled2} compiled")

        if hits2 >= 1:
            print("结论：G-27 不存在——缓存键与可执行文件的 mtime 无关，跨二进制可复用。")
            return 1
        print(
            "结论：G-27 仍在——缓存键折进了可执行文件的 mtime，"
            "同一份二进制换个 mtime 就 miss ⇒ CLI 预热对 LSP 不保证有效。",
            file=sys.stderr,
        )
        return 0
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
