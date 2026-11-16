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
