/* site/assets/i18n.js — 中英文自动识别（**唯一**的一份语言逻辑）。
 *
 * 做法（与 `ColorlessBoy/dsh-opencode-usage` 的 `docs/index.html` 同一套，用户 2026-10-09 点名）：
 *   · 静态 HTML **默认中文**（`<html lang="zh-CN">`），要翻译的元素带 `data-i18n="键"`
 *     （图片 alt 用 `data-i18n-alt="键"`，aria-label 用 `data-i18n-aria="键"`）；
 *   · 页尾的 `<script type="application/json" id="i18n-en">` 是**英文文案字典** ——
 *     纯数据（不是可执行脚本），所以 `scripts/check-site.py` 能直接解析它做"键对齐"判据：
 *     **DOM 里用到的键集合必须与字典的键集合逐个相等**（漏译/多译都判红）；
 *   · 本脚本**同步**执行（`<script src>` 不带 defer，位于 body 末尾）⇒ 换文案发生在
 *     首帧之前，英文读者不会先看到一屏中文；
 *   · 判定：`navigator.languages` 里**任何一个**以 `zh` 开头 ⇒ 保持中文；否则整页换英文。
 *     **没有开关、没有重定向、没有 `?lang=`**（用户点名不要）。
 *   · 运行期还要用到的字（复制按钮、主题按钮）挂到 `window.SOKO_I18N` 交给 `site.js`。
 *
 * 没有 JS 时：页面就是完整的中文版 ✓（语言识别是渐进增强，不是渲染前提）。
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

  var block = document.getElementById("i18n-en");
  var dictionary = null;
  if (block) {
    try {
      dictionary = JSON.parse(block.textContent);
    } catch (error) {
      dictionary = null;      /* 字典坏了就保持中文：宁可少一种语言，也不留半页英文 */
    }
  }

  var languages = navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language || ""];
  var chinese = false;
  for (var i = 0; i < languages.length; i++) {
    if (String(languages[i]).toLowerCase().indexOf("zh") === 0) { chinese = true; break; }
  }

  if (chinese || !dictionary) {
    window.SOKO_I18N = ZH;
    return;
  }

  document.documentElement.lang = "en";
  if (dictionary.title) document.title = dictionary.title;
  if (dictionary.description) {
    var meta = document.querySelector('meta[name="description"]');
    if (meta) meta.setAttribute("content", dictionary.description);
  }

  var nodes = document.querySelectorAll("[data-i18n]");
  for (var n = 0; n < nodes.length; n++) {
    var key = nodes[n].getAttribute("data-i18n");
    if (dictionary[key] !== undefined) nodes[n].innerHTML = dictionary[key];
  }
  var alts = document.querySelectorAll("[data-i18n-alt]");
  for (var a = 0; a < alts.length; a++) {
    var altKey = alts[a].getAttribute("data-i18n-alt");
    if (dictionary[altKey] !== undefined) alts[a].setAttribute("alt", dictionary[altKey]);
  }
  var labels = document.querySelectorAll("[data-i18n-aria]");
  for (var l = 0; l < labels.length; l++) {
    var ariaKey = labels[l].getAttribute("data-i18n-aria");
    if (dictionary[ariaKey] !== undefined) labels[l].setAttribute("aria-label", dictionary[ariaKey]);
  }

  window.SOKO_I18N = EN;
})();
