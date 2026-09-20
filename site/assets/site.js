/* site/assets/site.js — 全站唯一的脚本。渐进增强，绝不承担渲染职责。
 *
 * 契约（页面作者只需要知道这些）：
 *   <html> 上没有 data-theme 时跟随系统；[data-theme-toggle] 点击后在
 *   system → light → dark 之间循环，写 localStorage("soko-theme")。
 *   首帧的防闪烁由 <head> 里那段内联 bootstrap 负责（见 §内联 bootstrap）。
 *
 *   [data-copy="<css selector>"]   复制选择器命中的元素的 textContent
 *   [data-copy-self]               复制最近的 pre/code 的 textContent
 *   [data-filter="<css selector>"] 输入框：按文本筛选命中的元素
 *   [data-filter-count]            筛选结果计数写在这里
 *   [data-tabs]                    单选式标签页（无 JS 时用 :checked 也成立）
 *   [data-spy]                     滚动高亮：容器内 a[href^="#"] 跟随视口
 *   [data-site-version]            从 data/site.json 填入版本号
 *   [data-release]                 从 GitHub Releases API 填入（懒加载，仅请求方页）
 *   [data-root]                    在 <body> 上声明站点根前缀（"./" 或 "../"）
 *
 * 纪律：零依赖、零第三方、零分析；没有 JS 时每一页都必须完整可读可导航，
 * 所以这里只做"增强"，不生成任何正文。全站预算 8 KB。
 */
