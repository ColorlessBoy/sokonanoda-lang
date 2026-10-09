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

// **2026-10-08 用户两条**（同一条反馈里的 ① 与 ②）：
//   ①「infoview 最上面的条件，name 和 type 中间加一个冒号隔开」⇒ `h : A ⊆ B`；
//   ②「符号 notation 啥的不需要跳转链接，声明列表里开头的 theorem 名字能跳转就行」
//      ⇒ 带**源位置**的 run 也**不许**注册 click（G-53 的消费端按用户拍板回退 ✗）。
//
// ⚠ 夹具里**故意带 `start`/`end`**（旧代码正是据此加 `tok-clickable` + click ✗）——
// 不带位置的话这条判据**咬不住**旧行为（空转 ✗，AGENTS.md「验证设计纪律」第 3 条）。
test("state: hypotheses read `name : type`, and only the decl name is clickable (2026-10-08)", () => {
  const { root, send } = loadInfoview();
  const runs = (kind, text, start, end) => ({ text, kind, start, end });
  send({
    protocol: 1,
    type: "state",
    uri: "file:///repo/units/u01.sokonanoda",
    goal: "a ∈ A",
    goal_runs: [
      runs("binder", "a", 0, 1),
      { text: " " },
      runs("keyword", "∈", 2, 5),
      { text: " " },
      runs("binder", "A", 6, 7),
    ],
    binders: [
      {
        name: "h",
        ty: "A ⊆ B",
        ty_runs: [
          runs("binder", "A", 0, 1),
          { text: " " },
          runs("keyword", "⊆", 2, 5),
          { text: " " },
          runs("binder", "B", 6, 7),
        ],
      },
    ],
  });
  send({
    protocol: 1,
    type: "decls",
    decls: [
      {
        name: "mem_of_subset",
        kind: "theorem",
        status: "open",
        ty: "a ∈ A → a ∈ B",
        ty_runs: [runs("binder", "a", 0, 1), { text: " " }, runs("keyword", "∈", 2, 5)],
        range: { start: { line: 6, character: 8 } },
      },
    ],
  });

  // ① 冒号：**结构**上必须是 name → 冒号 → type（不是一个拼出来的字符串 ✓）。
  const row = byClass(root, "binder")[0];
  assert.ok(row, "假设行必须渲染");
  assert.deepStrictEqual(
    row.childNodes.map((node) => node.className),
    ["binder-name", "binder-colon", "binder-ty"],
    "假设行顺序必须是 name → `:` → type（用户 ①）",
  );
  assert.strictEqual(textOf(byClass(row, "binder-name")[0]), "h");
  assert.strictEqual(textOf(byClass(row, "binder-colon")[0]), ":");
  assert.strictEqual(textOf(byClass(row, "binder-ty")[0]), "A ⊆ B");

  // ② 可点面**枚举**（多一个都得是有意的 ✓）：目标栏的 `goal-head`（它是 `reveal`
  // ——「滚到这条目标的源码 span」，**不是**跳定义 ✗）与声明卡的 `decl-name`（E27 ✓）；
  // **所有 `tok-` run 一律不可点**（用户 ②：符号 notation 不需要跳转链接）。
  const clickable = descendants(root)
    .filter((node) => node._listeners.click)
    .map((node) => node.className);
  assert.deepStrictEqual(
    clickable,
    ["goal-head", "decl-name"],
    "可点面只能是 goal-head(reveal) + decl-name(定义)（用户 ②）",
  );
  assert.ok(
    !clickable.some((className) => className.includes("tok")),
    "带源位置的 run 不许可点（G-53 的消费端已按用户拍板回退 ✗）",
  );
  // 着色仍在（删的是链接，不是颜色 ✗）。
  assert.deepStrictEqual(
    descendants(row)
      .filter((node) => node.className.includes("tok-"))
      .map(textOf),
    ["A", "⊆", "B"],
  );
});

