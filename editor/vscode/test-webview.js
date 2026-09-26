// Behavioral tests for media/infoview.js (protocol 1).
// Run: node editor/vscode/test-webview.js
// No VS Code, no network, no Rust — pure Node with a minimal DOM stub and the
// script executed in a fresh `vm` context, exactly like test-server.js tests
// server.js. This is the layer that actually exercises the webview renderer:
// the host messages are driven by hand and the resulting DOM tree asserted on.
//
// Why not jsdom: the webview is deliberately tiny and uses only
// createElement/createTextNode/appendChild/removeChild/firstChild/
// className/textContent/setAttribute, so a ~40-line stub keeps the test
// dependency-free (matching the rest of the extension's unit tests).

const fs = require("fs");
const path = require("path");
const vm = require("vm");
const assert = require("assert");

// ── Minimal DOM ──────────────────────────────────────────────────────────
// A node tracks its children so `clear()` (while firstChild + removeChild)
// works. `textContent` returns the concatenation of the children when present
// (text nodes have none and keep their scalar), mirroring the browser enough
// to assert on rendered text.
function makeNode(tag) {
  const node = {
    tagName: tag,
    className: "",
    attributes: {},
    _listeners: {},
    _text: "",
    childNodes: [],
    appendChild(child) {
      this.childNodes.push(child);
      return child;
    },
    removeChild(child) {
      const index = this.childNodes.indexOf(child);
      if (index >= 0) this.childNodes.splice(index, 1);
      return child;
    },
    setAttribute(name, value) {
      this.attributes[name] = value;
    },
    addEventListener(type, handler) {
      this._listeners[type] = handler;
    },
  };
  Object.defineProperty(node, "firstChild", {
    get() {
      return this.childNodes[0] || null;
    },
  });
  Object.defineProperty(node, "textContent", {
    get() {
      if (this.childNodes.length > 0) {
        return this.childNodes.map((child) => child.textContent).join("");
      }
      return this._text;
    },
    set(value) {
      this._text = String(value);
      this.childNodes.length = 0;
    },
  });
  return node;
}

function makeDom() {
  const root = makeNode("div");
  root.id = "root";
  const body = makeNode("body");
  const document = {
    body,
    createElement: (tag) => makeNode(tag),
    createTextNode: (text) => {
      const node = makeNode("#text");
      node.textContent = String(text);
      return node;
    },
    getElementById: (id) => (id === "root" ? root : null),
  };
  return { document, root };
}

function loadInfoview() {
  const { document, root } = makeDom();
  const messages = [];
  const listeners = {};
  const sandbox = {
    document,
    window: {
      addEventListener(type, handler) {
        listeners[type] = handler;
      },
    },
    acquireVsCodeApi: () => ({ postMessage: (message) => messages.push(message) }),
    console,
  };
  const source = fs.readFileSync(path.join(__dirname, "media", "infoview.js"), "utf8");
  vm.runInNewContext(source, sandbox, { filename: "infoview.js" });
  const send = (message) => {
    assert.ok(listeners.message, "infoview.js must register a window message listener");
    listeners.message({ data: message });
  };
  return { document, root, messages, send };
}

// Depth-first descendants, including the node itself.
function descendants(node, out = []) {
  out.push(node);
  for (const child of node.childNodes) descendants(child, out);
  return out;
}

function byClass(root, className) {
  return descendants(root).filter(
    (node) =>
      typeof node.className === "string" &&
      node.className.split(" ").includes(className),
  );
}

function textOf(node) {
  return node ? node.textContent : "";
}

// ── Harness ──────────────────────────────────────────────────────────────
let passed = 0;
let failed = 0;

function test(name, fn) {
  try {
    fn();
    passed++;
    console.log(`  ✓ ${name}`);
  } catch (e) {
    failed++;
    console.error(`  ✗ ${name}`);
    console.error(`    ${e.stack ?? e.message}`);
  }
}

console.log("infoview.js webview tests\n");

test("renders a skeleton on load (never a silent blank)", () => {
  const { root } = loadInfoview();
  assert.strictEqual(textOf(byClass(root, "status")[0]), "正在渲染…");
  const empties = byClass(root, "empty").map(textOf);
  assert.ok(empties.includes("等待编译…"), `skeleton placeholders expected, got ${empties}`);
  const server = textOf(byClass(root, "server-line")[0]);
  assert.ok(server.includes("启动中"), `server skeleton expected, got ${server}`);
});

