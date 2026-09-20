#!/usr/bin/env python3
"""生成 `site/data/site.json` —— 官网唯一一份机器可读事实（单页站点）。

设计：`docs/design/site-single-page.md`。只用 python3 标准库。

站点**永不手写版本号**：页面上所有版本号与下载链接都由 `assets/site.js`
从这份 JSON 回填（占位符 `{v}`），而这里的版本号取自**最新的已发布 tag**
——不是 `Cargo.toml`。

为什么是 tag 而不是 `Cargo.toml`：站点写的是**已发布版本的事实**。本仓库
常有并行开发，`Cargo.toml` 会在 tag 之前就 bump 到下一个版本；照抄它就会
写出一个"没有 tag、没有产物、没有下载 URL"的版本号（2026-09-20 实测踩过，
详见 `docs/design/site-rebuild/STATE.md` §5 与 `#13`）。

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

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(REPO_ROOT, "site", "data", "site.json")
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


def build() -> dict:
    """组装事实字典。字段少了就是源缺了——**不补默认值**。"""
    tag = latest_release_tag()
    data: dict = {"schema": "soko.site/2"}
    if tag:
        data["tag"] = tag
        data["version"] = tag[1:]
        data["release_url"] = f"{REPO_URL}/releases/tag/{tag}"
    return data


def render(data: dict) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="生成 site/data/site.json")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--check", action="store_true", help="只比对磁盘上的文件，不一致 exit 1")
    group.add_argument("--print", dest="to_stdout", action="store_true", help="打到 stdout")
    args = parser.parse_args(argv)

    data = build()
    text = render(data)

    if not data.get("version"):
        print("site data: 解析不出已发布的 vX.Y.Z tag —— 不写、不判绿。", file=sys.stderr)
        print("  （浅克隆缺 tag 时先 `git fetch --tags`；CI 里 checkout 需 fetch-depth: 0）",
              file=sys.stderr)
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
            print(f"site data: {os.path.relpath(OUT, REPO_ROOT)} 与最新 tag 不一致"
                  f"（磁盘上是 {json.loads(on_disk).get('version')!r}，"
                  f"最新 tag 是 {data['version']!r}）—— 跑 python3 scripts/gen-site-data.py",
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
