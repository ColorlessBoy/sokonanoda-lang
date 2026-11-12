/*
 * app.js — analysis interactive book (M1+M2, all units).
 *
 * Deliberately tiny and dependency-free: no CDN, no math renderer, no framework.
 * Everything is delegated from a handful of data-* hooks so the static HTML
 * remains usable when JavaScript is disabled (native <details> still works).
 *
 * State kept in localStorage: the reader's layer choice only.  Expanded /
 * collapsed panel state is intentionally not persisted, so a fresh visit shows
 * the book's own defaults.
 */
(() => {
  "use strict";

  const body = document.body;
  const VIEW_KEY = "soko-book-v2-view";
  const views = ["both", "intuition", "formal"];
  const buttons = Array.from(document.querySelectorAll(".seg button[data-view]"));

  let currentView = "both";

  function setView(view, persist = true) {
    currentView = views.includes(view) ? view : "both";
    body.dataset.view = currentView;
    for (const button of buttons) {
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

  for (const button of buttons) {
    button.addEventListener("click", () => setView(button.dataset.view));
  }

  try {
    const saved = localStorage.getItem(VIEW_KEY);
    setView(views.includes(saved) ? saved : "both", false);
  } catch (_) {
    setView("both", false);
  }

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

  /* Chapter jump: a plain select keeps the toolbar compact on mobile. */
  const chapterJump = document.querySelector('select[data-action="chapter-jump"]');
  if (chapterJump) {
    chapterJump.addEventListener("change", () => {
      if (chapterJump.value) window.location.hash = chapterJump.value;
    });
    const syncChapter = () => {
      const hash = window.location.hash;
      if (hash.startsWith("#chapter-")) {
        chapterJump.value = hash;
      } else if (hash.startsWith("#unit-")) {
        const unit = document.getElementById(hash.slice(1));
        const chapter = unit && unit.closest(".chapter");
        if (chapter) chapterJump.value = "#" + chapter.id;
      }
    };
    window.addEventListener("hashchange", syncChapter);
    syncChapter();
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

  /* A small affordance for deep links: focus the target so keyboard users
     land on the section, not merely scroll past it. */
  function focusHash() {
    if (!window.location.hash) return;
    const target = document.getElementById(window.location.hash.slice(1));
    if (target && typeof target.focus === "function") {
      target.setAttribute("tabindex", "-1");
      target.focus({ preventScroll: true });
    }
  }

  window.addEventListener("hashchange", focusHash);
  focusHash();

  /* Expose the layer state for the headless DOM test without changing
     visible behaviour. */
  body.dataset.viewReady = "true";
  const syncViewState = () => {
    body.dataset.viewCurrent = body.dataset.view || currentView;
  };
  syncViewState();
  new MutationObserver(syncViewState).observe(body, {
    attributes: true,
    attributeFilter: ["data-view"],
  });
})();
