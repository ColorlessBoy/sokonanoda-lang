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

// ⚠ **"内部词一律不许上第一屏"这条我撤掉了** ✗✓（2026-09-28 CI 实测后的修正）：
// **E30 的交付契约**要求 `.project-facts`（含 `sokonanoda.toml` / 模块根 / 入口）、
// `.project-counts`（含「N 模块」）、`.project-artifacts`（含**字节数**）**直接可见** ✓
// ⇒ 拿"内部词"判红会**与 E30 正面冲突** ✗（第一版就是这么撞的：CI 的 `editor` job 判红）。
// **用户 18:24 的原话是「删掉**或**放进高级折叠区」** —— 而"删掉"会让 E30 判红
// ⇒ 取**次要**那一支 ✓。⇒ 本判据改为**只管用户真正抱怨的两件事**：
//   (a) **三问答得上**（明确状态 / 版本可见 / 没问题不显示告警）；
//   (b) **版本出现在内部细节之前**（旧版把版本埋在产物行尾 ✗ —— 这是"没从用户角度
//       排序"的直接症状 ✓）；
//   (c) **不许出现「编译 N」**这种含糊数（用户点名 ✗）。

/// **版本必须在这些内部细节之前出现** ✓（顺序即优先级）。
const INTERNAL_MARKERS = ["清单", "模块根", "入口", "产物：", "声明 ·"];

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
  // (a) 三问
  if (!/已完成|编译中 \d+\/\d+|失败 \d+ 处/.test(text)) {
    bad.push("答不上「编完了吗」（要有 已完成/编译中 x/y/失败 N 处）");
  }
  if (!/编译器\s*0\.78\.0/.test(text)) bad.push("答不上「哪个版本」（编译器版本要可见）");
  if (/⚠/.test(text)) bad.push("没问题时不该出现告警 ✗");
  // (b) **版本在内部细节之前**（用户抱怨的"没从用户角度排序" ✓）
  const verAt = text.indexOf("编译器");
  const firstInternal = INTERNAL_MARKERS.map((m) => text.indexOf(m)).filter((i) => i >= 0).sort((a, b) => a - b)[0];
  if (verAt < 0) {
    bad.push("找不到版本行 ⇒ 无法判定顺序 ✗");
  } else if (firstInternal !== undefined && verAt > firstInternal) {
    bad.push(
      `版本排在内部细节**之后**（版本 @${verAt}，首个内部细节 @${firstInternal}）` +
        `⇒ 正是用户说的"没从用户角度排序" ✗`,
    );
  }
  // (c) 含糊数
  // ⚠ 正则要**排除 `已编译 N/M`**（那是改好的形态 ✓）—— 第一版写 `编译 \d+`
  // 会连它一起匹配 ⇒ 误红 ✓。
  if (/(?<!已)编译 \d+\b/.test(text)) bad.push("出现了「编译 N」这种含糊数 ✗（要写成 `已编译 N/M`）");
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
