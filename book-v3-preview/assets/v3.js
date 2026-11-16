/* book v3 — shell behaviours: progress (with v1 migration), menu, TOC,
   copy-link, print, legacy single-page hash redirects, and search. */
(function () {
  "use strict";
  var OLD_KEY = "soko-book-progress-v1";        // v2 single-page: JSON array of unit numbers
  var KEY = "soko-book-v3-progress";            // v3: {"<unit>": true}
  var $ = function (s, r) { return (r || document).querySelector(s); };
  var $$ = function (s, r) { return Array.prototype.slice.call((r || document).querySelectorAll(s)); };
  function pageKind() { return document.body.getAttribute("data-page") || "page"; }

  /* ---------- progress ---------- */
  function readProgress() {
    var obj = {};
    try {
      var raw = localStorage.getItem(KEY);
      if (raw) return JSON.parse(raw) || {};
      var old = JSON.parse(localStorage.getItem(OLD_KEY) || "[]");
      if (Array.isArray(old) && old.length) {
        old.forEach(function (n) { obj[String(n)] = true; });
        localStorage.setItem(KEY, JSON.stringify(obj));
      }
    } catch (e) { /* storage unavailable: session-only */ }
    return obj;
  }
  function saveProgress(obj) {
    try { localStorage.setItem(KEY, JSON.stringify(obj)); } catch (e) {}
  }
  function refreshProgress() {
    var p = readProgress();
    $$("[data-progress-count]").forEach(function (el) {
      var total = parseInt(el.getAttribute("data-progress-total") || "72", 10);
      var n = Object.keys(p).length;
      el.textContent = "已读 " + n + " / " + total;
    });
    $$("[data-progressbar]").forEach(function (bar) {
      var total = parseInt(bar.getAttribute("aria-valuemax") || "72", 10);
      var n = Object.keys(p).length;
      bar.setAttribute("aria-valuenow", String(n));
      var fill = $("[data-progress-fill]", bar.parentNode) || $("[data-progress-fill]");
      if (fill) fill.style.width = (total ? Math.min(100, 100 * n / total) : 0) + "%";
    });
    $$("[data-progress-toggle]").forEach(function (btn) {
      var u = String(btn.getAttribute("data-unit"));
      var on = !!p[u];
      btn.setAttribute("aria-pressed", on ? "true" : "false");
      var txt = $(".unit-progress-text", btn);
      if (txt) txt.textContent = on ? "已读" : "标记已读";
    });
    $$(".unit-card[data-unit]").forEach(function (card) {
      var on = !!p[String(card.getAttribute("data-unit"))];
      card.classList.toggle("is-read", on);
      var b = $("[data-card-progress]", card);
      if (b) { b.setAttribute("aria-pressed", on ? "true" : "false"); b.textContent = on ? "已读" : "标记已读"; }
    });
  }
  function toggleUnit(u) {
    var p = readProgress();
    if (p[String(u)]) delete p[String(u)]; else p[String(u)] = true;
    saveProgress(p); refreshProgress();
  }
  $$("[data-progress-toggle]").forEach(function (btn) {
    btn.addEventListener("click", function () { toggleUnit(btn.getAttribute("data-unit")); });
  });
  $$("[data-card-progress]").forEach(function (btn) {
    btn.addEventListener("click", function (ev) {
      ev.preventDefault(); ev.stopPropagation();
      toggleUnit(btn.getAttribute("data-unit"));
    });
  });

  /* ---------- instant next page: prefetch/preload on intent ---------- */
  var prefetched = {};
  function prefetch(href) {
    if (!href || prefetched[href]) return;
    if (href.charAt(0) === "#" || /^(https?:|mailto:)/.test(href)) return;
    if (!/\.html(?:#|$)/.test(href)) return;
    prefetched[href] = true;
    var l = document.createElement("link");
    l.rel = "prefetch";
    l.href = href;
    l.as = "document";
    document.head.appendChild(l);
  }
  function intentPrefetch(ev) {
    var a = ev.target && ev.target.closest ? ev.target.closest("a[href]") : null;
    if (a) prefetch(a.getAttribute("href"));
  }
  document.addEventListener("pointerover", function (ev) {
    if (ev.pointerType === "mouse") intentPrefetch(ev);
  }, {passive: true});
  document.addEventListener("pointerdown", intentPrefetch, {passive: true});
  document.addEventListener("focusin", intentPrefetch, {passive: true});
  document.addEventListener("touchstart", intentPrefetch, {passive: true});

  /* ---------- reading layer (default intuition) ---------- */
  var VIEW_KEY = "soko-book-v3-view";
  function applyView(v) {
    if (!v) v = "intuition";
    document.body.setAttribute("data-view", v);
    $$("[data-view-btn]").forEach(function (b) {
      b.setAttribute("aria-pressed", b.getAttribute("data-view-btn") === v ? "true" : "false");
    });
    try { localStorage.setItem(VIEW_KEY, v); } catch (e) {}
  }
  var savedView = "intuition";
  try { savedView = localStorage.getItem(VIEW_KEY) || "intuition"; } catch (e) {}
  if (["intuition", "both", "formal"].indexOf(savedView) < 0) savedView = "intuition";
  applyView(savedView);
  $$("[data-view-btn]").forEach(function (b) {
    b.addEventListener("click", function () { applyView(b.getAttribute("data-view-btn")); });
  });

  /* ---------- unit sidebar (mobile) ---------- */
  var sideToggle = $("[data-side-toggle]");
  var sideEl = document.getElementById("site-side");
  function closeSide(restore) {
    if (!sideEl) return;
    sideEl.classList.remove("is-open");
    document.body.classList.remove("side-open");
    if (sideToggle) sideToggle.setAttribute("aria-expanded", "false");
    if (restore && sideToggle) sideToggle.focus();
  }
  if (sideToggle && sideEl) {
    sideToggle.addEventListener("click", function () {
      var open = !sideEl.classList.contains("is-open");
      sideEl.classList.toggle("is-open", open);
      document.body.classList.toggle("side-open", open);
      sideToggle.setAttribute("aria-expanded", open ? "true" : "false");
      if (open) {
        var first = sideEl.querySelector("a,button");
        if (first) first.focus();
      }
    });
    $$("#site-side a").forEach(function (a) {
      a.addEventListener("click", function () { closeSide(false); });
    });
    document.addEventListener("keydown", function (e) {
      if (e.key === "Escape" && sideEl.classList.contains("is-open")) closeSide(true);
      if (e.key === "Tab" && sideEl.classList.contains("is-open")) {
        var f = $$("#site-side a,#site-side button").filter(function (x) { return x.offsetParent !== null; });
        if (!f.length) return;
        var first = f[0], last = f[f.length - 1];
        if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
        else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
      }
    });
    var backdrop = document.createElement("div");
    backdrop.className = "side-backdrop";
    backdrop.addEventListener("click", function () { closeSide(false); });
    sideEl.addEventListener("click", function (e) { if (e.target === sideEl) closeSide(false); });
  }
  document.addEventListener("keydown", function (e) {
    if (e.key === "/" && !/^(INPUT|TEXTAREA|SELECT)$/.test((document.activeElement || {}).tagName || "")) {
      e.preventDefault();
      location.href = (document.body.getAttribute("data-page") === "search" ? "" : (document.querySelector('a[href$="search.html"]') || {}).href) || "search.html";
    }
  });

  /* ---------- mobile menu ---------- */
  var menu = $("[data-menu]");
  if (menu) {
    menu.addEventListener("click", function () {
      var nav = $("#site-nav");
      var open = nav.classList.toggle("is-open");
      menu.setAttribute("aria-expanded", open ? "true" : "false");
    });
  }
  /* ---------- TOC on desktop ---------- */
  var toc = $("[data-toc]");
  if (toc && window.matchMedia && window.matchMedia("(min-width:960px)").matches) toc.open = true;

  /* ---------- copy unit link ---------- */
  $$("[data-copy-link]").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var status = $("[data-copy-status]");
      var done = function (ok) {
        if (status) status.textContent = ok ? "链接已复制" : "复制失败：请手动复制地址栏";
      };
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(location.href).then(function () { done(true); }, function () { done(false); });
      } else { done(false); }
    });
  });

  /* ---------- print ---------- */
  $$("[data-print]").forEach(function (b) { b.addEventListener("click", function () { window.print(); }); });

  /* ---------- legacy single-page hash redirects (cover only) ---------- */
  if (pageKind() === "cover") {
    var h = decodeURIComponent(location.hash.replace(/^#/, ""));
    var m;
    if ((m = h.match(/^unit-(\d+)$/))) location.replace("units/unit" + pad(m[1]) + ".html");
    else if ((m = h.match(/^chapter-ii-(\d+)$/))) location.replace("chapters/ii-" + m[1] + ".html");
    else if ((m = h.match(/^appendix-unit-(\d+)$/))) location.replace("units/unit" + pad(m[1]) + ".html#appendix-unit-" + m[1]);
    else if ((m = h.match(/^(?:app-sol|sol)-(\d+)-\d+$/))) location.replace("units/unit" + pad(m[1]) + ".html#" + h);
    else if ((m = h.match(/^(?:decl|ex|section)-(\d+)-\d+$/))) location.replace("units/unit" + pad(m[1]) + ".html#" + h);
    else if (h === "toc") document.getElementById("path") && document.getElementById("path").scrollIntoView();
  }
  function pad(n) { n = String(parseInt(n, 10)); return n.length < 2 ? "0" + n : n; }

  /* ---------- search ---------- */
  var idxEl = document.getElementById("v3-search-index");
  if (idxEl) {
    var INDEX = [];
    try { INDEX = JSON.parse(idxEl.textContent || "[]"); } catch (e) {}
    var input = document.getElementById("q");
    var status = document.getElementById("status");
    var list = document.getElementById("results");
    var KIND = { chapter: "章", unit: "单元", def: "定义", theorem: "定理", exercise: "练习",
                 inductive: "归纳", section: "小节", appendix: "解答", solution: "解答" };
    function search(q) {
      q = (q || "").trim().toLowerCase();
      if (list) list.innerHTML = "";
      if (!q) { if (status) status.textContent = "输入关键词开始搜索（共 " + INDEX.length + " 条索引）。"; return; }
      var hits = INDEX.filter(function (e) { return (e.t || "").toLowerCase().indexOf(q) >= 0; }).slice(0, 60);
      if (status) status.textContent = hits.length ? ("找到 " + hits.length + " 条（最多显示 60 条）") : "没有匹配的结果。";
      if (!list) return;
      hits.forEach(function (e) {
        var li = document.createElement("li");
        var a = document.createElement("a");
        a.href = e.url; a.textContent = e.t;
        var k = document.createElement("span"); k.className = "kind"; k.textContent = KIND[e.k] || e.k;
        a.appendChild(k); li.appendChild(a);
        if (e.ctx) { var c = document.createElement("span"); c.className = "ctx"; c.textContent = e.ctx; li.appendChild(c); }
        list.appendChild(li);
      });
    }
    if (input) {
      input.addEventListener("input", function () { search(input.value); });
      var p = new URLSearchParams(location.search).get("q");
      if (p) { input.value = p; search(p); }
      else search("");
    }
    var clear = document.getElementById("clear");
    if (clear) clear.addEventListener("click", function () { if (input) { input.value = ""; search(""); input.focus(); } });
  }

  refreshProgress();
})();
