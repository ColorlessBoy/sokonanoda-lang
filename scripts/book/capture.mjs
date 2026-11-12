#!/usr/bin/env node
/*
 * capture.mjs — headless Chrome DOM test + screenshots for the analysis book.
 *
 * No npm dependencies: it launches Chrome with a remote-debugging port, talks
 * to it over the DevTools protocol with Node's built-in fetch/WebSocket, and
 * writes the same evidence files used by PR #35.
 *
 * Usage (from the repository root, after building):
 *
 *   python3 scripts/build_book_proto.py --out dist/book/book-v2
 *   node scripts/book/capture.mjs
 *
 * Environment:
 *   CHROME_BIN   override the Chrome/Chromium executable
 *   BOOK_HTML    override the generated page (default dist/book/book-v2/index.html)
 *
 * Exit code 0 only when every DOM assertion passes.
 */
import { spawn } from "node:child_process";
import { readFile, writeFile, access } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import os from "node:os";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..", "..");
const BOOK_HTML = resolve(ROOT, process.env.BOOK_HTML || "dist/book/book-v2/index.html");
const OUT_DIR = process.env.BOOK_SHOT_DIR || "/tmp";
const PDF_PATH = resolve(OUT_DIR, "book-v2-print.pdf");

const chromeCandidates = [
  process.env.CHROME_BIN,
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
  "/usr/bin/chromium-browser",
].filter(Boolean);

async function findChrome() {
  for (const candidate of chromeCandidates) {
    try {
      await access(candidate);
      return candidate;
    } catch {}
  }
  throw new Error("Chrome not found; set CHROME_BIN");
}

async function waitFile(path, tries = 120) {
  for (let i = 0; i < tries; i++) {
    try {
      return await readFile(path, "utf8");
    } catch {}
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`timed out waiting for ${path}`);
}

const chromePath = await findChrome();
const userDir = await import("node:fs/promises").then((fs) =>
  fs.mkdtemp(resolve(os.tmpdir(), "soko-book-chrome-"))
);
const chrome = spawn(chromePath, [
  "--headless=new",
  "--disable-gpu",
  "--hide-scrollbars",
  "--no-first-run",
  "--no-default-browser-check",
  "--allow-file-access-from-files",
  "--remote-debugging-port=0",
  `--user-data-dir=${userDir}`,
  "about:blank",
], { stdio: "ignore" });

async function stopChrome() {
  if (!chrome.killed) chrome.kill("SIGKILL");
}
process.on("exit", stopChrome);

