/*
 * app.js — analysis interactive book (M1+M2, all units).
 *
 * Deliberately tiny and dependency-free: no CDN, no math renderer, no framework.
 * Everything is delegated from a handful of data-* hooks so the static HTML
 * remains usable when JavaScript is disabled (native <details> still works,
 * and every TOC/unit link is a real #anchor).
 *
 * State kept in localStorage:
 *   soko-book-v2-view       the reader's layer choice
 *   soko-book-progress-v1   explicitly marked-read unit numbers (no accounts,
 *                           no time-on-page or scroll based auto marking)
 *
 * Navigation: same-page anchors go through one route (pushState + focus +
 * sticky-toolbar offset) and back/forward is handled from hashchange/popstate.
 */
(() => {
  "use strict";

  const body = document.body;
  const VIEW_KEY = "soko-book-v2-view";
  const PROGRESS_KEY = "soko-book-progress-v1";
  const SEARCH_LIMIT = 10;
  const views = ["both", "intuition", "formal"];
  const viewButtons = Array.from(document.querySelectorAll(".seg button[data-view]"));
  const KIND_LABELS = {
    chapter: "章",
    unit: "单元",
    section: "小节",
    exercise: "练习",
    appendix: "解答",
    solution: "解答",
    def: "定义",
    theorem: "定理",
    axiom: "公理",
    inductive: "归纳类型",
  };

  let currentView = "both";

  /* ── reading layers ───────────────────────────────────────────────────── */

  function setView(view, persist = true) {
    currentView = views.includes(view) ? view : "both";
    body.dataset.view = currentView;
    for (const button of viewButtons) {
      button.setAttribute("aria-pressed", String(button.dataset.view === currentView));
    }
    if (persist) {
      try {
        localStorage.setItem(VIEW_KEY, currentView);
      } catch (_) {
        /* file:// or private mode: the toggle still works for this session */
      }
    }
  }

  for (const button of viewButtons) {
    button.addEventListener("click", () => setView(button.dataset.view));
  }

  try {
    const saved = localStorage.getItem(VIEW_KEY);
    setView(views.includes(saved) ? saved : "both", false);
  } catch (_) {
    setView("both", false);
  }

  /* ── URL navigation ───────────────────────────────────────────────────── */

  function toolbarOffset() {
    const toolbar = document.querySelector(".toolbar");
    if (!toolbar) return 12;
    const position = getComputedStyle(toolbar).position;
    if (position !== "sticky" && position !== "fixed") return 12;
    return toolbar.getBoundingClientRect().height + 14;
  }

  function scrollToTarget(target, focus = true) {
    if (!target) return false;
    const top = target.getBoundingClientRect().top + window.scrollY - toolbarOffset();
    const previous = document.documentElement.style.scrollBehavior;
    document.documentElement.style.scrollBehavior = "auto";
    window.scrollTo(0, Math.max(0, top));
    document.documentElement.style.scrollBehavior = previous;
    if (focus && typeof target.focus === "function") {
      target.setAttribute("tabindex", "-1");
      target.focus({ preventScroll: true });
    }
    return true;
  }

  function writeHash(hash, push) {
    try {
      const state = { bookAnchor: hash.slice(1) };
      if (push) history.pushState(state, "", hash);
      else history.replaceState(state, "", hash);
      return true;
    } catch (_) {
      /* Some file:// contexts reject pushState; fall back to the native hash
         (which still keeps one history entry and fires hashchange). */
      if (push) window.location.hash = hash;
      else window.location.replace(hash);
      return false;
    }
  }

  function navigateTo(anchor, options = {}) {
    const id = String(anchor).replace(/^#/, "");
    const target = document.getElementById(id);
    if (!target) return false;
    const hash = "#" + id;
    if (window.location.hash !== hash) writeHash(hash, options.push !== false);
    const details = target.closest("details");
    if (details) details.open = true;
    scrollToTarget(target);
    return true;
  }

  function syncFromLocation(focus = true) {
    if (!window.location.hash) return;
    const target = document.getElementById(window.location.hash.slice(1));
    if (target) scrollToTarget(target, focus);
  }

  window.addEventListener("hashchange", () => {
    closeSearch({ clear: false });
    syncFromLocation();
  });
  window.addEventListener("popstate", () => syncFromLocation());

  /* Every same-page anchor shares the navigation path, so TOC links, unit
     prev/next, appendix links and the skip link all update the hash without a
     reload and land below the sticky toolbar. */
  document.addEventListener("click", (event) => {
    if (!(event.target instanceof Element)) return;
    const link = event.target.closest('a[href^="#"]');
    if (!link || event.defaultPrevented || event.button !== 0) return;
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    const href = link.getAttribute("href");
    const target = href && href.length > 1 ? document.getElementById(href.slice(1)) : null;
    if (!target) return;
    event.preventDefault();
    closeSearch({ clear: false });
    navigateTo(href, { push: true });
  });

  /* ── search ───────────────────────────────────────────────────────────── */

  const searchRoot = document.querySelector("[data-search]");
  const searchInput = document.getElementById("book-search");
  const searchPanel = document.getElementById("book-search-panel");
  const searchResults = document.getElementById("book-search-results");
  const searchStatus = document.getElementById("book-search-status");
  const searchIndexNode = document.getElementById("book-search-index");

  let searchEntries = [];
  if (searchIndexNode) {
    try {
      const parsed = JSON.parse(searchIndexNode.textContent || "[]");
      if (Array.isArray(parsed)) searchEntries = parsed;
    } catch (error) {
      console.warn("book search index is not valid JSON", error);
    }
  }
  const searchIndex = searchEntries
    .filter((entry) => entry && typeof entry.t === "string" && typeof entry.a === "string")
    .map((entry) => ({
      entry,
      kindLabel: KIND_LABELS[entry.k] || entry.k || "",
      haystack: [entry.t, entry.k, KIND_LABELS[entry.k], entry.h, entry.e]
        .filter(Boolean)
        .join(" ")
        .toLowerCase(),
    }));

  let activeResult = -1;
  let visibleEntries = [];

  function closeSearch(options = {}) {
    if (!searchPanel || !searchInput || !searchResults || !searchStatus) return;
    searchPanel.hidden = true;
    searchInput.setAttribute("aria-expanded", "false");
    searchInput.removeAttribute("aria-activedescendant");
    if (options.clear) searchInput.value = "";
    searchStatus.textContent = "";
    searchResults.textContent = "";
    activeResult = -1;
    visibleEntries = [];
  }

  function openSearch() {
    if (!searchPanel || !searchInput) return;
    searchPanel.hidden = false;
    searchInput.setAttribute("aria-expanded", "true");
  }

  function chooseSearchEntry(entry) {
    const moved = navigateTo(entry.a, { push: true });
    closeSearch({ clear: true });
    if (searchInput) searchInput.blur();
    return moved;
  }

  function setActiveResult(next) {
    const options = Array.from(searchResults.querySelectorAll('[role="option"]'));
    if (!options.length) {
      activeResult = -1;
      searchInput.removeAttribute("aria-activedescendant");
      return;
    }
    activeResult = ((next % options.length) + options.length) % options.length;
    options.forEach((option, index) => {
      option.setAttribute("aria-selected", String(index === activeResult));
    });
    const option = options[activeResult];
    searchInput.setAttribute("aria-activedescendant", option.id);
    if (typeof option.scrollIntoView === "function") {
      option.scrollIntoView({ block: "nearest" });
    }
  }

  function renderSearch(query) {
    if (!searchRoot || !searchInput || !searchResults || !searchStatus) return;
    const term = query.trim();
    if (!term) {
      closeSearch({ clear: false });
      return;
    }
    const lowered = term.toLowerCase();
    const matches = searchIndex.filter((item) => item.haystack.includes(lowered));
    const shown = matches.slice(0, SEARCH_LIMIT);

    openSearch();
    searchResults.textContent = "";
    activeResult = -1;
    searchInput.removeAttribute("aria-activedescendant");
    visibleEntries = shown.map((item) => item.entry);

    if (!matches.length) {
      openSearch();
      const empty = document.createElement("li");
      empty.className = "search-empty";
      empty.setAttribute("role", "presentation");
      empty.textContent = `没有匹配「${term}」的单元、声明或小节。`;
      searchResults.appendChild(empty);
      searchStatus.textContent = "0 条结果";
      searchInput.dataset.matchCount = "0";
      return;
    }

    shown.forEach((item, index) => {
      const entry = item.entry;
      const option = document.createElement("li");
      option.className = "search-result";
      option.id = `book-search-option-${index}`;
      option.setAttribute("role", "option");
      option.setAttribute("aria-selected", "false");
      option.dataset.anchor = entry.a;

      const kind = document.createElement("span");
      kind.className = "search-kind";
      kind.textContent = item.kindLabel;
      const title = document.createElement("span");
      title.className = "search-title";
      title.textContent = entry.t;
      option.append(kind, title);

      if (entry.h) {
        const context = document.createElement("span");
        context.className = "search-context";
        context.textContent = entry.h;
        option.appendChild(context);
      }
      option.addEventListener("click", () => chooseSearchEntry(entry));
      searchResults.appendChild(option);
    });

    if (matches.length > shown.length) {
      const more = document.createElement("li");
      more.className = "search-more";
      more.setAttribute("role", "presentation");
      more.textContent =
        `仅显示前 ${shown.length} 条，共 ${matches.length} 条命中；` +
        "请输入更具体的关键词。";
      searchResults.appendChild(more);
    }

    searchStatus.textContent =
      `找到 ${matches.length} 条结果` +
      (matches.length > shown.length ? `，显示前 ${shown.length} 条` : "");
    searchInput.dataset.matchCount = String(matches.length);
  }

  if (searchInput) {
    searchInput.addEventListener("input", () => renderSearch(searchInput.value));
    searchInput.addEventListener("focus", () => {
      if (searchInput.value.trim()) renderSearch(searchInput.value);
    });
    searchInput.addEventListener("keydown", (event) => {
      if (event.key === "ArrowDown") {
        if (searchPanel.hidden) renderSearch(searchInput.value);
        if (searchPanel.hidden) return;
        event.preventDefault();
        setActiveResult(activeResult + 1);
      } else if (event.key === "ArrowUp") {
        if (searchPanel.hidden) return;
        event.preventDefault();
        setActiveResult(activeResult - 1);
      } else if (event.key === "Enter") {
        if (!searchPanel.hidden && visibleEntries.length) {
          event.preventDefault();
          const entry = visibleEntries[activeResult >= 0 ? activeResult : 0];
          if (entry) chooseSearchEntry(entry);
        }
      } else if (event.key === "Escape") {
        if (!searchPanel.hidden) {
          event.preventDefault();
          closeSearch({ clear: true });
        }
      }
    });
    searchInput.addEventListener("blur", () => {
      window.setTimeout(() => {
        if (searchRoot && !searchRoot.contains(document.activeElement)) {
          closeSearch({ clear: false });
        }
      }, 120);
    });
    document.addEventListener("click", (event) => {
      if (searchRoot && !searchRoot.contains(event.target)) {
        closeSearch({ clear: false });
      }
    });
  }

  /* ── reading progress ─────────────────────────────────────────────────── */

  const progressToggles = Array.from(document.querySelectorAll("[data-progress-toggle]"));
  const unitByNumber = new Map(
    progressToggles.map((button) => [Number(button.dataset.unit), button])
  );
  const progressTotal = unitByNumber.size;
  const progressCounts = Array.from(document.querySelectorAll("[data-progress-count]"));
  const progressBars = Array.from(document.querySelectorAll("[data-progressbar]"));
  const resetStatus = document.querySelector("[data-reset-status]");
  const resetButton = document.querySelector('[data-action="reset-progress"]');
  const resetConfirm = document.querySelector("[data-reset-confirm]");
  const resetConfirmButton = document.querySelector(
    '[data-action="reset-progress-confirm"]'
  );
  const resetCancelButton = document.querySelector(
    '[data-action="reset-progress-cancel"]'
  );
  const readUnits = new Set();

  function setResetStatus(message, isError = false) {
    if (!resetStatus) return;
    resetStatus.textContent = message;
    resetStatus.className = "reset-status" + (isError ? " is-error" : "");
  }

  function loadProgress() {
    try {
      const raw = localStorage.getItem(PROGRESS_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return;
      for (const value of parsed) {
        const number = Number(value);
        if (unitByNumber.has(number)) readUnits.add(number);
      }
    } catch (_) {
      /* storage unavailable: keep the in-memory set */
    }
  }

  function saveProgress() {
    try {
      localStorage.setItem(
        PROGRESS_KEY,
        JSON.stringify(Array.from(readUnits).sort((left, right) => left - right))
      );
      return true;
    } catch (_) {
      setResetStatus("浏览器不允许本地存储：进度只保留在本次会话。", true);
      return false;
    }
  }

  function updateProgressUI() {
    const read = readUnits.size;
    for (const [number, button] of unitByNumber) {
      const isRead = readUnits.has(number);
      button.setAttribute("aria-pressed", String(isRead));
      const text = button.querySelector(".unit-progress-text");
      if (text) text.textContent = isRead ? "已读" : "标记已读";
      const unit = button.closest(".unit");
      if (unit) unit.classList.toggle("is-read", isRead);
    }
    for (const count of progressCounts) {
      count.textContent = `已读 ${read} / ${progressTotal}`;
    }
    for (const bar of progressBars) {
      bar.setAttribute("aria-valuenow", String(read));
      bar.setAttribute("aria-valuetext", `已读 ${read} / ${progressTotal} 单元`);
    }
    for (const fill of document.querySelectorAll("[data-progress-fill]")) {
      fill.style.width = progressTotal ? `${(read / progressTotal) * 100}%` : "0%";
    }
    body.dataset.progressRead = String(read);
    body.dataset.progressTotal = String(progressTotal);
  }

  for (const button of progressToggles) {
    button.addEventListener("click", () => {
      const number = Number(button.dataset.unit);
      if (!unitByNumber.has(number)) return;
      if (readUnits.has(number)) readUnits.delete(number);
      else readUnits.add(number);
      saveProgress();
      updateProgressUI();
    });
  }

  if (resetButton && resetConfirm) {
    resetButton.addEventListener("click", () => {
      if (!readUnits.size) {
        setResetStatus("还没有已读单元，无需重置。");
        resetConfirm.hidden = true;
        resetButton.setAttribute("aria-expanded", "false");
        return;
      }
      resetConfirm.hidden = false;
      resetButton.setAttribute("aria-expanded", "true");
      setResetStatus(`将清除 ${readUnits.size} 个已读标记，请确认。`);
      if (resetConfirmButton) resetConfirmButton.focus();
    });
    if (resetConfirmButton) {
      resetConfirmButton.addEventListener("click", () => {
        readUnits.clear();
        saveProgress();
        updateProgressUI();
        resetConfirm.hidden = true;
        resetButton.setAttribute("aria-expanded", "false");
        setResetStatus("阅读进度已重置。");
        resetButton.focus();
      });
    }
    if (resetCancelButton) {
      resetCancelButton.addEventListener("click", () => {
        resetConfirm.hidden = true;
        resetButton.setAttribute("aria-expanded", "false");
        setResetStatus("已取消重置。");
        resetButton.focus();
      });
    }
    document.addEventListener("keydown", (event) => {
      if (event.key !== "Escape" || resetConfirm.hidden) return;
      resetConfirm.hidden = true;
      resetButton.setAttribute("aria-expanded", "false");
      resetButton.focus();
    });
  }

  loadProgress();
  updateProgressUI();

  /* ── copy a stable unit link ──────────────────────────────────────────── */

  function legacyCopy(text) {
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.className = "copy-fallback";
    document.body.appendChild(area);
    area.select();
    area.setSelectionRange(0, text.length);
    let copied = false;
    try {
      copied = document.execCommand("copy");
    } catch (_) {
      copied = false;
    }
    area.remove();
    return copied;
  }

  function copyStatusFor(button) {
    const container = button.closest(".unit-actions");
    return container ? container.querySelector("[data-copy-status]") : null;
  }

  function showCopyStatus(status, message, isError) {
    if (!status) return;
    status.textContent = message;
    status.className = "unit-copy-status" + (isError ? " is-error" : " is-ok");
    if (isError) return;
    window.setTimeout(() => {
      if (status.textContent === message) {
        status.textContent = "";
        status.className = "unit-copy-status";
      }
    }, 4000);
  }

  async function copyUnitLink(button) {
    const anchor = button.dataset.copyLink;
    if (!anchor) return;
    const status = copyStatusFor(button);
    const url = new URL(window.location.href);
    url.hash = anchor;
    const text = url.toString();
    let copied = false;
    try {
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(text);
        copied = true;
      }
    } catch (_) {
      copied = false;
    }
    if (!copied) copied = legacyCopy(text);
    if (copied) {
      showCopyStatus(status, `已复制链接：#${anchor}`, false);
      return;
    }
    if (!status) return;
    /* Do not fail silently: show the URL in a selected, read-only field. */
    status.textContent = "无法自动复制，请手动复制：";
    status.className = "unit-copy-status is-error";
    const field = document.createElement("input");
    field.type = "text";
    field.readOnly = true;
    field.className = "copy-fallback-input";
    field.value = text;
    status.appendChild(field);
    field.focus();
    field.select();
  }

  for (const button of document.querySelectorAll("[data-copy-link]")) {
    button.addEventListener("click", () => {
      copyUnitLink(button);
    });
  }

  /* ── chapter jump ─────────────────────────────────────────────────────── */

  const chapterJump = document.querySelector('select[data-action="chapter-jump"]');
  if (chapterJump) {
    chapterJump.addEventListener("change", () => {
      if (chapterJump.value) navigateTo(chapterJump.value, { push: true });
    });
    const syncChapter = () => {
      const hash = window.location.hash;
      if (hash.startsWith("#chapter-")) {
        chapterJump.value = hash;
        return;
      }
      const target = hash.length > 1 ? document.getElementById(hash.slice(1)) : null;
      const chapter = target && target.closest(".chapter");
      if (chapter) chapterJump.value = "#" + chapter.id;
    };
    window.addEventListener("hashchange", syncChapter);
    window.addEventListener("popstate", syncChapter);
    syncChapter();
  }

  /* ── details controls / print ─────────────────────────────────────────── */

  function setAllDetails(open) {
    for (const item of document.querySelectorAll("details")) item.open = open;
  }

  for (const action of document.querySelectorAll("[data-action]")) {
    action.addEventListener("click", () => {
      const name = action.dataset.action;
      if (name === "expand-all") setAllDetails(true);
      else if (name === "collapse-all") setAllDetails(false);
      else if (name === "print") window.print();
    });
  }

  /*
   * Print wants the whole book, not the collapsed screen state: closed
   * <details> are not painted. Open everything for the print pass, then put
   * the reader's state back afterwards.
   */
  let printState = null;
  window.addEventListener("beforeprint", () => {
    const details = Array.from(document.querySelectorAll("details"));
    printState = details.map((item) => item.open);
    for (const item of details) item.open = true;
  });

  window.addEventListener("afterprint", () => {
    if (!printState) return;
    const details = Array.from(document.querySelectorAll("details"));
    details.forEach((item, index) => {
      if (index < printState.length) item.open = printState[index];
    });
    printState = null;
  });

  /* Deep link on load: the browser may already have jumped, but this applies
     the sticky-toolbar offset and puts keyboard focus on the target. */
  if (window.location.hash) {
    const target = document.getElementById(window.location.hash.slice(1));
    if (target) scrollToTarget(target);
  }

  /* Expose read-only state for the headless DOM test without changing
     visible behaviour. */
  body.dataset.viewReady = "true";
  body.dataset.searchEntries = String(searchIndex.length);
  body.dataset.searchReady = "true";
  const syncViewState = () => {
    body.dataset.viewCurrent = body.dataset.view || currentView;
  };
  syncViewState();
  new MutationObserver(syncViewState).observe(body, {
    attributes: true,
    attributeFilter: ["data-view"],
  });
})();
