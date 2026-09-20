#!/usr/bin/env python3
"""Render site pages to PNG with headless Chrome — the maintainer's eyes.

Why this exists: the site is judged on how it *looks*, but an agent writing it
cannot see. This script turns "does it look right" into files a human can open
in one command, and it is the input to the design checkpoints recorded in
`docs/design/site-rebuild/`.

It is a **maintainer tool**, like `scripts/gen-site-demos.py` (which needs
Pillow): the user-facing path stays zero-toolchain. It needs only python3
stdlib plus a Chrome/Chromium/Edge binary on the machine.

Usage:
  python3 scripts/site-shot.py                       # default pages × widths
  python3 scripts/site-shot.py site/index.html       # one page, default widths
  python3 scripts/site-shot.py site/index.html --width 390 --out .cache/shots
  python3 scripts/site-shot.py --list                # show the default matrix

Exit codes: 0 = every requested shot was written; 1 = at least one failed;
2 = usage error (no browser found). Failures are loud and per-page: a missing
screenshot must never look like a successful run.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

# 单张截图的墙钟上限。Chrome 冷启动 + 等字体，实测 3–6 秒；给足余量。
SHOT_TIMEOUT_S = 60

# macOS 上 headless Chrome 把**窗口**宽度钳在 500px：`--window-size=390,1200`
# 产出的 PNG 是 500px 布局的左侧裁切，不是 390px 的布局（实测：390 截图与
# 500 截图比左 390 列，22360/22360 像素全同）。所以窄屏不能用窗口宽度做，
# 否则"390px 没溢出"是假结论。窄屏改用一个定宽 iframe 包一层——iframe 有
# 独立视口，不受窗口钳制；窗口本身开到 WRAPPER_WIDTH 即可。
MIN_WINDOW_WIDTH = 500
WRAPPER_WIDTH = 520

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"

# Chrome 的候选路径：先环境变量，再各平台常见位置。找不到就 exit 2，
# 绝不静默跳过（"没截图" 和 "截图没问题" 必须能区分开）。
BROWSER_CANDIDATES = (
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/usr/bin/microsoft-edge",
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
)

# 默认矩阵：桌面 + 平板 + 手机。三档就够暴露"只在一种宽度下成立"的布局。
DEFAULT_WIDTHS = (1440, 900, 390)

DEFAULT_PAGES = (
    "index.html",
    "get-started.html",
    "language.html",
    "kernel.html",
    "editor.html",
    "course.html",
    "set-theory.html",
    "progress.html",
    "about.html",
    "en/index.html",
)


def find_browser() -> str | None:
    env = os.environ.get("SOKO_CHROME") or os.environ.get("CHROME_PATH")
    if env and Path(env).exists():
        return env
    for candidate in BROWSER_CANDIDATES:
        if Path(candidate).exists():
            return candidate
    for name in ("google-chrome", "chromium", "chrome", "msedge"):
        found = shutil.which(name)
        if found:
            return found
    return None


def shoot(browser: str, page: Path, width: int, out: Path, height: int) -> bool:
    """One Chrome invocation, one PNG. Returns success.

    Chrome 写完 PNG 后**不一定退出**（实测：图已落盘，进程仍挂到超时）。
    所以判定依据是"文件出现且非空"，不是"进程退出码"——拿退出码当判据会
    把成功的截图报成失败。进程在文件稳定后主动收掉。
    """
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.exists():
        out.unlink()

    narrow = width < MIN_WINDOW_WIDTH
    # 每次用独立的 profile：共用 profile 会撞 SingletonLock，而且上一次的
    # 未退出实例会让下一次直接 abort（实测踩过）。
    with tempfile.TemporaryDirectory(prefix="soko-shot-") as profile:
        if narrow:
            # 窄屏必须走定宽 iframe：窗口宽度被钳在 500px，用窗口宽度当窄屏
            # 拍出来的是"500px 布局的左侧裁切"，会让"390px 没溢出"变成假结论。
            # iframe 有独立视口，不受窗口钳制。
            wrapper = Path(profile) / "wrapper.html"
            wrapper.write_text(
                "<!DOCTYPE html><meta charset='utf-8'>"
                "<style>html,body{margin:0;background:#f5f7f9}"
                f"iframe{{display:block;width:{width}px;height:{height}px;border:0}}"
                "</style>"
                f"<iframe src='{page.resolve().as_uri()}'></iframe>",
                encoding="utf-8",
            )
            target_uri = wrapper.resolve().as_uri()
            window_width = WRAPPER_WIDTH
        else:
            target_uri = page.resolve().as_uri()
            window_width = width

        cmd = [
            browser,
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-extensions",
            "--disable-crash-reporter",
            "--disable-background-networking",
            "--hide-scrollbars",
            "--force-device-scale-factor=1",
            "--allow-file-access-from-files",
            f"--user-data-dir={profile}",
            f"--window-size={window_width},{height}",
            # 等字体与图片落定再拍；虚拟时间预算比 sleep 稳（无头下没有真实帧）。
            "--virtual-time-budget=4000",
            f"--screenshot={out}",
            target_uri,
        ]
        proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        deadline = time.monotonic() + SHOT_TIMEOUT_S
        settled = 0
        try:
            while time.monotonic() < deadline:
                if out.exists() and out.stat().st_size > 0:
                    # 文件大小连续两次读数一致 = 写完收笔（PNG 是流式写入的）。
                    size = out.stat().st_size
                    time.sleep(0.25)
                    if out.stat().st_size == size:
                        settled = size
                        break
                if proc.poll() is not None:
                    break
                time.sleep(0.2)
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()
            proc.stderr.close() if proc.stderr else None
        if not settled:
            print(f"  ✗ no output: {page.name} @ {width}", file=sys.stderr)
            return False
        return True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("pages", nargs="*", help="page paths relative to the repo root")
    parser.add_argument("--width", type=int, action="append", dest="widths",
                        help="viewport width; repeatable (default 1440/900/390)")
    parser.add_argument("--height", type=int, default=1200, help="viewport height")
    parser.add_argument("--out", default=".cache/shots", help="output directory")
    parser.add_argument("--list", action="store_true", help="list the default matrix")
    args = parser.parse_args()

    widths = tuple(args.widths) if args.widths else DEFAULT_WIDTHS

    if args.list:
        print("widths:", ", ".join(str(w) for w in widths))
        for page in DEFAULT_PAGES:
            print(" ", page)
        return 0

    pages = args.pages or list(DEFAULT_PAGES)
    browser = find_browser()
    if browser is None:
        print(
            "site-shot: no Chrome/Chromium/Edge found. Set SOKO_CHROME=<path>.",
            file=sys.stderr,
        )
        return 2

    out_root = ROOT / args.out
    failures = 0
    for raw in pages:
        page = ROOT / raw
        if not page.exists():
            print(f"site-shot: missing page {raw}", file=sys.stderr)
            failures += 1
            continue
        for width in widths:
            name = page.stem if page.parent.name == "site" else f"{page.parent.name}-{page.stem}"
            out = out_root / f"{name}-{width}.png"
            if shoot(browser, page, width, out, args.height):
                print(f"  ✓ {out.relative_to(ROOT)}")
            else:
                failures += 1

    if failures:
        print(f"site-shot: {failures} shot(s) failed", file=sys.stderr)
        return 1
    print(f"site-shot: ok -> {out_root.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
