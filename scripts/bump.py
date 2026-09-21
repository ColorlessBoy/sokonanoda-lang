#!/usr/bin/env python3
"""**版本一致**：bump 一处，全仓库跟着走（T-A08 / 用户判定的根因）。

用户原话：

> 开发没有默认提升这个 project 的 requires，这是根因。这个项目本身就和主项目
> 共同开发，版本本来应该一样的。

`requires` 是**手写**的，bump 时没人提升它 ⇒ 漂移。漂移本身只是 warning，
但它曾经让 `is_clean()` 为假 ⇒ **整个项目缓存永不写**（G-24；T-A05 已经让它
不再致命，但漂移本身还在，而且每个清单各写各的迟早会再漂）。

这个脚本是**单一来源**：版本号只在这里被写进去，别处都从它读。

    python3 scripts/bump.py 0.63.4          # 写四处 + 所有清单的 requires
    python3 scripts/bump.py 0.63.4 --check  # 只检查，不写（= check-manifests.py）

写的位置：

1. `Cargo.toml` 的 `[workspace.package] version`；
2. `editor/vscode/package.json` 的 `version`（CI 强制两者**相等**）；
3. `Cargo.lock` 里 workspace 自己的那几个包；
4. **仓库里每个声明了 `requires` 的 `sokonanoda.toml`**（`git ls-files` 得到，
   所以 `.cache/` 里那些不会被碰）。

`editor/vscode/CHANGELOG.md` **不**由它写：那是人写的（写清楚这一版改了什么），
脚本只负责"数字一致"。

退出码：0 = 一致/已写；1 = 有漂移（`--check`）；2 = 用法/环境错误。
"""

from __future__ import annotations

import argparse
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CARGO = ROOT / "Cargo.toml"
PACKAGE_JSON = ROOT / "editor" / "vscode" / "package.json"
LOCK = ROOT / "Cargo.lock"
VERSION_RE = re.compile(r'^version = "([^"]+)"', re.M)
REQUIRES_RE = re.compile(r'^(\s*requires\s*=\s*)"([^"]*)"', re.M)


def repo_version() -> str:
    match = VERSION_RE.search(CARGO.read_text(encoding="utf-8"))
    if not match:
        raise SystemExit("error: Cargo.toml 里找不到 version")
    return match.group(1)


def manifests() -> list[pathlib.Path]:
    """仓库里**被跟踪**的 `sokonanoda.toml`（`git ls-files` ⇒ 不含 `.cache/`）。"""
    out = subprocess.run(
        ["git", "ls-files", "*sokonanoda.toml"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return [ROOT / line for line in out.splitlines() if line.strip()]


def declared() -> list[tuple[pathlib.Path, str]]:
    """声明了 `requires` 的清单：`(路径, 值)`。"""
    found = []
    for path in manifests():
        text = path.read_text(encoding="utf-8")
        match = REQUIRES_RE.search(text)
        if match:
            found.append((path, match.group(2)))
    return found


def check() -> int:
    version = repo_version()
    problems: list[str] = []

    package = PACKAGE_JSON.read_text(encoding="utf-8")
    if f'"version": "{version}"' not in package:
        got = re.search(r'"version":\s*"([^"]+)"', package)
        problems.append(
            f"editor/vscode/package.json 的 version = {got.group(1) if got else '?'}"
            f"（CI 强制它与 Cargo.toml 相等：{version}）"
        )

    # `requires` 的语义只比 major.minor（`manifest.rs::version_warning`），
    # 但**同仓共同开发**的清单应当钉**完整版本**：这样 bump 时漏掉一个就会被抓住。
    for path, value in declared():
        if value != version:
            problems.append(
                f"{path.relative_to(ROOT)} 的 requires = {value}（应为 {version}）"
            )

    if problems:
        print(f"版本漂移（仓库版本 {version}）：", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        print(
            "\n修：python3 scripts/bump.py " + version + "（写全仓库，一处来源）",
            file=sys.stderr,
        )
        return 1

    total = len(declared())
    print(f"版本一致：{version}（Cargo.toml = package.json；{total} 个清单的 requires）")
    return 0


def write(version: str) -> int:
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        print(
            f"error: 版本必须是纯 x.y.z（带后缀会让 scripts/soko 按 G-16 拒绝运行）：{version}",
            file=sys.stderr,
        )
        return 2

    old = repo_version()
    CARGO.write_text(
        VERSION_RE.sub(f'version = "{version}"', CARGO.read_text(encoding="utf-8"), count=1),
        encoding="utf-8",
    )
    package = PACKAGE_JSON.read_text(encoding="utf-8")
    PACKAGE_JSON.write_text(
        re.sub(r'"version":\s*"[^"]+"', f'"version": "{version}"', package, count=1),
        encoding="utf-8",
    )
    # `Cargo.lock` 里**只改 workspace 自己的那几个包**。
    #
    # 陷阱（实测踩到）：`name = "sokonanoda[^"]*"` 会连**内核**一起匹配——
    # 内核在 lock 里就叫 `sokonanoda`，而它的版本是**独立**的（`0.5.0`，
    # 跟的是它自己那条线，`crates/kernel/Cargo.toml` 里写死的）。第一版正则把
    # 它改成了 0.63.3，`cargo test --locked` 立刻拒绝。
    # 用**精确的名字表**，不用前缀。
    lock = LOCK.read_text(encoding="utf-8")
    for name in ("sokonanoda-front", "sokonanoda-cli", "sokonanoda-lsp"):
        lock = re.sub(
            rf'(name = "{re.escape(name)}"\nversion = )"[^"]+"',
            rf'\g<1>"{version}"',
            lock,
        )
    LOCK.write_text(lock, encoding="utf-8")
    touched = 0
    for path, _ in declared():
        text = path.read_text(encoding="utf-8")
        path.write_text(
            REQUIRES_RE.sub(rf'\g<1>"{version}"', text, count=1), encoding="utf-8"
        )
        touched += 1

    print(f"{old} → {version}")
    print(f"  Cargo.toml · editor/vscode/package.json · Cargo.lock · {touched} 个清单的 requires")
    print("  别忘了手写 editor/vscode/CHANGELOG.md（脚本只管数字一致）")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="版本单一来源（T-A08）")
    parser.add_argument("version", nargs="?", help="新的 x.y.z（省略则只检查）")
    parser.add_argument("--check", action="store_true", help="只检查，不写")
    args = parser.parse_args()

    if args.version is None or args.check:
        return check()
    return write(args.version)


if __name__ == "__main__":
    raise SystemExit(main())