test("ready is posted to the host on load", () => {
  const { messages } = loadInfoview();
  assert.strictEqual(messages.length, 1, "exactly one ready message on load");
  assert.strictEqual(messages[0].protocol, 1);
  assert.strictEqual(messages[0].type, "ready");
});

test("state: goal line starts with ⊢ and classified runs are tok- spans", () => {
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "state",
    goal: "A",
    goal_runs: [{ text: "A", kind: "axiom_use" }],
  });
  const goal = byClass(root, "goal-ty")[0];
  assert.ok(goal, "a goal <pre> must be rendered");
  assert.ok(
    textOf(goal).startsWith("⊢ "),
    `the goal line must start with ⊢ , got ${JSON.stringify(textOf(goal))}`,
  );
  const toks = descendants(root).filter((node) =>
    typeof node.className === "string" && node.className.includes("tok-"),
  );
  assert.strictEqual(toks.length, 1, "the classified run must be one tok- span");
  assert.ok(
    toks[0].className.includes("tok-axiom_use"),
    `expected tok-axiom_use, got ${toks[0].className}`,
  );
  assert.strictEqual(textOf(toks[0]), "A");
});

// T-C32：**记法符号要看得见**。线 C 之后 goal / 类型行里出现 `∈`/`⊆`/`∧`/`↔`，
// 服务端把它们着成 `kind: "keyword"`（T-C30）——webview 这一侧必须真的把它渲染成
// 一个 `tok-keyword` span（`.tok-*` 规则读 `--soko-keyword`，主题里都定义了）。
// 这条是"着色在 Infoview 里可见"的判据：**服务端给对了**与**用户看得见**是两件事。
test("state: notation symbols render as tok-keyword spans", () => {
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "state",
    goal: "A ⊆ B -> (A ↔ B)",
    goal_runs: [
      { text: "A" },
      { text: " " },
      { text: "⊆", kind: "keyword" },
      { text: " " },
      { text: "B" },
      { text: " " },
      { text: "->" },
      { text: " " },
      { text: "(" },
      { text: "A" },
      { text: " " },
      { text: "↔", kind: "keyword" },
      { text: " " },
      { text: "B" },
      { text: ")" },
    ],
  });
  const goal = byClass(root, "goal-ty")[0];
  assert.ok(goal, "a goal <pre> must be rendered");
  assert.strictEqual(
    textOf(goal),
    "⊢ A ⊆ B -> (A ↔ B)",
    "the runs must reconstruct the goal text byte for byte",
  );
  const toks = descendants(goal).filter(
    (node) =>
      typeof node.className === "string" && node.className.includes("tok-"),
  );
  assert.deepStrictEqual(
    toks.map((n) => n.className.split(" ").filter((c) => c.startsWith("tok-"))),
    [["tok-keyword"], ["tok-keyword"]],
    "each notation symbol must be its own tok-keyword span",
  );
  assert.deepStrictEqual(toks.map(textOf), ["⊆", "↔"]);
});

// 同一件事在**声明卡片**那一侧（`decls` 消息的 `ty_runs`）——两个 surface 都要有。
test("decls: notation symbols render as tok-keyword spans in the type line", () => {
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "decls",
    decls: [
      {
        name: "subset_refl",
        ty: "A ⊆ A",
        ty_runs: [
          { text: "A" },
          { text: " " },
          { text: "⊆", kind: "keyword" },
          { text: " " },
          { text: "A" },
        ],
        range: { start: { line: 3, character: 0 } },
      },
    ],
  });
  const row = byClass(root, "decl")[0];
  const ty = byClass(row, "decl-ty")[0];
  assert.strictEqual(textOf(ty), "A ⊆ A");
  const toks = descendants(ty).filter(
    (node) =>
      typeof node.className === "string" && node.className.includes("tok-"),
  );
  assert.strictEqual(toks.length, 1);
  assert.ok(
    toks[0].className.includes("tok-keyword"),
    `expected a tok-keyword span, got ${toks[0].className}`,
  );
  assert.strictEqual(textOf(toks[0]), "⊆");
});

