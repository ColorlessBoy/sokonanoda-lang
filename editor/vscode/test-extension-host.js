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
  textDocument: emitter(),
};

// Configuration overrides for the stub: `get(key, fallback)` returns the
// override when a test set one, the declared default otherwise (exactly what
// the real host does for an unset setting).
const configValues = {};
function setConfig(key, value) {
  configValues[key] = value;
}
function resetConfig() {
  for (const key of Object.keys(configValues)) delete configValues[key];
}

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
  // The notation-input rewriter (NI-2) builds `new vscode.Range(line, start,
  // line, end)` — the four-number constructor, exactly like the real API.
  Position: class Position {
    constructor(line, character) {
      this.line = line;
      this.character = character;
    }
  },
  Range: class Range {
    constructor(startLine, startCharacter, endLine, endCharacter) {
      if (typeof startLine === "object") {
        this.start = startLine;
        this.end = startCharacter;
      } else {
        this.start = { line: startLine, character: startCharacter };
        this.end = { line: endLine, character: endCharacter };
      }
      this.isEmpty =
        this.start.line === this.end.line && this.start.character === this.end.character;
    }
  },
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
    // Handlers are captured by id: the notation-input tests drive the real
    // command (`sokonanoda.input.replaceAbbreviation`) instead of re-implementing
    // its logic, and the keybinding contract is asserted in Rust.
    registerCommand: (id, handler) => {
      vscodeStub.__commands = vscodeStub.__commands ?? {};
      vscodeStub.__commands[id] = handler;
      return makeDisposable();
    },
    executeCommand: async (id, ...args) => {
      // `setContext` is how the extension tells VS Code whether Tab belongs to
      // the rewriter; capture the values so tests can assert the `when` clause.
      if (id === "setContext") {
        vscodeStub.__contexts = vscodeStub.__contexts ?? {};
        vscodeStub.__contexts[args[0]] = args[1];
      }
      return undefined;
    },
  },
  window: {
    activeTextEditor: undefined,
    visibleTextEditors: [],
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
      get: (key, fallback) => (key in configValues ? configValues[key] : fallback),
      update: async () => undefined,
    }),
    onDidChangeConfiguration: (listener) => listeners.configuration.event(listener),
    onDidChangeTextDocument: (listener) => listeners.textDocument.event(listener),
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
// Text coordinates: everything below is UTF-16 code units, like VS Code.
function offsetOf(text, position) {
  const lines = text.split("\n");
  let offset = 0;
  for (let i = 0; i < position.line && i < lines.length; i++) offset += lines[i].length + 1;
  return offset + position.character;
}

function fakeDocument(fsPath, languageId = "sokonanoda", text = "") {
  const uri = vscodeStub.Uri.file(fsPath);
  const document = {
    uri,
    languageId,
    version: 1,
    getText: (range) =>
      range === undefined
        ? text
        : text.slice(offsetOf(text, range.start), offsetOf(text, range.end)),
    lineAt: (line) => {
      const value = text.split("\n")[line] ?? "";
      return {
        text: value,
        lineNumber: line,
        range: { start: { line, character: 0 }, end: { line, character: value.length } },
      };
    },
    get lineCount() {
      return text.split("\n").length;
    },
    offsetAt: (position) => offsetOf(text, position),
    positionAt: (offset) => {
      const before = text.slice(0, offset).split("\n");
      return { line: before.length - 1, character: before[before.length - 1].length };
    },
    // One entry per `editor.edit()` call — the real host's undo unit.
    __undoStack: [],
    __setText: (next) => {
      text = next;
      document.version += 1;
    },
    selection: { active: { line: 0, character: 0 } },
  };
  return document;
}

// A fake editor with a faithful `edit()`: the builder collects replacements,
// they are applied back-to-front, and **the whole call is one undo entry**.
// That is exactly the property the rewriter relies on ("one undo takes the
// replacement back in one step"), so the test asserts on this stack rather
// than on a re-implementation of the rewriter.
function fakeEditor(document, positions, { anchor } = {}) {
  const points = positions ?? [{ line: 0, character: 0 }];
  const selections = points.map((active) => ({
    active,
    anchor: anchor ?? active,
    isEmpty: anchor === undefined || (anchor.line === active.line && anchor.character === active.character),
  }));
  const editor = {
    document,
    selections,
    selection: selections[0],
    editCalls: 0,
    async edit(callback) {
      editor.editCalls += 1;
      const edits = [];
      callback({
        replace: (range, newText) => edits.push({ range, newText }),
        insert: (position, newText) =>
          edits.push({ range: { start: position, end: position, isEmpty: true }, newText }),
        delete: (range) => edits.push({ range, newText: "" }),
      });
      const before = document.getText();
      const withOffsets = edits
        .map((edit) => ({
          start: offsetOf(before, edit.range.start),
          end: offsetOf(before, edit.range.end),
          newText: edit.newText,
        }))
        .sort((a, b) => b.start - a.start);
      let next = before;
      for (const edit of withOffsets) {
        next = next.slice(0, edit.start) + edit.newText + next.slice(edit.end);
      }
      document.__undoStack.push({ before });
      document.__setText(next);
      return true;
    },
    revealRange() {},
  };
  return editor;
}