try {
  const portFile = await waitFile(resolve(userDir, "DevToolsActivePort"));
  const port = Number(portFile.split("\n")[0]);
  if (!port) throw new Error("Chrome did not expose a debugging port");
  const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, {
    method: "PUT",
  })).json();

  const ws = new WebSocket(target.webSocketDebuggerUrl);
  let sequence = 0;
  const pending = new Map();
  const events = new Map();

  ws.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve: done, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) reject(new Error(JSON.stringify(message.error)));
      else done(message.result);
    } else if (message.method && events.has(message.method)) {
      for (const handler of [...events.get(message.method)]) handler(message.params);
    }
  });
  await new Promise((done, fail) => {
    ws.addEventListener("open", done);
    ws.addEventListener("error", fail);
  });

  function send(method, params = {}) {
    return new Promise((done, fail) => {
      const id = ++sequence;
      pending.set(id, { resolve: done, reject: fail });
      ws.send(JSON.stringify({ id, method, params }));
    });
  }

  function once(method, timeout = 30000) {
    return new Promise((done, fail) => {
      if (!events.has(method)) events.set(method, new Set());
      const handler = (params) => {
        clearTimeout(timer);
        events.get(method).delete(handler);
        done(params);
      };
      events.get(method).add(handler);
      const timer = setTimeout(() => {
        events.get(method).delete(handler);
        fail(new Error(`timeout waiting for ${method}`));
      }, timeout);
    });
  }

  async function evaluate(expression) {
    const result = await send("Runtime.evaluate", {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    if (result.exceptionDetails) {
      throw new Error(result.exceptionDetails.exception?.description || "evaluate failed");
    }
    return result.result.value;
  }

  async function setViewport(width, height, mobile = false) {
    await send("Emulation.setDeviceMetricsOverride", {
      width,
      height,
      deviceScaleFactor: 1,
      mobile,
    });
  }

  async function load(width, height, mobile = false) {
    await setViewport(width, height, mobile);
    const loaded = once("Page.loadEventFired");
    await send("Page.navigate", { url: `file://${BOOK_HTML}` });
    await loaded;
    await new Promise((r) => setTimeout(r, 450));
  }

  async function reload() {
    const loaded = once("Page.loadEventFired");
    await send("Page.reload", { ignoreCache: true });
    await loaded;
    await new Promise((r) => setTimeout(r, 350));
  }

  async function scrollTo(selector, offset = 70) {
    await evaluate(`(() => {
      const element = document.querySelector(${JSON.stringify(selector)});
      if (!element) return false;
      const top = element.getBoundingClientRect().top + window.scrollY - ${offset};
      const previous = document.documentElement.style.scrollBehavior;
      document.documentElement.style.scrollBehavior = "auto";
      window.scrollTo(0, Math.max(0, top));
      document.documentElement.style.scrollBehavior = previous;
      return true;
    })()`);
    await new Promise((r) => setTimeout(r, 400));
  }

  async function screenshot(name, clip = null) {
    const params = {
      format: "png",
      fromSurface: true,
      captureBeyondViewport: Boolean(clip),
    };
    if (clip) params.clip = clip;
    const result = await send("Page.captureScreenshot", params);
    await writeFile(resolve(OUT_DIR, name), Buffer.from(result.data, "base64"));
  }

  const failures = [];
  const assert = (condition, label, detail = "") => {
    if (!condition) failures.push(`${label}${detail ? `: ${detail}` : ""}`);
  };

  await send("Page.enable");
  await send("Runtime.enable");
  await load(1440, 1000);

  const desktop = await evaluate(`(() => {
    const ids = [...document.querySelectorAll("[id]")].map((el) => el.id);
    const duplicateIds = [...new Set(ids.filter((id, index) => ids.indexOf(id) !== index))];
    const missingAnchors = [...document.querySelectorAll('a[href^="#"]')]
      .map((anchor) => anchor.getAttribute("href"))
      .filter((href) => href.length > 1 && !document.getElementById(href.slice(1)));
    const units = [...document.querySelectorAll(".unit")];
    const emptyUnits = units
      .filter((unit) => !unit.querySelector(".decl, .exercise, .block.prose"))
      .map((unit) => unit.id);
    const toc = document.getElementById("toc");
    return {
      units: units.length,
      chapters: document.querySelectorAll(".chapter").length,
      exercises: document.querySelectorAll(".exercise").length,
      tocUnitLinks: toc.querySelectorAll('a[href^="#unit-"]').length,
      tocChapterLinks: toc.querySelectorAll('a[href^="#chapter-"]').length,
      duplicateIds,
      missingAnchors: [...new Set(missingAnchors)],
      emptyUnits,
      scrollWidth: document.documentElement.scrollWidth,
      bodyScrollWidth: document.body.scrollWidth,
      innerWidth,
      controls: {
        views: document.querySelectorAll(".seg button[data-view]").length,
        expand: Boolean(document.querySelector('[data-action="expand-all"]')),
        collapse: Boolean(document.querySelector('[data-action="collapse-all"]')),
        print: Boolean(document.querySelector('[data-action="print"]')),
        chapterJump: Boolean(document.querySelector('[data-action="chapter-jump"]')),
      },
      coverHeight: Math.ceil(document.querySelector(".cover").getBoundingClientRect().height),
    };
  })()`);

  assert(desktop.units === 72, "desktop unit count", String(desktop.units));
  assert(desktop.chapters === 7, "desktop chapter count", String(desktop.chapters));
  assert(desktop.tocUnitLinks === desktop.units, "TOC unit links", String(desktop.tocUnitLinks));
  assert(desktop.tocChapterLinks === desktop.chapters, "TOC chapter links", String(desktop.tocChapterLinks));
  assert(desktop.duplicateIds.length === 0, "duplicate ids", desktop.duplicateIds.join(","));
  assert(desktop.missingAnchors.length === 0, "missing anchors", desktop.missingAnchors.join(","));
  assert(desktop.emptyUnits.length === 0, "empty units", desktop.emptyUnits.join(","));
  assert(desktop.scrollWidth <= desktop.innerWidth + 1, "desktop overflow", `${desktop.scrollWidth}`);
  assert(desktop.controls.views === 3 && desktop.controls.expand && desktop.controls.collapse && desktop.controls.print && desktop.controls.chapterJump, "reading controls");

  await evaluate("window.scrollTo(0, 0)");
  await new Promise((r) => setTimeout(r, 200));
  await screenshot("book-v2-cover.png", {
    x: 0,
    y: 0,
    width: 1440,
    height: Math.min(desktop.coverHeight, 1600),
    scale: 1,
  });

  await scrollTo("#toc");
  await screenshot("book-v2-toc.png");

  const layers = await evaluate(`(() => {
    const formal = document.querySelector(".proof-panel, .code-panel");
    const intuition = document.querySelector(".hint, .block.prose");
    const visible = (element) => element ? getComputedStyle(element).display !== "none" : null;
    const snap = () => ({
      view: document.body.dataset.view,
      formal: visible(formal),
      intuition: visible(intuition),
      pressed: [...document.querySelectorAll(".seg button[data-view]")]
        .map((button) => button.getAttribute("aria-pressed")),
    });
    document.querySelector('[data-view="formal"]').click();
    const formalState = snap();
    document.querySelector('[data-view="intuition"]').click();
    const intuitionState = snap();
    document.querySelector('[data-view="both"]').click();
    return { formalState, intuitionState, bothState: snap() };
  })()`);
  assert(layers.formalState.view === "formal" && layers.formalState.formal === true && layers.formalState.intuition === false, "formal layer");
  assert(layers.intuitionState.view === "intuition" && layers.intuitionState.formal === false && layers.intuitionState.intuition === true, "intuition layer");
  assert(layers.bothState.formal === true && layers.bothState.intuition === true, "both layer");

  const panels = await evaluate(`(() => {
    const details = [...document.querySelectorAll("details")];
    document.querySelector('[data-action="expand-all"]').click();
    const expanded = details.filter((detail) => detail.open).length;
    document.querySelector('[data-action="collapse-all"]').click();
    const collapsed = details.filter((detail) => detail.open).length;
    document.querySelector('[data-action="expand-all"]').click();
    return { total: details.length, expanded, collapsed };
  })()`);
  assert(panels.total > 2000, "details count", String(panels.total));
  assert(panels.expanded === panels.total, "expand all", `${panels.expanded}/${panels.total}`);
  assert(panels.collapsed === 0, "collapse all", String(panels.collapsed));

  await evaluate('document.querySelector(\'[data-view="formal"]\').click()');
  await new Promise((r) => setTimeout(r, 100));
  await reload();
  const remembered = await evaluate("document.body.dataset.view");
  assert(remembered === "formal", "localStorage view memory", remembered);
  await evaluate("try { localStorage.clear(); } catch (error) {}");
  await reload();

  await evaluate('document.querySelector(\'[data-view="formal"]\').click()');
  await scrollTo("#unit-20");
  await screenshot("book-v2-formal.png");
  await evaluate('document.querySelector(\'[data-view="both"]\').click()');

  await scrollTo("#unit-69");
  await screenshot("book-v2-unit-ii7.png");

  const jump = await evaluate(`(() => {
    const select = document.querySelector('select[data-action="chapter-jump"]');
    select.value = "#chapter-ii-7";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    return { hash: window.location.hash, options: select.options.length };
  })()`);
  assert(jump.hash === "#chapter-ii-7" && jump.options === 7, "chapter jump", JSON.stringify(jump));

  await send("Emulation.setEmulatedMedia", { media: "print" });
  const printCss = await evaluate(`(() => {
    const chapter = document.querySelector(".chapter");
    const unit = document.querySelector(".chapter .unit + .unit");
    return {
      toolbar: getComputedStyle(document.querySelector(".toolbar")).display,
      chapterBreak: getComputedStyle(chapter).breakBefore,
      unitBreak: getComputedStyle(unit).breakBefore,
    };
  })()`);
  assert(printCss.toolbar === "none", "print hides toolbar", printCss.toolbar);
  assert(printCss.chapterBreak === "page", "print chapter break", printCss.chapterBreak);
  assert(printCss.unitBreak === "page", "print unit break", printCss.unitBreak);
  await send("Emulation.setEmulatedMedia", { media: "" });

  await evaluate('document.querySelector(\'[data-action="expand-all"]\').click()');
  await new Promise((r) => setTimeout(r, 250));
  const pdf = await send("Page.printToPDF", {
    printBackground: true,
    preferCSSPageSize: true,
  });
  const pdfBytes = Buffer.from(pdf.data, "base64");
  await writeFile(PDF_PATH, pdfBytes);
  const pageCounts = [...pdfBytes.toString("latin1").matchAll(/\/Count\s+(\d+)/g)]
    .map((match) => Number(match[1]));
  const pageCount = pageCounts.length ? Math.max(...pageCounts) : null;

  await evaluate("try { localStorage.clear(); } catch (error) {}");
  await load(390, 844, true);
  const mobile = await evaluate(`(() => {
    const missingAnchors = [...document.querySelectorAll('a[href^="#"]')]
      .map((anchor) => anchor.getAttribute("href"))
      .filter((href) => href.length > 1 && !document.getElementById(href.slice(1)));
    return {
      units: document.querySelectorAll(".unit").length,
      chapters: document.querySelectorAll(".chapter").length,
      scrollWidth: document.documentElement.scrollWidth,
      bodyScrollWidth: document.body.scrollWidth,
      innerWidth,
      missingAnchors: [...new Set(missingAnchors)],
    };
  })()`);
  assert(mobile.units === 72, "mobile unit count", String(mobile.units));
  assert(mobile.chapters === 7, "mobile chapter count", String(mobile.chapters));
  assert(mobile.scrollWidth <= mobile.innerWidth + 1, "mobile overflow", `${mobile.scrollWidth}`);
  assert(mobile.bodyScrollWidth <= mobile.innerWidth + 1, "mobile body overflow", `${mobile.bodyScrollWidth}`);
  assert(mobile.missingAnchors.length === 0, "mobile anchors", mobile.missingAnchors.join(","));
  await evaluate("window.scrollTo(0, 0)");
  await new Promise((r) => setTimeout(r, 200));
  await screenshot("book-v2-mobile.png");

  const mobileControls = await evaluate(`(() => {
    document.querySelector('[data-view="formal"]').click();
    const formal = document.body.dataset.view === "formal";
    document.querySelector('[data-action="expand-all"]').click();
    const opened = [...document.querySelectorAll("details")].every((detail) => detail.open);
    document.querySelector('[data-action="collapse-all"]').click();
    const closed = [...document.querySelectorAll("details")].every((detail) => !detail.open);
    return { formal, opened, closed };
  })()`);
  assert(mobileControls.formal && mobileControls.opened && mobileControls.closed, "mobile controls");

  ws.close();

  console.log(JSON.stringify({
    desktop,
    layers,
    panels,
    jump,
    printCss,
    mobile,
    pdf: { path: PDF_PATH, bytes: pdfBytes.length, pages: pageCount },
    screenshots: ["cover", "toc", "unit-ii7", "formal", "mobile"].map(
      (name) => `${OUT_DIR}/book-v2-${name}.png`
    ),
  }, null, 2));
  if (failures.length) {
    console.error("DOM TEST: FAIL\n" + failures.join("\n"));
    process.exitCode = 1;
  } else {
    console.log("DOM TEST: PASS");
  }
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  stopChrome();
}
