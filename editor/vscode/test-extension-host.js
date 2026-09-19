// Behavioral tests for editor/vscode/extension.js under a stubbed extension
// host: pure Node, no VS Code, no network, no Rust, no subprocesses.
//
// Why this layer exists: the audit of 0.57.0 found two real defects that no
// existing test could see —
//   1. the `onDidChangeDiagnostics` listener was global and unfiltered, so ANY
//      extension's diagnostics (a `.ts` file from another extension) triggered
//      `soko/goals` + a full tree/Infoview rebuild, and a burst of project-mode
//      publishes triggered it several times;
//   2. `loadDeclarations()` built tree rows from `this.uri` **after** the await,
//      so switching documents mid-flight produced rows naming file A whose click
//      command revealed a range in file B.
// Both are invisible to the string-presence contract tests
// (crates/cli/tests/extension.rs) and to the VS Code integration suite (it only
// opens one document). Here the host is stubbed so we can count requests,
// postMessages and tree rows deterministically.
//
// Run: node editor/vscode/test-extension-host.js   (also part of `npm run test:unit`)

const assert = require("assert");
const Module = require("module");
const path = require("path");
const { EventEmitter } = require("events");

// ── fake timers ──────────────────────────────────────────────────────────
// The extension debounces selection (200 ms) and diagnostics (150 ms); tests
// fire the callbacks by hand instead of sleeping.
const realSetTimeout = global.setTimeout;
const realClearTimeout = global.clearTimeout;
let timers = [];
let timerSeq = 0;
global.setTimeout = (fn, ms, ...args) => {
  const id = ++timerSeq;
  timers.push({ id, fn, ms, args });
  return id;
};
global.clearTimeout = (id) => {
  timers = timers.filter((timer) => timer.id !== id);
};
function fireTimers({ min = 0 } = {}) {
  const due = timers.filter((timer) => timer.ms >= min);
  timers = timers.filter((timer) => timer.ms < min);
  for (const timer of due) timer.fn(...timer.args);
  return due.length;
}
function pendingTimers() {
  return timers.map((timer) => timer.ms);
}

// ── fake vscode ──────────────────────────────────────────────────────────
function makeDisposable() {
  return { dispose() {} };
}
function emitter() {
  const listeners = [];
  return {
    event: (listener) => {
      listeners.push(listener);
      return makeDisposable();
    },
    fire: (value) => listeners.forEach((listener) => listener(value)),
    count: () => listeners.length,
  };
}

const listeners = {
  diagnostics: emitter(),
  selection: emitter(),
  activeEditor: emitter(),
  configuration: emitter(),
  theme: emitter(),
};

class TreeItem {
  constructor(label, collapsibleState) {
    this.label = label;
    this.collapsibleState = collapsibleState;
    this.children = undefined;
  }
}