(function () {
  "use strict";

  var doc = document.documentElement;
  var body = document.body;
  var root = (body && body.getAttribute("data-root")) || "./";
  doc.classList.add("js");

  /* ── 主题 ────────────────────────────────────────────────────────── */
  var THEME_KEY = "soko-theme";
  var ORDER = ["system", "light", "dark"];
  var LABEL = { system: "跟随系统", light: "浅色", dark: "深色" };

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
      /* 隐私模式下写不进去不是错误：主题只是本次会话有效。 */
    }
    var buttons = document.querySelectorAll("[data-theme-toggle]");
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute("aria-label", "配色：" + LABEL[mode] + "（点击切换）");
      buttons[i].setAttribute("data-theme-state", mode);
    }
  }

  function wireTheme() {
    var buttons = document.querySelectorAll("[data-theme-toggle]");
    if (!buttons.length) return;
    applyTheme(readTheme());
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].addEventListener("click", function () {
        var next = ORDER[(ORDER.indexOf(readTheme()) + 1) % ORDER.length];
        applyTheme(next);
      });
    }
  }

  /* ── 复制 ────────────────────────────────────────────────────────── */
  function copyText(text, button) {
    var done = function () {
      var was = button.getAttribute("data-copy-state");
      button.setAttribute("data-copy-state", "done");
      button.textContent = button.getAttribute("data-copy-done") || "已复制";
      window.setTimeout(function () {
        button.setAttribute("data-copy-state", was || "");
        button.textContent = button.getAttribute("data-copy-idle") || "复制";
      }, 1600);
    };
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(done, function () { fallback(text, done); });
    } else {
      fallback(text, done);
    }
  }

  /* clipboard API 在 file:// 与部分旧浏览器上不可用；退回到 selection。
     退不回就什么都不做——静默失败好过谎报"已复制"。 */
  function fallback(text, done) {
    try {
      var ta = document.createElement("textarea");
      ta.value = text;
      ta.setAttribute("readonly", "");
      ta.style.position = "fixed";
      ta.style.top = "-1000px";
      document.body.appendChild(ta);
      ta.select();
      var ok = document.execCommand("copy");
      document.body.removeChild(ta);
      if (ok) done();
    } catch (e) {
      /* 无剪贴板权限：保持原状。 */
    }
  }

  function wireCopy() {
    var buttons = document.querySelectorAll("[data-copy], [data-copy-self]");
    for (var i = 0; i < buttons.length; i++) {
      (function (button) {
        if (!button.hasAttribute("data-copy-idle")) {
          button.setAttribute("data-copy-idle", button.textContent.trim());
        }
        button.addEventListener("click", function () {
          var sel = button.getAttribute("data-copy");
          var source = sel ? document.querySelector(sel) : null;
          if (!source) source = button.closest("figure, .code, .cmd, pre") || button.parentNode;
          var target = source.querySelector("pre, code") || source;
          copyText(target.textContent.replace(/\s+$/, ""), button);
        });
      })(buttons[i]);
    }
  }

  /* ── 筛选（诊断字典、台账、时间线共用） ───────────────────────────── */
  function wireFilter() {
    var inputs = document.querySelectorAll("[data-filter]");
    for (var i = 0; i < inputs.length; i++) {
      (function (input) {
        var list = document.querySelector(input.getAttribute("data-filter"));
        if (!list) return;
        var counter = document.querySelector("[data-filter-count]");
        var items = list.querySelectorAll("[data-filter-item]");
        var total = items.length;
        var apply = function () {
          var q = input.value.trim().toLowerCase();
          var shown = 0;
          for (var j = 0; j < items.length; j++) {
            var hit = !q || items[j].textContent.toLowerCase().indexOf(q) >= 0;
            items[j].hidden = !hit;
            if (hit) shown++;
          }
          if (counter) counter.textContent = shown + " / " + total;
          list.setAttribute("data-filter-empty", shown === 0 ? "true" : "false");
        };
        input.addEventListener("input", apply);
        input.addEventListener("search", apply);
        // 无 JS 时输入框不该出现（它是增强件），所以这里负责"有 JS 才显示"。
        input.hidden = false;
        apply();
      })(inputs[i]);
    }
  }

  /* ── 标签页：无 JS 时 :checked 已经成立，这里只补键盘与 aria ───────── */
  function wireTabs() {
    var groups = document.querySelectorAll("[data-tabs]");
    for (var i = 0; i < groups.length; i++) {
      (function (group) {
        var radios = group.querySelectorAll('input[type="radio"]');
        var tabs = group.querySelectorAll("[data-tab]");
        if (!radios.length) return;
        var sync = function () {
          for (var k = 0; k < tabs.length; k++) {
            var on = radios[k] && radios[k].checked;
            tabs[k].setAttribute("aria-selected", on ? "true" : "false");
            tabs[k].tabIndex = on ? 0 : -1;
          }
        };
        for (var k = 0; k < radios.length; k++) radios[k].addEventListener("change", sync);
        for (var k = 0; k < tabs.length; k++) {
          (function (index) {
            tabs[index].addEventListener("keydown", function (ev) {
              var step = ev.key === "ArrowRight" ? 1 : ev.key === "ArrowLeft" ? -1 : 0;
              if (!step) return;
              ev.preventDefault();
              var next = (index + step + radios.length) % radios.length;
              radios[next].checked = true;
              radios[next].dispatchEvent(new Event("change"));
              tabs[next].focus();
            });
          })(k);
        }
        sync();
      })(groups[i]);
    }
  }

  /* ── 滚动高亮：只在有 [data-spy] 的页面上跑 ──────────────────────── */
  function wireSpy() {
    var nav = document.querySelector("[data-spy]");
    if (!nav || !("IntersectionObserver" in window)) return;
    var links = nav.querySelectorAll('a[href^="#"]');
    if (!links.length) return;
    var byId = {};
    var targets = [];
    for (var i = 0; i < links.length; i++) {
      var id = links[i].getAttribute("href").slice(1);
      var el = id && document.getElementById(id);
      if (el) {
        byId[id] = links[i];
        targets.push(el);
      }
    }
    var visible = {};
    var observer = new IntersectionObserver(
      function (entries) {
        for (var j = 0; j < entries.length; j++) {
          visible[entries[j].target.id] = entries[j].isIntersecting;
        }
        var current = null;
        for (var k = 0; k < targets.length; k++) {
          if (visible[targets[k].id]) { current = targets[k].id; break; }
        }
        for (var id2 in byId) byId[id2].removeAttribute("aria-current");
        if (current && byId[current]) byId[current].setAttribute("aria-current", "true");
      },
      { rootMargin: "-10% 0px -70% 0px", threshold: 0 }
    );
    for (var t = 0; t < targets.length; t++) observer.observe(targets[t]);
  }

  /* ── 数据回填：版本号一律来自 site.json，永不手写 ────────────────── */
  function fillVersion() {
    var slots = document.querySelectorAll("[data-site-version]");
    if (!slots.length) return;
    // 拉 `version.json`（几十字节）而不是 `site.json`（12 KB）：28 页每页都要
    // 填一次版本号，为这一个字符串付 12 KB 不划算。两份文件由同一个生成器
    // 产出，`check-site.py` 断言它们一致。
    fetch(root + "data/version.json", { cache: "no-cache" })
      .then(function (r) { return r.ok ? r.json() : null; })
      .then(function (data) {
        if (!data || !data.version) return;
        for (var i = 0; i < slots.length; i++) {
          slots[i].textContent = "v" + data.version;
        }
        // 只给"确实在等版本号"的 CTA 解锁，避免无 JS 时出现空按钮。
        var gated = document.querySelectorAll("[data-needs-version]");
        for (var j = 0; j < gated.length; j++) gated[j].hidden = false;
      })
      .catch(function () {
        /* 拿不到就保持占位，不编造版本号。 */
      });
  }

  function wire() {
    wireTheme();
    wireCopy();
    wireFilter();
    wireTabs();
    wireSpy();
    fillVersion();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", wire);
  } else {
    wire();
  }
})();