// T-A5 / R-2 ②：**声明卡片里的目标行要看得见，而且要有颜色**。
//
// 用户报的是"infoview 里的目标也没有高亮"。服务端现在给 `goal_runs`（父）与
// `goals_runs`（子，与 `goals` 对齐）——这一层验的是**渲染结果**：卡片里真的多出
// 一行 `⊢ …`，且记法符号是 `tok-keyword` span（不是一整段纯文本）。
// "字段在 wire 里"由 `scripts/audit-wire-fields.py` 守，两层各司其职。
test("decls: an open declaration renders its goal as tok- spans", () => {
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "decls",
    decls: [
      {
        name: "open_subset",
        kind: "theorem",
        status: "open",
        goal: "A ⊆ B",
        goal_runs: [
          { text: "A" },
          { text: " " },
          { text: "⊆", kind: "keyword" },
          { text: " " },
          { text: "B" },
        ],
        goals: ["A ⊆ B"],
        goals_runs: [
          [
            { text: "A" },
            { text: " " },
            { text: "⊆", kind: "keyword" },
            { text: " " },
            { text: "B" },
          ],
        ],
      },
    ],
  });
  const row = byClass(root, "decl")[0];
  const line = byClass(row, "decl-goal-line")[0];
  assert.ok(line, "开放声明的卡片必须有目标行（R-2 ②：以前根本不画）");
  assert.strictEqual(
    textOf(byClass(line, "decl-goal-label")[0]),
    "目标",
    "目标行要有标签，否则与类型行分不清",
  );
  const goal = byClass(line, "decl-goal")[0];
  assert.ok(goal, "目标行必须是一个代码块");
  assert.strictEqual(
    textOf(goal),
    "⊢ A ⊆ B",
    "目标行必须逐字节重建文本（前缀 ⊢）",
  );
  const toks = descendants(goal).filter(
    (node) =>
      typeof node.className === "string" && node.className.includes("tok-"),
  );
  assert.deepStrictEqual(
    toks.map((n) => n.className.split(" ").filter((c) => c.startsWith("tok-"))),
    [["tok-keyword"]],
    "记法符号必须是一个 tok-keyword span（这就是「看得见的高亮」）",
  );
  assert.strictEqual(textOf(toks[0]), "⊆");
});

// 多目标：每个子目标一行，标签是 `目标 i/n`——与目标面板/练习树同一口径。
test("decls: every open goal gets its own labelled row", () => {
  const { root, send } = loadInfoview();
  const runsOf = (text) => [{ text: text, kind: "unknown_ident" }];
  send({
    protocol: 1,
    type: "decls",
    decls: [
      {
        name: "both",
        kind: "theorem",
        status: "open",
        goal: "P",
        goal_runs: runsOf("P"),
        goals: ["P", "Q"],
        goals_runs: [runsOf("P"), runsOf("Q")],
      },
    ],
  });
  const row = byClass(root, "decl")[0];
  const lines = byClass(row, "decl-goal-line");
  assert.strictEqual(lines.length, 2, "两个子目标 = 两行");
  assert.deepStrictEqual(
    lines.map((line) => textOf(byClass(line, "decl-goal-label")[0])),
    ["目标 1/2", "目标 2/2"],
    "多目标要标出 i/n",
  );
  assert.deepStrictEqual(
    lines.map((line) => textOf(byClass(line, "decl-goal")[0])),
    ["⊢ P", "⊢ Q"],
    "goals_runs[i] 必须贴到 goals[i] 上（错位就会串行）",
  );
});

// 反例守卫：**闭合**声明（没有 goal/goals）不许出现目标行——否则每个 theorem
// 都会多出一行空的 `⊢ `。
test("decls: a closed declaration renders no goal row", () => {
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "decls",
    decls: [
      {
        name: "mem_self",
        kind: "theorem",
        status: "checked",
        ty: "a ∈ A -> a ∈ A",
        ty_runs: [{ text: "a ∈ A -> a ∈ A" }],
        goal: null,
        goal_runs: [],
        goals: [],
        goals_runs: [],
      },
    ],
  });
  const row = byClass(root, "decl")[0];
  assert.strictEqual(
    byClass(row, "decl-goal-line").length,
    0,
    "闭合声明没有目标，就不该有目标行",
  );
});