const vscodeStub = {
  version: "1.106.0",
  Uri: {
    file: (fsPath) => ({
      scheme: "file",
      fsPath,
      path: fsPath,
      toString: () => `file://${fsPath}`,
    }),
    parse: (value) => {
      const fsPath = String(value).replace(/^file:\/\//, "");
      return { scheme: "file", fsPath, path: fsPath, toString: () => String(value) };
    },
    joinPath: (base, ...parts) => vscodeStub.Uri.file(path.join(base.fsPath, ...parts)),
  },
  TreeItem,
  TreeItemCollapsibleState: { None: 0, Collapsed: 1, Expanded: 2 },
  ThemeIcon: class ThemeIcon {
    constructor(id) {
      this.id = id;
    }
  },
  ThemeColor: class ThemeColor {
    constructor(id) {
      this.id = id;
    }
  },
  MarkdownString: class MarkdownString {
    constructor(value = "") {
      // Faithful to the real API: `new MarkdownString(text)` starts with that
      // text (the extension builds status-bar tooltips this way).
      this.value = value;
    }
    appendCodeblock(text) {
      this.value += text;
      return this;
    }
    appendMarkdown(text) {
      this.value += text;
      return this;
    }
  },
  StatusBarAlignment: { Left: 1, Right: 2 },
  ViewColumn: { One: 1, Beside: 2 },
  ConfigurationTarget: { Global: 1, Workspace: 2 },
  RelativePattern: class RelativePattern {
    constructor(base, pattern) {
      this.base = base;
      this.pattern = pattern;
    }
  },
  Disposable: { from: () => makeDisposable() },
  EventEmitter: class EventEmitter {
    constructor() {
      this._emitter = emitter();
      this.event = this._emitter.event;
    }
    fire(value) {
      this._emitter.fire(value);
    }
    dispose() {}
  },
  commands: {
    registerCommand: () => makeDisposable(),
    executeCommand: async () => undefined,
  },
  window: {
    activeTextEditor: undefined,
    onDidChangeActiveTextEditor: (listener) => listeners.activeEditor.event(listener),
    onDidChangeTextEditorSelection: (listener) => listeners.selection.event(listener),
    onDidChangeVisibleTextEditors: () => makeDisposable(),
    onDidChangeActiveColorTheme: (listener) => listeners.theme.event(listener),
    createTreeView: (id, options) => {
      // Keyed by view id: activation creates the exercise tree and the course
      // tree, and the course one would otherwise overwrite the capture.
      vscodeStub.__trees = vscodeStub.__trees ?? {};
      vscodeStub.__trees[id] = options?.treeDataProvider;
      return { dispose() {}, reveal: async () => {} };
    },
    createStatusBarItem: () => {
      const item = { show() {}, hide() {}, dispose() {}, text: "", tooltip: "" };
      vscodeStub.__statusBar = item;
      return item;
    },
    createOutputChannel: () => ({ appendLine() {}, append() {}, show() {}, dispose() {} }),
    registerWebviewViewProvider: (_id, provider) => {
      vscodeStub.__infoview = provider;
      return makeDisposable();
    },
    showInformationMessage: async () => undefined,
    showWarningMessage: async () => undefined,
    showErrorMessage: async () => undefined,
    showTextDocument: async () => undefined,
    registerUriHandler: () => makeDisposable(),
    withProgress: async (_options, task) => task({ report() {} }),
  },
  languages: {
    onDidChangeDiagnostics: (listener) => listeners.diagnostics.event(listener),
  },
  workspace: {
    workspaceFolders: [{ uri: { fsPath: "/repo" }, name: "repo" }],
    getConfiguration: () => ({
      get: (key, fallback) => fallback,
      update: async () => undefined,
    }),
    onDidChangeConfiguration: (listener) => listeners.configuration.event(listener),
    createFileSystemWatcher: () => ({
      onDidChange: () => makeDisposable(),
      onDidCreate: () => makeDisposable(),
      onDidDelete: () => makeDisposable(),
      dispose() {},
    }),
    openTextDocument: async () => ({ uri: { toString: () => "file:///x" } }),
    asRelativePath: (value) => String(value),
    findFiles: async () => [],
    textDocuments: [],
  },
  extensions: { getExtension: () => undefined },
};

// ── fake language client ─────────────────────────────────────────────────
const requests = [];
const stateEmitters = [];
class LanguageClient {
  constructor() {
    this.outputChannel = { appendLine() {}, append() {}, show() {}, dispose() {} };
    stateEmitters.push(this);
  }
  onDidChangeState(listener) {
    this._stateListener = listener;
    return makeDisposable();
  }
  async start() {}
  async stop() {}
  async sendRequest(method, params) {
    requests.push({ method, params });
    return stubbedResponses[method]?.(params) ?? null;
  }
}
const vscodeLanguageclientStub = {
  LanguageClient,
  TransportKind: { stdio: 0, ipc: 1, pipe: 2, socket: 3 },
  State: { Stopped: 1, Running: 2, Starting: 3 },
};

// Handler map: tests replace entries to control answers.
const stubbedResponses = {
  "soko/version": () => ({ version: "0.58.0", pid: 4242 }),
  "soko/goals": () => ({ decls: [] }),
  "soko/project": () => ({ uri: "", version: 1, project: null, reason: "no-imports" }),
  "soko/stateAt": () => ({ decls: [], goals: [] }),
  "soko/nextHole": () => null,
  "soko/hints": () => ({ hints: [] }),
};

// ── fake child_process (course tree) ─────────────────────────────────────
const spawns = [];
// 课程树的一次运行默认**不吐数据**（树保持空，除非测试自己驱动）。
// `setCourseEvents` 让测试喂一份 `course.unit` 事件流：v1 平铺 / v2 分组的
// 两条路径因此都能被真渲染出来（台账 G-07，设计 course-manifest-v2.md §4.3）。
let courseEvents = [];
function setCourseEvents(events) {
  courseEvents = events || [];
}
function fakeSpawn(command, args) {
  spawns.push({ command, args });
  const child = new EventEmitter();
  const stream = () =>
    Object.assign(new EventEmitter(), { setEncoding: () => {}, resume: () => {} });
  child.stdout = stream();
  child.stderr = stream();
  child.kill = () => {};
  process.nextTick(() => {
    if (Array.isArray(args) && args[0] === "course" && courseEvents.length) {
      child.stdout.emit("data", courseEvents.map((event) => JSON.stringify(event)).join("\n") + "\n");
    }
    child.emit("close", 0);
  });
  return child;
}

// ── module loading ───────────────────────────────────────────────────────
const extensionPath = path.join(__dirname, "extension.js");
const serverStub = {
  resolveServerForStart: async () => ({ command: "/stub/sokonanoda-lsp", source: "stub" }),
  resolveServerCommand: () => ({ command: "/stub/sokonanoda-lsp", source: "stub" }),
  resolveCliCommand: () => "/stub/sokonanoda",
  resolveCourseManifest: () => "/repo/course/course.json",
  firstExisting: (candidates) => candidates[0],
  platformTarget: () => "darwin-arm64",
  serverVersionMarker: () => "/stub/version",
  newestInstalledExtensionVersion: () => undefined,
  downloadLspBinary: async () => "/stub/sokonanoda-lsp",
};
const originalLoad = Module._load;
Module._load = function patched(request, parent, isMain) {
  if (request === "vscode") return vscodeStub;
  if (request === "vscode-languageclient/node") return vscodeLanguageclientStub;
  if (request === "child_process") return { spawn: fakeSpawn, execFile: () => {}, execSync: () => "" };
  if (request === "./server" && parent && parent.filename === extensionPath) return serverStub;
  return originalLoad.call(this, request, parent, isMain);
};

const extension = require(extensionPath);
Module._load = originalLoad;

// ── harness ──────────────────────────────────────────────────────────────
function fakeDocument(fsPath, languageId = "sokonanoda", text = "") {
  const uri = vscodeStub.Uri.file(fsPath);
  return {
    uri,
    languageId,
    getText: () => text,
    lineCount: 1,
    selection: { active: { line: 0, character: 0 } },
  };
}

// The real host sets `window.activeTextEditor` *before* firing the event.
function editorFor(document) {
  return { document, selection: { active: { line: 0, character: 0 } } };
}
function focus(document) {
  const editor = editorFor(document);
  vscodeStub.window.activeTextEditor = editor;
  listeners.activeEditor.fire(editor);
  return editor;
}

function resetListeners() {
  for (const key of Object.keys(listeners)) listeners[key] = emitter();
}

async function activateExtension() {
  requests.length = 0;
  spawns.length = 0;
  timers = [];
  // 每个测试重新激活一次：监听器必须重新挂，否则上一个测试的监听器还在
  //（一次事件会被处理两遍——这正是我们要测的那类放大问题）。
  resetListeners();
  vscodeStub.window.activeTextEditor = undefined;
  const context = {
    subscriptions: [],
    extensionPath: __dirname,
    extension: { packageJSON: { version: "0.58.0" } },
    globalState: { get: () => undefined, update: async () => undefined },
    workspaceState: { get: () => undefined, update: async () => undefined },
  };
  await extension.activate(context);
  // Activation kicks off fire-and-forget work (server start + doctor); let the
  // event loop drain it so tests start from a quiet state.
  for (let i = 0; i < 50; i++) await Promise.resolve();
  requests.length = 0;
  return context;
}

function statusBarStub() {
  return vscodeStub.__statusBar;
}

function goalsRequests() {
  return requests.filter((request) => request.method === "soko/goals");
}
function stateRequests() {
  return requests.filter((request) => request.method === "soko/stateAt");
}

const tests = [];
function test(name, fn) {
  tests.push({ name, fn });
}

// ── tests ────────────────────────────────────────────────────────────────

test("diagnostics from other languages never drive soko/goals", async () => {
  await activateExtension();
  focus(fakeDocument("/repo/playground.sokonanoda"));
  await Promise.resolve();
  requests.length = 0;

  // A TypeScript file (or ESLint) reporting diagnostics is not our business.
  listeners.diagnostics.fire({
    uris: [vscodeStub.Uri.file("/repo/src/app.ts"), vscodeStub.Uri.file("/repo/README.md")],
  });
  fireTimers();
  await Promise.resolve();
  assert.deepStrictEqual(
    requests.map((request) => request.method),
    [],
    "an unrelated diagnostics event must not trigger soko/goals or soko/stateAt",
  );
});

test("a burst of project diagnostics collapses into one refresh", async () => {
  await activateExtension();
  focus(fakeDocument("/repo/course/unit11-project/Canvas.sokonanoda"));
  await Promise.resolve();
  requests.length = 0;

  // Project mode publishes the entry and its dependency in one go, and the
  // client may coalesce several events: all of them must produce one refresh.
  const canvas = vscodeStub.Uri.file("/repo/course/unit11-project/Canvas.sokonanoda");
  const logic = vscodeStub.Uri.file("/repo/course/unit11-project/Logic.sokonanoda");
  listeners.diagnostics.fire({ uris: [canvas] });
  listeners.diagnostics.fire({ uris: [logic] });
  listeners.diagnostics.fire({ uris: [canvas, logic] });
  assert.deepStrictEqual(pendingTimers(), [150], "diagnostics are debounced");
  fireTimers();
  for (let i = 0; i < 20; i++) await Promise.resolve();

  assert.strictEqual(goalsRequests().length, 1, "one refresh ⇒ exactly one soko/goals");
  assert.strictEqual(stateRequests().length, 1, "one refresh ⇒ exactly one soko/stateAt");
});

test("concurrent loads share a single soko/goals round trip", async () => {
  await activateExtension();
  focus(fakeDocument("/repo/playground.sokonanoda"));
  await Promise.resolve();
  requests.length = 0;

  // One diagnostics event makes both the tree (which re-resolves its root)
  // and the debounced refresh ask for declarations at the same time.
  const tree = vscodeStub.__trees?.["sokonanoda.goals"];
  assert.ok(tree, "the exercise tree must be registered");
  listeners.diagnostics.fire({ uris: [vscodeStub.Uri.file("/repo/playground.sokonanoda")] });
  const resolving = tree.getChildren(); // the view re-resolves immediately
  fireTimers(); // …and the debounced refresh fires while that is in flight
  await resolving;
  for (let i = 0; i < 20; i++) await Promise.resolve();
  assert.strictEqual(goalsRequests().length, 1, "in-flight calls must be merged");
});

test("switching documents mid-flight drops the stale answer", async () => {
  await activateExtension();
  const canvas = fakeDocument("/repo/course/unit11-project/Canvas.sokonanoda");
  focus(canvas);
  await Promise.resolve();

  // Answer the next soko/goals slowly so the test can switch documents first.
  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  stubbedResponses["soko/goals"] = async () => {
    await gate;
    return {
      decls: [
        { name: "stale_decl", kind: "theorem", status: "open", holes: [{ id: "h0", range: {} }] },
      ],
    };
  };
  requests.length = 0;
  listeners.diagnostics.fire({ uris: [canvas.uri] });
  fireTimers();
  await Promise.resolve();
  assert.strictEqual(goalsRequests().length, 1, "the load is in flight");

  // The learner switches to the other file while the request is pending.
  focus(fakeDocument("/repo/course/unit11-project/Logic.sokonanoda"));
  release();
  for (let i = 0; i < 30; i++) await Promise.resolve();

  // The answer belongs to the document we left: it must be dropped, not turned
  // into rows that name A's declaration but act on B (which is what the
  // pre-fix code did: `revealRange(B, A 的洞)`).
  stubbedResponses["soko/goals"] = () => ({ decls: [] });
  const tree = vscodeStub.__trees?.["sokonanoda.goals"];
  assert.ok(tree, "the exercise tree must be registered");
  const labels = ((await tree.getChildren()) ?? []).map((item) => String(item.label));
  assert.ok(
    !labels.includes("stale_decl"),
    `a row built from the stale answer leaked into the new document: ${labels.join(", ")}`,
  );
});

test("identical declaration lists are not posted to the Infoview twice", async () => {
  const context = await activateExtension();
  const provider = vscodeStub.__infoview;
  assert.ok(provider, "the Infoview provider must be registered");
  const posts = [];
  provider._view = { webview: { postMessage: (message) => posts.push(message) } };
  provider._ready = true;

  const decls = [
    { name: "and_swap", kind: "theorem", status: "open", holes: [{ id: "h0", range: {} }] },
  ];
  provider.setDecls(decls);
  provider.setDecls(decls.map((decl) => ({ ...decl }))); // same content, new objects
  assert.strictEqual(
    posts.filter((message) => message.type === "decls").length,
    1,
    "the webview rebuilds the whole list; identical payloads must be dropped",
  );
  provider.setDecls([{ name: "other", kind: "theorem", status: "checked" }]);
  assert.strictEqual(
    posts.filter((message) => message.type === "decls").length,
    2,
    "a real change must still be posted",
  );
  void context;
});

test("an answer that names another document is dropped", async () => {
  await activateExtension();
  const canvas = fakeDocument("/repo/course/unit11-project/Canvas.sokonanoda");
  focus(canvas);
  await Promise.resolve();
  requests.length = 0;

  // 服务端会回显它答的是哪份文档：这里故意答另一份 ⇒ 必须丢弃，不能让树显示
  // 别的文件的声明（快速切换文件时最危险）。
  stubbedResponses["soko/goals"] = () => ({
    uri: "file:///repo/course/unit11-project/Logic.sokonanoda",
    version: 1,
    decls: [{ name: "stale_decl", kind: "theorem", status: "open", holes: [] }],
  });
  listeners.diagnostics.fire({ uris: [canvas.uri] });
  fireTimers();
  for (let i = 0; i < 20; i++) await Promise.resolve();

  stubbedResponses["soko/goals"] = () => ({ decls: [] });
  const tree = vscodeStub.__trees?.["sokonanoda.goals"];
  const labels = ((await tree.getChildren()) ?? []).map((item) => String(item.label));
  assert.ok(
    !labels.includes("stale_decl"),
    `a mismatched answer must be dropped: ${labels.join(", ")}`,
  );
});

test("the project tree renders the closure the server describes", async () => {
  await activateExtension();
  const canvas = fakeDocument("/repo/course/unit11-project/Exercises.sokonanoda");
  focus(canvas);
  await Promise.resolve();

  stubbedResponses["soko/project"] = () => ({
    uri: canvas.uri.toString(),
    version: 3,
    project: {
      entry: "Exercises",
      root: "/repo/course/unit11-project",
      manifest: "/repo/course/unit11-project/sokonanoda.toml",
      requires_warning: null,
      counts: { modules: 2, compiled: 2, failed: 0, blocked: 0, decls: 7, errors: 0, warnings: 0, open_exercises: 2 },
      diagnostics: [],
      modules: [
        { name: "Logic", path: "/repo/course/unit11-project/Logic.sokonanoda", status: "compiled", entry: false, imports: [], decls: 5, errors: 0, warnings: 0, open_exercises: 0, message: null },
        { name: "Exercises", path: "/repo/course/unit11-project/Exercises.sokonanoda", status: "compiled", entry: true, imports: ["Logic"], decls: 2, errors: 0, warnings: 0, open_exercises: 2, message: null },
      ],
    },
    reason: null,
  });
  listeners.diagnostics.fire({ uris: [canvas.uri] });
  fireTimers();
  for (let i = 0; i < 20; i++) await Promise.resolve();

  const tree = vscodeStub.__trees?.["sokonanoda.project"];
  assert.ok(tree, "the project tree must be registered");
  const roots = await tree.getChildren();
  assert.strictEqual(roots.length, 1, "one root node describes the project");
  assert.strictEqual(String(roots[0].label), "unit11-project");
  assert.strictEqual(String(roots[0].description), "2 模块 · 2 练习");
  assert.ok(
    String(roots[0].tooltip).includes("清单：/repo/course/unit11-project/sokonanoda.toml"),
    `the root must name the manifest source: ${roots[0].tooltip}`,
  );
  const modules = await tree.getChildren(roots[0]);
  assert.deepStrictEqual(
    modules.map((item) => String(item.label)),
    ["Logic", "Exercises"],
    "topological order, entry last",
  );
  assert.strictEqual(String(modules[0].description), "依赖 · 5 声明");
  assert.strictEqual(String(modules[1].description), "入口 · 2 声明 · 2 练习");
  assert.strictEqual(modules[0].command.command, "vscode.open", "clicking opens the module");
  assert.strictEqual(modules[0].command.arguments[0].fsPath, "/repo/course/unit11-project/Logic.sokonanoda");

  // 状态栏 tooltip 带上项目那一行（不新开第二个 status item）。
  const tooltip = statusBarStub()?.tooltip;
  assert.ok(tooltip, "a status bar item carries a tooltip");
  assert.ok(
    String(tooltip.value ?? tooltip).includes("项目：/repo/course/unit11-project"),
    `the status bar tooltip carries the project line: ${tooltip?.value ?? tooltip}`,
  );
  stubbedResponses["soko/project"] = () => ({ uri: "", version: 1, project: null, reason: "no-imports" });
});

test("a single file shows the placeholder instead of an empty project tree", async () => {
  await activateExtension();
  const document = fakeDocument("/repo/playground.sokonanoda");
  focus(document);
  await Promise.resolve();
  stubbedResponses["soko/project"] = () => ({
    uri: document.uri.toString(),
    version: 1,
    project: null,
    reason: "no-imports",
  });
  listeners.diagnostics.fire({ uris: [document.uri] });
  fireTimers();
  for (let i = 0; i < 20; i++) await Promise.resolve();

  const tree = vscodeStub.__trees?.["sokonanoda.project"];
  const roots = await tree.getChildren();
  assert.strictEqual(roots.length, 1);
  assert.strictEqual(String(roots[0].label), "单文件（无 import）");
  assert.deepStrictEqual(await tree.getChildren(roots[0]), []);
  stubbedResponses["soko/project"] = () => ({ uri: "", version: 1, project: null, reason: "no-imports" });
});

test("a project answer for another document is dropped", async () => {
  await activateExtension();
  const canvas = fakeDocument("/repo/course/unit11-project/Exercises.sokonanoda");
  focus(canvas);
  await Promise.resolve();
  stubbedResponses["soko/project"] = () => ({
    uri: "file:///repo/somewhere-else.sokonanoda",
    version: 9,
    project: { entry: "Other", root: "/repo", manifest: null, requires_warning: null, counts: { modules: 1 }, diagnostics: [], modules: [] },
    reason: null,
  });
  listeners.diagnostics.fire({ uris: [canvas.uri] });
  fireTimers();
  for (let i = 0; i < 20; i++) await Promise.resolve();

  const tree = vscodeStub.__trees?.["sokonanoda.project"];
  const roots = await tree.getChildren();
  assert.ok(
    roots.length === 0 || String(roots[0].label) !== "repo",
    `an answer for another document must not paint this tree: ${roots.map((r) => r.label)}`,
  );
  stubbedResponses["soko/project"] = () => ({ uri: "", version: 1, project: null, reason: "no-imports" });
});

test("cursor moves ask only for the caret state", async () => {
  await activateExtension();
  const document = fakeDocument("/repo/playground.sokonanoda");
  focus(document);
  await Promise.resolve();
  requests.length = 0;

  listeners.selection.fire({
    textEditor: { document, selection: { active: { line: 3, character: 2 } } },
  });
  assert.deepStrictEqual(pendingTimers(), [200], "selection moves are debounced");
  fireTimers();
  for (let i = 0; i < 20; i++) await Promise.resolve();

  assert.strictEqual(stateRequests().length, 1, "one debounced move ⇒ one soko/stateAt");
  assert.strictEqual(goalsRequests().length, 0, "cursor movement never re-fetches decls");
});

test("the course tree caches one CLI run across repeated resolves", async () => {
  await activateExtension();
  const courseSpawns = () => spawns.filter((spawn) => spawn.args?.[0] === "course");
  const tree = vscodeStub.__trees?.["sokonanoda.courseMap"];
  assert.ok(tree, "the course tree must be registered");
  for (let i = 0; i < 20; i++) await Promise.resolve();
  assert.strictEqual(
    courseSpawns().length,
    0,
    "activation alone must not spawn; the view's resolve drives it",
  );

  // 第一次 resolve 起一个进程（11 个单元在**一个**进程里编译）；
  // 之后反复 resolve（展开/可见性变化都会）必须复用缓存。
  await tree.getChildren();
  assert.strictEqual(courseSpawns().length, 1, "one resolve ⇒ one CLI run");
  for (let i = 0; i < 5; i++) await tree.getChildren();
  assert.strictEqual(
    courseSpawns().length,
    1,
    "repeated resolves must reuse the cached course run (one run = 11 compiles, ~320ms)",
  );

  // 显式刷新仍然要真的重跑。
  tree.refresh();
  await tree.getChildren();
  assert.strictEqual(courseSpawns().length, 2, "an explicit refresh re-runs the CLI");
});

// 台账 G-07：v2 清单（事件带 volume/chapter）按 卷 → 章 → 单元 分组；
// v1 清单（事件不带）保持平铺——两条路径都必须真渲染出来。
test("the course tree groups volumes and chapters for a v2 manifest", async () => {
  setCourseEvents([
    {
      type: "course.unit", file: "units/a.sokonanoda", title: "A", unit: 1,
      checked: 2, open: 6, failed: 0,
      volume: { id: "I", title: "卷 I 集合论" },
      chapter: { id: "I.1", title: "集合与运算", tags: ["membership"] },
      tags: ["membership"],
    },
    {
      type: "course.unit", file: "units/b.sokonanoda", title: "B", unit: 2,
      checked: 1, open: 10, failed: 0,
      volume: { id: "I", title: "卷 I 集合论" },
      chapter: { id: "I.1", title: "集合与运算", tags: ["membership"] },
      tags: ["membership"],
    },
    {
      type: "course.unit", file: "units/c.sokonanoda", title: "C", unit: 3,
      checked: 3, open: 5, failed: 0,
      volume: { id: "I", title: "卷 I 集合论" },
      chapter: { id: "I.2", title: "关系与函数", tags: ["relation"] },
      tags: ["relation"],
    },
  ]);
  try {
    await activateExtension();
    const tree = vscodeStub.__trees?.["sokonanoda.courseMap"];
    const roots = await tree.getChildren();
    assert.strictEqual(roots.length, 1, "one volume node");
    assert.match(String(roots[0].label), /卷 I/);
    assert.strictEqual(roots[0].collapsibleState, vscodeStub.TreeItemCollapsibleState.Collapsed);

    const chapters = await tree.getChildren(roots[0]);
    assert.deepStrictEqual(
      chapters.map((chapter) => chapter.label),
      ["I.1 集合与运算", "I.2 关系与函数"],
      "chapters keep the CLI's document order",
    );
    const units = await tree.getChildren(chapters[0]);
    assert.deepStrictEqual(
      units.map((unit) => unit.label),
      ["unit 1 A", "unit 2 B"],
      "units hang under their own chapter",
    );
    assert.strictEqual(
      units[0].collapsibleState,
      vscodeStub.TreeItemCollapsibleState.None,
      "unit nodes stay leaves (v1 behaviour preserved)",
    );
    assert.strictEqual(units[0].command.command, "vscode.open");
    assert.ok(
      String(units[0].command.arguments[0].fsPath).endsWith("/repo/course/units/a.sokonanoda"),
      "unit paths still resolve against the manifest directory",
    );
  } finally {
    setCourseEvents([]);
  }
});

test("the course tree stays flat for a v1 manifest", async () => {
  setCourseEvents([
    { type: "course.unit", file: "units/a.sokonanoda", title: "A", unit: 1, checked: 2, open: 6, failed: 0 },
    { type: "course.unit", file: "units/b.sokonanoda", title: "B", unit: 2, checked: 1, open: 10, failed: 0 },
  ]);
  try {
    await activateExtension();
    const tree = vscodeStub.__trees?.["sokonanoda.courseMap"];
    const roots = await tree.getChildren();
    assert.deepStrictEqual(
      roots.map((item) => item.label),
      ["unit 1 A", "unit 2 B"],
      "a v1 manifest keeps the flat unit list (no regression)",
    );
    assert.strictEqual(
      roots[0].collapsibleState,
      vscodeStub.TreeItemCollapsibleState.None,
      "v1 units are leaves at the root",
    );
    assert.deepStrictEqual(await tree.getChildren(roots[0]), [], "and have no children");
  } finally {
    setCourseEvents([]);
  }
});

test("the course tree keeps unreadable units visible when grouping", async () => {
  setCourseEvents([
    {
      type: "course.unit", file: "units/a.sokonanoda", title: "A", unit: 1,
      checked: 2, open: 6, failed: 0,
      volume: { id: "I", title: "卷 I" },
      chapter: { id: "I.1", title: "第一章", tags: [] },
    },
    { type: "course.unit", file: "ghost.sokonanoda", title: "幽灵", unit: 9, error: "cannot read" },
  ]);
  try {
    await activateExtension();
    const tree = vscodeStub.__trees?.["sokonanoda.courseMap"];
    const roots = await tree.getChildren();
    assert.strictEqual(roots.length, 2, "the volume plus a fallback group");
    assert.match(String(roots[1].label), /无法分组/);
    const orphans = await tree.getChildren(roots[1]);
    assert.deepStrictEqual(orphans.map((item) => item.label), ["unit 9 幽灵"]);
  } finally {
    setCourseEvents([]);
  }
});

// ── runner ───────────────────────────────────────────────────────────────
(async () => {
  let failed = 0;
  for (const { name, fn } of tests) {
    try {
      await fn();
      console.log(`ok   ${name}`);
    } catch (error) {
      failed += 1;
      console.error(`FAIL ${name}\n     ${error.message}`);
    }
  }
  console.log(`\n${tests.length - failed}/${tests.length} passed`);
  global.setTimeout = realSetTimeout;
  global.clearTimeout = realClearTimeout;
  process.exit(failed === 0 ? 0 : 1);
})();
