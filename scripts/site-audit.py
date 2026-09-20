#!/usr/bin/env python3
"""Measure rendered pages in headless Chrome — the objective half of design review.

Why this exists: the agent writing the site cannot see it. `scripts/site-shot.py`
produces PNGs for a *human* to judge; this script produces *numbers* an agent can
act on, so layout defects are caught mechanically instead of being shipped and
noticed later. It is the counterpart to the screenshots, not a replacement.

It answers, per page per width:
  - does anything overflow horizontally (the single most common responsive bug)
  - which element overflows, and by how much
  - are the design devices actually present (turnstile, semantic colours, grid)
  - does the type scale apply (are headings using the tokens)
  - is the graph paper actually painted
  - is `⊢` resolving to the self-hosted symbol face rather than a platform font

How it works: Chrome loads a generated harness page that embeds the target in a
same-origin iframe at an exact width, measures inside it, and writes the result
into the DOM. `--dump-dom` then hands the numbers back. No websocket client, no
third-party dependency, python3 stdlib only.

Usage:
  python3 scripts/site-audit.py                      # every page, default widths
  python3 scripts/site-audit.py site/styleguide.html
  python3 scripts/site-audit.py --width 390 --json

Exit codes: 0 = no overflow anywhere; 1 = at least one overflow; 2 = usage error.
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

# 虚拟时间预算：iframe 里要等 CSS + 字体落定。太小会量到未排版的中间态。
VIRTUAL_TIME_MS = 10000
# Chrome 在 dump-dom 之后不退出，所以这是"读够输出就收工"的上限，不是等待上限。
AUDIT_TIMEOUT_S = 45

BROWSER_CANDIDATES = (
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
)

DEFAULT_WIDTHS = (1440, 900, 390)

# 探针：在 iframe 内部跑，量完把结果写回父文档的 <pre id="result">。
# 只读测量，不改被测页面。
PROBE_JS = r"""
(function () {
  var out = [];
  var frames = Array.prototype.slice.call(document.querySelectorAll("iframe.audit"));
  var pending = frames.length;
  function done() {
    if (--pending === 0) {
      document.getElementById("result").textContent = JSON.stringify(out, null, 1);
    }
  }

  function measure(frame) {
    var w = frame.getAttribute("data-width");
    var d, win;
    try { d = frame.contentDocument; win = frame.contentWindow; } catch (e) {
      out.push({ width: Number(w), page: frame.getAttribute("data-page"), error: String(e) });
      return done();
    }
    var rec = { width: Number(w), page: frame.getAttribute("data-page"), overflow: null, offenders: [] };
    try {
      var de = d.documentElement;
      rec.scrollWidth = de.scrollWidth;
      rec.clientWidth = de.clientWidth;
      rec.viewport = win.innerWidth + "x" + win.innerHeight;
      rec.overflow = de.scrollWidth > de.clientWidth + 1;

      if (rec.overflow) {
        var all = d.querySelectorAll("body *");
        var bad = [];
        for (var i = 0; i < all.length; i++) {
          var r = all[i].getBoundingClientRect();
          if (r.width > 0 && r.right > de.clientWidth + 1) {
            var cn = all[i].className;
            bad.push({
              tag: all[i].tagName.toLowerCase(),
              cls: (cn && cn.baseVal !== undefined ? cn.baseVal : cn) || "",
              right: Math.round(r.right),
              width: Math.round(r.width)
            });
          }
        }
        bad.sort(function (a, b) { return b.right - a.right; });
        rec.offenders = bad.slice(0, 5);
      }

      function count(sel) { return d.querySelectorAll(sel).length; }
      rec.devices = {
        turnstile: count(".turnstile"),
        primaryBtn: count(".btn--primary"),
        calloutVerified: count(".callout--verified"),
        calloutLimit: count(".callout--limit"),
        calloutNote: count(".callout--note"),
        chip: count(".chip"),
        h1: count("h1"),
        h2: count("h2"),
        img: count("img")
      };

      var bodyBg = win.getComputedStyle(d.body);
      rec.grid = {
        image: (bodyBg.backgroundImage || "none").slice(0, 60),
        painted: (bodyBg.backgroundImage || "none") !== "none"
      };

      // h1 距页面顶端的距离：面包屑 + 区块间距 + 页头内边距三者会叠加，
      // 曾经叠出 ~216px 的空白（390px 上尤其难看）。量出来才谈得上"改好了"。
      var h1el = d.querySelector("h1");
      rec.h1Top = h1el ? Math.round(h1el.getBoundingClientRect().top + win.scrollY) : null;
      // 拆开看：h1 上方到底是哪几块占的。
      function h(sel) { var e = d.querySelector(sel); return e ? Math.round(e.getBoundingClientRect().height) : 0; }
      var hdr = d.querySelector('.site-header');
      rec.above = { header: h('.site-header'), crumbs: h('.breadcrumbs'), pageHead: h('.page-head'),
                    headerPosition: hdr ? win.getComputedStyle(hdr).position : null };

      var h1 = d.querySelector("h1"), h2 = d.querySelector("h2");
      rec.type = {
        h1: h1 ? win.getComputedStyle(h1).fontSize : null,
        h2: h2 ? win.getComputedStyle(h2).fontSize : null,
        body: win.getComputedStyle(d.body).fontSize
      };

      var turn = d.querySelector(".turnstile__turn, .goal-turn");
      if (turn) {
        rec.turnstile = {
          family: win.getComputedStyle(turn).fontFamily.split(",")[0].replace(/"/g, ""),
          text: (turn.textContent || "").trim()
        };
      }

      rec.fontsLoaded = [];
      try {
        ["400 17px 'Soko Mono'", "400 17px 'Soko Symbols'"].forEach(function (f) {
          if (win.document.fonts.check(f)) rec.fontsLoaded.push(f);
        });
      } catch (e) {}

      // 语义色真的算出来了吗。
      //
      // 这一条是被一个真 bug 逼出来的：site.css 引用了 tokens.css 里**不存在**
      // 的 --mark*，CSS 自定义属性解析失败是**静默**的——声明作废，元素拿到
      // transparent，页面照样"没报错"。肉眼要盯着看才发现"这个提示框怎么没底色"。
      // 所以这里把关键语义件的计算值抓出来，凡是该有底色/边框却是透明的就报出来。
      var COLOR_PROBES = [
        [".callout--limit", "backgroundColor"],
        [".callout--verified", "backgroundColor"],
        [".callout--note", "backgroundColor"],
        [".diag", "backgroundColor"],
        [".chip--open", "backgroundColor"],
        [".chip--checked", "backgroundColor"],
        [".squiggle", "textDecorationColor"]
      ];
      rec.colors = [];
      COLOR_PROBES.forEach(function (pair) {
        var el = d.querySelector(pair[0]);
        if (!el) return;
        var v = win.getComputedStyle(el)[pair[1]];
        rec.colors.push({
          sel: pair[0], prop: pair[1], value: v,
          // transparent / rgba(0,0,0,0) 都算"没算出来"
          blank: !v || v === "transparent" || /rgba?\(0,\s*0,\s*0,\s*0\)/.test(v)
        });
      });
      rec.blankColors = rec.colors.filter(function (c) { return c.blank; }).length;

      // ── 性能：真的加载一次，读浏览器自己的账 ────────────────────────────
      //
      // 用户明确说过"快也很重要"。此前只用**文件大小**间接验证——那不等于快：
      // 请求数、渲染阻塞资源、字体加载时机都会影响。这里读 Performance API 的
      // 真实数字：请求数、传输字节、DOMContentLoaded、load、以及首字节。
      try {
        var perf = win.performance;
        var nav = (perf.getEntriesByType("navigation") || [])[0] || {};
        var res = perf.getEntriesByType("resource") || [];
        var byType = {};
        var bytes = 0;
        res.forEach(function (r) {
          var t = r.initiatorType || "other";
          byType[t] = (byType[t] || 0) + 1;
          bytes += r.transferSize || r.encodedBodySize || 0;
        });
        rec.perf = {
          // 请求数**不含**文档本身，所以 +1 才是总数
          requests: res.length + 1,
          byType: byType,
          transferKB: Math.round(bytes / 1024),
          domContentLoaded: nav.domContentLoadedEventEnd ? Math.round(nav.domContentLoadedEventEnd) : null,
          load: nav.loadEventEnd ? Math.round(nav.loadEventEnd) : null,
          ttfb: nav.responseStart ? Math.round(nav.responseStart) : null,
          // 渲染阻塞的样式表数量（字体与脚本不算阻塞，样式表算）
          blockingCSS: res.filter(function (r) { return r.initiatorType === "link" || r.initiatorType === "css"; }).length,
          // 列出每个资源：光看总数发现不了"明明不用衬线却在下载衬线字体"这类浪费。
          resources: res.map(function (r) {
            return (r.name.split("/").pop() || r.name) + " " +
                   Math.round((r.transferSize || r.encodedBodySize || 0) / 1024) + "KB";
          })
        };
      } catch (e) { rec.perf = { error: String(e) }; }

      // ── 可访问性：**在渲染结果上**量对比度与焦点，不是读令牌注释 ──────────
      //
      // tokens.css 的注释里每个颜色都算过对比度，但那是"设计意图"；
      // 真正决定可读性的是**最终渲染出来的**前景色与背景色——中间隔着继承、
      // 半透明层、以及"某个容器忘了给不透明底"这类问题。
      function parseRGB(str) {
        var m = /rgba?\(([^)]+)\)/.exec(str || "");
        if (!m) return null;
        var parts = m[1].split(",").map(function (x) { return parseFloat(x); });
        return { r: parts[0], g: parts[1], b: parts[2], a: parts.length > 3 ? parts[3] : 1 };
      }
      function lin(c) { c /= 255; return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4); }
      function lum(c) { return 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b); }
      function ratio(f, b) {
        var lf = lum(f), lb = lum(b), hi = Math.max(lf, lb), lo = Math.min(lf, lb);
        return (hi + 0.05) / (lo + 0.05);
      }
      // 背景要**往上找**：很多元素自己是透明的，实际底色来自祖先。
      function effectiveBg(el) {
        var node = el;
        while (node && node.nodeType === 1) {
          var c = parseRGB(win.getComputedStyle(node).backgroundColor);
          if (c && c.a > 0.9) return c;
          node = node.parentElement;
        }
        return parseRGB(win.getComputedStyle(d.body).backgroundColor) || { r: 255, g: 255, b: 255, a: 1 };
      }
      var SAMPLES = ["p", "li", ".lead", ".small", "h1", "h2", "h3", "a", "td", "th",
                     ".chip", ".stats__label", ".footer-note", ".timeline time", ".diag__hint"];
      var worst = null, measured = 0, lowCount = 0, lowSamples = [];
      SAMPLES.forEach(function (sel) {
        var els = d.querySelectorAll(sel);
        for (var i = 0; i < Math.min(els.length, 6); i++) {
          var el = els[i];
          var txt = (el.textContent || "").trim();
          if (txt.length < 2) continue;
          var cs = win.getComputedStyle(el);
          if (cs.display === "none" || cs.visibility === "hidden" || parseFloat(cs.opacity) < 0.5) continue;
          var fg = parseRGB(cs.color), bg = effectiveBg(el);
          if (!fg || !bg) continue;
          var r = ratio(fg, bg);
          var size = parseFloat(cs.fontSize);
          var bold = parseInt(cs.fontWeight, 10) >= 700;
          // WCAG AA：正文 4.5:1；≥18.66px 或 ≥24px 的粗体大字 3:1。
          var need = (size >= 24 || (size >= 18.66 && bold)) ? 3.0 : 4.5;
          measured++;
          if (!worst || r < worst.ratio) {
            worst = { sel: sel, ratio: Math.round(r * 100) / 100, need: need,
                      fg: cs.color, size: Math.round(size), text: txt.slice(0, 30) };
          }
          if (r < need - 0.01) {
            lowCount++;
            if (lowSamples.length < 3) {
              lowSamples.push(sel + " " + (Math.round(r * 100) / 100) + ":1 (需 " + need + ") — " + txt.slice(0, 24));
            }
          }
        }
      });
      rec.a11y = { measured: measured, worst: worst, belowAA: lowCount, samples: lowSamples };

      // 焦点可见性：真的 focus() 一个链接，看它有没有可见的轮廓。
      // D1 要求用 outline（不占空间、跟随圆角、高对比模式下也在）。
      try {
        var firstLink = d.querySelector("main a[href]") || d.querySelector("a[href]");
        firstLink.focus();
        var fcs = win.getComputedStyle(firstLink);
        var outlineW = parseFloat(fcs.outlineWidth) || 0;
        rec.a11y.focus = {
          outlineWidth: outlineW,
          style: fcs.outlineStyle,
          color: fcs.outlineColor,
          ok: outlineW > 0 && fcs.outlineStyle !== "none"
        };
        firstLink.blur();
      } catch (e) { rec.a11y.focus = { ok: false, error: String(e) }; }

      // 暗色模式（"夜读"）。切换按钮写的是 <html data-theme="dark">，所以直接把
      // 属性打上再量一遍——这是真用户能到的那条路径，不是另造一个模式。
      // 之前整套审计**只量了亮色**，暗色一直是未验证状态。
      try {
        var root = d.documentElement;
        var before = root.getAttribute("data-theme");
        root.setAttribute("data-theme", "dark");
        var dcs = win.getComputedStyle(root);
        var dark = {
          paper: dcs.getPropertyValue("--paper").trim(),
          ink: dcs.getPropertyValue("--ink").trim(),
          accInk: dcs.getPropertyValue("--acc-ink").trim(),
          vermInk: dcs.getPropertyValue("--verm-ink").trim()
        };
        // 语义件在暗色下也必须真的算出来（同样的静默失败风险）。
        var blanks = 0;
        COLOR_PROBES.forEach(function (pair) {
          var el = d.querySelector(pair[0]);
          if (!el) return;
          var v = win.getComputedStyle(el)[pair[1]];
          if (!v || v === "transparent" || /rgba?\(0,\s*0,\s*0,\s*0\)/.test(v)) blanks++;
        });
        dark.blankColors = blanks;
        dark.bodyBg = win.getComputedStyle(d.body).backgroundColor;
        rec.dark = dark;
        if (before === null) root.removeAttribute("data-theme");
        else root.setAttribute("data-theme", before);
      } catch (e) { rec.dark = { error: String(e) }; }
    } catch (e) {
      rec.error = String(e);
    }
    out.push(rec);
    done();
  }

  frames.forEach(function (frame) {
    // 两个坑，都踩过：
    //  1. 脚本在 iframe 之后解析，但 `contentDocument` 此时是**初始 about:blank**，
    //     它 readyState 就是 "complete" 且**有 body** —— 只查这两样会把空文档
    //     当成加载完，量出一堆"390 不溢出、turnstile 0"的假绿（第一版正是如此）。
    //     所以必须校验 d.URL 不是 about:blank。
    //  2. 快的页面可能在监听挂上之前就 load 完了，那时 load 不再触发，
    //     pending 永不归零 → 探针永远 "pending"。
    // 两个一起解决：先试一次（带 URL 校验），不行就挂 load 监听。
    function ready(d) {
      try { return d && d.URL && d.URL.indexOf("about:blank") !== 0 && d.body; }
      catch (e) { return false; }
    }
    var d0 = null;
    try { d0 = frame.contentDocument; } catch (e) {}
    if (ready(d0)) {
      measure(frame);
    } else {
      frame.addEventListener("load", function () { measure(frame); });
    }
  });
})();
"""

HARNESS_TMPL = """<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>audit</title>
<style>
  body {{ margin: 0; font: 12px/1.4 monospace; }}
  iframe.audit {{ border: 0; display: block; height: 900px; }}
  #result {{ white-space: pre-wrap; }}
