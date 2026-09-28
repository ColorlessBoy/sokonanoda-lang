#!/usr/bin/env node
/// **Infoview「项目」区块的第一屏文本**（G-67 判据的可执行形态，2026-09-28）。
///
/// **为什么要有它**：用户 18:24 要求「给出**改前 / 改后的第一屏渲染文本对照**（贴出来），
/// 并回答一句：一个不懂实现的人，看第一屏能不能立刻知道：**编完了吗？哪个版本？有没有问题？**」
/// —— 而 stub 宿主那套测的是**消息通道**，看不到 webview 真正渲染出什么 ✗。
///
/// **做法**：用一个**极简 DOM shim** 直接跑**真的** `media/infoview.js`（不是重写一份渲染 ✗），
/// 喂一份 `soko/project` 回答，然后把**未被 `<details>` 收起**的部分拼成纯文本 ——
/// 那就是**第一屏** ✓（收起的内容读不到 ⇒ 不算第一屏 ✓）。
///
/// 用法：
///     node editor/vscode/test-project-first-screen.js            # 打印第一屏
///     node editor/vscode/test-project-first-screen.js --check    # 判据：三问必须答得上
///
/// 退出码：0 = 通过 · 1 = 判红（第一屏答不上三问，或出现内部词）· 2 = 环境错。
"use strict";
const fs = require("fs");
const path = require("path");
const vm = require("vm");

const MEDIA = path.join(__dirname, "media");

/// 极简 DOM：够 `infoview.js` 的 `el()` / `appendChild` / `textContent` 用 ✓。
function makeDom() {
  class Node {
    constructor(tag) {
      this.tagName = String(tag || "").toUpperCase();
      this.childNodes = [];
      this.attributes = {};
      this._text = "";
    }
    appendChild(c) {
      this.childNodes.push(c);
      return c;
    }
    setAttribute(k, v) {
      this.attributes[k] = String(v);
    }
    hasAttribute(k) {
      return Object.prototype.hasOwnProperty.call(this.attributes, k);
    }
    get className() {
      return this.attributes.class || "";
    }
    set className(v) {
      this.attributes.class = v;
    }
    /// 只有**叶子**才有 textContent（与浏览器近似：有子节点时由子节点贡献 ✓）。
    get textContent() {
      return this._text;
    }
    set textContent(v) {
      this._text = String(v);
    }
  }
  const document = {
    createElement: (tag) => new Node(tag),
    getElementById: () => new Node("div"),
    body: new Node("body"),
    addEventListener: () => {},
  };
  const window = { addEventListener: () => {}, document };
  return { Node, document, window };
}

/// 跑真 `infoview.js`，返回它渲染完的 `projectBody` 节点。
function renderProject(msg) {
  const dom = makeDom();
  // ⚠ `getElementById` **必须返回同一个节点** ✗（第一版每次 new 一个 ⇒ 渲染到了
  // 被丢弃的对象上 ⇒ 第一屏**空**，而且看不出错 ✗✓）。⇒ 用一张缓存表 ✓。
  const byId = new Map();
  dom.document.getElementById = (id) => {
    if (!byId.has(id)) {
      const n = new dom.Node("div");
      n.setAttribute("id", id);
      byId.set(id, n);
    }
    return byId.get(id);
  };
  // `message` handler 要在**跑 IIFE 之前**就装好收集器 ✓。
  const handlers = [];
  dom.window.addEventListener = (type, fn) => {
    if (type === "message") handlers.push(fn);
  };
  const sandbox = {
    document: dom.document,
    window: dom.window,
    acquireVsCodeApi: () => ({ postMessage: () => {}, getState: () => undefined, setState: () => {} }),
    console,
    setTimeout,
    clearTimeout,
  };
  sandbox.globalThis = sandbox;
  vm.createContext(sandbox);
  try {
    vm.runInContext(fs.readFileSync(path.join(MEDIA, "infoview.js"), "utf8"), sandbox, {
      filename: "infoview.js",
    });
  } catch (e) {
    throw new Error(`跑 infoview.js 失败：${e.message}`);
  }
  if (handlers.length === 0) throw new Error("infoview.js 没注册 message handler ⇒ shim 与实现脱节 ✗");
  for (const h of handlers) h({ data: { protocol: 1, type: "project", ...msg } });
  // ⚠ `projectBody` 是按 **class** 建的（`el("div", "project")`）✗ 不是 id ⇒
  // 从 `root` 容器**遍历**找它 ✓（第二版踩的坑：按 id 找 ⇒ 找不到 ✗✓）。
  const root = byId.get("root") || dom.document.body;
  const find = (n) => {
    if (!n) return null;
    if (n.tagName === "DIV" && n.className === "project") return n;
    for (const c of n.childNodes || []) {
      const hit = find(c);
      if (hit) return hit;
    }
    return null;
  };
  const body = find(root);
  if (!body) throw new Error("找不到 .project 容器 ⇒ shim 与实现脱节 ✗");
  return body;
}

