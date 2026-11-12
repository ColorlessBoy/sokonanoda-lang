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

const wait = (ms) => new Promise((r) => setTimeout(r, ms));

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

  function track(method, handler) {
    if (!events.has(method)) events.set(method, new Set());
    events.get(method).add(handler);
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

  let loadSequence = 0;
  async function load(width, height, mobile = false, hash = "") {
    await setViewport(width, height, mobile);
    const loaded = once("Page.loadEventFired");
    // A unique query forces a real document load even when only the hash
    // differs; navigating to the same document with a new fragment does not
    // fire Page.loadEventFired.
    await send("Page.navigate", {
      url: `file://${BOOK_HTML}?capture=${++loadSequence}${hash}`,
    });
    await loaded;
    await wait(500);
  }

  async function reload() {
    const loaded = once("Page.loadEventFired");
    await send("Page.reload", { ignoreCache: true });
    await loaded;
    await wait(400);
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
    await wait(400);
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

  const externalRequests = [];
  const consoleErrors = [];
  const pageErrors = [];
  track("Network.requestWillBeSent", ({ request }) => {
    if (/^https?:/i.test(request.url)) externalRequests.push(request.url);
  });
  track("Runtime.exceptionThrown", ({ exceptionDetails }) => {
    pageErrors.push(
      exceptionDetails.exception?.description ||
        exceptionDetails.text ||
        "uncaught exception"
    );
  });
  track("Runtime.consoleAPICalled", ({ type, args }) => {
    if (type !== "error") return;
    consoleErrors.push(
      (args || []).map((arg) => arg.value ?? arg.description ?? "").join(" ")
    );
  });
  track("Log.entryAdded", ({ entry }) => {
    if (entry.level === "error") consoleErrors.push(entry.text || "log error");
  });

  const failures = [];
  const assert = (condition, label, detail = "") => {
    if (!condition) failures.push(`${label}${detail ? `: ${detail}` : ""}`);
  };

  await send("Page.enable");
  await send("Runtime.enable");
  await send("Network.enable");
  await send("Log.enable");
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
        search: Boolean(document.getElementById("book-search")),
        progress: Boolean(document.querySelector('[data-progress-widget] [role="progressbar"]')),
        reset: Boolean(document.querySelector('[data-action="reset-progress"]')),
        copy: document.querySelectorAll("[data-copy-link]").length,
      },
      searchEntries: Number(document.body.dataset.searchEntries || 0),
      searchVisible: (() => {
        const input = document.getElementById("book-search");
        const rect = input.getBoundingClientRect();
        const style = getComputedStyle(input);
        return style.display !== "none" && style.visibility !== "hidden" && rect.width > 80;
      })(),
      searchWidth: Math.round(document.getElementById("book-search").getBoundingClientRect().width),
      progressToggles: document.querySelectorAll("[data-progress-toggle]").length,
      progressCount: document.querySelector('[data-progress-widget] [data-progress-count]').textContent,
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
  assert(desktop.controls.search && desktop.controls.progress && desktop.controls.reset, "search/progress controls");
  assert(desktop.controls.copy === desktop.units, "copy-link controls", `${desktop.controls.copy}/${desktop.units}`);
  assert(desktop.searchEntries > desktop.units + desktop.chapters, "search index entries", String(desktop.searchEntries));
  assert(desktop.searchVisible && desktop.searchWidth > 80, "search input is visible", String(desktop.searchWidth));
  assert(desktop.progressToggles === desktop.units, "progress toggles", String(desktop.progressToggles));
  assert(desktop.progressCount === "已读 0 / 72", "initial progress count", desktop.progressCount);

  await evaluate("window.scrollTo(0, 0)");
  await wait(200);
  await screenshot("book-v2-cover.png", {
    x: 0,
    y: 0,
    width: 1440,
    height: Math.min(desktop.coverHeight, 1600),
    scale: 1,
  });

  await scrollTo("#toc");
  await screenshot("book-v2-toc.png");

  /* ── search ──────────────────────────────────────────────────────────── */

  const searchBasics = await evaluate(`(() => {
    const input = document.getElementById("book-search");
    input.focus();
    input.value = "极限";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    const options = [...document.querySelectorAll("#book-search-results [role=option]")];
    const first = options[0];
    const panel = document.getElementById("book-search-panel");
    const panelRect = panel.getBoundingClientRect();
    return {
      panelLeft: Math.round(panelRect.left),
      panelRight: Math.round(panelRect.right),
      innerWidth,
      indexEntries: JSON.parse(document.getElementById("book-search-index").textContent).length,
      matchCount: Number(input.dataset.matchCount),
      optionCount: options.length,
      firstTitle: first ? first.querySelector(".search-title").textContent : "",
      firstAnchor: first ? first.dataset.anchor : "",
      status: document.getElementById("book-search-status").textContent,
      expanded: input.getAttribute("aria-expanded"),
      panelHidden: document.getElementById("book-search-panel").hidden,
      listboxRole: document.getElementById("book-search-results").getAttribute("role"),
      live: document.getElementById("book-search-status").getAttribute("aria-live"),
    };
  })()`);
  assert(searchBasics.matchCount >= 1, "search 极限 match count", String(searchBasics.matchCount));
  assert(searchBasics.optionCount >= 1, "search 极限 option count", String(searchBasics.optionCount));
  assert(searchBasics.status.includes("找到"), "search result count status", searchBasics.status);
  assert(searchBasics.expanded === "true" && searchBasics.panelHidden === false, "search panel open");
  assert(searchBasics.listboxRole === "listbox" && searchBasics.live === "polite", "search ARIA semantics");
  assert(searchBasics.panelLeft >= 0 && searchBasics.panelRight <= searchBasics.innerWidth + 1, "desktop search panel does not overflow", `${searchBasics.panelLeft}..${searchBasics.panelRight} / ${searchBasics.innerWidth}`);

  const searchArrows = await evaluate(`(() => {
    const input = document.getElementById("book-search");
    const dispatch = (key) => input.dispatchEvent(new KeyboardEvent("keydown", {
      key, bubbles: true, cancelable: true,
    }));
    input.value = "theorem";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    const optionCount = document.querySelectorAll('#book-search-results [role="option"]').length;
    dispatch("ArrowDown");
    const firstActive = input.getAttribute("aria-activedescendant");
    dispatch("ArrowDown");
    const secondActive = input.getAttribute("aria-activedescendant");
    dispatch("ArrowUp");
    const backActive = input.getAttribute("aria-activedescendant");
    return { optionCount, firstActive, secondActive, backActive };
  })()`);
  assert(searchArrows.optionCount === 10, "arrow test has multiple results", String(searchArrows.optionCount));
  assert(searchArrows.firstActive && searchArrows.secondActive && searchArrows.firstActive !== searchArrows.secondActive, "search ArrowDown moves the active option", `${searchArrows.firstActive} / ${searchArrows.secondActive}`);
  assert(searchArrows.backActive === searchArrows.firstActive, "search ArrowUp moves back", searchArrows.backActive);

  const searchNav = await evaluate(`(() => {
    const input = document.getElementById("book-search");
    const dispatch = (key) => input.dispatchEvent(new KeyboardEvent("keydown", {
      key, bubbles: true, cancelable: true,
    }));
    input.value = "极限";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    dispatch("ArrowDown");
    const selected = document.querySelector('#book-search-results [aria-selected="true"]');
    const anchor = selected ? selected.dataset.anchor : "";
    dispatch("Enter");
    const target = anchor ? document.getElementById(anchor) : null;
    return {
      activeBeforeEnter: Boolean(selected),
      expectedHash: anchor ? "#" + anchor : "",
      hash: window.location.hash,
      inUnit: Boolean(target && target.closest(".unit")),
      panelHidden: document.getElementById("book-search-panel").hidden,
      expanded: input.getAttribute("aria-expanded"),
    };
  })()`);
  assert(searchNav.activeBeforeEnter, "search ArrowDown selects a result");
  assert(searchNav.hash === searchNav.expectedHash && searchNav.hash.length > 1, "search Enter navigates to hash", `${searchNav.hash} != ${searchNav.expectedHash}`);
  assert(searchNav.inUnit, "search result target is inside a unit");
  assert(searchNav.panelHidden && searchNav.expanded === "false", "search closes after Enter");

  const searchClick = await evaluate(`(() => {
    const input = document.getElementById("book-search");
    input.focus();
    input.value = "极限";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    const option = document.querySelector('#book-search-results [role="option"]');
    const anchor = option ? option.dataset.anchor : "";
    if (option) option.click();
    return {
      anchor,
      hash: window.location.hash,
      panelHidden: document.getElementById("book-search-panel").hidden,
      inUnit: Boolean(document.getElementById(anchor)?.closest(".unit")),
    };
  })()`);
  assert(searchClick.anchor && searchClick.hash === "#" + searchClick.anchor && searchClick.panelHidden && searchClick.inUnit, "search result click navigates and closes", JSON.stringify(searchClick));

  const searchNames = await evaluate(`(() => {
    const input = document.getElementById("book-search");
    const entries = JSON.parse(document.getElementById("book-search-index").textContent);
    const decl = entries.find((entry) => entry.t === "demo_intRel")
      || entries.find((entry) => ["def", "theorem", "inductive"].includes(entry.k) && entry.t.length > 5);
    const probe = (query) => {
      input.focus();
      input.value = query;
      input.dispatchEvent(new Event("input", { bubbles: true }));
      const options = [...document.querySelectorAll("#book-search-results [role=option]")];
      return {
        count: Number(input.dataset.matchCount),
        optionCount: options.length,
        firstTitle: options[0] ? options[0].querySelector(".search-title").textContent : "",
        more: Boolean(document.querySelector(".search-more")),
        empty: Boolean(document.querySelector(".search-empty")),
        status: document.getElementById("book-search-status").textContent,
      };
    };
    const byName = probe(decl.t);
    const byType = probe("theorem");
    const byUpperType = probe("THEOREM");
    const byChineseType = probe("定理");
    const none = probe("zzzz-no-such-entry");
    input.value = "";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    return { declName: decl.t, byName, byType, byUpperType, byChineseType, none };
  })()`);
  assert(searchNames.byName.count >= 1, "search by declaration name", `${searchNames.declName} → ${searchNames.byName.count}`);
  assert(searchNames.byName.firstTitle.toLowerCase() === searchNames.declName.toLowerCase(), "declaration name is the first hit", searchNames.byName.firstTitle);
  assert(searchNames.byType.count >= 1 && searchNames.byType.optionCount === 10 && searchNames.byType.more, "search by type caps at 10 with 更多 note", JSON.stringify(searchNames.byType));
  assert(searchNames.byUpperType.count === searchNames.byType.count, "search is case-insensitive", `${searchNames.byUpperType.count} vs ${searchNames.byType.count}`);
  assert(searchNames.byChineseType.count >= 1, "search by Chinese type 定理", String(searchNames.byChineseType.count));
  assert(searchNames.none.count === 0 && searchNames.none.optionCount === 0 && searchNames.none.empty, "search empty state", JSON.stringify(searchNames.none));
  assert(searchNames.none.status.includes("0"), "search empty count status", searchNames.none.status);

  const searchEscape = await evaluate(`(() => {
    const input = document.getElementById("book-search");
    input.focus();
    input.value = "定理";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    const opened = !document.getElementById("book-search-panel").hidden;
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    return {
      opened,
      hiddenAfter: document.getElementById("book-search-panel").hidden,
      valueAfter: input.value,
      expandedAfter: input.getAttribute("aria-expanded"),
    };
  })()`);
  assert(searchEscape.opened && searchEscape.hiddenAfter && searchEscape.valueAfter === "" && searchEscape.expandedAfter === "false", "search Escape closes and clears", JSON.stringify(searchEscape));

  await evaluate("window.scrollTo(0, 0)");
  await evaluate(`(() => {
    const input = document.getElementById("book-search");
    input.focus();
    input.value = "极限";
    input.dispatchEvent(new Event("input", { bubbles: true }));
  })()`);
  await wait(250);
  await screenshot("book-v2-search.png");
  await evaluate(`(() => {
    const input = document.getElementById("book-search");
    input.value = "";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.blur();
  })()`);

  /* ── reading progress ────────────────────────────────────────────────── */

  await load(1440, 1000);
  const progressInit = await evaluate(`(() => {
    try { localStorage.removeItem("soko-book-progress-v1"); } catch (_) {}
    return {
      key: (() => { try { return localStorage.getItem("soko-book-progress-v1"); } catch (_) { return "unavailable"; } })(),
      count: document.querySelector('[data-progress-widget] [data-progress-count]').textContent,
      read: document.body.dataset.progressRead,
    };
  })()`);
  assert(progressInit.count === "已读 0 / 72" && progressInit.read === "0", "initial progress is zero", JSON.stringify(progressInit));

  const progressMark = await evaluate(`(() => {
    const toggles = [...document.querySelectorAll("[data-progress-toggle]")];
    const one = toggles.find((button) => button.dataset.unit === "1");
    const two = toggles.find((button) => button.dataset.unit === "2");
    one.click();
    two.click();
    const bar = document.querySelector('[data-progress-widget] [role="progressbar"]');
    return {
      pressed: [one.getAttribute("aria-pressed"), two.getAttribute("aria-pressed")],
      labels: [
        one.querySelector(".unit-progress-text").textContent,
        two.querySelector(".unit-progress-text").textContent,
      ],
      unitClass: document.getElementById("unit-1").classList.contains("is-read"),
      countTop: document.querySelector('[data-progress-widget] [data-progress-count]').textContent,
      countToc: document.querySelector('[data-progress-settings] [data-progress-count]').textContent,
      now: bar.getAttribute("aria-valuenow"),
      valuetext: bar.getAttribute("aria-valuetext"),
      fill: document.querySelector('[data-progress-widget] [data-progress-fill]').style.width,
      stored: JSON.parse(localStorage.getItem("soko-book-progress-v1")),
      read: document.body.dataset.progressRead,
    };
  })()`);
  assert(progressMark.pressed[0] === "true" && progressMark.pressed[1] === "true", "progress aria-pressed", progressMark.pressed.join(","));
  assert(progressMark.labels[0] === "已读" && progressMark.labels[1] === "已读", "progress labels", progressMark.labels.join(","));
  assert(progressMark.unitClass, "read unit class");
  assert(progressMark.countTop === "已读 2 / 72" && progressMark.countToc === "已读 2 / 72", "topbar/TOC progress count", `${progressMark.countTop} | ${progressMark.countToc}`);
  assert(progressMark.now === "2" && progressMark.valuetext.includes("2 / 72"), "progressbar aria values", `${progressMark.now} ${progressMark.valuetext}`);
  assert(progressMark.stored.join(",") === "1,2" && progressMark.read === "2", "progress localStorage", JSON.stringify(progressMark.stored));

  await scrollTo("#unit-1");
  await screenshot("book-v2-progress.png");

  await reload();
  const progressReload = await evaluate(`(() => {
    const bar = document.querySelector('[data-progress-widget] [role="progressbar"]');
    return {
      count: document.querySelector('[data-progress-widget] [data-progress-count]').textContent,
      now: bar.getAttribute("aria-valuenow"),
      stored: JSON.parse(localStorage.getItem("soko-book-progress-v1")),
      pressed: [...document.querySelectorAll("[data-progress-toggle]")]
        .filter((button) => button.getAttribute("aria-pressed") === "true")
        .map((button) => button.dataset.unit),
    };
  })()`);
  assert(progressReload.count === "已读 2 / 72", "progress survives reload", progressReload.count);
  assert(progressReload.now === "2" && progressReload.stored.join(",") === "1,2", "progress bar/storage after reload", JSON.stringify(progressReload));
  assert(progressReload.pressed.join(",") === "1,2", "progress toggles after reload", progressReload.pressed.join(","));

  const progressToggle = await evaluate(`(() => {
    const one = [...document.querySelectorAll("[data-progress-toggle]")]
      .find((button) => button.dataset.unit === "1");
    one.click();
    const afterUnmark = {
      count: document.querySelector('[data-progress-widget] [data-progress-count]').textContent,
      stored: JSON.parse(localStorage.getItem("soko-book-progress-v1")),
    };
    one.click();
    const afterRemark = {
      count: document.querySelector('[data-progress-widget] [data-progress-count]').textContent,
      stored: JSON.parse(localStorage.getItem("soko-book-progress-v1")),
    };
    return { afterUnmark, afterRemark };
  })()`);
  assert(progressToggle.afterUnmark.count === "已读 1 / 72" && progressToggle.afterUnmark.stored.join(",") === "2", "unmark updates count", JSON.stringify(progressToggle.afterUnmark));
  assert(progressToggle.afterRemark.count === "已读 2 / 72" && progressToggle.afterRemark.stored.join(",") === "1,2", "re-mark updates count", JSON.stringify(progressToggle.afterRemark));

  const resetFlow = await evaluate(`(() => {
    const reset = document.querySelector('[data-action="reset-progress"]');
    const confirmBox = document.querySelector('[data-reset-confirm]');
    const confirmButton = document.querySelector('[data-action="reset-progress-confirm"]');
    const count = () => document.querySelector('[data-progress-widget] [data-progress-count]').textContent;
    reset.click();
    const afterFirstClick = {
      visible: !confirmBox.hidden,
      count: count(),
      stored: localStorage.getItem("soko-book-progress-v1"),
      status: document.querySelector('[data-reset-status]').textContent,
      expanded: reset.getAttribute("aria-expanded"),
    };
    confirmButton.click();
    return {
      afterFirstClick,
      countAfter: count(),
      storedAfter: JSON.parse(localStorage.getItem("soko-book-progress-v1")),
      pressedAfter: [...document.querySelectorAll("[data-progress-toggle]")]
        .filter((button) => button.getAttribute("aria-pressed") === "true").length,
      statusAfter: document.querySelector('[data-reset-status]').textContent,
    };
  })()`);
  assert(resetFlow.afterFirstClick.visible, "reset asks for confirmation");
  assert(resetFlow.afterFirstClick.count === "已读 2 / 72" && resetFlow.afterFirstClick.stored === "[1,2]", "reset does not clear before confirmation", JSON.stringify(resetFlow.afterFirstClick));
  assert(resetFlow.countAfter === "已读 0 / 72" && resetFlow.storedAfter.length === 0, "reset clears progress", `${resetFlow.countAfter} ${JSON.stringify(resetFlow.storedAfter)}`);
  assert(resetFlow.pressedAfter === 0 && resetFlow.statusAfter.includes("重置"), "reset updates toggles and status", resetFlow.statusAfter);

  await reload();
  const progressAfterReset = await evaluate(`(() => ({
    count: document.querySelector('[data-progress-widget] [data-progress-count]').textContent,
    stored: JSON.parse(localStorage.getItem("soko-book-progress-v1")),
  }))()`);
  assert(progressAfterReset.count === "已读 0 / 72" && progressAfterReset.stored.length === 0, "reset survives reload", JSON.stringify(progressAfterReset));

  /* ── per-unit URLs / copy link / back-forward ────────────────────────── */

  await load(1440, 1000, false, "#unit-42");
  const deepLink = await evaluate(`(() => {
    const unit = document.getElementById("unit-42");
    return {
      hash: window.location.hash,
      scrollY: window.scrollY,
      top: Math.round(unit.getBoundingClientRect().top),
      active: document.activeElement.id,
      chapterValue: document.querySelector('select[data-action="chapter-jump"]').value,
    };
  })()`);
  assert(deepLink.hash === "#unit-42", "deep link hash", deepLink.hash);
  assert(deepLink.scrollY > 0, "deep link scrolled", String(deepLink.scrollY));
  assert(deepLink.top >= -8 && deepLink.top <= 170, "deep link lands below the toolbar", String(deepLink.top));
  assert(deepLink.active === "unit-42", "deep link focuses unit", deepLink.active);
  assert(deepLink.chapterValue.startsWith("#chapter-"), "deep link syncs chapter select", deepLink.chapterValue);

  const nextNav = await evaluate(`(() => {
    const next = document.querySelector('#unit-42 .unit-nav a[rel="next"]');
    next.click();
    return new Promise((done) => setTimeout(() => done({
      hash: window.location.hash,
      active: document.activeElement.id,
      top: Math.round(document.getElementById("unit-43").getBoundingClientRect().top),
    }), 180));
  })()`);
  assert(nextNav.hash === "#unit-43", "prev/next updates hash", nextNav.hash);
  assert(nextNav.active === "unit-43" && nextNav.top >= -8 && nextNav.top <= 170, "next unit scrolls/focuses", JSON.stringify(nextNav));

  const backNav = await evaluate(`(() => {
    history.back();
    return new Promise((done) => setTimeout(() => done({
      hash: window.location.hash,
      active: document.activeElement.id,
      top: Math.round(document.getElementById("unit-42").getBoundingClientRect().top),
    }), 450));
  })()`);
  assert(backNav.hash === "#unit-42", "back restores hash", backNav.hash);
  assert(backNav.active === "unit-42" && backNav.top >= -8 && backNav.top <= 170, "back restores scroll/focus", JSON.stringify(backNav));

  const copyLink = await evaluate(`(async () => {
    const button = document.querySelector('[data-copy-link="unit-42"]');
    button.click();
    await new Promise((done) => setTimeout(done, 250));
    const status = button.closest(".unit-actions").querySelector("[data-copy-status]");
    const field = status.querySelector("input");
    return {
      hasButton: Boolean(button),
      label: button.getAttribute("aria-label"),
      status: status.textContent,
      fallback: field ? field.value : "",
    };
  })()`);
  assert(copyLink.hasButton && copyLink.label.includes("42"), "copy-link control exists", JSON.stringify(copyLink));
  assert(copyLink.status.includes("#unit-42") || copyLink.fallback.includes("#unit-42"), "copy-link reports success or shows the URL", JSON.stringify(copyLink));
  await screenshot("book-v2-unit-url.png");

  await load(1440, 1000, false, "#chapter-ii-7");
  const chapterDeep = await evaluate(`(() => ({
    hash: window.location.hash,
    active: document.activeElement.id,
    top: Math.round(document.getElementById("chapter-ii-7").getBoundingClientRect().top),
  }))()`);
  assert(chapterDeep.hash === "#chapter-ii-7", "chapter deep link hash", chapterDeep.hash);
  assert(chapterDeep.active === "chapter-ii-7", "chapter deep link focuses chapter", chapterDeep.active);
  assert(chapterDeep.top >= -8 && chapterDeep.top <= 170, "chapter deep link lands below the toolbar", String(chapterDeep.top));

  await load(1440, 1000, false, "#appendix-unit-42");
  const appendixDeep = await evaluate(`(() => ({
    hash: window.location.hash,
    active: document.activeElement.id,
    top: Math.round(document.getElementById("appendix-unit-42").getBoundingClientRect().top),
  }))()`);
  assert(appendixDeep.hash === "#appendix-unit-42", "appendix deep link hash", appendixDeep.hash);
  assert(appendixDeep.active === "appendix-unit-42", "appendix deep link focuses section", appendixDeep.active);
  assert(appendixDeep.top >= -8 && appendixDeep.top <= 170, "appendix deep link lands below the toolbar", String(appendixDeep.top));

  await load(1440, 1000);
  const tocNav = await evaluate(`(() => {
    const link = document.querySelector('#toc a[href="#unit-5"]');
    link.click();
    return new Promise((done) => setTimeout(() => done({
      hash: window.location.hash,
      active: document.activeElement.id,
      top: Math.round(document.getElementById("unit-5").getBoundingClientRect().top),
    }), 180));
  })()`);
  assert(tocNav.hash === "#unit-5" && tocNav.active === "unit-5", "TOC link updates hash without reload", JSON.stringify(tocNav));

  const hashChange = await evaluate(`(() => {
    window.location.hash = "#unit-7";
    return new Promise((done) => setTimeout(() => done({
      hash: window.location.hash,
      active: document.activeElement.id,
      top: Math.round(document.getElementById("unit-7").getBoundingClientRect().top),
    }), 450));
  })()`);
  assert(hashChange.hash === "#unit-7" && hashChange.active === "unit-7", "hashchange scrolls and focuses", JSON.stringify(hashChange));

  await load(1440, 1000);

  /* ── layers / details / print (unchanged features) ───────────────────── */

  await scrollTo("#toc");
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
  await wait(100);
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
  await wait(250);
  const pdf = await send("Page.printToPDF", {
    printBackground: true,
    preferCSSPageSize: true,
  });
  const pdfBytes = Buffer.from(pdf.data, "base64");
  await writeFile(PDF_PATH, pdfBytes);
  const pageCounts = [...pdfBytes.toString("latin1").matchAll(/\/Count\s+(\d+)/g)]
    .map((match) => Number(match[1]));
  const pageCount = pageCounts.length ? Math.max(...pageCounts) : null;

  /* ── mobile ──────────────────────────────────────────────────────────── */

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
      search: Boolean(document.getElementById("book-search")),
      progressToggles: document.querySelectorAll("[data-progress-toggle]").length,
      copyLinks: document.querySelectorAll("[data-copy-link]").length,
    };
  })()`);
  assert(mobile.units === 72, "mobile unit count", String(mobile.units));
  assert(mobile.chapters === 7, "mobile chapter count", String(mobile.chapters));
  assert(mobile.scrollWidth <= mobile.innerWidth + 1, "mobile overflow", `${mobile.scrollWidth}`);
  assert(mobile.bodyScrollWidth <= mobile.innerWidth + 1, "mobile body overflow", `${mobile.bodyScrollWidth}`);
  assert(mobile.missingAnchors.length === 0, "mobile anchors", mobile.missingAnchors.join(","));
  assert(mobile.search && mobile.progressToggles === 72 && mobile.copyLinks === 72, "mobile search/progress controls", JSON.stringify(mobile));

  const mobileSearch = await evaluate(`(() => {
    const input = document.getElementById("book-search");
    input.focus();
    input.value = "定理";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    const panel = document.getElementById("book-search-panel");
    const rect = panel.getBoundingClientRect();
    return {
      hidden: panel.hidden,
      count: Number(input.dataset.matchCount),
      optionCount: document.querySelectorAll("#book-search-results [role=option]").length,
      left: Math.round(rect.left),
      right: Math.round(rect.right),
      scrollWidth: document.documentElement.scrollWidth,
      innerWidth,
    };
  })()`);
  assert(!mobileSearch.hidden && mobileSearch.count > 0 && mobileSearch.optionCount === 10, "mobile search opens", JSON.stringify(mobileSearch));
  assert(mobileSearch.left >= -1 && mobileSearch.right <= mobileSearch.innerWidth + 1, "mobile search panel does not overflow", JSON.stringify(mobileSearch));
  assert(mobileSearch.scrollWidth <= mobileSearch.innerWidth + 1, "mobile search no page overflow", `${mobileSearch.scrollWidth}`);
  await evaluate(`(() => {
    const input = document.getElementById("book-search");
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    input.blur();
  })()`);
  await wait(150);

  await evaluate("window.scrollTo(0, 0)");
  await wait(200);
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

  assert(externalRequests.length === 0, "zero external requests", externalRequests.slice(0, 5).join(","));
  assert(pageErrors.length === 0, "no uncaught page errors", pageErrors.slice(0, 3).join(" | "));
  assert(consoleErrors.length === 0, "no console errors", consoleErrors.slice(0, 3).join(" | "));

  ws.close();

  console.log(JSON.stringify({
    desktop,
    search: { basics: searchBasics, arrows: searchArrows, nav: searchNav, click: searchClick, names: searchNames },
    progress: { init: progressInit, mark: progressMark, reload: progressReload, toggle: progressToggle, reset: resetFlow, afterReset: progressAfterReset },
    urls: { deepLink, nextNav, backNav, copyLink, chapterDeep, appendixDeep, tocNav, hashChange },
    layers,
    panels,
    jump,
    printCss,
    mobile,
    mobileSearch,
    network: { externalRequests, consoleErrors, pageErrors },
    pdf: { path: PDF_PATH, bytes: pdfBytes.length, pages: pageCount },
    screenshots: ["cover", "toc", "search", "progress", "unit-url", "formal", "unit-ii7", "mobile"].map(
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
