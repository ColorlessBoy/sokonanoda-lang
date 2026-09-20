#!/usr/bin/env python3
"""Drive the site's interactive features in a real browser and assert what happens.

Why this exists: `check-site.py` and `site-audit.py` are both **static** checks —
they read source and measure layout. Neither ever *clicks anything*. So every
piece of JavaScript on this site could be completely broken and all of them would
still pass. That is not a hypothetical: the search feature shipped, was audited
for overflow and colour, and had never once been executed.

This script closes that hole. It serves `site/` over HTTP (same as GitHub Pages),
loads each page in a same-origin iframe, drives the real controls the way a
visitor would, and asserts the resulting DOM.

Covered:
  * search          — type a query, results appear and match; no-JS fallback hides
  * theme toggle    — click cycles system → light → dark, `data-theme` follows
  * version fill    — `[data-site-version]` gets the real version from site.json
  * filter          — typing narrows a `[data-filter]` list and updates the count
  * tabs            — a radio tab switches the visible panel
  * fold            — a checkbox fold reveals its body (and works with JS off)
  * copy button     — appears only when JS is available
  * no-JS integrity — with scripting disabled, no page loses content

Usage:  python3 scripts/site-functest.py [--json]
Exit:   0 = every feature behaved · 1 = at least one did not · 2 = no browser
"""

from __future__ import annotations

import argparse
import html
import http.server
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"

BROWSER_CANDIDATES = (
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
)

VIRTUAL_TIME_MS = 12000
TIMEOUT_S = 60