test("decls: name, 1-based line hint and type line; rows are not interactive", () => {
  const { root, messages, send } = loadInfoview();
  const before = messages.length;
  send({
    protocol: 1,
    type: "decls",
    decls: [
      {
        name: "t",
        ty: "Nat",
        ty_runs: [{ text: "Nat", kind: "unknown_ident" }],
        range: { start: { line: 11, character: 0 } },
      },
    ],
  });
  const rows = byClass(root, "decl");
  assert.strictEqual(rows.length, 1, "exactly one declaration row");
  const row = rows[0];
  assert.strictEqual(textOf(byClass(row, "decl-name")[0]), "t");
  assert.strictEqual(
    textOf(byClass(row, "decl-line-hint")[0]),
    "L12",
    "the line hint must be the 1-based start line (line 11 -> L12)",
  );
  const ty = byClass(row, "decl-ty")[0];
  assert.ok(ty, "the type line must render");
  assert.strictEqual(textOf(ty), "Nat");
  // Non-interactive: no click listener anywhere in the row, and nothing may
  // be posted (jumping is the tree's job; the webview is read-only).
  for (const node of descendants(row)) {
    assert.ok(
      !node._listeners.click,
      "declaration rows must not register click handlers",
    );
  }
  assert.strictEqual(messages.length, before, "rows must not postMessage");
  assert.ok(
    messages.every((message) => message.type !== "focusExercise"),
    "the webview must not emit focusExercise",
  );
});

test("decls empty: the three reasons are told apart (T-B12)", () => {
  // 计划 T-B12：声明栏为空时以前一律「暂无声明。」——用户看到的是"插件坏了"，
  // 而其实可能只是还在编译、或者那次请求根本没成功。三种原因必须说清楚。
  // **限定在声明区**：目标区也有一条 `empty` 占位（「等待编译…」），
  // 直接 `byClass(root, "empty")` 会同时命中两条。
  const declsBodyOf = (root) => {
    const bodies = byClass(root, "decls");
    assert.strictEqual(bodies.length, 1, "声明区必须只有一个容器");
    return bodies[0];
  };
  const emptyText = (send, root) => {
    const empty = byClass(declsBodyOf(root), "empty");
    assert.strictEqual(empty.length, 1, "声明区空态必须有一行说明");
    return textOf(empty[0]);
  };

  // ① 真的没有声明（状态 ready、列表为空）。
  {
    const { root, send } = loadInfoview();
    send({ protocol: 1, type: "status", state: "ready", decls: 0 });
    send({ protocol: 1, type: "decls", decls: [] });
    assert.strictEqual(emptyText(send, root), "这个文件没有声明。");
  }

  // ② 服务器还没编译完。
  {
    const { root, send } = loadInfoview();
    send({ protocol: 1, type: "status", state: "loading" });
    send({ protocol: 1, type: "decls", decls: [] });
    assert.strictEqual(emptyText(send, root), "编译中…（声明列表稍后出现）");
  }

  // ③ 读取失败。
  {
    const { root, send } = loadInfoview();
    send({ protocol: 1, type: "status", state: "error" });
    send({ protocol: 1, type: "decls", decls: [] });
    assert.ok(
      emptyText(send, root).startsWith("读取声明失败"),
      "读取失败必须说出来（而不是假装「没有声明」）",
    );
  }

  // ④ 状态**后**到也要刷新那句话（先画空列表、后收到状态的顺序很常见）。
  {
    const { root, send } = loadInfoview();
    send({ protocol: 1, type: "decls", decls: [] });
    send({ protocol: 1, type: "status", state: "loading" });
    assert.strictEqual(emptyText(send, root), "编译中…（声明列表稍后出现）");
  }

  // ⑤ 有声明时不许出现空态行。
  {
    const { root, send } = loadInfoview();
    send({ protocol: 1, type: "status", state: "ready", decls: 1 });
    send({ protocol: 1, type: "decls", decls: [{ name: "t", kind: "theorem", status: "checked" }] });
    assert.strictEqual(byClass(declsBodyOf(root), "empty").length, 0, "有声明时不该有空态行");
  }
});

test("status: loading shows 编译中…", () => {
  const { root, send } = loadInfoview();
  send({ protocol: 1, type: "status", state: "loading" });
  assert.strictEqual(textOf(byClass(root, "status")[0]), "编译中…");
});

test("status: ready shows the declaration count", () => {
  const { root, send } = loadInfoview();
  send({ protocol: 1, type: "status", state: "ready", decls: 3 });
  assert.strictEqual(textOf(byClass(root, "status")[0]), "已就绪 · 3 个声明");
});

test("status: idle waits for a .sokonanoda file", () => {
  const { root, send } = loadInfoview();
  send({ protocol: 1, type: "status", state: "idle" });
  assert.strictEqual(textOf(byClass(root, "status")[0]), "等待 .sokonanoda 文件");
});

test("server: running snapshot renders version and pid without throwing", () => {
  const { root, send } = loadInfoview();
  send({ protocol: 1, type: "server", running: true, version: "1.2.3", pid: 7 });
  const line = textOf(byClass(root, "server-line")[0]);
  assert.ok(line.includes("1.2.3"), `version expected in ${line}`);
  assert.ok(line.includes("pid 7"), `pid expected in ${line}`);
});

