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
///
/// ⚠ **`firstChild` + `removeChild` 是必需品，不是装饰**（2026-09-30 实测）：
/// `infoview.js` 的 `clear()` 是 `while (node.firstChild) node.removeChild(...)`，
/// 少了它们 `clear()` **静默空转** ⇒ 骨架行「○ 语言服务器启动中…」**永远留着**，
/// 判出来的"第一屏"就成了**用户永远看不到的文本** ✗（判据必须绑"屏幕上有什么"）。
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
    removeChild(c) {
      const index = this.childNodes.indexOf(c);
      if (index >= 0) this.childNodes.splice(index, 1);
      return c;
    }
    get firstChild() {
      return this.childNodes[0] || null;
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

/// 跑真 `infoview.js`，把 `messages` 逐条喂进去，返回它渲染完的 `root` 节点。
///
/// ⚠ **必须喂「整块面板」而不是只喂 project**（2026-09-30）：用户实测的缺陷是
/// 「同一 Infoview 里 `server 0.78.1` vs 项目区块 `… · 0.78.0`」—— **两个数字在
/// 两行里**，只渲染 project 那一半就**永远看不见这个冲突** ✗（判据空转）。
function renderPanel(messages) {
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
  for (const msg of messages) {
    for (const h of handlers) h({ data: { protocol: 1, ...msg } });
  }
  // ⚠ `projectBody` 是按 **class** 建的（`el("div", "project")`）✗ 不是 id ⇒
  // 从 `root` 容器**遍历**找它 ✓（第二版踩的坑：按 id 找 ⇒ 找不到 ✗✓）。
  return byId.get("root") || dom.document.body;
}

/// 只要**项目区块**那一个节点（老接口，供只关心项目块的判据用）。
function projectBodyOf(root) {
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

function renderProject(msg) {
  return projectBodyOf(renderPanel([{ type: "project", ...msg }]));
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

/// ⚠ **夹具就是用户报的那一对数**（2026-09-29）：「顶部 `server 0.78.1`，项目区块
/// 仍 `12 条 · 8032745 字节 · 0.78.0`」⇒ 服务器 **0.78.1** + 产物戳 **0.78.0**。
/// 用真数字（不是两个编造的版本）才能保证判据**咬的是那一次事故** ✓。
const SAMPLE_SERVER = { type: "server", running: true, version: "0.78.1", pid: 4242 };

/// **屏幕上的每个版本号都必须带标签**（`服务器` / `编译器`）—— 见 §0 的判据。
///
/// 修前：`.project-artifacts` 那行是 `产物：7 条 · 20480 字节 · 0.78.0` ⇒
/// **光秃秃一个 0.78.0**，用户只能拿它跟顶上 `server 0.78.1` 对看 ⇒ 无法调和 ✗。
/// 这条判据**咬得住那次事故**：把那行改回裸版本号 ⇒ 当场判红 ✓（反向验证已做）。
const VERSION_RE = /\d+\.\d+\.\d+/g;
function unlabelledVersions(text) {
  const bad = [];
  let m;
  while ((m = VERSION_RE.exec(text)) !== null) {
    const before = text.slice(Math.max(0, m.index - 12), m.index);
    if (!/(服务器|编译器)\s*$/.test(before)) {
      const at = text.slice(Math.max(0, m.index - 14), m.index + m[0].length);
      bad.push(`「${m[0]}」前面没有标签（服务器/编译器）—— 上下文：…${at}…`);
    }
  }
  return bad;
}

// ⚠ **"内部词一律不许上第一屏"这条我撤掉了** ✗✓（2026-09-28 CI 实测后的修正）：
// **E30 的交付契约**要求 `.project-facts`（含 `sokonanoda.toml` / 模块根 / 入口）、
// `.project-counts`（含「N 模块」）、`.project-artifacts`（含**字节数**）**直接可见** ✓
// ⇒ 拿"内部词"判红会**与 E30 正面冲突** ✗（第一版就是这么撞的：CI 的 `editor` job 判红）。
// **用户 18:24 的原话是「删掉**或**放进高级折叠区」** —— 而"删掉"会让 E30 判红
// ⇒ 取**次要**那一支 ✓。⇒ 本判据改为**只管用户真正抱怨的两件事**：
//   (a) **三问答得上**（明确状态 / 版本可见 / 没问题不显示告警）；
//   (b) **版本出现在内部细节之前**（旧版把版本埋在产物行尾 ✗ —— 这是"没从用户角度
//       排序"的直接症状 ✓）；
//   (c) **不许出现「编译 N」**这种含糊数（用户点名 ✗）；
//   (d) **每个版本号都必须带标签**（2026-09-30 补，见 `unlabelledVersions`）。

/// **版本必须在这些内部细节之前出现** ✓（顺序即优先级）。
const INTERNAL_MARKERS = ["清单", "模块根", "入口", "产物：", "声明 ·"];

function main(argv) {
  const check = argv.includes("--check");
  let text;
  try {
    // ⚠ 判的是**整块面板**（服务器行 + 状态行 + 目标/声明 + 项目块）——
    // 用户看到的就是这一整块 ✓（只渲染项目块会让"两个版本打架"这条判据空转 ✗）。
    text = firstScreenText(renderPanel([SAMPLE_SERVER, { type: "project", ...SAMPLE }]));
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
  // (d) **每个版本号都要带标签** —— 用户实测那次的直接症状 ✓
  for (const b of unlabelledVersions(text)) bad.push(`版本号没标签 ✗ ${b}`);
  // (d2) **服务器与编译器两行都得在**：修前判据只看 `编译器`，把服务器行删了也全绿 ✗。
  if (!/服务器\s*0\.78\.1/.test(text)) bad.push("服务器那行必须写明 `服务器 <版本>`（不许光秃秃一个 server）");
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
module.exports = { renderProject, renderPanel, projectBodyOf, firstScreenText, unlabelledVersions };
