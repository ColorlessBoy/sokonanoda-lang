#!/usr/bin/env python3
"""生成 `site/data/site.json` —— 官网唯一一份机器可读事实（单页站点）。

设计：`docs/design/site-single-page.md`。只用 python3 标准库。

站点**永不手写版本号**：页面上所有版本号与下载链接都由 `assets/site.js`
从这份 JSON 回填（占位符 `{v}`），而这里的版本号取自**版本钉**
（`Cargo.toml` 的 `[workspace.package].version`）。

为什么改成版本钉（2026-10-24）：旧口径是"最新的已发布 tag"，它有一个**固定点**
——tag 由 `auto-tag` 从 bump 那个 commit 打出，而 bump 提交里 `site.json` 必然
还是旧版 ⇒ 打出来的 tag **自带**一条 `check-site data` 红（或要人手工补一条
"发版后重跑生成器"的提交，`docs/RELEASE.md` §6.5，已漏过两次）。
现在 `scripts/bump.py` 在写版本号的同时把 `site.json` 写成**同一个版本**，tag
由同一个 commit 打出 ⇒ `tag == Cargo.toml == site.json` 由构造保证。

用法：

```bash
python3 scripts/gen-site-data.py            # 写入 site/data/site.json
python3 scripts/gen-site-data.py --check    # 只比对，不写；不一致 exit 1
python3 scripts/gen-site-data.py --print    # 打到 stdout
```

解析不出 tag 时**不编造**：`version` 字段留空，页面保持中性文字
（"当前发布版本"），`--check` 在那个情况下判 **exit 3**（无法判定 ≠ 绿）。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(REPO_ROOT, "site", "data", "site.json")
ARENA = os.path.join(REPO_ROOT, "site", "data", "arena.json")
REPO_URL = "https://github.com/ColorlessBoy/sokonanoda-lang"

TAG_RE = re.compile(r"^v(\d+\.\d+\.\d+)$")


def latest_release_tag() -> str | None:
    """最新的 `vX.Y.Z` tag（按版本序，不是按字典序）。

    只看**本仓库**的 tag。浅克隆（depth 1）里没有 tag ⇒ 返回 None，
    此时绝不退回 `Cargo.toml`（那正是要避免的漂移）。
    """
    try:
        proc = subprocess.run(
            ["git", "tag", "--list", "v*", "--sort=-v:refname"],
            cwd=REPO_ROOT, capture_output=True, text=True, timeout=30,
            env=git_env.clean_git_env(),
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if proc.returncode != 0:
        return None
    for line in proc.stdout.splitlines():
        tag = line.strip()
        if TAG_RE.match(tag):
            return tag
    return None


def cargo_version() -> str | None:
    """版本钉：`Cargo.toml` 的 `[workspace.package].version`（站点写的就是它）。"""
    try:
        text = open(os.path.join(REPO_ROOT, "Cargo.toml"), encoding="utf-8").read()
    except OSError:
        return None
    match = re.search(r'^version = "([0-9]+\.[0-9]+\.[0-9]+)"', text, re.MULTILINE)
    return match.group(1) if match else None


def arena_facts() -> dict | None:
    """**官方最新语料的一次通过结果**（`site/data/arena.json`，由 arena-gate 写）。

    缺文件/没通过 ⇒ 返回 None，站点显示中性占位（不编造数字）。
    """
    try:
        with open(ARENA, encoding="utf-8") as handle:
            raw = json.load(handle)
    except (OSError, json.JSONDecodeError):
        return None
    if raw.get("passed") is not True:
        return None
    try:
        return {
            "corpus": f"master@{str(raw['revision'])[:9]}",
            "revision": str(raw["revision"]),
            "run_id": str(raw["run_id"]),
            "created_at": raw.get("created_at"),
            "good": int(raw["good"]),
            "bad": int(raw["bad"]),
            "accepted": int(raw["accepted"]),
            "rejected": int(raw["rejected"]),
            "declined": int(raw["declined"]),
            "known_unsupported": int(raw.get("known_unsupported", 0)),
            "known_slow": int(raw.get("known_slow", 0)),
            "slow_timeouts": int(raw.get("slow_timeouts", 0)),
        }
    except (KeyError, TypeError, ValueError):
        return None


def build(version: str | None = None) -> dict:
    """组装事实字典。字段少了就是源缺了——**不补默认值**。"""
    version = version or cargo_version()
    tag = f"v{version}" if version else None
    data: dict = {"schema": "soko.site/2"}
    if tag:
        data["tag"] = tag
        data["version"] = tag[1:]
        data["release_url"] = f"{REPO_URL}/releases/tag/{tag}"
    arena = arena_facts()
    if arena:
        data["arena"] = arena
    return data


def render(data: dict) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="生成 site/data/site.json")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--check", action="store_true", help="只比对磁盘上的文件，不一致 exit 1")
    group.add_argument("--print", dest="to_stdout", action="store_true", help="打到 stdout")
    parser.add_argument("--version", metavar="X.Y.Z", help="显式指定版本（bump.py 用；默认读 Cargo.toml）")
    args = parser.parse_args(argv)

    data = build(args.version)
    text = render(data)

    if not data.get("version"):
        print("site data: 解析不出版本钉（Cargo.toml 的 [workspace.package].version）—— 不写、不判绿。", file=sys.stderr)
        return 3

    if args.to_stdout:
        sys.stdout.write(text)
        return 0

    if args.check:
        try:
            on_disk = open(OUT, encoding="utf-8").read()
        except OSError as error:
            print(f"site data: 读不到 {OUT}：{error}", file=sys.stderr)
            return 1
        if on_disk != text:
            print(f"site data: {os.path.relpath(OUT, REPO_ROOT)} 与版本钉不一致"
                  f"（磁盘上是 {json.loads(on_disk).get('version')!r}，"
                  f"版本钉是 {data['version']!r}）—— 跑 python3 scripts/bump.py <版本> 或 "
                  f"python3 scripts/gen-site-data.py",
                  file=sys.stderr)
            return 1
        print(f"site data: ok（{data['tag']}）")
        return 0

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as handle:
        handle.write(text)
    print(f"site data: 写入 {os.path.relpath(OUT, REPO_ROOT)} —— {data['tag']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