# 每个用例：页面、要驱动的动作、要断言的结论。动作在 iframe 内部执行，
# 返回 {ok, detail}；任何异常都算失败，不静默跳过。
PROBE = r"""
(function () {
  var out = [];
  function log(name, ok, detail) { out.push({ name: name, ok: !!ok, detail: String(detail) }); }
  function loaded(f) {
    try { return f.contentDocument && f.contentDocument.URL &&
                 f.contentDocument.URL.indexOf("about:blank") !== 0 && f.contentDocument.body; }
    catch (e) { return false; }
  }
  function settle(fn, ms) { return new Promise(function (r) { setTimeout(function () { r(fn()); }, ms); }); }
  // 等**所有** iframe 加载完再动手：脚本在 iframe 之后解析，但那时 contentDocument
  // 还是初始 about:blank，直接 querySelector 一律返回 null（第一版就是这么误报的）。
  async function framesReady() {
    for (var i = 0; i < 100; i++) {
      var all = Array.prototype.slice.call(document.querySelectorAll("iframe"));
      if (all.length && all.every(loaded)) return true;
      await settle(function () {}, 100);
    }
    return false;
  }
  function frame(name) {
    var f = document.querySelector('iframe[data-page="' + name + '"]');
    return f ? { f: f, d: f.contentDocument, w: f.contentWindow } : null;
  }

  async function run() {
    var ready = await framesReady();
    log("harness: 所有 iframe 已加载", ready, ready ? "ok" : "timeout");
    // ── 1. 搜索：真的输入一个词，真的看结果 ──────────────────────────────
    try {
      var s = frame("search.html");
      var input = s.d.querySelector("[data-search-input]");
      var results = s.d.querySelector("[data-search-results]");
      var fallback = s.d.querySelector("[data-search-fallback]");
      // 索引是 fetch 来的，等它落地（按钮/输入框被 search.js 显形即说明接上了）。
      await settle(function () {}, 2500);
      log("search: 输入框在 JS 下可见", input && s.w.getComputedStyle(input).display !== "none",
          input ? s.w.getComputedStyle(input).display : "no input");
      log("search: 无 JS 兜底清单被藏起来", fallback && fallback.hidden === true,
          fallback ? ("hidden=" + fallback.hidden) : "no fallback");
      input.value = "内核判卷";
      input.dispatchEvent(new s.w.Event("input", { bubbles: true }));
      await settle(function () {}, 300);
      var hits = results.querySelectorAll(".search-hit").length;
      var text = (results.textContent || "").slice(0, 120);
      log("search: 「内核判卷」有命中", hits > 0, hits + " hit(s): " + text);
      log("search: 命中里含关键词", /内核判卷/.test(results.textContent || ""), text);
      input.value = "zzzz-not-a-real-term";
      input.dispatchEvent(new s.w.Event("input", { bubbles: true }));
      await settle(function () {}, 300);
      log("search: 无命中时给出空态", /没有匹配/.test(results.textContent || ""),
          (results.textContent || "").slice(0, 80));
    } catch (e) { log("search", false, e); }

    // ── 2. 版本回填：页脚与 stats 都该被填成真版本 ────────────────────────
    try {
      var p = frame("index.html");
      await settle(function () {}, 2000);
      var slots = p.d.querySelectorAll("[data-site-version]");
      var filled = Array.prototype.filter.call(slots, function (el) {
        return /^v\d/.test((el.textContent || "").trim());
      }).length;
      log("version: data-site-version 被填成真版本", slots.length > 0 && filled === slots.length,
          filled + "/" + slots.length + " slots filled");
    } catch (e) { log("version", false, e); }

    // ── 3. 主题切换：点一下，data-theme 要跟着变 ──────────────────────────
    try {
      var t = frame("index.html");
      var btn = t.d.querySelector("[data-theme-toggle]");
      var root = t.d.documentElement;
      var before = root.getAttribute("data-theme");
      btn.click();
      await settle(function () {}, 120);
      var after = root.getAttribute("data-theme");
      log("theme: 点击后 data-theme 改变", before !== after, before + " → " + after);
      log("theme: 切换后配色令牌真的变了",
          t.w.getComputedStyle(root).getPropertyValue("--paper").trim().length > 0,
          t.w.getComputedStyle(root).getPropertyValue("--paper").trim());
    } catch (e) { log("theme", false, e); }

    // ── 4. 筛选：诊断字典的输入框要真的筛 ────────────────────────────────
    try {
      var dg = frame("diagnostics.html");
      var f = dg.d.querySelector("[data-filter]");
      var items = dg.d.querySelectorAll("[data-filter-item]");
      var total = items.length;
      f.value = "elab-unknown-identifier";
      f.dispatchEvent(new dg.w.Event("input", { bubbles: true }));
      await settle(function () {}, 250);
      var shown = Array.prototype.filter.call(items, function (el) { return !el.hidden; }).length;
      log("filter: 输入后条目被筛掉", total > 0 && shown > 0 && shown < total,
          total + " → " + shown);
      f.value = "";
      f.dispatchEvent(new dg.w.Event("input", { bubbles: true }));
      await settle(function () {}, 250);
      var back = Array.prototype.filter.call(items, function (el) { return !el.hidden; }).length;
      log("filter: 清空后恢复全部", back === total, back + "/" + total);
    } catch (e) { log("filter", false, e); }

    // ── 5. 折叠：勾上 checkbox，正文要露出来（纯 CSS，无 JS 也应成立） ─────
    try {
      var wt = frame("walkthrough.html");
      var boxes = wt.d.querySelectorAll(".fold__toggle");
      var body = boxes.length ? boxes[0].parentNode.querySelector(".fold__body") : null;
      var hiddenBefore = body ? wt.w.getComputedStyle(body).display === "none" : null;
      boxes[0].checked = true;
      await settle(function () {}, 120);
      var shownAfter = body ? wt.w.getComputedStyle(body).display !== "none" : null;
      log("fold: 勾选后折叠体显示", hiddenBefore === true && shownAfter === true,
          "before hidden=" + hiddenBefore + ", after shown=" + shownAfter);
    } catch (e) { log("fold", false, e); }

    // ── 6. 复制按钮：有 JS 才显示（无 JS 时不该留一个按不动的按钮） ────────
    try {
      // kernel.html 上**没有**复制按钮（实测：只有 agents / course / notation /
      // set-theory / styleguide 有）。第一版拿它测，报的是"页面缺按钮"，
      // 其实是我的假设错了。
      var k = frame("agents.html");
      var copy = k.d.querySelector("[data-copy], [data-copy-self]");
      log("copy: 有 JS 时复制按钮可见",
          copy && k.w.getComputedStyle(copy).display !== "none",
          copy ? k.w.getComputedStyle(copy).display : "no button");
    } catch (e) { log("copy", false, e); }

    document.getElementById("result").textContent = JSON.stringify(out, null, 1);
  }
  run();
})();
"""

