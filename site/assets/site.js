/* site/assets/site.js — 单页站点唯一的脚本。渐进增强，绝不承担渲染职责。
 *
 * 契约（页面作者只需要知道这些）：
 *   [data-theme-toggle]        在 跟随系统 → 浅色 → 深色 之间循环，写 localStorage
 *   [data-site-version]        从 data/site.json 填入**已发布版本号**
 *   [data-version-text]        属性值里的 {v} 替换成版本号，写回 textContent
 *                              （用于下载命令；版本号**永不手写进 HTML**）
 *   [data-version-href]        同上，替换后写回 href
 *   [data-copy]                复制 CSS 选择器命中元素的 textContent
 *
 * 纪律：零依赖、零第三方、零分析。没有 JS 时这份页面必须完整可读：
 * 版本号处留了"当前发布版本"这样的中性文字，下载命令给出 Releases 页直链，
 * 所以关掉 JS 也能装、也能读懂这个产品是什么。
 */
(function () {
  "use strict";

  var doc = document.documentElement;
  var THEME_KEY = "soko-theme";
  var ORDER = ["system", "light", "dark"];
  var LABEL = { system: "跟随系统", light: "浅色", dark: "深色" };

  /* ── 主题 ───────────────────────────────────────────────────────── */
  function readTheme() {
    try {
      var v = localStorage.getItem(THEME_KEY);
      return ORDER.indexOf(v) >= 0 ? v : "system";
    } catch (e) {
      return "system";
    }
  }

  function applyTheme(mode) {
    if (mode === "system") doc.removeAttribute("data-theme");
    else doc.setAttribute("data-theme", mode);
    try {
      localStorage.setItem(THEME_KEY, mode);
    } catch (e) {
      /* 隐私模式下写不进去不是错误：主题只对本次会话有效。 */
    }
    var buttons = document.querySelectorAll("[data-theme-toggle]");
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute("aria-label", "配色：" + LABEL[mode] + "（点击切换）");
      buttons[i].textContent = LABEL[mode];
    }
  }

  function wireTheme() {
    var buttons = document.querySelectorAll("[data-theme-toggle]");
    if (!buttons.length) return;
    applyTheme(readTheme());
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].addEventListener("click", function () {
        applyTheme(ORDER[(ORDER.indexOf(readTheme()) + 1) % ORDER.length]);
      });
    }
  }

  /* ── 版本号：唯一来源是 site/data/site.json（生成物，见 gen-site-data.py）── */
  function fillVersion(version) {
    var text = document.querySelectorAll("[data-site-version]");
    for (var i = 0; i < text.length; i++) text[i].textContent = version;

    var tpl = document.querySelectorAll("[data-version-text]");
    for (var j = 0; j < tpl.length; j++) {
      tpl[j].textContent = tpl[j].getAttribute("data-version-text").split("{v}").join(version);
    }

    var links = document.querySelectorAll("[data-version-href]");
    for (var k = 0; k < links.length; k++) {
      links[k].setAttribute("href", links[k].getAttribute("data-version-href").split("{v}").join(version));
    }
  }

  function wireVersion() {
    if (!document.querySelector("[data-site-version],[data-version-text],[data-version-href]")) return;
    fetch("data/site.json", { cache: "no-cache" })
      .then(function (r) { return r.ok ? r.json() : null; })
      .then(function (data) {
        if (data && data.version) fillVersion(String(data.version));
      })
      .catch(function () {
        /* 拿不到就保持中性文字：宁可少一个版本号，也不写一个可能过期的。 */
      });
  }

  /* ── 复制 ───────────────────────────────────────────────────────── */
  function wireCopy() {
    var buttons = document.querySelectorAll("[data-copy]");
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].addEventListener("click", function () {
        var button = this;
        var target = document.querySelector(button.getAttribute("data-copy"));
        if (!target) return;
        var text = target.textContent;
        var done = function () {
          button.setAttribute("data-copy-state", "done");
          button.textContent = "已复制";
          window.setTimeout(function () {
            button.removeAttribute("data-copy-state");
            button.textContent = "复制";
          }, 1600);
        };
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(text).then(done, function () {});
        } else {
          var ta = document.createElement("textarea");
          ta.value = text;
          document.body.appendChild(ta);
          ta.select();
          try { document.execCommand("copy"); done(); } catch (e) {}
          document.body.removeChild(ta);
        }
      });
    }
  }

  wireTheme();
  wireVersion();
  wireCopy();
})();
