#!/usr/bin/env python3
"""启动器「VS Code 扩展自带」链接的守卫（洞 `SOKO-EXTENSION-LINK-UNWIRED`，2026-10-15 LINK-B）。

## 为什么需要它

`scripts/soko` 的头注释与 `AGENTS.md` 一直写着解析链含「VS Code 扩展自带」，
`extensionServer()` 正是为它写的函数、`canExec()` 也留着 `extension` 分支 ——
但**没有任何调用点**：`grep -n extensionServer scripts/soko` = 1 定义 / **0 调用**，
真实链 = override → repo-build → cache → download ⇒ 那一格是**死的** ✗。
后果是**产品行为**（不只是文档）：**通用 VSIX + 用户没跑过「Install CLI」+ 离线**
⇒ `scripts/soko lsp` 只能下载（或被 offline 拒绝），而盘上明明有一份能用的服务器 ✗。

⚠ **为什么不能只靠 `crates/cli/tests/launcher.rs`**：那是**冻结用例**，而且它的夹具
**不设 `HOME`**（`run_env` 只 `env_remove` 了几个 `SOKONANODA_*`）⇒ 它测不了
「扩展在场 / 不在场 / 版本对不对」这一格（本机装了同版本 VSIX 时反而会**偶然**走到）。
本守卫自带夹具：**临时 HOME + 临时缓存 + 空 `CARGO_TARGET_DIR` + `SOKONANODA_OFFLINE=1`**
⇒ 一个字节都不碰真环境 ✓（真 `~/.vscode` 里那 8 个扩展目录不参与）。

## 判据（四条，都是「用户动作 ⇒ 可见结果」）

| # | 夹具 | 期望 |
|---|---|---|
| C1 | 钉 0.90.0 + 扩展自带 0.90.0 + 空缓存 + 离线 | `version --json` 的 `lsp.source == "extension"` 且路径 = 自带那份；`scripts/soko lsp` **真的跑**它（哨兵 + rc 0） |
| C2 | 同上，扩展是 0.89.0（**与钉不匹配**） | `lsp.source != "extension"`；`lsp` **rc 3 且哨兵没跑**（`canExec` 不许松 ✓）；拒绝消息**点名**那份扩展（否则用户看着已装的 VSIX 无从理解 ✗） |
| C3 | 版本**匹配的缓存条目** + 匹配的扩展 | `lsp.source == "cache"` —— 缓存先于扩展（顺序与头注释 / opencode 插件一致 ✓） |
| C4 | 空缓存 + 匹配的扩展，走 **CLI**（`grade`） | CLI 仍是「什么都没解析到」（rc 3）—— 扩展那一格**只给语言服务器** ✓（文档承诺的就是它） |

⚠ `target` 从**启动器自己**的 `version --json` 读（不在这里复制一份平台映射 ✗）。
⚠ **Windows**：夹具里的哨兵是 `sh` 脚本（跑不了）⇒ C1/C3 的「真的跑」退化成
`doctor --json` 的 `lsp.ready == true`（**仍然**是「exec 闸门开了」的读数），
并**如实打印**这个降级 ✓（不是静默跳过 ✗）。

## 用法

    python3 scripts/check-launcher-extension-link.py             # 判据（gate）
    python3 scripts/check-launcher-extension-link.py --json      # 机器可读
    python3 scripts/check-launcher-extension-link.py --selftest  # 反向验证：把链接拆掉 ⇒ 必须判红

退出码：0 = 链接接着且四条都成立 · 1 = **判红**（链接断了 / 语义松了）·
2 = 形状异常（夹具造不出来 / 反向验证造不出反例）· 3 = 拿不到 node 或 `scripts/soko`
（与 gate 其它步同一条纪律：探不到就 exit 3，**绝不静默通过** ✗）。
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve()
REPO = HERE.parent.parent
LAUNCHER = REPO / "scripts" / "soko"

# 夹具自己的版本钉（夹具仓没有 `Cargo.toml` ⇒ 走 `sokonanoda-version.txt` ✓）。
# **有意不读仓库版本**：判据与"今天是哪个版本"无关 ⇒ 不会每次 bump 就假红 ✓。
PIN = "0.90.0"
MISMATCH = "0.89.0"

WINDOWS = sys.platform == "win32"
SENTINEL_EXT = "SENTINEL-EXTENSION-LSP-EXECUTED"
SENTINEL_CACHE = "SENTINEL-CACHE-LSP-EXECUTED"


def _env(home: Path, cache: Path, target_dir: Path) -> dict[str, str]:
    """最小环境：真环境里的 `SOKONANODA_*` / 真 `HOME` 一律不带进去 ✓。"""
    env = {
        "PATH": os.environ.get("PATH", ""),
        "HOME": str(home),
        "USERPROFILE": str(home),  # Windows 上启动器读的是它
        "SOKONANODA_CACHE_DIR": str(cache),
        "CARGO_TARGET_DIR": str(target_dir),
        "SOKONANODA_OFFLINE": "1",
    }
    if os.environ.get("SystemRoot"):
        env["SystemRoot"] = os.environ["SystemRoot"]  # node 在 Windows 上要它
    return env


class Fixture:
    """一个自足的假课程仓：`repo/scripts/soko` + 版本钉 + 假 HOME / 缓存 / 空 target。"""

    def __init__(self, root: Path, launcher: Path):
        self.root = root
        self.launcher = launcher
        self.home = root / "home"
        self.cache = root / "cache"
        self.empty_target = root / "empty-target"
        self.repo = root / "repo"
        for path in (self.home, self.cache, self.empty_target, self.repo / "scripts"):
            path.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(launcher, self.repo / "scripts" / "soko")
        (self.repo / "sokonanoda-version.txt").write_text(f"{PIN}\n", encoding="utf-8")

    def run(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["node", str(self.repo / "scripts" / "soko"), *args],
            capture_output=True,
            text=True,
            env=_env(self.home, self.cache, self.empty_target),
            timeout=120,
        )

    def binary_name(self, base: str) -> str:
        return f"{base}.exe" if WINDOWS else base

    def _sentinel(self, path: Path, marker: str) -> Path:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f'#!/bin/sh\necho "{marker}"\n', encoding="utf-8")
        path.chmod(0o755)
        return path

    def extension(self, version: str, target: str, marker: str) -> Path:
        """通用 VSIX 的安装目录形状（`editor/vscode/server.js:bundledBinaryPath`）。"""
        folder = self.home / ".vscode" / "extensions" / f"sokonanoda-lang.sokonanoda-{version}-{target}"
        return self._sentinel(folder / "bin" / target / self.binary_name("sokonanoda-lsp"), marker)

    def cache_entry(self, base: str, marker: str | None, sentinel: str) -> Path:
        binary = self._sentinel(self.cache / self.binary_name(base), sentinel)
        if marker is not None:
            (self.cache / f"{base}.version").write_text(f"{marker}\n", encoding="utf-8")
        return binary


def _version_json(fx: Fixture) -> dict:
    out = fx.run("version", "--json")
    if out.returncode != 0 or not out.stdout.strip():
        raise RuntimeError(f"`version --json` 没给出 JSON（rc={out.returncode}）: {out.stderr.strip()[:400]}")
    return json.loads(out.stdout)


def _exec_readings(fx: Fixture, sentinel: str) -> tuple[bool, str]:
    """「真的跑了自带那份」的读数；Windows 上退化成 `doctor --json` 的 `lsp.ready` ✓。"""
    if WINDOWS:
        out = fx.run("doctor", "--json")
        try:
            ready = bool(json.loads(out.stdout).get("lsp", {}).get("ready"))
        except (ValueError, AttributeError):
            ready = False
        return ready, f"doctor --json lsp.ready={ready}（Windows：sh 哨兵跑不了 ⇒ 只读闸门 ✓）"
    out = fx.run("lsp")
    ran = sentinel in out.stdout and out.returncode == 0
    return ran, f"`lsp` rc={out.returncode} stdout={out.stdout.strip()[:60]!r}"


def _same_path(a: str | None, b: Path) -> bool:
    if not a:
        return False
    return Path(a).resolve() == b.resolve()


def check_all(launcher: Path, root: Path) -> tuple[list[dict], str | None]:
    """四条判据。返回 (results, fatal)；`fatal` 非空 = 形状异常（exit 2）。"""
    results: list[dict] = []

    def add(cid: str, what: str, ok: bool, detail: str) -> None:
        results.append({"id": cid, "what": what, "ok": ok, "detail": detail})

    # `target` 从启动器自己读（不复制平台映射 ✓）
    probe = Fixture(root / "probe", launcher)
    target = _version_json(probe).get("target")
    if not target:
        return results, f"启动器没报出 target（`version --json` 的 `target` 为空）⇒ 夹具无法造"

    # C1 —— 洞本身：通用 VSIX + 空缓存 + 离线 ⇒ 必须走扩展自带
    fx = Fixture(root / "c1", launcher)
    bundled = fx.extension(PIN, target, SENTINEL_EXT)
    report = _version_json(fx)
    ran, reading = _exec_readings(fx, SENTINEL_EXT)
    add(
        "C1",
        f"空缓存 + 扩展自带 v{PIN}（= 钉）+ 离线 ⇒ lsp 走 extension 且真的跑它",
        report.get("lsp", {}).get("source") == "extension"
        and _same_path(report.get("lsp", {}).get("path"), bundled)
        and ran,
        f"source={report.get('lsp', {}).get('source')!r} path={report.get('lsp', {}).get('path')!r} · {reading}",
    )

    # C2 —— 不许松：版本不匹配的扩展**不能**被 exec，而且要被点名
    fx = Fixture(root / "c2", launcher)
    fx.extension(MISMATCH, target, SENTINEL_EXT)
    report = _version_json(fx)
    out = fx.run("lsp")
    named = "extension" in out.stderr and MISMATCH in out.stderr
    add(
        "C2",
        f"扩展自带 v{MISMATCH}（≠ 钉 v{PIN}）⇒ 不 exec + 拒绝时点名",
        report.get("lsp", {}).get("source") != "extension"
        and out.returncode == 3
        and SENTINEL_EXT not in out.stdout
        and named,
        f"source={report.get('lsp', {}).get('source')!r} rc={out.returncode} "
        f"点名={'yes' if named else 'NO'} stdout={out.stdout.strip()[:40]!r}",
    )

    # C3 —— 顺序：版本匹配的缓存条目仍然先赢（回归守卫）
    fx = Fixture(root / "c3", launcher)
    fx.extension(PIN, target, SENTINEL_EXT)
    cached = fx.cache_entry("sokonanoda-lsp", f"{PIN} {target}", SENTINEL_CACHE)
    report = _version_json(fx)
    ran, reading = _exec_readings(fx, SENTINEL_CACHE)
    add(
        "C3",
        "版本匹配的缓存条目 + 匹配的扩展 ⇒ lsp 仍走 cache（缓存先于扩展）",
        report.get("lsp", {}).get("source") == "cache"
        and _same_path(report.get("lsp", {}).get("path"), cached)
        and ran,
        f"source={report.get('lsp', {}).get('source')!r} · {reading}",
    )

    # C4 —— 范围：这一格只给语言服务器；CLI 不许因此从扩展里被 exec
    fx = Fixture(root / "c4", launcher)
    fx.extension(PIN, target, SENTINEL_EXT)
    report = _version_json(fx)
    out = fx.run("grade", "u.sokonanoda")
    add(
        "C4",
        "同一夹具走 CLI（grade）⇒ 仍然拒绝（扩展那一格只给 LSP）",
        "source" not in report.get("cli", {})
        and out.returncode == 3
        and SENTINEL_EXT not in out.stdout,
        f"cli.source={report.get('cli', {}).get('source')!r} rc={out.returncode}",
    )

    return results, None


def _print(results: list[dict], as_json: bool) -> None:
    if as_json:
        print(json.dumps({"results": results, "ok": all(r["ok"] for r in results)}, ensure_ascii=False, indent=2))
        return
    print("启动器「VS Code 扩展自带」链接守卫（scripts/soko）")
    for r in results:
        print(f"  {'✓' if r['ok'] else '✗'} {r['id']} {r['what']}")
        print(f"      {r['detail']}")
    bad = [r["id"] for r in results if not r["ok"]]
    print("  ⇒ " + ("全部成立 ✓" if not bad else f"判红：{', '.join(bad)} ✗"))


def _unwire(launcher: Path, dest: Path) -> bool:
    """把链接**拆掉**（正是洞的形状：函数在、调用点不在）—— 反向验证用。"""
    text = launcher.read_text(encoding="utf-8")
    wired = "      const bundled = this.extensionLink()\n      if (bundled) return bundled\n"
    if wired not in text:
        return False
    dest.write_text(text.replace(wired, "      // unwired (--selftest)\n"), encoding="utf-8")
    return True


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--json", action="store_true", help="机器可读")
    ap.add_argument("--selftest", action="store_true", help="反向验证：拆掉链接必须判红")
    args = ap.parse_args(argv)

    if shutil.which("node") is None:
        print("check-launcher-extension-link: 找不到 node —— 启动器就是 Node 脚本，判据无法跑（exit 3）✗", file=sys.stderr)
        return 3
    if not LAUNCHER.is_file():
        print(f"check-launcher-extension-link: 找不到 {LAUNCHER}（exit 3）✗", file=sys.stderr)
        return 3

    with tempfile.TemporaryDirectory(prefix="soko-ext-link-") as tmp:
        root = Path(tmp)
        if args.selftest:
            unwired = root / "soko-unwired.mjs"
            if not _unwire(LAUNCHER, unwired):
                print("check-launcher-extension-link --selftest: 造不出反例（找不到要拆的那两行）⇒ exit 2 ✗", file=sys.stderr)
                return 2
            results, fatal = check_all(unwired, root / "unwired")
            if fatal:
                print(f"check-launcher-extension-link --selftest: {fatal} ⇒ exit 2 ✗", file=sys.stderr)
                return 2
            _print(results, args.json)
            if all(r["ok"] for r in results):
                print("  ⇒ **咬不住**：把链接拆掉之后四条判据仍然全绿 ⇒ 这守卫是空的 ✗", file=sys.stderr)
                return 1
            print("  ⇒ 反向验证成立：拆掉链接 ⇒ C1 判红 ✓（守卫真的在咬）")
            return 0

        results, fatal = check_all(LAUNCHER, root / "live")
        if fatal:
            print(f"check-launcher-extension-link: {fatal} ⇒ exit 2 ✗", file=sys.stderr)
            return 2
        _print(results, args.json)
        return 0 if all(r["ok"] for r in results) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