// The real host sets `window.activeTextEditor` *before* firing the event.
function editorFor(document) {
  return fakeEditor(document);
}
function focus(document, positions, options) {
  const editor = fakeEditor(document, positions, options);
  vscodeStub.window.activeTextEditor = editor;
  vscodeStub.window.visibleTextEditors = [editor];
  listeners.activeEditor.fire(editor);
  return editor;
}

// 撤销一次 = 弹一条 `editor.edit` 记录（stub 的粒度与真宿主一致）。
function undo(editor) {
  const entry = editor.document.__undoStack.pop();
  if (!entry) return false;
  editor.document.__setText(entry.before);
  return true;
}

// 模拟"用户刚敲进去一段文本"：先改文档，再发 `onDidChangeTextDocument`。
//
// ⚠️ 坐标口径按真宿主来（**不是**我们方便的口径）：VS Code 的
// `TextDocumentContentChangeEvent.range` 是"被替换掉的范围"（**旧文档坐标**），
// 纯插入时它是插入点，`range.end` 在插入的文本**之前**。取证（VS Code 1.138.0
// 自带源码）：`TextModel._doApplyEdits` 产出的 change 是 `{range: 旧范围,
// text: 新文本}`，经 `ApplyEditsResult(reverseEdits, changes, …)` 的第二个字段
// 上报（`extensionHostProcess.js` 的
// `OS=class{constructor(t,e,n){this.reverseEdits=t;this.changes=e;…}}`）。
// stub 要是写成"新坐标"，eager 路径在真宿主里就会错位而测试全绿。
function typeText(document, position, text) {
  const start = offsetOf(document.getText(), position);
  const before = document.getText();
  document.__setText(before.slice(0, start) + text + before.slice(start));
  listeners.textDocument.fire({
    document,
    contentChanges: [
      {
        range: new vscodeStub.Range(position.line, position.character, position.line, position.character),
        rangeOffset: start,
        rangeLength: 0,
        text,
      },
    ],
  });
}

function resetListeners() {
  for (const key of Object.keys(listeners)) listeners[key] = emitter();
}