/// **第一屏文本**：`<details>` 收起 ⇒ 只算它的 `<summary>` ✓。
function firstScreenText(node) {
  const walk = (n) => {
    if (!n) return "";
    if (n.tagName === "DETAILS" && !n.hasAttribute("open")) {
      const sum = (n.childNodes || []).find((c) => c.tagName === "SUMMARY");
      return sum ? walk(sum) : "";
    }
    const leaf = (n.childNodes || []).length === 0 && n.textContent ? n.textContent : "";
    return [leaf, ...(n.childNodes || []).map(walk)].filter(Boolean).join(" ");
  };
  return walk(node).replace(/\s+/g, " ").trim();
}

const SAMPLE = {
  project: {
    entry: "units.u01",
    root: "/repo/courses/set-theory",
    manifest: "/repo/courses/set-theory/sokonanoda.toml",
    modules: [
      { name: "lib.Set", path: "lib/Set.sokonanoda", status: "compiled", entry: false, imports: [], decls: 28, errors: 0, warnings: 0, open_exercises: 0 },
      { name: "units.u01", path: "units/u01.sokonanoda", status: "compiled", entry: true, imports: [], decls: 5, errors: 0, warnings: 0, open_exercises: 0 },
    ],
    diagnostics: [],
    counts: { modules: 2, decls: 33, compiled: 2, failed: 0, open_exercises: 0 },
    artifacts: { entries: 7, bytes: 20480, compiler: "0.78.0" },
  },
};

const FORBIDDEN = ["manifest", "artifacts", "sokonanoda.toml", "字节", "/repo/courses/set-theory"];

function main(argv) {
  const check = argv.includes("--check");
  let text;
  try {
    text = firstScreenText(renderProject(SAMPLE));
  } catch (e) {
    console.error(`project-first-screen: ${e.message} ⇒ 环境错 ✗`);
    return 2;
  }
  if (!check) {
    console.log(text);
    return 0;
  }
  const bad = [];
  if (!/已完成|编译中 \d+\/\d+|失败 \d+ 处/.test(text)) bad.push("答不上「编完了吗」（要有 已完成/编译中 x/y/失败 N 处）");
  if (/编译 \d+\b/.test(text)) bad.push("出现了「编译 N」这种含糊数 ✗");
  if (!/编译器\s*0\.78\.0/.test(text)) bad.push("答不上「哪个版本」（编译器版本要单独可见）");
  if (/⚠/.test(text)) bad.push("没问题时不该出现告警 ✗");
  // ⚠ 判据只看**内容**，不看折叠区的**标题**（"逐个文件"/"高级"是给人看的入口词 ✓，
  // 不是内部词 ✗）—— 第一版把折叠标题也算进去 ⇒ 误红 ✓。
  const content = text.replace(/逐个文件|高级/g, "");
  for (const f of FORBIDDEN) if (content.includes(f)) bad.push(`第一屏出现内部信息 \`${f}\`（应在折叠区）✗`);
  if (bad.length) {
    console.error("project-first-screen：第一屏答不上三问 ✗");
    for (const b of bad) console.error(`  ✗ ${b}`);
    console.error(`\n第一屏实际是：${text}`);
    return 1;
  }
  console.log(`project-first-screen: ✓ 第一屏答得上三问 —— ${text}`);
  return 0;
}

if (require.main === module) process.exit(main(process.argv.slice(2)));
module.exports = { renderProject, firstScreenText };