// **需求 2（2026-10-09 用户实测）**：目标栏的版式必须与 Lean 4 Infoview 一致 ——
// **条件在上、目标在下**（`case …` → 假设列表 → `⊢ 目标`）。
//
// 修前 `renderGoals` 先 append `goal-ty` 再 append `binders` ⇒ 目标在最上面、
// 条件在它下面 ✗（用户报的正是这个）。判据看的是**父节点的子节点顺序**（结构事实），
// 不是文本——文本顺序对不上时 `textContent` 也能"看着像"，咬不住 ✗。
// `goal-head`（"目标 N/M" 的可点 reveal 按钮）留在最上：它对应 Lean 的 `case` 行 ✓。
test("state: hypotheses render ABOVE the goal (Lean 4 layout, 2026-10-09)", () => {
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "state",
    goal: "A",
    goal_runs: [{ text: "A", kind: "axiom_use" }],
    binders: [
      { name: "h", ty: "B", ty_runs: [{ text: "B", kind: "binder" }] },
    ],
  });
  const goal = byClass(root, "goal")[0];
  assert.ok(goal, "目标块必须渲染");
  assert.deepStrictEqual(
    goal.childNodes.map((node) => node.className),
    ["goal-head", "binders", "goal-ty"],
    "版式必须是 目标头 → 条件 → 目标（条件在目标**上面**，用户需求 2）",
  );
  // 冒号那条（2026-10-08 用户）挪位置后不许丢 ✗。
  assert.strictEqual(textOf(byClass(goal, "binder-colon")[0]), ":");
  assert.strictEqual(textOf(byClass(goal, "goal-ty")[0]), "⊢ A");
  assert.strictEqual(textOf(byClass(goal, "binders")[0]), "h:B");
});

// 条件为空时版式不受影响（只有 `goal-head` + `goal-ty`，不留空的 `binders` 壳）。
test("state: an empty context renders no binders list at all (2026-10-09)", () => {
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "state",
    goal: "A",
    goal_runs: [{ text: "A", kind: "axiom_use" }],
    binders: [],
  });
  const goal = byClass(root, "goal")[0];
  assert.deepStrictEqual(
    goal.childNodes.map((node) => node.className),
    ["goal-head", "goal-ty"],
    "没有条件时不许画空列表",
  );
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

// **E21 防漂移判据（2026-09-27 用户拍板：不改，保留）**。
//
// 用户 I1 报的是「infoview 里的声明里的 'theorem' 有多余的 '目标 ⊢ A = B' 这条
// 语句，没有人要求增加」⇒ 计划里写的是"去掉这一行"。**复核结论：前提不成立** ✓
// —— 那行显示的是**还没证完的目标**：对一道 `:= by sorry` 的练习，剩余目标确实
// 等于整个命题（`units/I.1/unit01` 的 `eq_of_same_elements : A = B := by sorry`，
// `soko/goals` 实测 `goal="A = B"`、`goals=["A = B"]`），那不是冗余，那正是
// "你还欠什么没证"的如实表达；证明动过之后它显示的是**剩下的**目标。
// ⇒ E21 结案为「不改（resolved-no-change）」，依据原文 ⇒ `git log --all -- docs/gaps/criteria-census.md`。
//
// 这条测试就是那次拍板的**守卫**：谁再把"目标 == 语句"的卡片过滤掉（即把 I1 的
// 原判当成 bug 修一遍），它当场判红 ✓ —— 反向验证实测过（见提交说明）。
test("decls: a step-0 open exercise still shows its goal row (E21)", () => {
  const { root, send } = loadInfoview();
  // 夹具逐字取自 `courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda`
  // 的 `eq_of_same_elements`（`soko/goals` 的实测载荷）。
  send({
    protocol: 1,
    type: "decls",
    decls: [
      {
        name: "eq_of_same_elements",
        kind: "theorem",
        status: "open",
        ty: "∀ (α : Type 0) (A B : Set α), (∀ (x : α), A x ↔ B x) → A = B",
        ty_runs: [
          { text: "∀ (α : Type 0) (A B : Set α), (∀ (x : α), A x ↔ B x) → A = B" },
        ],
        goal: "A = B",
        goal_runs: [
          { text: "A" },
          { text: " " },
          { text: "=", kind: "keyword" },
          { text: " " },
          { text: "B" },
        ],
        goals: ["A = B"],
        goals_runs: [
          [
            { text: "A" },
            { text: " " },
            { text: "=", kind: "keyword" },
            { text: " " },
            { text: "B" },
          ],
        ],
      },
    ],
  });
  const row = byClass(root, "decl")[0];
  const lines = byClass(row, "decl-goal-line");
  assert.strictEqual(
    lines.length,
    1,
    "E21：`theorem` + `:= by sorry` 的练习**必须**保留「目标 ⊢ …」行" +
      "（用户 2026-09-27 拍板：它显示的是「还没证完的目标」，不是冗余）——" +
      `实际渲染了 ${lines.length} 行`,
  );
  assert.strictEqual(
    textOf(byClass(lines[0], "decl-goal")[0]),
    "⊢ A = B",
    "目标行必须逐字节等于服务端给的剩余目标（`⊢ ` 前缀）",
  );
});