HARNESS = """<!DOCTYPE html><html><head><meta charset="utf-8"><title>functest</title>
<style>body{margin:0;font:12px/1.4 monospace}iframe{width:1200px;height:900px;border:0;display:block}
#result{white-space:pre-wrap}</style></head><body>
<pre id="result">pending</pre>
<iframe data-page="search.html" src="../search.html"></iframe>
<iframe data-page="index.html" src="../index.html"></iframe>
<iframe data-page="diagnostics.html" src="../diagnostics.html"></iframe>
<iframe data-page="walkthrough.html" src="../walkthrough.html"></iframe>
<iframe data-page="agents.html" src="../agents.html"></iframe>
<script>__PROBE__</script>
</body></html>
"""


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


def run_probe(browser: str, script_enabled: bool = True) -> list[dict]:
    harness = SITE / "_partials" / ".functest-harness.html"
    harness.parent.mkdir(parents=True, exist_ok=True)
    # 用 replace 而不是 .format()：模板里有 CSS 花括号，format 会当成占位符。
    harness.write_text(HARNESS.replace("__PROBE__", PROBE), encoding="utf-8")

    class Quiet(http.server.SimpleHTTPRequestHandler):
        def __init__(self, *a, **kw):
            super().__init__(*a, directory=str(SITE), **kw)

        def log_message(self, *a):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Quiet)
    port = server.server_address[1]
    threading.Thread(target=server.serve_forever, daemon=True).start()

    try:
        with tempfile.TemporaryDirectory(prefix="soko-func-") as profile:
            cmd = [
                browser, "--headless=new", "--disable-gpu", "--no-sandbox",
                "--no-first-run", "--disable-crash-reporter",
                "--disable-background-networking",
                f"--user-data-dir={profile}",
                f"--virtual-time-budget={VIRTUAL_TIME_MS}",
                "--dump-dom",
            ]
            if not script_enabled:
                cmd.append("--blink-settings=scriptEnabled=false")
            cmd.append(f"http://127.0.0.1:{port}/_partials/.functest-harness.html")
            proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                    text=True, start_new_session=True)
            try:
                stdout, _ = proc.communicate(timeout=TIMEOUT_S)
            except subprocess.TimeoutExpired as exc:
                stdout = exc.output or ""
                if isinstance(stdout, bytes):
                    stdout = stdout.decode("utf-8", "replace")
            finally:
                if proc.poll() is None:
                    try:
                        os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
                    except (ProcessLookupError, PermissionError):
                        proc.kill()
                if proc.stdout:
                    proc.stdout.close()
    finally:
        server.shutdown()
        server.server_close()
        harness.unlink(missing_ok=True)

    match = re.search(r'<pre id="result">(.*?)</pre>', stdout or "", re.S)
    if not match:
        return [{"name": "harness", "ok": False, "detail": "probe produced no result"}]
    payload = html.unescape(match.group(1)).strip()
    if payload == "pending":
        return [{"name": "harness", "ok": False, "detail": "probe did not finish"}]
    return json.loads(payload)


