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
        hits1, compiled1, failed1 = (int(m1.group(i)) for i in (1, 2, 3))
        hits2, compiled2, failed2 = (int(m2.group(i)) for i in (1, 2, 3))

        print("== 同一份二进制内容、只有 mtime 不同：第二次 build 是否命中 ==")
        print(f"   mtime=2001 的副本（预热）= {hits1} hit, {compiled1} compiled, {failed1} failed")
        print(f"   mtime=2023 的副本（再跑）= {hits2} hit, {compiled2} compiled, {failed2} failed")

        # ⭐ **预热那跑什么都没缓存 ⇒ 观测不到主体 ⇒ exit 2**（2026-10-15，洞
        # `G27-ZERO-HITS-PROVES-NOTHING`；同族 `G25-PROBE-DEGENERATE-IN-CI` 与
        # `G112-REPRO-STALE-BLIND-SPOT` 的"先核前提，前提不成立就不许下结论"）。
        #
        # 判据 `hits2 == 0`（"换个 mtime 就 miss"）的**前提**是：预热那跑**真的往缓存里
        # 写了东西** ✓。`hits1 == 0 && compiled1 == 0` 说明预热**一条都没写**
        # （`total = hit + compiled + failed`，`crates/cli/src/build.rs:862` ⇒ 三者全 0
        # 就是"这次 build 一个文件都没处理"）⇒ 缓存目录本来就是空的 ⇒ 第二次 `hits2 == 0`
        # 是**必然**的，与"键里有没有 mtime"**毫无关系** ✗。
        # 旧写法在这种形状下**无条件**报「G-27 仍在」（`exit 0`）✗ —— 把"我没预热成功"
        # 说成了"缺口还在"；而台账是契约（`fixed` ⇒ 复现件应转绿）⇒ 判红 ✗。
        # 这正是台账 `today` 里那条「`0 hit, 0 compiled` 却报仍复现」的形状 ✓。
        # ⇒ 现在：**说不清观测到什么，就不许下结论**；把"看到了什么"原样带出来 ✓。
        # ⚠ **判据一条都没松**：预热真的写了东西时仍走下面那条 `hits2 >= 1`，
        # 真缺口照样 `exit 0` ✓（正对照读数见 `.cache/ux/fix-jbZ/VERDICT.md`）。
        if hits1 == 0 and compiled1 == 0:
            print(
                "结论：**观测不到主体** ⇒ 环境/形状异常（exit 2），**不是**「缺口仍在」✗\n"
                f"   · 预热那跑（mtime=2001 的副本）= {hits1} hit, {compiled1} compiled, {failed1} failed\n"
                "     ⇒ 它**一条缓存都没写**（`hit + compiled + failed` 就是这次 build 处理的文件数）\n"
                "   · 于是缓存目录是空的 ⇒ 第二次 `0 hit` 是**必然**的，证明不了键里有没有 mtime ✗\n"
                "   · 常见原因（按可能性）：预热那次 build **失败**（看上面的 `failed` 数：夹具被\n"
                "     编译器拒 / 入口读不到）· 入口路径或模块根解析不到 · `SOKONANODA_CACHE_DIR`\n"
                "     不可写 · `scripts/soko` 解析到的不是本仓库构建\n"
                "   · 先查：照抄夹具手工跑一次 `build`，读那行 "
                "`built N file(s) — H hit, C compiled, F failed`（`failed > 0` 时紧跟失败明细）\n",
                file=sys.stderr,
            )
            return 2

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
