/* site/assets/i18n.js — 中英文自动识别 + topbar 手动切换（**唯一**的一份语言逻辑）。
 *
 * 做法（与 `ColorlessBoy/dsh-opencode-usage` 的 `docs/index.html` 同一套，用户 2026-10-09 点名）：
 *   · 静态 HTML **默认中文**（`<html lang="zh-CN">`），要翻译的元素带 `data-i18n="键"`
 *     （图片 alt 用 `data-i18n-alt="键"`，aria-label 用 `data-i18n-aria="键"`）；
 *   · 页尾的 `<script type="application/json" id="i18n-en">` 是**英文文案字典** ——
 *     纯数据（不是可执行脚本），所以 `scripts/check-site.py` 能直接解析它做"键对齐"判据：
 *     **DOM 里用到的键集合必须与字典的键集合逐个相等**（漏译/多译都判红）；
 *   · 本脚本**同步**执行（`<script src>` 不带 defer，位于 body 末尾）⇒ 换文案发生在
 *     首帧之前，英文读者不会先看到一屏中文；
 *   · **判定顺序**：① `localStorage["soko-lang"]` 里的**手动选择**（点了 topbar 那个按钮
 *     才会有）→ ② `navigator.languages` 里**任何一个**以 `zh` 开头 ⇒ 中文，否则英文。
 *     即：**默认自动识别**；手动选择只覆盖这一台浏览器、且可撤销（按钮再点回去）。
 *     没有 `?lang=`、没有重定向、没有第二个页面。
 *   · 运行期还要用到的字（复制按钮、主题按钮）挂到 `window.SOKO_I18N` 交给 `site.js`，
 *     并在换语言后发一个 `soko:lang` 事件让那些按钮跟着换（否则按钮会停在旧语言 ✗）。
 *
 * 没有 JS 时：页面就是完整的中文版 ✓（语言识别与按钮都是渐进增强，不是渲染前提）。
 */
(function () {
  "use strict";

  var ZH = {
    copy: { idle: "复制", done: "已复制" },
    theme: { system: "跟随系统", light: "浅色", dark: "深色", prefix: "配色：", suffix: "（点击切换）" }
  };
  var EN = {
    copy: { idle: "Copy", done: "Copied" },
    theme: { system: "System", light: "Light", dark: "Dark", prefix: "Theme: ", suffix: " (click to change)" }
  };
  var STORE_KEY = "soko-lang";

  var block = document.getElementById("i18n-en");
  var dictionary = null;
  if (block) {
    try {
      dictionary = JSON.parse(block.textContent);
    } catch (error) {
      dictionary = null;      /* 字典坏了就保持中文：宁可少一种语言，也不留半页英文 */
    }
  }

  var descriptionMeta = document.querySelector('meta[name="description"]');
  /* 中文原文：**首次替换之前**抓下来 —— 切回中文时没有它就回不去了（英文已写进 DOM ✗）。 */
  var originals = {
    text: {}, alt: {}, aria: {},
    title: document.title,
    description: descriptionMeta ? descriptionMeta.getAttribute("content") : null
  };
  var nodesText = document.querySelectorAll("[data-i18n]");
  var nodesAlt = document.querySelectorAll("[data-i18n-alt]");
  var nodesAria = document.querySelectorAll("[data-i18n-aria]");
  var i;
  for (i = 0; i < nodesText.length; i++) {
    var textKey = nodesText[i].getAttribute("data-i18n");
    if (originals.text[textKey] === undefined) originals.text[textKey] = nodesText[i].innerHTML;
  }
  for (i = 0; i < nodesAlt.length; i++) {
    originals.alt[nodesAlt[i].getAttribute("data-i18n-alt")] = nodesAlt[i].getAttribute("alt");
  }
  for (i = 0; i < nodesAria.length; i++) {
    originals.aria[nodesAria[i].getAttribute("data-i18n-aria")] = nodesAria[i].getAttribute("aria-label");
  }

  function stored() {
    try {
      var v = localStorage.getItem(STORE_KEY);
      return v === "zh" || v === "en" ? v : null;
    } catch (error) {
      return null;            /* 隐私模式：读不到就当没选过（自动识别照常工作） */
    }
  }

  function detect() {
    var chosen = stored();
    if (chosen) return chosen;
    var languages = navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language || ""];
    for (var n = 0; n < languages.length; n++) {
      if (String(languages[n]).toLowerCase().indexOf("zh") === 0) return "zh";
    }
    return "en";
  }

  function apply(lang) {
    var chinese = lang !== "en";
    document.documentElement.lang = chinese ? "zh-CN" : "en";
    if (!dictionary) {        /* 字典缺失 ⇒ 只能维持中文原文（英文没得换）✓ */
      window.SOKO_I18N = ZH;
      return;
    }
    document.title = chinese ? originals.title : (dictionary.title || originals.title);
    if (descriptionMeta) {
      descriptionMeta.setAttribute("content",
        chinese ? (originals.description || "") : (dictionary.description || originals.description || ""));
    }
    for (var t = 0; t < nodesText.length; t++) {
      var key = nodesText[t].getAttribute("data-i18n");
      var value = chinese ? originals.text[key] : dictionary[key];
      if (value !== undefined) nodesText[t].innerHTML = value;
    }
    for (var a = 0; a < nodesAlt.length; a++) {
      var altKey = nodesAlt[a].getAttribute("data-i18n-alt");
      var altValue = chinese ? originals.alt[altKey] : dictionary[altKey];
      if (altValue !== undefined) nodesAlt[a].setAttribute("alt", altValue);
    }
    for (var l = 0; l < nodesAria.length; l++) {
      var ariaKey = nodesAria[l].getAttribute("data-i18n-aria");
      var ariaValue = chinese ? originals.aria[ariaKey] : dictionary[ariaKey];
      if (ariaValue !== undefined) nodesAria[l].setAttribute("aria-label", ariaValue);
    }
    window.SOKO_I18N = chinese ? ZH : EN;
    /* 运行期那些按钮（复制/主题）由 `site.js` 画，它们也要跟着换 —— 用事件而不是直接调用：
       两个脚本都在 body 末尾，谁先跑由 HTML 决定，事件对两种顺序都成立 ✓。 */
    try {
      window.dispatchEvent(new Event("soko:lang"));
    } catch (error) {
      /* 老浏览器：按钮停在旧语言，不报错。 */
    }
  }

  /* topbar 的切换按钮：**渐进增强**（HTML 里 `hidden` 起步，JS 才把它放出来）。
     标签写"切过去会变成什么"（中文时显示 `EN`，英文时显示 `中文`）。 */
  function wireToggle() {
    var buttons = document.querySelectorAll("[data-lang-toggle]");
    for (var b = 0; b < buttons.length; b++) {
      buttons[b].hidden = false;
      buttons[b].addEventListener("click", function () {
        var next = detect() === "en" ? "zh" : "en";
        try {
          localStorage.setItem(STORE_KEY, next);
        } catch (error) {
          /* 写不进去也照样切（只是刷新后回到自动识别） */
        }
        apply(next);
      });
    }
  }

  apply(detect());
  wireToggle();
})();