async function activateExtension(config = {}) {
  requests.length = 0;
  spawns.length = 0;
  timers = [];
  // 每个测试重新激活一次：监听器必须重新挂，否则上一个测试的监听器还在
  //（一次事件会被处理两遍——这正是我们要测的那类放大问题）。
  resetListeners();
  resetConfig();
  // `config` 在 `resetConfig()` **之后**、`activate()` **之前**生效：有些设置
  // 只在激活时读一次（T-A52 的 `warmCacheOnOpen` 就是），测试没法事后开。
  for (const [key, value] of Object.entries(config)) setConfig(key, value);
  vscodeStub.window.activeTextEditor = undefined;
  vscodeStub.window.visibleTextEditors = [];
  vscodeStub.__commands = {};
  vscodeStub.__contexts = {};
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

// 排空已排队的微任务。切活动文档现在会**主动**取一次 `soko/goals`（声明卡片
// 不能等树可见/等诊断），所以「先 focus、再清 `requests`」的测试必须先让那一次
// 落定，否则它会被算进后面的断言里（或与后面的请求合并掉）。
async function settle() {
  for (let i = 0; i < 40; i++) await Promise.resolve();
}
function stateRequests() {
  return requests.filter((request) => request.method === "soko/stateAt");
}

const tests = [];
function test(name, fn) {
  tests.push({ name, fn });
}

// ── tests ────────────────────────────────────────────────────────────────

test("warmCacheOnOpen off by default: activation spawns no build", async () => {
  // T-A52（用户拍板：**做，但默认关**）。默认关这条最要紧——它占 CPU/IO，而
  // "打开编辑器"本身会因此变慢，那正是另一面的抱怨。
  await activateExtension();
  await settle();
  const builds = spawns.filter((s) => Array.isArray(s.args) && s.args[0] === "build");
  assert.deepStrictEqual(builds, [], `默认关时不许起 build：${JSON.stringify(spawns)}`);
});

test("warmCacheOnOpen on: activation builds the workspace root once", async () => {
  const context = await activateExtension({ warmCacheOnOpen: true });
  await settle();
  const builds = spawns.filter((s) => Array.isArray(s.args) && s.args[0] === "build");
  assert.strictEqual(builds.length, 1, `开时正好起一次 build：${JSON.stringify(spawns)}`);
  // 目标是**工作区根**（不是某个文件）：激活时还没有活动编辑器。
  assert.deepStrictEqual(
    builds[0].args,
    ["build", "--json", "/repo"],
    "build 的目标必须是工作区根",
  );
  assert.ok(context, "activate 必须返回 context");
});

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
  await settle();
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
  await settle();
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
  await settle();

  // Answer the next soko/goals slowly so the test can switch documents first.
  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  // **按 URI 应答**（真实服务端就是这样）：A 慢且回旧声明，别的文档立刻答空。
  // 不区分 URI 的话，B 自己的那一趟（T-B09 起它会真的发出去）也会拿到 stale_decl，
  // 那测的就不是"过期答案被丢弃"了。
  stubbedResponses["soko/goals"] = async (params) => {
    if (params?.textDocument?.uri !== String(canvas.uri)) return { decls: [] };
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

test("switching documents mid-flight fetches the new document too", async () => {
  // E4（计划 T-B08）：A 的请求在飞时切到 B —— 过期答案要丢，**但 B 的取数必须
  // 补上**。改前是 `if (requestedUri !== this.uri) return;`：什么都不做，
  // 面板一直停在上一份文档的卡片上（而 A 是慢编译的项目文件时必中）。
  await activateExtension();
  const provider = vscodeStub.__infoview;
  assert.ok(provider, "the Infoview provider must be registered");
  const posts = [];
  provider._view = { webview: { postMessage: (message) => posts.push(message) } };
  provider._ready = true;

  const a = fakeDocument("/repo/course/unit11-project/Canvas.sokonanoda");
  focus(a);
  await settle();

  // A 的请求卡住不返回。
  let releaseA;
  const gateA = new Promise((resolve) => {
    releaseA = resolve;
  });
  stubbedResponses["soko/goals"] = async (params) => {
    if (params?.textDocument?.uri !== String(a.uri)) return { decls: [] };
    await gateA;
    return {
      decls: [
        { name: "stale_decl", kind: "theorem", status: "open", holes: [{ id: "h0", range: {} }] },
      ],
    };
  };
  requests.length = 0;
  listeners.diagnostics.fire({ uris: [a.uri] });
  fireTimers();
  await Promise.resolve();
  assert.strictEqual(goalsRequests().length, 1, "A 的取数在飞");

  // 切到 B。**先换好 stub 再切**：`trackEditor(B)` 会立刻发 B 的那一趟
  // （T-B09 起它不再被 A 的在飞 promise 吃掉），stub 换晚了 B 拿到的就是旧答案。
  const b = fakeDocument("/repo/course/unit11-project/Logic.sokonanoda");
  stubbedResponses["soko/goals"] = (params) =>
    params?.textDocument?.uri === String(b.uri)
      ? { decls: [{ name: "b_decl", kind: "theorem", status: "checked", holes: [] }] }
      : { decls: [] };
  focus(b);
  releaseA();
  for (let i = 0; i < 60; i++) await Promise.resolve();

  const posted = posts.filter((message) => message.type === "decls");
  const names = posted.flatMap((message) => (message.decls ?? []).map((decl) => decl.name));
  assert.ok(
    names.includes("b_decl"),
    `切到 B 之后必须补取 B 的声明并推给 Infoview，实际推过 = ${JSON.stringify(names)}`,
  );
  assert.ok(
    !names.includes("stale_decl"),
    `A 的过期答案不许推给 Infoview，实际推过 = ${JSON.stringify(names)}`,
  );
});

test("an empty declaration list is not treated as already fetched", async () => {
  // E6（计划 T-B10）：**"取过了"不能看 `declItems` 的真值**——服务器答"没有声明"
  // 是合法的（空壳模块），那时 `declItems = []`，而 `![]` 是 **false** ⇒ 之后
  // 每一次 `ensureDeclarations()`（含 `rootChildren` 里那份同样的判断）全空转，
  // 面板永远停在「等待编译…」。
  //
  // 这里直接编码契约（不依赖激活时序，那个在 stub 宿主里会被诊断事件掩盖）：
  // **上一次取数失败**（`_declsUri` 未记）之后再问，必须真的再发一次请求。
  await activateExtension();
  const tree = vscodeStub.__trees?.["sokonanoda.goals"];
  assert.ok(tree, "the exercise tree must be registered");
  focus(fakeDocument("/repo/playground.sokonanoda"));
  await settle();

  // 造出"上一次失败"的现场：结果为空，但**没有**记下"为这个 URI 取过"。
  tree.declItems = [];
  tree._declsUri = undefined;
  requests.length = 0;
  stubbedResponses["soko/goals"] = () => ({
    decls: [{ name: "recovered", kind: "theorem", status: "checked", holes: [] }],
  });
  await tree.ensureDeclarations();

  assert.strictEqual(
    goalsRequests().length,
    1,
    "`declItems = []` 不能让 `ensureDeclarations()` 永久空转（E6）",
  );
  assert.ok(
    (tree.declItems ?? []).some((item) => String(item.label) === "recovered"),
    "补取之后树必须拿到声明",
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

test("a declaration whose type changed is reposted even if nothing else did", async () => {
  // E8（计划 T-B11）：指纹只含 `name/kind/status/holes[0].id` 时，"只改了类型"
  // 会被判成"没变" ⇒ 卡片上的类型行与 `L<n>` 停在旧值（树已经重建过了，
  // 面板却还是旧的——最难发现的那一类不一致）。
  const context = await activateExtension();
  const provider = vscodeStub.__infoview;
  assert.ok(provider, "the Infoview provider must be registered");
  const posts = [];
  provider._view = { webview: { postMessage: (message) => posts.push(message) } };
  provider._ready = true;
  const declsPosted = () => posts.filter((message) => message.type === "decls").length;

  const base = {
    name: "and_swap",
    kind: "theorem",
    status: "checked",
    holes: [],
    ty: "And a b -> And b a",
    range: { start: { line: 3 } },
  };
  provider.setDecls([base]);
  const afterFirst = declsPosted();

  // 只换类型（名字/种类/状态/洞 id 全不变）。
  provider.setDecls([{ ...base, ty: "And b a -> And a b" }]);
  assert.strictEqual(
    declsPosted(),
    afterFirst + 1,
    "只改类型也必须重发（E8）",
  );

  // 只换行号（类型文本不变）。
  provider.setDecls([{ ...base, ty: "And b a -> And a b", range: { start: { line: 9 } } }]);
  assert.strictEqual(
    declsPosted(),
    afterFirst + 2,
    "只改行号也必须重发（卡片上的 `L<n>` 跟着 span 走）",
  );

  // 完全一样的一份（新对象、内容相同）仍然不许重发。
  provider.setDecls([{ ...base, ty: "And b a -> And a b", range: { start: { line: 9 } } }]);
  assert.strictEqual(
    declsPosted(),
    afterFirst + 2,
    "内容没变就别重发（整表重建 50 条 ≈ 1200 个 DOM 节点）",
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

test("focusing another document refreshes the Infoview cards without a diagnostics event", async () => {
  await activateExtension();
  const provider = vscodeStub.__infoview;
  assert.ok(provider, "the Infoview provider must be registered");
  const posts = [];
  provider._view = { webview: { postMessage: (message) => posts.push(message) } };
  provider._ready = true;

  // 第一份：单文件，焦点进入即拿到它的声明卡片。
  const single = fakeDocument("/repo/playground.sokonanoda");
  stubbedResponses["soko/goals"] = () => ({
    decls: [{ name: "decl_single", kind: "theorem", status: "checked", holes: [] }],
  });
  focus(single);
  for (let i = 0; i < 40; i++) await Promise.resolve();
  const afterSingle = posts.filter((message) => message.type === "decls");
  assert.ok(
    afterSingle.at(-1)?.decls?.some((decl) => decl.name === "decl_single"),
    `focusing a single file must push its cards: ${JSON.stringify(afterSingle)}`,
  );

  // 第二份：**项目模块**，而且**不发诊断**（项目模式下打开模块的常见情形）。
  // 这一条钉住的是：`refresh()` 只重建树，而树只有在可见时才走 `getChildren`
  // ——Infoview 的声明卡片必须由 `trackEditor` 主动推。
  const projectFile = fakeDocument("/repo/course/unit11-project/Exercises.sokonanoda");
  stubbedResponses["soko/goals"] = () => ({
    decls: [
      { name: "decl_project", kind: "theorem", status: "open", holes: [{ id: "h0", range: {} }] },
    ],
  });
  focus(projectFile);
  for (let i = 0; i < 60; i++) await Promise.resolve();

  const declsPosts = posts.filter((message) => message.type === "decls");
  assert.ok(
    declsPosts.at(-1)?.decls?.some((decl) => decl.name === "decl_project"),
    `a focus switch alone must push the new document's cards: ${JSON.stringify(declsPosts.at(-1))}`,
  );
  assert.ok(
    goalsRequests().length >= 2,
    "switching focus must ask the server for the new document's declarations",
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

// ── notation input (NI-2, docs/design/notation-input.md) ─────────────────
// The table itself is pinned against `front::notation_input` by the Rust
// contract test; here we drive the **real** registered command and the real
// change listener, so the state machine (prefix trap / lone `\` / multi-cursor
// / one-undo-unit) is exercised, not re-implemented.

const REPLACE_COMMAND = "sokonanoda.input.replaceAbbreviation";
const CONTEXT_KEY = "sokonanoda.input.abbreviationBeforeCursor";
const cursor = (line, character) => ({ line, character });

function commandHandler(id) {
  const handler = vscodeStub.__commands?.[id];
  assert.ok(handler, `${id} must be registered`);
  return handler;
}

async function drain() {
  for (let i = 0; i < 10; i++) await Promise.resolve();
}

test("Tab rewrites \\and to ∧", async () => {
  await activateExtension();
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\and");
  const editor = focus(document, [cursor(0, 4)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(document.getText(), "∧", "`\\and` + Tab must become `∧`");
  assert.strictEqual(editor.editCalls, 1, "one edit call is what makes it one undo unit");
});

test("Tab rewrites \\in even though \\in prefixes \\inter", async () => {
  // Tab is an explicit command: the prefix trap only guards eager replacement
  // (`\in` is a prefix of `\inter`, but a learner who typed Tab wants `∈`).
  await activateExtension();
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\in");
  focus(document, [cursor(0, 3)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(document.getText(), "∈");
});

test("Tab leaves an incomplete or unknown word alone", async () => {
  await activateExtension();
  // `\an` is a prefix of `\and`, not a table abbreviation: nothing to replace.
  const prefix = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\an");
  focus(prefix, [cursor(0, 3)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(prefix.getText(), "\\an", "an incomplete abbreviation must not be rewritten");
  assert.strictEqual(prefix.__undoStack.length, 0, "no edit may be issued at all");

  // Case-sensitive, like Lean: `\And` is not `\and`.
  const wrongCase = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\And");
  focus(wrongCase, [cursor(0, 4)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(wrongCase.getText(), "\\And", "abbreviations are case-sensitive");
});

test("a lone \\ (set difference) is never rewritten", async () => {
  // R-2: `\` is both the leader and the set-difference symbol. Only `\` +
  // a **complete** table word may be rewritten.
  await activateExtension();
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "A \\ B");
  focus(document, [cursor(0, 3)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(document.getText(), "A \\ B", "the lone backslash must survive untouched");
  assert.strictEqual(document.__undoStack.length, 0);
});

test("one undo takes the whole replacement back", async () => {
  await activateExtension();
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\and");
  const editor = focus(document, [cursor(0, 4)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(document.getText(), "∧");
  assert.strictEqual(document.__undoStack.length, 1, "one edit call = one undo entry");
  undo(editor);
  assert.strictEqual(document.getText(), "\\and", "a single undo must restore the abbreviation");
});

test("multi-cursor rewrites every abbreviation in one edit", async () => {
  await activateExtension();
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\and ∨ \\in");
  const editor = focus(document, [cursor(0, 4), cursor(0, 10)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(document.getText(), "∧ ∨ ∈", "every cursor gets its own replacement");
  assert.strictEqual(editor.editCalls, 1, "all cursors travel in one edit (one undo unit)");
  undo(editor);
  assert.strictEqual(document.getText(), "\\and ∨ \\in");
});

test("a non-empty selection is never rewritten", async () => {
  await activateExtension();
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\and");
  // The whole word is selected: Tab means "indent" there, not "rewrite".
  focus(document, [cursor(0, 4)], { anchor: cursor(0, 0) });
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(document.getText(), "\\and");
  assert.strictEqual(document.__undoStack.length, 0, "a selection must not be corrupted");
});

test("eager replacement is off by default", async () => {
  await activateExtension();
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\an");
  focus(document, [cursor(0, 3)]);
  typeText(document, cursor(0, 3), "d"); // the learner finishes typing `\and`
  await drain();
  assert.strictEqual(document.getText(), "\\and", "the default path is Tab, not eager");
  assert.strictEqual(document.__undoStack.length, 0);
});

test("eager mode rewrites the moment the word is complete", async () => {
  await activateExtension();
  setConfig("input.eager", true);
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\an");
  focus(document, [cursor(0, 3)]);
  typeText(document, cursor(0, 3), "d");
  await drain();
  assert.strictEqual(document.getText(), "∧", "eager mode replaces on word completion");
  assert.strictEqual(document.__undoStack.length, 1);
});

test("eager mode waits out the prefix trap", async () => {
  await activateExtension();
  setConfig("input.eager", true);
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\i");
  focus(document, [cursor(0, 2)]);
  // `\i` → `\in` → `\int` → `\inte` → `\inter`: only the last one is complete.
  const steps = [
    [cursor(0, 2), "n", "\\in"],
    [cursor(0, 3), "t", "\\int"],
    [cursor(0, 4), "e", "\\inte"],
    [cursor(0, 5), "r", "∩"],
  ];
  for (const [position, character, expected] of steps) {
    typeText(document, position, character);
    await drain();
    assert.strictEqual(
      document.getText(),
      expected,
      `after typing \`${character}\` the text must be \`${expected}\``,
    );
  }
  assert.strictEqual(document.__undoStack.length, 1, "only the complete word was replaced");
});

test("eager mode closes the word on a separator", async () => {
  // 前缀只在"还在敲字母"时挡：`\in` 是 `\inter` 的前缀 ⇒ 敲 `n` 时不落定；
  // 但空格一到，这个词就封口了，前缀不再是理由（与 Lean 同规则）。
  await activateExtension();
  setConfig("input.eager", true);
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\i");
  focus(document, [cursor(0, 2)]);
  typeText(document, cursor(0, 2), "n");
  await drain();
  assert.strictEqual(document.getText(), "\\in", "a prefix must not be rewritten while typing");
  typeText(document, cursor(0, 3), " ");
  await drain();
  assert.strictEqual(document.getText(), "∈ ", "the separator closes the word: `\\in ` → `∈ `");
  assert.strictEqual(document.__undoStack.length, 1);
});

test("eager mode never fires on a deletion or an undo", async () => {
  await activateExtension();
  setConfig("input.eager", true);
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\and");
  focus(document, [cursor(0, 4)]);
  // A deletion (and an undo, which is a replacement) must not re-trigger the
  // state machine — otherwise undo would immediately re-apply the symbol.
  listeners.textDocument.fire({
    document,
    contentChanges: [
      { range: new vscodeStub.Range(0, 3, 0, 4), rangeOffset: 3, rangeLength: 1, text: "" },
    ],
  });
  await drain();
  assert.strictEqual(document.getText(), "\\and");
  assert.strictEqual(document.__undoStack.length, 0);
});

test("the Tab context key follows the word before the cursor", async () => {
  await activateExtension();
  const contexts = () => vscodeStub.__contexts ?? {};

  const plain = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "def x := 1");
  focus(plain, [cursor(0, 3)]);
  await drain();
  assert.strictEqual(contexts()[CONTEXT_KEY], false, "a plain identifier leaves Tab to VS Code");

  const abbreviation = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\an");
  focus(abbreviation, [cursor(0, 3)]);
  await drain();
  assert.strictEqual(contexts()[CONTEXT_KEY], true, "a `\\`-word hands Tab to the rewriter");

  const lone = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "A \\ B");
  focus(lone, [cursor(0, 3)]);
  await drain();
  assert.strictEqual(contexts()[CONTEXT_KEY], false, "a lone `\\` keeps Tab out of it");
});

test("the rewriter stays out of other languages", async () => {
  await activateExtension();
  const document = fakeDocument("/repo/notes.txt", "plaintext", "\\and");
  focus(document, [cursor(0, 4)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(document.getText(), "\\and", "only .sokonanoda documents are rewritten");

  setConfig("input.eager", true);
  typeText(document, cursor(0, 4), "x");
  await drain();
  assert.strictEqual(document.getText(), "\\andx", "eager mode must not touch other languages");
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