test("decls: clicking a name asks for the DEFINITION, never a reveal (E27)", () => {
  // **E27（用户 [53][54]：「Infoview 里点 `{a}`/`∈`/声明名**点不动**」）**。
  //
  // 这一条钉**发出端**（PLAN §E27 判据 ①）：点击后 webview 发出的消息**语义必须
  // 是 go-to-definition**（带要解析的源位置），**不许发 `reveal`** ✗ ——
  // 这一条**直接封死**历史上那个假绿：`reveal` 只是"滚到源码 span"，而它与
  // "跳到定义"**表现太像**（编辑器都跳了一下）⇒ 被当成跳转成功 ✗✓。
  const { root, messages, send } = loadInfoview();
  send({
    protocol: 1,
    type: "state",
    uri: "file:///repo/units/u01.sokonanoda",
    version: 1,
  });
  send({
    protocol: 1,
    type: "decls",
    decls: [
      {
        name: "mem_self",
        kind: "theorem",
        status: "checked",
        range: { start: { line: 7, character: 8 }, end: { line: 7, character: 16 } },
      },
    ],
  });
  const before = messages.length;
  const name = byClass(root, "decl-name")[0];
  assert.ok(name._listeners.click, "声明名必须可点（E27：以前是纯 span，点了没反应 ✗）");
  name._listeners.click();
  const posted = messages.slice(before);
  assert.strictEqual(posted.length, 1, `点一次发一条：${JSON.stringify(posted)}`);
  assert.strictEqual(
    posted[0].type,
    "definition",
    `语义必须是 go-to-definition（**不是** reveal ✗）：${JSON.stringify(posted[0])}`,
  );
  assert.deepStrictEqual(
    posted[0].position,
    { line: 7, character: 8 },
    "要发**源位置**（`decl.range.start`，LSP 0-based）—— webview 不猜定义在哪",
  );
  assert.strictEqual(posted[0].uri, "file:///repo/units/u01.sokonanoda");
  assert.ok(
    !posted.some((message) => message.type === "reveal"),
    "绝不许退化成 reveal（那是「滚到源码位置」，不是跳定义）",
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
  // **E27 起**：**只有声明名**可点（跳到定义），**行/其它部分仍然不可交互** ✓ ——
  // 这条断言从"整行都不可点"改成"除名字外都不可点"（用户报的正是
  // 「声明名**点不动**」，见 PLAN §E27）。
  const clickable = descendants(row).filter((node) => node._listeners.click);
  assert.deepStrictEqual(
    clickable.map((node) => node.className),
    ["decl-name"],
    "只有 `decl-name` 可以有 click（E27）；行的其它部分必须仍然不可交互",
  );
  assert.strictEqual(messages.length, before, "渲染本身不许 postMessage");
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

test("project: the manifest warning is visible text, never a tooltip (E30)", () => {
  // **E30（用户：「监测到 toml 等项目信息也应该在 infoview 里展现出来」）**。
  //
  // 这一条钉的是 PLAN §E30 的**最高优先级**：`requires_warning` 必须**看得见**
  //（不是 tooltip ✗）。后果很重（G-24）：`requires` 不跟着 bump ⇒ `is_clean()`
  // 为假 ⇒ **项目编译缓存被静默关掉** ⇒ 整个卷 I 每个文件每次打开都从零重编，
  // 而界面上什么都看不出来 —— 用户只会觉得「编译坏了/特别慢」✗。
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "project",
    project: {
      entry: "units.u01",
      root: "/repo/courses/set-theory",
      manifest: "/repo/courses/set-theory/sokonanoda.toml",
      requires_warning: "清单 requires 0.73.0 与当前 0.74.0 不一致：项目编译缓存已关闭。",
      modules: [
        { name: "lib.Set", status: "compiled", decls: 12, errors: 0, warnings: 0, entry: false },
        { name: "units.u01", status: "compiled", decls: 5, errors: 0, warnings: 2, entry: true },
      ],
      counts: { modules: 2, decls: 17, compiled: 2, failed: 0, open_exercises: 3 },
      artifacts: { dir: "/repo/courses/set-theory/.sokonanoda/compiled", entries: 2, bytes: 2048, compiler: "0.73.0" },
    },
    reason: null,
  });
  const body = byClass(root, "project")[0];
  assert.ok(body, "必须有「项目」区块（E30）");
  const warning = byClass(body, "project-warning")[0];
  assert.ok(warning, "requires_warning 必须有**独立可见**的元素（不是 tooltip ✗）");
  assert.ok(
    textOf(warning).includes("requires"),
    `告警文本必须原样看得见（含 requires 细节）：${textOf(warning)}`,
  );
  // **顺序**：告警排在模块列表/计数**之前**（"显眼位置"= 区块第一眼）。
  const order = body.childNodes.map((node) => node.className);
  assert.ok(
    order.indexOf("project-warning") < order.indexOf("project-modules"),
    `告警必须排在模块列表之前（实际顺序 ${JSON.stringify(order)}）`,
  );
  // 清单 / 模块根 / 入口。
  const facts = textOf(byClass(body, "project-facts")[0]);
  for (const needle of ["sokonanoda.toml", "/repo/courses/set-theory", "units.u01"]) {
    assert.ok(facts.includes(needle), `事实行必须含 ${needle}：${facts}`);
  }
  // 模块列表（入口带 ★）。
  const rows = byClass(body, "project-module");
  assert.strictEqual(rows.length, 2, `两个模块两行：${rows.length}`);
  assert.ok(
    textOf(rows[1]).startsWith("★"),
    `入口模块要有标记：${textOf(rows[1])}`,
  );
  assert.ok(
    textOf(rows[1]).includes("5 声明") && textOf(rows[1]).includes("2 警"),
    `模块行要有计数：${textOf(rows[1])}`,
  );
  // 计数汇总 + 产物。
  assert.ok(
    textOf(byClass(body, "project-counts")[0]).includes("2 模块"),
    "计数汇总必须显示",
  );
  assert.ok(
    textOf(byClass(body, "project-artifacts")[0]).includes("2 KiB"),
    `产物行必须显示 entries/bytes/compiler（字节按数量级换单位）：${textOf(byClass(body, "project-artifacts")[0])}`,
  );
});

// **2026-10-08 用户**：「infoview 里『1160652678 字节』可以改成更智能的单位，根据数值大小变化」。
//
// 口径 = **1024 进制 + IEC 名字**（`B`/`KiB`/`MiB`/`GiB`）：`artifacts.bytes` 是文件字节数，
// 写 `KB` 会把 1024 说成 1000 ✗。判据钉**渲染出来的文本**（用户看得见的那一层 ✓），
// 不是"函数返回值" ✗；并**反向**断言裸字节数不再出现（旧形状 `N 字节` 必须消失 ✗）。
test("project: artifact bytes scale to B/KiB/MiB/GiB (user 2026-10-08)", () => {
  const cases = [
    [0, "0 B"],
    [512, "512 B"],
    [2048, "2 KiB"],
    [20480, "20 KiB"],
    [1160652678, "1.1 GiB"],
  ];
  for (const [bytes, expected] of cases) {
    const { root, send } = loadInfoview();
    send({
      protocol: 1,
      type: "project",
      project: {
        entry: "units.u01",
        root: "/repo/courses/set-theory",
        manifest: null,
        modules: [],
        counts: { modules: 1, decls: 0, compiled: 1, failed: 0, open_exercises: 0 },
        artifacts: { entries: 7, bytes, compiler: "0.83.0" },
      },
      reason: null,
    });
    const line = textOf(byClass(root, "project-artifacts")[0]);
    assert.ok(line.includes(expected), `${bytes} 字节 ⇒ 期望含 ${expected}：${line}`);
    assert.ok(!/\d 字节/.test(line), `${bytes} 字节 ⇒ 不许再出现裸字节数：${line}`);
  }
});

// **需求 1B（2026-10-09 用户实测）**：`artifacts.compiler` 缺失时**不许画裸问号**
// 「由编译器 ? 写入」——用户读不出那是"还没写/读不到"还是"插件坏了" ✗。
// 与上面 ② 那行 `project-version` 同一个友好兜底（「（未知）」）✓。
//
// 服务端那半边（旧 schema ⇒ 不报 `compiler`）由 `update_index` 的 schema 升级修掉
// （判据在 `crates/front/src/project/cache.rs` 与 `crates/cli/tests/artifacts.rs`）；
// **这一条钉的是兜底**：真读不到时屏幕上也是人话。
test("project: a missing compiler version reads （未知）, never a bare ? (2026-10-09)", () => {
  for (const compiler of [null, undefined, ""]) {
    const { root, send } = loadInfoview();
    send({
      protocol: 1,
      type: "project",
      project: {
        entry: "units.u01",
        root: "/repo/courses/set-theory",
        manifest: null,
        modules: [],
        counts: { modules: 1, decls: 0, compiled: 1, failed: 0, open_exercises: 0 },
        artifacts: { entries: 7, bytes: 2048, compiler },
      },
      reason: null,
    });
    const line = textOf(byClass(root, "project-artifacts")[0]);
    assert.ok(
      line.includes("由编译器 （未知） 写入"),
      `compiler=${JSON.stringify(compiler)} ⇒ 期望友好文案：${line}`,
    );
    assert.ok(
      !line.includes("?"),
      `compiler=${JSON.stringify(compiler)} ⇒ 不许出现裸问号：${line}`,
    );
  }
});

test("project: no project tells the reason apart, never a blank (E30)", () => {
  // 没有项目时也要**说出原因**（单文件 / 解不出路径 / 先修语法）——
  // 空区块会让用户以为"面板坏了"✗（与 T-B12 的三态同一个道理）。
  const { root, send } = loadInfoview();
  send({ protocol: 1, type: "project", project: null, reason: "no-imports" });
  const body = byClass(root, "project")[0];
  assert.ok(
    textOf(body).includes("单文件"),
    `无项目要说清是"单文件"：${textOf(body)}`,
  );
  send({ protocol: 1, type: "project", project: null, reason: "parse-error" });
  assert.ok(
    textOf(body).includes("语法错误"),
    `parse 失败要说清"先修语法"：${textOf(body)}`,
  );
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

// ── 编译进度（P3，2026-09-26 用户需求：慢文件编译时面板像"冻住了"）──────────
//
// 判据钉**用户看得见的三件事**：① `begin` 画出**恰好 3 行**（标题/进度条/明细）；
// ② `report` **就地更新**（不许叠第二块 —— 那是进度条最常见的 bug）；
// ③ `end` 把整块**移除**（不留空壳 ⇒ 面板回到原样）。
test("progress: begin renders three lines, report updates in place, end removes it", () => {
  const { root, send } = loadInfoview();
  send({ protocol: 1, type: "progress", phase: "begin", label: "编译 units/u01.sokonanoda" });
  let blocks = byClass(root, "progress");
  assert.strictEqual(blocks.length, 1, "`begin` 必须画出进度块");
  assert.strictEqual(
    blocks[0].childNodes.length,
    3,
    "进度块必须**恰好 3 行**（标题 / 进度条 / 明细）",
  );
  assert.strictEqual(
    byClass(root, "progress-label")[0].textContent,
    "编译 units/u01.sokonanoda",
    "第一行是**在编什么**",
  );
  assert.strictEqual(byClass(root, "progress-bar-fill").length, 1, "第二行必须是进度条");
  assert.strictEqual(
    byClass(root, "progress-detail")[0].textContent,
    "进行中…",
    "还没报百分比时明细要有话说（不许空着）",
  );
  send({
    protocol: 1,
    type: "progress",
    phase: "report",
    label: "编译 units/u01.sokonanoda",
    percent: 42,
  });
  blocks = byClass(root, "progress");
  assert.strictEqual(blocks.length, 1, "`report` 必须**就地更新**，不许叠第二块");
  assert.strictEqual(
    byClass(root, "progress-bar-fill")[0].attributes["style"],
    "width: 42%",
  );
  assert.strictEqual(byClass(root, "progress-detail")[0].textContent, "42%");
  send({ protocol: 1, type: "progress", phase: "end" });
  assert.strictEqual(
    byClass(root, "progress").length,
    0,
    "`end` 必须把整块**移除**（不留空壳）",
  );
});

// **需求 3（2026-10-09 用户拍板）：无目标只有**一句**文案**。
//
// 以前这里分两套（B2/2026-10-08）：在末条 tactic 闭合 ⇒「🎉 恭喜，证完了（Q.E.D.）」，
// 其余（含**尾部还有 `sorry`** 的）⇒「已无目标 ✓」。用户要求统一成「🎉 已无目标 ✓」
// ——"已无目标"是**看得见的事实**，"证完了"是替学习者下的判决 ✗。
//
// 判据形状：**同一格**（`goals: []` + `decl` 在场）不管 `status`/`step`/`total` 怎么变，
// 渲染出来的那一行必须**逐字节相同**。下面两条故意用"以前会分叉"的两组字段取值。
test("state: a closed proof shows the unified no-goal line (2026-10-09)", () => {
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "state",
    decl: { name: "done", kind: "theorem", status: "checked" },
    goals: [],
    goal: null,
    step: 1,
    total: 2,
  });
  const solved = byClass(root, "solved")[0];
  assert.ok(solved, "the solved line must be rendered");
  assert.strictEqual(textOf(solved), "🎉 已无目标 ✓");
  assert.ok(
    !textOf(root).includes("Q.E.D."),
    "「恭喜，证完了（Q.E.D.）」那套已被用户删掉（需求 3）",
  );
});

test("state: every other no-goal shape shows the SAME line (2026-10-09)", () => {
  // ① `def`/`axiom`：`total: 0`（没有 tactic）。
  let { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "state",
    decl: { name: "id", kind: "def", status: "checked" },
    goals: [],
    goal: null,
    step: -1,
    total: 0,
  });
  assert.strictEqual(textOf(byClass(root, "solved")[0]), "🎉 已无目标 ✓");

  // ② 开放练习（尾部还有 `sorry` 的那类）：字段取值与 ①/③ 都不同，字样必须一样。
  ({ root, send } = loadInfoview());
  send({
    protocol: 1,
    type: "state",
    decl: { name: "open", kind: "theorem", status: "open" },
    goals: [],
    goal: null,
    step: 1,
    total: 2,
  });
  assert.strictEqual(textOf(byClass(root, "solved")[0]), "🎉 已无目标 ✓");

  // ③ 失败的声明。
  ({ root, send } = loadInfoview());
  send({
    protocol: 1,
    type: "state",
    decl: { name: "bad", kind: "theorem", status: "failed" },
    goals: [],
    goal: null,
    step: 1,
    total: 2,
  });
  assert.strictEqual(textOf(byClass(root, "solved")[0]), "🎉 已无目标 ✓");
});