</style></head>
<body>
<pre id="result">pending</pre>
{iframe}
<script>{probe}</script>
</body></html>
"""


def find_browser() -> str | None:
    import os

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


def pages_from_args(raw: list[str]) -> list[Path]:
    if raw:
        return [ROOT / p for p in raw]
    return sorted(p for p in SITE.rglob("*.html") if "_partials" not in p.parts)


def audit(browser: str, pages: list[Path], widths: tuple[int, ...]) -> list[dict]:
    """Serve `site/` over HTTP, load a harness, measure inside iframes.

    为什么走 HTTP 而不是 file://：`404.html` 的资源路径必须是**站点根绝对路径**
    （`/assets/…`），因为 GitHub Pages 会在任意深度提供它。file:// 下 `/assets/…`
    解析到文件系统根、必然 404，于是审计对那一页永远报假红。本地起一个静态
    服务器把 site/ 当根，才和 GitHub Pages 的真实行为一致——顺带也不再需要
    `--allow-file-access-from-files`。

    Chrome **不退出**：`--dump-dom` 写完 stdout 后进程仍挂着（与 `--screenshot`
    同款行为）。所以不能等进程结束——用 communicate(timeout) 拿部分输出，
    再连**进程组**一起收掉（Chrome 会 fork，只 kill 父进程会留孤儿）。
    """
    frames = []
    for page in pages:
        for width in widths:
            rel = page.relative_to(SITE).as_posix()
            frames.append(
                f'<iframe class="audit" data-page="{html.escape(rel)}" '
                f'data-width="{width}" width="{width}" '
                f'src="../{html.escape(rel)}"></iframe>'
            )

    # 探针放进 `_partials/`：那里已经被 check-site.py 排除在页面扫描之外，
    # 而且它是站点目录里的真实路径，HTTP 服务器能直接提供。
    harness = SITE / "_partials" / ".audit-harness.html"
    harness.parent.mkdir(parents=True, exist_ok=True)
    harness.write_text(
        HARNESS_TMPL.format(iframe="\n".join(frames), probe=PROBE_JS),
        encoding="utf-8",
    )

    class QuietHandler(http.server.SimpleHTTPRequestHandler):
        def __init__(self, *a, **kw):
            super().__init__(*a, directory=str(SITE), **kw)

        def log_message(self, *a):  # 服务器日志会污染 stdout 的 DOM dump
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), QuietHandler)
    port = server.server_address[1]
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()

    try:
        with tempfile.TemporaryDirectory(prefix="soko-audit-") as profile:
            cmd = [
                browser,
                "--headless=new",
                "--disable-gpu",
                "--no-sandbox",
                "--no-first-run",
                "--disable-crash-reporter",
                "--disable-background-networking",
                f"--user-data-dir={profile}",
                f"--virtual-time-budget={VIRTUAL_TIME_MS}",
                "--dump-dom",
                f"http://127.0.0.1:{port}/_partials/.audit-harness.html",
            ]
            proc = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
                text=True,
                start_new_session=True,
            )
            try:
                stdout, _ = proc.communicate(timeout=AUDIT_TIMEOUT_S)
            except subprocess.TimeoutExpired as exc:
                # 超时**不能**再去 proc.stdout.read()——管道没关，read() 会永久
                # 阻塞（实测把整条命令拖死）。TimeoutExpired 自带已读到的部分输出。
                stdout = exc.output or ""
                if isinstance(stdout, bytes):
                    stdout = stdout.decode("utf-8", "replace")
            finally:
                if proc.poll() is None:
                    try:
                        os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
                    except (ProcessLookupError, PermissionError):
                        proc.kill()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    pass
                if proc.stdout:
                    proc.stdout.close()
    finally:
        server.shutdown()
        server.server_close()
        harness.unlink(missing_ok=True)

    match = re.search(r'<pre id="result">(.*?)</pre>', stdout or "", re.S)
    if not match:
        print("site-audit: harness produced no result", file=sys.stderr)
        return []
    payload = html.unescape(match.group(1))
    if payload.strip() == "pending":
        print(
            "site-audit: harness did not finish (a frame never reported; "
            "raise VIRTUAL_TIME_MS)",
            file=sys.stderr,
        )
        return []
    return json.loads(payload)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("pages", nargs="*", help="page paths relative to the repo root")
    parser.add_argument("--width", type=int, action="append", dest="widths")
    parser.add_argument("--json", action="store_true", help="emit raw JSON")
    args = parser.parse_args()

    browser = find_browser()
    if browser is None:
        print("site-audit: no Chrome/Chromium found. Set SOKO_CHROME=<path>.", file=sys.stderr)
        return 2

    widths = tuple(args.widths) if args.widths else DEFAULT_WIDTHS
    pages = pages_from_args(args.pages)
    if not pages:
        print("site-audit: no pages found", file=sys.stderr)
        return 2

    records = audit(browser, pages, widths)
    if args.json:
        print(json.dumps(records, ensure_ascii=False, indent=1))
        return 0 if not any(r.get("overflow") for r in records) else 1

    overflows = 0
    blank_total = 0
    for rec in records:
        flag = "OVERFLOW" if rec.get("overflow") else "ok"
        if rec.get("overflow"):
            overflows += 1
        dev = rec.get("devices", {})
        blank = rec.get("blankColors", 0)
        blank_total += blank
        a11y = rec.get("a11y") or {}
        low = a11y.get("belowAA") or 0
        focus_ok = (a11y.get("focus") or {}).get("ok")
        if low:
            blank_total += low
            for smp in (a11y.get("samples") or []):
                print(f"           ↳ 对比度低于 AA: {smp}")
        if focus_ok is False:
            blank_total += 1
            print("           ↳ 焦点不可见（outline 缺失）")
        dark = rec.get("dark") or {}
        dark_blank = dark.get("blankColors")
        if dark_blank:
            blank_total += dark_blank
            print(f"           ↳ 暗色下 {dark_blank} 个语义色算不出来")
        print(
            f"{flag:8s} {rec['page']} @{rec['width']:>5}  "
            f"scroll {rec.get('scrollWidth')}/{rec.get('clientWidth')}  "
            f"turnstile {dev.get('turnstile', 0)}  h1 {dev.get('h1', 0)}  "
            f"grid {'yes' if rec.get('grid', {}).get('painted') else 'NO'}  "
            f"h1@{rec.get('h1Top')}px  hdr {(rec.get('above') or {}).get('headerPosition')}  "
            f"a11y {((rec.get('a11y') or {}).get('worst') or {}).get('ratio')}:1  "
            f"{((rec.get('perf') or {}).get('requests'))}req/{((rec.get('perf') or {}).get('transferKB'))}KB  "
            f"dark {'ok' if (dark.get('paper') and not dark.get('blankColors')) else 'FAIL'}"
            + (f"  BLANK-COLOR {blank}" if blank else "")
        )
        for off in rec.get("offenders", []):
            print(f"           ↳ <{off['tag']} class=\"{off['cls']}\"> right={off['right']} width={off['width']}")
        for c in rec.get("colors", []):
            if c["blank"]:
                print(f"           ↳ {c['sel']} {c['prop']} computed to `{c['value']}` — a token reference failed to resolve")
        if rec.get("error"):
            print(f"           ↳ error: {rec['error']}")

    if overflows:
        print(f"\nsite-audit: {overflows} overflow(s)", file=sys.stderr)
        return 1
    if blank_total:
        print(f"\nsite-audit: {blank_total} unresolved semantic colour(s)", file=sys.stderr)
        return 1
    print(f"\nsite-audit: ok ({len(records)} measurements, no overflow, no blank colours)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