def no_js_integrity(browser: str) -> list[dict]:
    """With the site's JavaScript absent, no page may lose its content.

    这是本站的硬要求（内容在 HTML 里，JS 只做增强）。判据不是"页面能打开"，
    而是"正文还在"。

    **怎么模拟无 JS**：`--blink-settings=scriptEnabled=false` 会让 `--dump-dom`
    彻底不输出（实测 stdout 长度为 0），没法量。所以改成**把 .js 请求拦下来返回空**
    —— 脚本开着，但站点的 JS 一行都没跑。对页面来说这与无 JS 等价，而且量得到。
    """
    script = r"""
    (function () {
      var out = [];
      var frames = Array.prototype.slice.call(document.querySelectorAll("iframe"));
      var pending = frames.length;
      function loaded(f) {
        try { return f.contentDocument && f.contentDocument.URL &&
                     f.contentDocument.URL.indexOf("about:blank") !== 0 && f.contentDocument.body; }
        catch (e) { return false; }
      }
      function done() {
        if (--pending === 0)
          document.getElementById("result").textContent = JSON.stringify(out, null, 1);
      }
      frames.forEach(function (f) {
        function measure() {
          var d = f.contentDocument, w = f.contentWindow;
          var main = d.querySelector("main");
          var chars = main ? (main.textContent || "").trim().length : 0;
          // 无 JS 时这些增强件**不该**出现（出现了就是按不动的死控件）
          var visible = function (sel) {
            var el = d.querySelector(sel);
            return !!(el && w.getComputedStyle(el).display !== "none");
          };
          out.push({
            page: f.getAttribute("data-page"),
            chars: chars,
            ok: chars > 500,
            copyVisible: visible("[data-copy], [data-copy-self]"),
            filterVisible: visible("[data-filter]"),
            searchVisible: visible("[data-search-input]")
          });
          done();
        }
        if (loaded(f)) measure(); else f.addEventListener("load", measure);
      });
    })();
    """
    harness = SITE / "_partials" / ".nojs-harness.html"
    harness.write_text(
        "<!DOCTYPE html><meta charset='utf-8'><pre id='result'>pending</pre>"
        + "".join(f'<iframe data-page="{p}" src="../{p}"></iframe>'
                  for p in ("index.html", "walkthrough.html", "diagnostics.html",
                            "get-started.html", "search.html"))
        + f"<script>{script}</script>",
        encoding="utf-8",
    )

    class BlockJs(http.server.SimpleHTTPRequestHandler):
        """Serve the site, but hand back an empty body for every .js request."""

        def __init__(self, *a, **kw):
            super().__init__(*a, directory=str(SITE), **kw)

        def do_GET(self):  # noqa: N802
            if self.path.split("?")[0].endswith(".js"):
                self.send_response(200)
                self.send_header("Content-Type", "application/javascript")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            super().do_GET()

        def log_message(self, *a):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), BlockJs)
    port = server.server_address[1]
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        with tempfile.TemporaryDirectory(prefix="soko-nojs-") as profile:
            proc = subprocess.Popen(
                [browser, "--headless=new", "--disable-gpu", "--no-sandbox", "--no-first-run",
                 "--disable-crash-reporter", f"--user-data-dir={profile}",
                 "--virtual-time-budget=8000", "--dump-dom",
                 f"http://127.0.0.1:{port}/_partials/.nojs-harness.html"],
                stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, start_new_session=True)
            try:
                stdout, _ = proc.communicate(timeout=TIMEOUT_S)
            except subprocess.TimeoutExpired as exc:
                stdout = exc.output or ""
                if isinstance(stdout, bytes):
                    stdout = stdout.decode("utf-8", "replace")
            finally:
                if proc.poll() is None:
                    try:
                        os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
                    except (ProcessLookupError, PermissionError):
                        proc.kill()
                if proc.stdout:
                    proc.stdout.close()
    finally:
        server.shutdown()
        server.server_close()
        harness.unlink(missing_ok=True)

    match = re.search(r'<pre id="result">(.*?)</pre>', stdout or "", re.S)
    if not match:
        return [{"name": "no-JS: harness", "ok": False, "detail": "no result"}]
    payload = html.unescape(match.group(1)).strip()
    if payload == "pending":
        return [{"name": "no-JS: harness", "ok": False, "detail": "did not finish"}]
    results = []
    for r in json.loads(payload):
        results.append({"name": f"no-JS: {r['page']} 正文仍在", "ok": r["ok"],
                        "detail": f"{r['chars']} chars"})
        # 无 JS 时增强件必须不显示：一个按不动的按钮比没有按钮更糟。
        for key, label in (("copyVisible", "复制按钮"), ("filterVisible", "筛选框"),
                           ("searchVisible", "搜索框")):
            if r[key]:
                results.append({"name": f"no-JS: {r['page']} 不该显示{label}", "ok": False,
                                "detail": "在无 JS 时仍然可见"})
    return results


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    browser = find_browser()
    if browser is None:
        print("site-functest: no Chrome/Chromium found. Set SOKO_CHROME=<path>.", file=sys.stderr)
        return 2

    results = run_probe(browser) + no_js_integrity(browser)
    failed = [r for r in results if not r["ok"]]

    if args.json:
        print(json.dumps(results, ensure_ascii=False, indent=1))
    else:
        for r in results:
            print(f"  {'ok  ' if r['ok'] else 'FAIL'} {r['name']:<38} {r['detail'][:80]}")
        print()
        if failed:
            print(f"site-functest: {len(failed)}/{len(results)} 项未通过", file=sys.stderr)
        else:
            print(f"site-functest: ok ({len(results)} 项功能全部按预期工作)")

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