// **C3（2026-10-08）**：`#check` / `#print` 的输出要**看得见**（用户 P5：「在 infoview
// 没有内容」✗）。选择语义在服务端（`soko/stateAt.messages`，**按行取** = Lean 的
// `getInteractiveDiagnostics{lineRange?}` 口径 ✓），这里只钉**渲染结果** ✓。
// ⚠ 光标停在 `#check` 那一行时**不在任何声明里** ⇒ 面板**不许**用"光标不在任何声明内"
// 盖过命令输出（那正是修好前用户看到的东西 ✗）。
test("state: command outputs (#check / #print) render as a messages block", () => {
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "state",
    decl: null,
    goal: null,
    goals: [],
    step: -1,
    total: 0,
    messages: [
      { kind: "check", text: "myid : Nat", range: {} },
      { kind: "print", text: "def myid (x : Nat) : Nat := x", range: {} },
    ],
  });
  const box = byClass(root, "messages")[0];
  assert.ok(box, "a messages block must be rendered");
  assert.strictEqual(textOf(byClass(root, "messages-head")[0]), "命令输出");
  assert.deepStrictEqual(
    byClass(root, "message-kind").map(textOf),
    ["#check", "#print"],
    "each output must be labelled by its command",
  );
  const texts = byClass(root, "message-text").map(textOf);
  assert.strictEqual(texts.length, 2, "two output rows");
  assert.ok(texts[0].includes("Nat"), `#check shows the type, got ${JSON.stringify(texts[0])}`);
  assert.ok(texts[1].includes(":="), `#print shows the body, got ${JSON.stringify(texts[1])}`);
  assert.ok(
    !textOf(root).includes("光标不在任何声明内"),
    "command output must not be hidden behind the not-in-a-declaration placeholder",
  );
});