test("state: a sort run renders a tok-sort span (palette covers every kind)", () => {
  // Regression for the reported uncoloured `Prop`/`Type`/`Sort`: the webview
  // must emit the exact `tok-sort` class the stylesheet's guaranteed-fallback
  // palette keys off (crates/cli/tests/extension.rs locks the CSS side).
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "state",
    goal: "Type",
    goal_runs: [{ text: "Type", kind: "sort" }],
  });
  const toks = byClass(root, "tok-sort");
  assert.strictEqual(toks.length, 1, "a sort run must be one tok-sort span");
  assert.strictEqual(textOf(toks[0]), "Type");
});

test("theme: message keys the CSS palette off body[data-theme]", () => {
  const { document, send } = loadInfoview();
  send({ protocol: 1, type: "theme", kind: "light" });
  assert.strictEqual(
    document.body.attributes["data-theme"],
    "light",
    "the theme message must set body[data-theme]",
  );
  send({ protocol: 1, type: "theme" });
  assert.strictEqual(
    document.body.attributes["data-theme"],
    "dark",
    "a theme message without a kind must default to dark",
  );
});

// ── Infoview 字号（2026-09-26 用户反馈「字太小太暗」）────────────────────────
//
// 判据分两半，缺一不可：
//   ① **CSS 契约**（静态解析 `media/infoview.css`）：声明类型/值/目标的字号必须
//      在 **1em 级别**（不是修订前的 `0.78em`）、行高 ≥ 1.5、**不许**再叠
//      `opacity`（前景色已经在压一档，再乘透明度就是**双重压暗** ✗）；
//      按默认基字号 13px 折算出的**实际像素必须 ≥ 12px** ✓。
//   ② **倍率管道**：主机发的 `fontScale` 必须落到 `body` 的行内 style 上
//      ⇒ 改设置后立刻生效，不用重开面板 ✓。
test("CSS: declaration type/value/goal text is body-sized, bright and airy", () => {
  const css = fs.readFileSync(path.join(__dirname, "media", "infoview.css"), "utf8");
  const rule = (selector) => {
    const m = new RegExp("\\" + selector + "\\s*\\{([^}]*)\\}").exec(css);
    assert.ok(m, `infoview.css must define ${selector}`);
    return m[1];
  };
  // VS Code 的基字号兜底（`:root` 里的 `--vscode-font-size, 13px`）。
  const BASE_PX = 13;
  for (const selector of [".decl-ty", ".decl-val", ".decl-goal"]) {
    const body = rule(selector);
    const size = /font-size:\s*([^;]+);/.exec(body);
    assert.ok(size, `${selector} must set a font-size`);
    const px = /calc\(1em \* var\(--soko-font-scale/.test(size[1])
      ? BASE_PX
      : parseFloat(size[1]) * BASE_PX;
    assert.ok(
      px >= 12,
      `${selector} 的实际字号必须 ≥ 12px（用户反馈「太小」），实际 ${px}px（${size[1].trim()}）`,
    );
    const lh = /line-height:\s*([\d.]+)/.exec(body);
    assert.ok(
      lh && parseFloat(lh[1]) >= 1.5,
      `${selector} 的行高必须 ≥ 1.5，实际 ${lh ? lh[1] : "（没写）"}`,
    );
    assert.ok(
      !/opacity:/.test(body),
      `${selector} **不许**再叠 opacity —— 前景色已经压了一档，再乘透明度就是双重压暗 ✗`,
    );
  }
  assert.ok(
    /\.decl\s*\{[^}]*padding:\s*4px 8px;/.test(css),
    "`.decl` 的内边距必须是 4px 8px（行与行不再挤在一起）",
  );
});

test("font scale: the host message lands on --soko-font-scale", () => {
  const { document, send } = loadInfoview();
  send({ protocol: 1, type: "state", fontScale: 1.25 });
  assert.strictEqual(
    document.body.attributes["style"],
    "--soko-font-scale: 1.25",
    "`fontScale` 必须落到 body 的行内 style 上（改设置后立刻生效 ✓）",
  );
  send({ protocol: 1, type: "state" });
  assert.strictEqual(
    document.body.attributes["style"],
    "--soko-font-scale: 1",
    "缺省/非法值必须回落到 1 ✓",
  );
});

console.log(`\n${passed + failed} tests, ${passed} passed, ${failed} failed\n`);
process.exit(failed > 0 ? 1 : 0);
