/*
 * app.js — II.1 interactive book prototype.
 *
 * Deliberately tiny and dependency-free: no CDN, no math renderer, no framework.
 * Everything is delegated from a handful of data-* hooks so the static HTML
 * remains usable when JavaScript is disabled (native <details> still works).
 */
(() => {
  "use strict";

  const body = document.body;
  const VIEW_KEY = "soko-book-proto-view";
  const views = ["both", "intuition", "formal"];
  const buttons = Array.from(document.querySelectorAll("[data-view]"));

  function setView(view) {
    nextView = views.includes(view) ? view : "both";
    body.dataset.view = nextView;
    for (const button of buttons) {
      button.setAttribute("aria-pressed", String(button.dataset.view === nextView));
    }
    try {
      localStorage.setItem(VIEW_KEY, nextView);
    } catch (_) {
      /* file:// or private mode: the toggle still works for this session */
    }
  }

  let nextView = "both";
  for (const button of buttons) {
    button.addEventListener("click", () => setView(button.dataset.view));
  }

  try {
    const saved = localStorage.getItem(VIEW_KEY);
    if (views.includes(saved)) setView(saved);
    else setView("both");
  } catch (_) {
    setView("both");
  }

  function setAllDetails(open) {
    const details = document.querySelectorAll("details");
    for (const item of details) item.open = open;
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
})();
