/* site/assets/search.js — 只给 search.html 用
 *
 * 为什么单独一个文件而不是塞进 site.js：搜索是**一页**的功能，而 site.js 是
 * 每一页都要下载的。放这里让 27 个页面不付这份代价（约 3 KB）。
 *
 * 契约（页面必须提供）：
 *   [data-search-input]   输入框
 *   [data-search-results] 结果容器
 *   [data-search-count]   计数
 *   [data-search-index]   索引 URL（由页面给出，便于 en/ 这类子目录）
 *   [data-search-fallback] 无 JS 时的完整页面清单；JS 一旦接管就隐藏它
 *
 * 零依赖、零第三方。取不到索引时**明说**，不假装"没有结果"。
 */
(function () {
  "use strict";

  var input = document.querySelector("[data-search-input]");
  var results = document.querySelector("[data-search-results]");
  if (!input || !results) return;

  var count = document.querySelector("[data-search-count]");
  var fallback = document.querySelector("[data-search-fallback]");
  var indexUrl = input.getAttribute("data-search-index") || "data/search.json";
  var root = (document.body && document.body.getAttribute("data-root")) || "./";

  var entries = [];
  var ready = false;

  function esc(text) {
    return String(text).replace(/[&<>"]/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c];
    });
  }

  // 高亮命中的片段：把匹配处包进 <mark>。先转义再插标记，避免把用户输入
  // 当 HTML 解析（搜索框是最容易被拿来注入的地方）。
  function highlight(text, needle) {
    var safe = esc(text);
    if (!needle) return safe;
    var at = safe.toLowerCase().indexOf(needle.toLowerCase());
    if (at < 0) return safe;
    return (
      safe.slice(0, at) +
      "<mark>" +
      safe.slice(at, at + needle.length) +
      "</mark>" +
      safe.slice(at + needle.length)
    );
  }

  function render(query) {
    var q = query.trim().toLowerCase();
    var hits = [];
    for (var i = 0; i < entries.length && hits.length < 60; i++) {
      var e = entries[i];
      var hay = (e.heading + " " + e.text + " " + e.page).toLowerCase();
      if (!q || hay.indexOf(q) >= 0) hits.push(e);
    }

    if (!q) {
      results.innerHTML =
        '<p class="is-empty">输入关键词开始搜索。索引覆盖全站 ' +
        entries.length +
        " 个小节。</p>";
    } else if (!hits.length) {
      results.innerHTML =
        '<p class="is-empty">没有匹配「' + esc(query) + '」的条目。</p>';
    } else {
      var html = '<ul class="search-hits">';
      for (var j = 0; j < hits.length; j++) {
        var h = hits[j];
        var href = root + h.page + (h.anchor ? "#" + h.anchor : "");
        html +=
          '<li class="search-hit">' +
          '<a class="search-hit__link" href="' + esc(href) + '">' +
          highlight(h.heading, query) +
          "</a>" +
          '<span class="search-hit__page">' + esc(h.page) + "</span>" +
          (h.text ? '<p class="search-hit__text">' + highlight(h.text, query) + "</p>" : "") +
          "</li>";
      }
      html += "</ul>";
      results.innerHTML = html;
    }

    if (count) count.textContent = q ? hits.length + " 条结果" : "";
  }

  fetch(indexUrl, { cache: "no-cache" })
    .then(function (r) {
      if (!r.ok) throw new Error("HTTP " + r.status);
      return r.json();
    })
    .then(function (data) {
      (data.pages || []).forEach(function (page) {
        (page.entries || []).forEach(function (entry) {
          entries.push({
            page: page.page,
            anchor: entry.anchor || "",
            heading: entry.heading || page.title || page.page,
            text: entry.text || ""
          });
        });
      });
      ready = true;
      // JS 接管了：隐藏无 JS 用的完整清单，显示输入框。
      if (fallback) fallback.hidden = true;
      input.hidden = false;
      render(input.value);
      input.focus();
    })
    .catch(function (err) {
      // 拿不到索引就说清楚，别让读者以为"站里没有这个东西"。
      results.innerHTML =
        '<p class="is-empty">搜索索引加载失败（' +
        esc(err.message) +
        "）。下面的完整页面清单仍然可用。</p>";
      input.hidden = true;
    });

  input.addEventListener("input", function () {
    if (ready) render(input.value);
  });
})();