// **2026-10-09 用户实测**：`#check` 的输出"缺少 notation 显示和语法高亮" ✗ ——
// 命令输出以前走 `el("pre", …, m.text)`（纯文本）⇒ 同一面板里目标/条件有记法有色、
// 命令输出一色。修法 = **与目标/条件同一个 `codeBlock`**（`runs` 分段 + `tok-*`）✓。
//
// 判据钉**渲染结果**（不是"数据在"）：① 有 `runs` ⇒ 画成 `tok-*` span；
// ② 没有 `runs`（老服务端 / 契约破坏）⇒ 仍然看得见文本（`text` 兜底 ✓）。
test("state: command outputs render through the shared runs renderer (notation + highlight)", () => {
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "state",
    decl: null,
    goal: null,
    goals: [],
    step: -1,
    total: 0,
    messages: [
      {
        kind: "check",
        text: "twice : (n : Nat) → Nat",
        runs: [
          { text: "twice", kind: "def_use" },
          { text: " : " },
          { text: "(n : Nat)", kind: "binder" },
          { text: " → " },
          { text: "Nat", kind: "inductive_use" },
        ],
        range: {},
      },
      { kind: "print", text: "def twice : Nat → Nat := fun (n : Nat) => n + n", runs: [], range: {} },
    ],
  });
  const rows = byClass(root, "message-text");
  assert.strictEqual(rows.length, 2, "two output rows");
  // ① 有 runs ⇒ 与目标/条件**同一条渲染路**：`tok` + `tok-<kind>` span。
  const kinds = byClass(rows[0], "tok").map((node) => node.className);
  assert.deepStrictEqual(
    kinds,
    ["tok tok-def_use", "tok tok-binder", "tok tok-inductive_use"],
    "runs must become tok-* spans (highlighting comes from the shared renderer)",
  );
  assert.strictEqual(textOf(rows[0]), "twice : (n : Nat) → Nat", "runs join back to the text");
  assert.ok(
    textOf(rows[0]).includes("→"),
    "the notation-folded arrow must be what the user sees",
  );
  // ② 没有 runs ⇒ 兜底仍然是**看得见的文本**（不静默变空）。
  assert.strictEqual(textOf(rows[1]), "def twice : Nat → Nat := fun (n : Nat) => n + n");
  assert.strictEqual(byClass(rows[1], "tok").length, 0, "no runs ⇒ plain text, not empty");
});

test("state: no command output ⇒ no messages block, placeholder stays", () => {
  const { root, send } = loadInfoview();
  send({
    protocol: 1,
    type: "state",
    decl: null,
    goal: null,
    goals: [],
    step: -1,
    total: 0,
  });
  assert.strictEqual(byClass(root, "messages").length, 0, "no empty shell");
  assert.ok(textOf(root).includes("光标不在任何声明内"), "the placeholder is still there");
});

console.log(`\n${passed + failed} tests, ${passed} passed, ${failed} failed\n`);
process.exit(failed > 0 ? 1 : 0);
