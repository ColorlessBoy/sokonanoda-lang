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
  // 真 API 的取值（`vscode.OverviewRulerLane`）：Left=1 / Center=2 / Right=4 / Full=7 ✓。
  OverviewRulerLane: { Left: 1, Center: 2, Right: 4, Full: 7 },
  // E27 的跳转走 `editor.revealRange(range, TextEditorRevealType.InCenter)` ⇒
  // stub 缺这个枚举就会 TypeError（真 API 的取值：Default=0 / InCenter=1 /
  // InCenterIfOutsideViewport=2 / AtTop=3）✓。
  TextEditorRevealType: { Default: 0, InCenter: 1, InCenterIfOutsideViewport: 2, AtTop: 3 },
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
  // 真 API 的取值（VS Code `ProgressLocation`：SourceControl=1 / Window=10 /
  // Notification=15）✓ —— E23 的 `withProgress` 要读它，stub 缺了就会 TypeError。
  ProgressLocation: { SourceControl: 1, Window: 10, Notification: 15 },
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
  // E27：跳转落点用 `editor.selection = new vscode.Selection(start, end)` ⇒
  // stub 缺它就会 TypeError（真 API 里 Selection 是 Range 的子类 ✓）。
  Selection: class Selection {
    constructor(start, end) {
      this.start = start;
      this.end = end ?? start;
      this.isEmpty =
        this.start.line === this.end.line && this.start.character === this.end.character;
    }
    get active() {
      return this.end;
    }
    get anchor() {
      return this.start;
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
      // **E27**：记下每一次 `executeCommand` —— 判据要区分
      // `vscode.executeDefinitionProvider`（= 编辑器 F12 的那条命令 ✓）
      // 与 `sokonanoda.revealRange`（只是"滚到源码 span" ✗）。
      vscodeStub.__commandsCalled = vscodeStub.__commandsCalled ?? [];
      vscodeStub.__commandsCalled.push({ id, args });
      if (id === "vscode.executeDefinitionProvider") {
        return vscodeStub.__definitions ?? [];
      }
      return undefined;
    },
  },
  window: {
    activeTextEditor: undefined,
    visibleTextEditors: [],
    // **P4**（空间进度）：装饰类型与 `editor.setDecorations` 的 stub —— 真宿主里
    // 前者是 `createTextEditorDecorationType`，后者挂在 editor 上 ✓。
    createTextEditorDecorationType: (options) => {
      vscodeStub.__decorationOptions = options;
      return { key: "sokonanoda-compiling", dispose() {} };
    },
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
      const item = { show() {}, hide() {}, dispose() {}, tooltip: "" };
      // **E23**：状态栏文案要能**逐帧**看 —— 「3/13 文件 · …」是**中间态**，
      // 只在跑完看一眼是抓不到的 ✗（而那正是"有没有进度"的全部证据）。
      vscodeStub.__statusBarHistory = [];
      let text = "";
      Object.defineProperty(item, "text", {
        get: () => text,
        set: (value) => {
          text = value;
          vscodeStub.__statusBarHistory.push(value);
        },
      });
      vscodeStub.__statusBar = item;
      return item;
    },
    createOutputChannel: () => ({ appendLine() {}, append() {}, show() {}, dispose() {} }),
    registerWebviewViewProvider: (_id, provider) => {
      vscodeStub.__infoview = provider;
      return makeDisposable();
    },
    // **P0 判据要用**：记录弹出的信息提示 ⇒ 断言"点声明名**不弹提示**" ✓
    //（原来这里是个丢弃返回值的空实现 ⇒ 弹了什么**测不到** ✗）。
    showInformationMessage: async (msg) => {
      (vscodeStub.__messages ??= []).push(String(msg));
      return undefined;
    },
    showWarningMessage: async (msg) => {
      (vscodeStub.__messages ??= []).push(String(msg));
      return undefined;
    },
    // **Q2**：错误提示也要**可观测** —— 原来这里是丢弃返回值的空实现
    // ⇒ "不一致时有没有报错"**测不到** ✗（与上面 `showInformationMessage`
    // 那条注释同一个理由：判据要断言"屏幕上弹了什么"）。
    showErrorMessage: async (msg) => {
      (vscodeStub.__messages ??= []).push(String(msg));
      return undefined;
    },
    showTextDocument: async () => undefined,
    registerUriHandler: () => makeDisposable(),
    // **E23**：`withProgress` 不再是空壳 —— 它记下 options、把每次 `report`
    // 收进 `__progress.reports`（判据要断言**中间态发生过**：至少一次
    // `increment > 0`，只断言首尾 = 等于没进度 ✗），并给出**取消令牌**
    // （`runBuild` 的取消要一路传到 `child.kill()`）。
    withProgress: async (options, task) => {
      vscodeStub.__progress = { options, reports: [] };
      const token = {
        isCancellationRequested: false,
        onCancellationRequested: (handler) => {
          vscodeStub.__progress.cancelHandler = handler;
          return makeDisposable();
        },
      };
      return task(
        { report: (value) => vscodeStub.__progress.reports.push(value) },
        token,
      );
    },
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
    // **G-53**：`definition` 的 **offset 变体**要靠 `document.positionAt(offset)` 换算 ✓
    // ⇒ 桩必须返回**带 `positionAt`** 的真 `fakeDocument` ✓（旧的固定假对象没有它 ✗
    // ⇒ 那条路在桩里会静默失败 ✗）。文本用 `__docText`（默认空 ✓）。
    openTextDocument: async (uri) => {
      const fsPath = uri && typeof uri.fsPath === "string" ? uri.fsPath : "/x";
      return fakeDocument(fsPath, "sokonanoda", vscodeStub.__docText ?? "");
    },
    asRelativePath: (value) => String(value),
    findFiles: async () => [],
    textDocuments: [],
  },
  extensions: { getExtension: () => undefined },
};

// ── fake language client ─────────────────────────────────────────────────
const requests = [];
const stateEmitters = [];
/// 扩展注册的通知处理器（按方法名）——测试用 `notify(method, params)` 驱动 ✓。
const notificationHandlers = {};
class LanguageClient {
  constructor() {
    this.outputChannel = { appendLine() {}, append() {}, show() {}, dispose() {} };
    stateEmitters.push(this);
  }
  onDidChangeState(listener) {
    this._stateListener = listener;
    return makeDisposable();
  }
  // **P1→P2/P3 的接缝**（2026-09-26）：扩展现在**裸读** `$/progress` 自己渲染
  // ⇒ 假客户端必须也提供 `onNotification`，否则 `activate()` 直接 TypeError、
  // 34 条宿主测试全红 ✗（S2 调研的 T4）。
  onNotification(type, handler) {
    notificationHandlers[type] = handler;
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
// **E23**：`build --json` 的事件流（默认空 ⇒ 老用例行为不变 ✓）。
let buildEvents = [];
function setBuildEvents(events) {
  buildEvents = events || [];
}
/// `child.kill()` 的调用记录（E23 的取消判据要它）。
const kills = [];
function fakeSpawn(command, args) {
  spawns.push({ command, args });
  const child = new EventEmitter();
  const stream = () =>
    Object.assign(new EventEmitter(), { setEncoding: () => {}, resume: () => {} });
  child.stdout = stream();
  child.stderr = stream();
  // **E23**：取消要能验 —— 记下 `child.kill()` 真的被叫过（不是只断言
  // "注册了回调"✗：回调里不 kill 的话，用户点取消什么也不会发生）。
  child.kill = () => {
    kills.push({ command, args });
  };
  process.nextTick(() => {
    if (Array.isArray(args) && args[0] === "course" && courseEvents.length) {
      child.stdout.emit("data", courseEvents.map((event) => JSON.stringify(event)).join("\n") + "\n");
    }
    if (Array.isArray(args) && args[0] === "build" && buildEvents.length) {
      // **E23**：像真子进程那样**按行**吐 `build.begin` / `build.file` /
      // `build.summary`（`--json` 事件流）—— 扩展是流式消费的，一次全给
      // 也能逐行处理，但"中间态"必须真的被记下来（状态栏历史 ✓）。
      child.stdout.emit("data", buildEvents.map((event) => JSON.stringify(event)).join("\n") + "\n");
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
  // **Q2**：stub 的"装自带 CLI"。默认**对齐**（与插件同版本）—— 不一致那一档
  // 由专门用例覆写 `__cliVersion` 来造 ✓。
  installBundledCli: () => {
    if (vscodeStub.__noBundledCli) return { source: undefined, aligned: false };
    return {
      dest: "/stub/bin/sokonanoda",
      source: "/stub/extension/bin/darwin-arm64/sokonanoda",
      version: "0.58.0",
      aligned: false,
    };
  },
};
const originalLoad = Module._load;
Module._load = function patched(request, parent, isMain) {
  if (request === "vscode") return vscodeStub;
  if (request === "vscode-languageclient/node") return vscodeLanguageclientStub;
  if (request === "child_process") {
    return {
      spawn: fakeSpawn,
      // **Q2**：`installCli` 靠它跑 `--version`。回调形态
      //（`(cmd, args, cb)`）—— stub 输出由 `__cliVersion` 控制 ✓。
      execFile: (cmd, args, cb) => {
        const out = `sokonanoda ${vscodeStub.__cliVersion ?? "0.58.0"}\n`;
        if (typeof cb === "function") cb(null, out, "");
        return { on: () => {} };
      },
      execSync: () => "",
    };
  }
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
  // 见上：P4 的装饰记录（`[{type, ranges}]`，便于断言"成对"✓）。
  const decorationCalls = [];
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
    decorationCalls,
    setDecorations(type, ranges) {
      decorationCalls.push({ type, ranges });
    },
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
  kills.length = 0;
  buildEvents = [];
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

test("build/rebuild target the project root, not the active file (E22)", async () => {
  // **E22（用户 I2①：「点哪个文件，编译哪个文件」）**。扩展以前把**活动文件**
  // 当目标（`buildTarget()`：活动 `.sokonanoda` 文件 → 工作区根），而 CLI 拿到
  // 文件就只编那一个 ⇒ 用户点 Build 看到的永远是"1 个文件"。
  //
  // 现在目标是**项目**：模块根取自服务端 `soko/project` 的 `root`（客户端**不自己
  // 找清单** —— 那是第二份真相 ✗），服务端还没答时退回工作区根（**也绝不是文件**）。
  //
  // 这条判据钉的是**扩展真的让 CLI 编了什么**（`spawns` 的 argv），比"通知里有
  // 几个文件"更靠上游，且不需要真宿主。夹具故意让**三者互不相同**：
  // 活动文件 `/repo/courses/set-theory/units/u01.sokonanoda`、工作区根 `/repo`、
  // 模块根 `/repo/courses/set-theory` ⇒ 断言能分辨"取自哪一份"。
  const root = "/repo/courses/set-theory";
  stubbedResponses["soko/project"] = () => ({
    uri: "file:///repo/courses/set-theory/units/u01.sokonanoda",
    version: 1,
    project: {
      entry: "units.u01",
      root,
      manifest: `${root}/sokonanoda.toml`,
      requires_warning: null,
      modules: [],
      diagnostics: [],
      counts: { modules: 1, compiled: 0, failed: 0, open: 0 },
    },
    reason: null,
  });
  try {
    await activateExtension();
    focus(fakeDocument("/repo/courses/set-theory/units/u01.sokonanoda"));
    await settle();
    spawns.length = 0;

    await vscodeStub.__commands["sokonanoda.build"]();
    assert.deepStrictEqual(
      spawns.map((s) => s.args),
      [["build", "--json", root]],
      "Build 必须编**项目**（服务端给的模块根），不是活动文件",
    );

    // rebuild = 先清**这个项目**的缓存，再编项目。
    // ⚠ 清缓存那一步以前**不带目标** ⇒ CLI 只清全局、项目条目留在原地 ⇒
    // 紧接着的 build 全是 `hit` ⇒「Rebuild」其实什么都没重编（实测
    // `build --json --clean` 给 `{"global":0,"project":0,"removed":0}`）。
    spawns.length = 0;
    await vscodeStub.__commands["sokonanoda.rebuild"]();
    assert.deepStrictEqual(
      spawns.map((s) => s.args),
      [
        ["build", "--json", "--clean", root],
        ["build", "--json", root],
      ],
      "Rebuild 必须先清**项目**的缓存（带目标）再编项目",
    );
  } finally {
    // 假客户端是模块级共享的：还原默认答案，别污染后面的测试。
    stubbedResponses["soko/project"] = () => ({
      uri: "",
      version: 1,
      project: null,
      reason: "no-imports",
    });
  }
});

test("build streams per-file progress to the status bar and the Infoview (E23)", async () => {
  // **E23（用户 I2②：「我要求有进度条，**现在是没有进度**」）**。
  //
  // 修前的形态：`runBuildProcess` 把子进程 stdout **攒到最后**才 resolve，
  // 而 `parseBuildEvents` 只用了 `build.clean` / `build.summary` ⇒ 用户点
  // Build 看到的是"弹一个面板 + 滚 JSON + 结束才弹通知" ✗ —— **没有状态栏、
  // 没有 Infoview 进度区、没有概览尺**。
  //
  // 这条判据钉**三处同时、数字同一份**（设计 `docs/protocol.md` 的 `$/progress` + `REQUIREMENTS.md` §9.5）：
  //   ① 状态栏**逐帧**（`__statusBarHistory`：`0/3` → `1/3` → `2/3` → `3/3`）；
  //   ② Infoview 的进度载荷（真 provider 的 `setProgress`，逐帧记下来）；
  //   ③ 原生进度条的 `report` —— **必须有 `increment > 0` 的那一次**（只断言
  //      首尾 = 等于没进度 ✗，这是设计里点名的假绿形态）。
  // 外加取消：`withProgress` 必须 `cancellable`，且令牌真的接到 `child.kill()`。
  // ⚠ 输出面板**不是**进度：这里一条都不拿它当证据 ✗。
  await activateExtension();
  focus(fakeDocument("/repo/units/u01.sokonanoda"));
  await settle();
  const provider = vscodeStub.__infoview;
  assert.ok(provider, "activate() 必须建 Infoview provider");
  const frames = [];
  const original = provider.setProgress.bind(provider);
  provider.setProgress = (progress) => {
    frames.push(progress);
    return original(progress);
  };
  setBuildEvents([
    { type: "build.begin", files: 3 },
    { type: "build.file", file: "/repo/lib/Set.sokonanoda", status: "compiled" },
    { type: "build.file", file: "/repo/units/u01.sokonanoda", status: "hit" },
    { type: "build.file", file: "/repo/units/u02.sokonanoda", status: "hit" },
    { type: "build.summary", files: 3, hit: 2, compiled: 1, failed: 0 },
  ]);
  vscodeStub.__statusBarHistory = [];
  spawns.length = 0;
  let summary;
  try {
    summary = await vscodeStub.__commands["sokonanoda.build"]();
  } finally {
    provider.setProgress = original;
  }

  // ① 状态栏：**每一帧都在**（顺序即进度顺序）。只看 `$(sync~spin)` 那几帧 ——
  // 收工后状态栏会退回"本文件 N"态（那也含"文件"两个字，别混进来 ✗）。
  const statusFrames = vscodeStub.__statusBarHistory.filter((line) =>
    String(line).startsWith("$(sync~spin)"),
  );
  assert.deepStrictEqual(
    statusFrames,
    [
      "$(sync~spin) Sokonanoda: 0/3 文件",
      "$(sync~spin) Sokonanoda: 1/3 文件 · Set.sokonanoda ✓",
      "$(sync~spin) Sokonanoda: 2/3 文件 · u01.sokonanoda ✓",
      "$(sync~spin) Sokonanoda: 3/3 文件 · u02.sokonanoda ✓",
    ],
    "状态栏必须逐文件更新（这些就是「中间态」）",
  );

  // ② Infoview：同一份数字、同样的顺序，且 `end` 收干净。
  const infoviewFrames = frames.filter((f) => f && f.phase !== "end");
  assert.deepStrictEqual(
    infoviewFrames.map((f) => f.label),
    [
      "编译项目（3 个文件）",
      "1/3 · Set.sokonanoda",
      "2/3 · u01.sokonanoda",
      "3/3 · u02.sokonanoda",
    ],
    "Infoview 进度区必须与状态栏同一份进度",
  );
  assert.deepStrictEqual(
    infoviewFrames.map((f) => f.percent),
    [0, 33, 67, 100],
    "百分比必须随推进递增（不是假的常量）",
  );
  assert.strictEqual(
    frames[frames.length - 1].phase,
    "end",
    "结束必须发 end（否则进度块永远留在面板上）",
  );

  // ③ 原生进度条：**至少一次 increment > 0 的 report**（反假绿第 ② 条）。
  const reports = vscodeStub.__progress?.reports ?? [];
  assert.ok(
    reports.some((r) => Number(r.increment) > 0),
    `原生进度必须有中间推进（只报首尾 = 没进度）：${JSON.stringify(reports)}`,
  );
  assert.ok(
    reports.some((r) => String(r.message).includes("2/3")),
    `原生进度的 message 要带逐文件计数：${JSON.stringify(reports)}`,
  );

  // ④ 可取消：令牌已注册，且回调**真的** kill 了子进程。
  assert.strictEqual(
    vscodeStub.__progress.options.cancellable,
    true,
    "build 必须可取消（修前完全不能取消）",
  );
  assert.strictEqual(typeof vscodeStub.__progress.cancelHandler, "function", "取消回调必须注册");
  kills.length = 0;
  vscodeStub.__progress.cancelHandler();
  assert.strictEqual(kills.length, 1, "点取消必须 kill 掉 CLI 子进程");

  assert.ok(
    String(summary).includes("3 个文件"),
    `结束通知仍是 build.summary 的真实计数：${summary}`,
  );
});

test("build progress keeps moving inside a file (P2: decl + tick)", async () => {
  // **P2（用户 2026-09-28）**：最小进度粒度以前是**文件** ⇒ 大文件时 UI 长时间不动
  // 像卡死（实测冷编课程 `unit08-solution` **39s 无输出** ✗）。
  // 判据**绑用户可见结果**：`build.decl`（声明级）与 `build.tick`（心跳）必须
  // **真的改到状态栏与 Infoview 的文本**上 —— 不是"链路通"就完事 ✗。
  await activateExtension();
  focus(fakeDocument("/repo/units/u01.sokonanoda"));
  await settle();
  const provider = vscodeStub.__infoview;
  assert.ok(provider, "activate() 必须建 Infoview provider");
  const frames = [];
  const original = provider.setProgress.bind(provider);
  provider.setProgress = (progress) => {
    frames.push(progress);
    return original(progress);
  };
  setBuildEvents([
    { type: "build.begin", files: 2 },
    { type: "build.decl", file: "/repo/units/big.sokonanoda", module: "big", index: 0, total: 31 },
    { type: "build.decl", file: "/repo/units/big.sokonanoda", module: "big", index: 6, total: 31 },
    { type: "build.tick", file: "/repo/units/big.sokonanoda", elapsed_ms: 12345 },
    { type: "build.file", file: "/repo/units/big.sokonanoda", status: "compiled" },
    { type: "build.summary", files: 2, hit: 1, compiled: 1, failed: 0 },
  ]);
  vscodeStub.__statusBarHistory = [];
  try {
    await vscodeStub.__commands["sokonanoda.build"]();
  } finally {
    provider.setProgress = original;
  }
  const statusFrames = vscodeStub.__statusBarHistory.filter((line) =>
    String(line).startsWith("$(sync~spin)"),
  );
  assert.ok(
    statusFrames.some((l) => String(l).includes("声明 1/31")),
    `声明级进度必须上状态栏：${JSON.stringify(statusFrames)}`,
  );
  assert.ok(
    statusFrames.some((l) => String(l).includes("声明 7/31")),
    `同一条文件内的后续声明也要更新（不是只报第一拍）：${JSON.stringify(statusFrames)}`,
  );
  assert.ok(
    statusFrames.some((l) => String(l).includes("已用 12.3s")),
    `心跳必须把"已用时"显示出来（用户可见面持续在动）：${JSON.stringify(statusFrames)}`,
  );
  const details = frames.filter((f) => f && f.phase === "report").map((f) => f.detail);
  assert.ok(
    details.some((d) => String(d).includes("声明 7/31")) && details.some((d) => String(d).includes("已用 12.3s")),
    `Infoview 同一份数字也要更新：${JSON.stringify(details)}`,
  );
});

test("Install Command Line copies the bundled CLI and checks its version (Q2)", async () => {
  // **Q2（用户 09-29 18:53）**：「**直接装插件自带的 cli，版本还能对齐**」。
  //
  // 判据绑**用户动作 ⇒ 可见结果**：
  //   ① 命令真的注册了（`sokonanoda.installCli`）；
  //   ② 跑一次 ⇒ 弹**信息**提示（成功路径），**且**返回值里写明装到哪；
  //   ③ **版本对齐是"值"断言**：stub 自述与插件**一致** ⇒ ✓；
  //      自述**不一致** ⇒ **弹错误**（不是静默成功 ✗）。
  // ③ 是关键：Q2 之前「版本对齐」是一句**没有守卫的声明**
  //（`extension.rs` 只比 `Cargo.toml` ↔ `package.json`，**没有任何东西跑那个二进制**）。
  await activateExtension({});
  const handler = vscodeStub.__commands["sokonanoda.installCli"];
  assert.ok(handler, "sokonanoda.installCli 必须注册（Q2）");

  // ① 对齐：stub 的 `--version` == 插件版本（context.extension.packageJSON.version）
  vscodeStub.__cliVersion = "0.58.0";
  vscodeStub.__messages = [];
  const ok = await handler();
  assert.match(String(ok), /已安装到/, `报告要写明装到哪：${ok}`);
  assert.match(String(ok), /版本对齐/, `对齐时必须说清"版本对齐"：${ok}`);
  assert.ok(
    (vscodeStub.__messages ?? []).some((m) => /命令行已安装/.test(m)),
    `成功路径必须给可见提示：${JSON.stringify(vscodeStub.__messages)}`,
  );

  // ② 不对齐：自述 0.74.0 而插件 0.58.0 ⇒ **必须报错**（用户现场那个形状）
  vscodeStub.__cliVersion = "0.74.0";
  vscodeStub.__messages = [];
  const bad = await handler();
  assert.match(String(bad), /版本不一致/, `不一致时必须判红：${bad}`);
  assert.match(String(bad), /0\.74\.0/, `要报出实际自述版本：${bad}`);
  assert.ok(
    (vscodeStub.__messages ?? []).length > 0,
    "不一致时必须有可见错误提示（不许静默）",
  );

  // ③ 没有自带二进制：给人话，不假装成功
  vscodeStub.__noBundledCli = true;
  vscodeStub.__messages = [];
  const none = await handler();
  assert.match(String(none), /没有/, `通用包要给人话：${none}`);
  vscodeStub.__noBundledCli = false;
});

test("Clean Cache clears both stores and does not compile anything (E31)", async () => {
  // **E31（用户：「vscode 命令缺一个 clean，清除缓存」）**。
  //
  // CLI 早就有「只清不编」（`build --clean` 清完立刻 return ✓），**扩展里没有入口** ✗：
  // 15 条命令只有 build/rebuild，而 Rebuild 是「clean → build 串成一步」
  // ⇒ 用户没有"清完就停"的办法。
  //
  // 判据（PLAN §E31 的三条 + 反假绿）：
  //   ① spawn **只有一次**、且 argv 是 `build --json --clean <模块根>`
  //      —— **绝不能跟着第二次 build** ✗（那正是 Rebuild 的行为，也是"clean 偷偷
  //      编了一次"这个假动作）；
  //   ② 报告里的**三个数**（removed/global/project）**逐字来自 CLI 的 `build.clean`
  //      事件** —— 前端自己数就错了 ✗（只给总数还会把"只清了全局"藏起来）；
  //   ③ 状态栏先动起来、结束退回（与 E23/E29 一致）。
  const root = "/repo/courses/set-theory";
  stubbedResponses["soko/project"] = () => ({
    uri: "file:///repo/courses/set-theory/units/u01.sokonanoda",
    version: 1,
    project: {
      entry: "units.u01",
      root,
      manifest: `${root}/sokonanoda.toml`,
      requires_warning: null,
      modules: [],
      diagnostics: [],
      counts: { modules: 1, compiled: 3, failed: 0, open: 0 },
    },
    reason: null,
  });
  try {
    await activateExtension();
    focus(fakeDocument("/repo/courses/set-theory/units/u01.sokonanoda"));
    await settle();
    assert.ok(
      Object.keys(vscodeStub.__commands).includes("sokonanoda.clean"),
      "package.json 与 registerCommand 都必须有 sokonanoda.clean",
    );
    // 三个数**故意互不相同**：总数 7 ≠ 全局 5 + 项目 2 之外的样子一眼可辨
    //（谁把 project 写成 0、或前端自己数，断言就会红 ✓）。
    setBuildEvents([{ type: "build.clean", removed: 7, global: 5, project: 2 }]);
    spawns.length = 0;
    vscodeStub.__statusBarHistory = [];
    const text = await vscodeStub.__commands["sokonanoda.clean"]();

    // ① 只清、不编：**一次** spawn，argv 带模块根。
    assert.deepStrictEqual(
      spawns.map((s) => s.args),
      [["build", "--json", "--clean", root]],
      "Clean 只能跑一次 `build --clean <模块根>`（**不许**跟着再 build 一次）",
    );
    // ② 三个数来自 CLI 事件（`（Nms）` 是耗时后缀，允许变化）。
    assert.ok(
      String(text).startsWith("sokonanoda clean：清掉 7 条缓存（全局 5 · 项目 2）（"),
      `报告必须给三个数、且逐字来自 build.clean 事件：${text}`,
    );
    // ③ 状态栏：编译中那几帧里出现过"清除缓存"，且收工后不再有。
    const frames = vscodeStub.__statusBarHistory.filter((line) =>
      String(line).startsWith("$(sync~spin)"),
    );
    assert.deepStrictEqual(
      frames,
      ["$(sync~spin) Sokonanoda: 清除缓存…"],
      `Clean 期间状态栏要有反馈：${JSON.stringify(vscodeStub.__statusBarHistory)}`,
    );
    assert.ok(
      !String(vscodeStub.__statusBar.text).startsWith("$(sync~spin)"),
      "结束必须退回（不能把「清除缓存」永远留在状态栏）",
    );
  } finally {
    stubbedResponses["soko/project"] = () => ({
      uri: "",
      version: 1,
      project: null,
      reason: "no-imports",
    });
  }
});

test("Infoview empty states: loading / ready-empty / error are told apart (E28)", async () => {
  // **E28**：三种空态在 **webview 侧**早就有渲染判据（`test-webview.js` 的
  // `decls empty: the three reasons are told apart (T-B12)`），但**主机侧到底发了
  // 哪个 `status.state`** 从来没人守 ✗ ⇒ 面板完全可能永远停在「编译中…」、
  // 或者把"请求失败"说成"这个文件没有声明"（用户看到的是"插件坏了"✗）。
  //
  // 这条判据钉**主机发出的三态**（真 provider 的 `setStatus`，逐次记下来）：
  //   ① 请求**在飞** ⇒ `loading`（= "服务器还没编完"）；
  //   ② 空数组**成功**返回 ⇒ `ready` + `decls: 0`（= "真的没声明"）—— **必须与 ① 不同**；
  //   ③ 请求**失败** ⇒ `error`（= "读取声明失败"，webview 会提示看输出面板/重启服务器）。
  await activateExtension();
  const provider = vscodeStub.__infoview;
  assert.ok(provider, "activate() 必须建 Infoview provider");
  const states = [];
  const original = provider.setStatus.bind(provider);
  provider.setStatus = (status) => {
    states.push(status);
    return original(status);
  };
  const docA = "/repo/units/u01.sokonanoda";
  const docB = "/repo/units/u02.sokonanoda";
  try {
    // ① 在飞 ⇒ loading。用一个**不 resolve** 的 promise 把请求钉在半空。
    let release;
    stubbedResponses["soko/goals"] = () =>
      new Promise((resolve) => {
        release = () => resolve({ decls: [] });
      });
    focus(fakeDocument(docA));
    await settle();
    assert.strictEqual(
      states[states.length - 1]?.state,
      "loading",
      `取数在飞时必须发 loading（否则"还在编译"会被说成"没有声明"）：${JSON.stringify(states)}`,
    );

    // ② 空数组成功返回 ⇒ ready + 0（**与 loading 是两态**）。
    release();
    await settle();
    assert.deepStrictEqual(
      states[states.length - 1],
      { state: "ready", decls: 0 },
      `空数组是"真的没声明"（ready + 0），不是 loading：${JSON.stringify(states)}`,
    );

    // ③ 请求失败 ⇒ error（切走再切回来触发一次新的取数）。
    stubbedResponses["soko/goals"] = () => {
      throw new Error("boom");
    };
    focus(fakeDocument(docB));
    await settle();
    focus(fakeDocument(docA));
    await settle();
    assert.strictEqual(
      states[states.length - 1]?.state,
      "error",
      `请求失败必须发 error（面板要提示看输出面板/重启服务器）：${JSON.stringify(states)}`,
    );
  } finally {
    provider.setStatus = original;
    stubbedResponses["soko/goals"] = () => ({ decls: [] });
  }
});

// **G-67 的判据不在这里** ✓：这一层（stub 宿主）只能验**消息通道**，看不到 webview
// 真正渲染出什么 ✗ —— 而用户要的正是「**第一屏渲染文本对照**」⇒ 判据落在
// `editor/vscode/test-project-first-screen.js`（跑**真的** `media/infoview.js` +
// 极简 DOM shim，取**未被 `<details>` 收起**的文本）✓：
//     node editor/vscode/test-project-first-screen.js --check
// 实测（2026-09-28）——
//   改前：`清单 /repo/…/sokonanoda.toml 模块根 /repo/courses/set-theory 入口 units.u01
//          lib.Set compiled 28 声明 · 0 错 · 0 警 ★ units.u01 compiled 5 声明 · 0 错 · 0 警
//          2 模块 · 33 声明 · 编译 2 · 失败 0 · 开放练习 0 产物：7 条 · 20480 字节 · 0.78.0`
//         ⇒ **答不上三问**（无"编完了吗"、版本埋在产物行、内部路径/字节数占第一屏）✗
//   改后：`已完成 2 个文件 · 33 条声明 编译器 0.78.0 逐个文件 高级`
//         ⇒ 三问一眼答得上 ✓，模块列表与内部信息在折叠区 ✓

test("Infoview receives the project view the server answered (E30)", async () => {
  // **E30**：Infoview 的「项目」区块**不自己取数** ✗ —— CLI 的 `query project`
  // 会写/删 `<模块根>/.sokonanoda/compiled/*.tmp`（刷新一次就重编一次 ✗），
  // 所以数据是主机**转发**的 `soko/project` 回答（它本来就在 `loadProject()` 里
  // 拿这份答案，项目树/状态栏都在用）。
  //
  // 判据钉的是**转发这件事**：`soko/project` 答了 ⇒ Infoview 的 `_lastProject`
  // 必须拿到 `{project, reason}`（`requires_warning` **原样**转发 —— 它是
  // "项目缓存被静默关掉"的唯一可见信号，见 G-24）。
  const root = "/repo/courses/set-theory";
  const warning = "清单 requires 0.73.0 与当前 0.74.0 不一致：项目编译缓存已关闭。";
  stubbedResponses["soko/project"] = () => ({
    uri: "file:///repo/courses/set-theory/units/u01.sokonanoda",
    version: 1,
    project: {
      entry: "units.u01",
      root,
      manifest: `${root}/sokonanoda.toml`,
      requires_warning: warning,
      modules: [{ name: "units.u01", path: "units/u01.sokonanoda", status: "compiled", entry: true, imports: [], decls: 5, errors: 0, warnings: 2, open_exercises: 1 }],
      diagnostics: [],
      counts: { modules: 1, decls: 5, compiled: 1, failed: 0, open_exercises: 1 },
    },
    reason: null,
  });
  try {
    await activateExtension();
    focus(fakeDocument("/repo/courses/set-theory/units/u01.sokonanoda"));
    await settle();
    const posted = vscodeStub.__infoview.lastProject();
    assert.ok(posted, "Infoview 必须收到项目载荷（E30）");
    assert.strictEqual(posted.project.root, root, "模块根要原样转发");
    assert.strictEqual(
      posted.project.requires_warning,
      warning,
      "requires_warning 必须**原样**转发（它是「缓存被静默关掉」的唯一可见信号）",
    );
    assert.strictEqual(posted.reason, null, "有项目时 reason 是 null");
  } finally {
    stubbedResponses["soko/project"] = () => ({
      uri: "",
      version: 1,
      project: null,
      reason: "no-imports",
    });
  }
});

test("Infoview 记法符号可点：offset 变体换算后走同一条 F12 链 (G-53)", async () => {
  // **G-53**：wire 现在给 semantic run 带**源位置**（字节 offset ✓，`67bf0895` ✓）⇒ Infoview 里
  // 点记法符号（`∈` / `{a}` ✓）发的是 **offset 变体** ✓。webview **没有源文本** ⇒ 换算由扩展做 ✓：
  // `TextDocument.positionAt(offset)`（**UTF-16 码元**语义 ✓ = LSP `Position` ✓，与 G-36 同源 ✓）
  // ⇒ 换算后**汇合**到**同一条** `gotoDefinition` ✓（**不复制第二条跳转路径** ✗）。
  //
  // ⚠ 判据绑**用户动作的后果** ✓（AGENTS.md 第 0 条 (a) ✓）：不是"消息发出去了" ✗，而是
  // **问 F12 那条命令、且位置是按源文本换算出来的** ✓。
  await activateExtension();
  const provider = vscodeStub.__infoview;
  assert.ok(provider, "activate() 必须建 Infoview provider");
  const clicked = "file:///repo/units/u01.sokonanoda";
  const source = "def p : Prop := Prop\n";
  vscodeStub.__docText = source;
  const definition = {
    uri: vscodeStub.Uri.file("/repo/lib/Set.sokonanoda"),
    range: new vscodeStub.Range(11, 4, 11, 20),
  };
  vscodeStub.__definitions = [definition];
  vscodeStub.__commandsCalled = [];
  const editor = editorFor(fakeDocument("/repo/lib/Set.sokonanoda"));
  vscodeStub.window.activeTextEditor = editor;
  // 用户点的是 `Prop`（第二个，offset 17 ✓ —— `def p : Prop := ` 之后 ✓）。
  const offset = source.lastIndexOf("Prop");
  await provider._onMessage({ protocol: 1, type: "definition", uri: clicked, offset });

  const calls = vscodeStub.__commandsCalled;
  const query = calls.find((call) => call.id === "vscode.executeDefinitionProvider");
  assert.ok(
    query,
    `offset 变体必须走 F12 同一条命令：${JSON.stringify(calls.map((c) => c.id))}`,
  );
  assert.strictEqual(query.args[0].toString(), clicked, "问的是点击处那份文档");
  assert.strictEqual(query.args[1].line, 0, "offset 换算成行（0-based）");
  assert.strictEqual(
    query.args[1].character,
    offset,
    "offset 换算成列 —— 单行 ASCII 源里列 == offset（换算走 positionAt ✓）",
  );
});

test("Infoview 'definition' jumps like F12, never a plain reveal (E27)", async () => {
  // **E27**：点 Infoview 的声明名 ⇒ 扩展必须走**与编辑器 F12 同一条**命令
  //（`vscode.executeDefinitionProvider`），落点 = 定义所在文件 + 行（**跨文件** ✓）。
  //
  // ⚠ 这条判据是针对性的：历史上"跳转失败"的真因不是坏了，而是**从来不存在**
  //（webview 只发 `ready`/`reveal`，扩展侧没有 `definition` 分支 ✗），而且
  // `reveal`（滚到源码 span）与"跳到定义"**表现太像** ⇒ 拿"编辑器动了一下"
  // 当判据会**假绿** ✗✓（PLAN §E27 的机制 2）。所以这里同时钉两端：
  //   ① 必须问 `vscode.executeDefinitionProvider`（F12 语义）；
  //   ② 最终落点必须是**定义返回的** uri+range（跨文件），不是点击处那个 span。
  await activateExtension();
  const provider = vscodeStub.__infoview;
  assert.ok(provider, "activate() 必须建 Infoview provider");
  const clicked = "file:///repo/units/u01.sokonanoda";
  const definition = {
    uri: vscodeStub.Uri.file("/repo/lib/Set.sokonanoda"),
    range: new vscodeStub.Range(11, 4, 11, 20),
  };
  vscodeStub.__definitions = [definition];
  vscodeStub.__commandsCalled = [];
  // 落点端：把**定义所在文件**设成活动编辑器 ⇒ 跳转会把光标落在那一行
  //（`revealRange` 走"已经是活动文档"那一支 ⇒ 直接 `selection = range` ✓）。
  const target = fakeDocument("/repo/lib/Set.sokonanoda");
  const editor = editorFor(target);
  vscodeStub.window.activeTextEditor = editor;
  await provider._onMessage({
    protocol: 1,
    type: "definition",
    uri: clicked,
    position: { line: 3, character: 8 },
  });

  const calls = vscodeStub.__commandsCalled;
  const query = calls.find((call) => call.id === "vscode.executeDefinitionProvider");
  assert.ok(
    query,
    `必须走 F12 同一条命令（vscode.executeDefinitionProvider）：${JSON.stringify(calls.map((c) => c.id))}`,
  );
  assert.strictEqual(query.args[0].toString(), clicked, "问的是点击处那份文档");
  assert.strictEqual(query.args[1].line, 3, "位置用点击处的源位置（行）");
  assert.strictEqual(query.args[1].character, 8, "位置用点击处的源位置（列）");

  // **落点**：定义所在文件 + 定义那一行（跨文件 ✓）。
  assert.strictEqual(
    editor.document.uri.toString(),
    "file:///repo/lib/Set.sokonanoda",
    "落点必须是**定义所在文件**（跨文件 ✓），不是点击处那个 span ✗",
  );
  assert.strictEqual(editor.selection.start.line, 11, "光标必须落在定义那一行");
  assert.strictEqual(editor.selection.start.character, 4, "列也要用定义的范围");
  // 反向守卫：落点**不等于**点击处 ⇒ 不是 `reveal`（点哪滚哪 = 假跳转）。
  assert.notStrictEqual(
    editor.document.uri.toString(),
    clicked,
    "落点等于点击处 ⇒ 那是 reveal（滚到源码位置），不是跳定义 ✗",
  );
});

test("clicking a declaration name never pops an empty notice (P0)", async () => {
  // **P0（2026-09-28 用户实测）**：点 Infoview 的**声明名** ⇒ 右下角弹
  // 「sokonanoda: 这里没有可跳转的定义」✗。
  //
  // **根因**：webview 发的是 `decl.range.start`（**声明名**的位置），而本 LSP 的
  // definition 解析的是**使用处** ⇒ 在声明名处返回 `null` ⇒ 弹提示。
  // ⚠ E27 的旧判据用**使用处**（`∈`，能跳）验，**绕开了用户实际点的位置** ✗ ——
  // 这就是 AGENTS.md 验证设计纪律**第 0 条 (a)** 记的那次事故 ✓。
  //
  // **判据绑用户动作**：点声明名 ⇒ ① **编辑器滚到/选中该声明**（可见结果 ✓）；
  // ② **不许弹任何信息提示** ✗。反向：真没有定义时**仍要如实说**（不许静默 ✗）。
  await activateExtension();
  const provider = vscodeStub.__infoview;
  assert.ok(provider, "activate() 必须建 Infoview provider");

  // 情形 ①：服务端答 `null`（= 声明名处的真实行为）⇒ 必须 reveal 到点击处、**不弹提示**
  const declUri = "file:///repo/units/u01.sokonanoda";
  const target = fakeDocument("/repo/units/u01.sokonanoda");
  const editor = editorFor(target);
  vscodeStub.window.activeTextEditor = editor;
  vscodeStub.__definitions = []; // ← 服务端在声明名处答的就是这个
  vscodeStub.__messages = [];
  vscodeStub.__commandsCalled = [];
  await provider._onMessage({
    protocol: 1,
    type: "definition",
    uri: declUri,
    position: { line: 31, character: 10 },
  });
  assert.deepStrictEqual(
    vscodeStub.__messages,
    [],
    `点声明名**不许弹提示**（用户实测的就是这条 ✗）：${JSON.stringify(vscodeStub.__messages)}`,
  );
  assert.strictEqual(
    editor.selection.start.line,
    31,
    "必须**真的动**：光标落到声明名那一行（用户动作 → 可见结果 ✓）",
  );
  assert.strictEqual(editor.selection.start.character, 10, "列也用点击处的位置");
  // 仍然走 F12 同一条命令（E27 的语义没被削弱 ✓）—— 先问服务端，答不出才降级。
  assert.ok(
    vscodeStub.__commandsCalled.some((c) => c.id === "vscode.executeDefinitionProvider"),
    "降级前仍必须先问 definition（不许跳过服务端直接 reveal ✗）",
  );
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

test("Tab leaves an unknown word alone (and takes Lean's short keys)", async () => {
  await activateExtension();
  // `\zz` 不在表里（也不是任何键的前缀）⇒ 什么都不做：**一次编辑都不发**。
  const unknown = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\zz");
  focus(unknown, [cursor(0, 3)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(unknown.getText(), "\\zz", "an unknown word must not be rewritten");
  assert.strictEqual(unknown.__undoStack.length, 0, "no edit may be issued at all");

  // 对照：`\an` **是** Lean 的键（∧）⇒ Tab 换成 ∧。它同时是 `\and` 的前缀，
  // 但 Tab 是显式命令，前缀不是理由（与 `\in` → ∈ 同一条规则）。
  const short = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\an");
  focus(short, [cursor(0, 3)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(short.getText(), "∧", "Lean's short key `\\an` must produce ∧");

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
  // `\in` 是 `\inter` 的前缀，`\inter` 又是 `\intersection` 的前缀 ⇒ 一路都不落定，
  // 直到 `\intersection` **既是键、又不是更长键的前缀**（与 Lean 的
  // `isAbbreviationUniqueAndComplete` 同口径）。
  const steps = "ntersection".split("");
  for (const [index, character] of steps.entries()) {
    typeText(document, cursor(0, 2 + index), character);
    await drain();
    const last = index === steps.length - 1;
    assert.strictEqual(
      document.getText(),
      last ? "∩" : `\\i${steps.slice(0, index + 1).join("")}`,
      `after typing \`${character}\` the text must be ${last ? "∩" : "the abbreviation"}`,
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

// ── 希腊字母（用户反馈：`α` 没有快捷输入，设计 D1/D2）────────────────────
// `\a` 同时是 `\alpha`/`\approx`/`\and` 的前缀 —— 这正是"完整表词"两态口径
// 存在的理由：Tab 与分隔符封口时它落定，还在敲字母时它等。

test("Tab rewrites \\a to α even though \\a prefixes \\alpha", async () => {
  await activateExtension();
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\a");
  focus(document, [cursor(0, 2)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(document.getText(), "α", "`\\a` + Tab must become `α`");
});

test("Tab rewrites the spelled-out greek names", async () => {
  await activateExtension();
  for (const [abbreviation, symbol] of [
    ["\\alpha", "α"],
    ["\\beta", "β"],
    ["\\Gamma", "Γ"],
    ["\\Delta", "Δ"],
    ["\\Omega", "Ω"],
  ]) {
    const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", abbreviation);
    focus(document, [cursor(0, abbreviation.length)]);
    await commandHandler(REPLACE_COMMAND)();
    assert.strictEqual(document.getText(), symbol, `\`${abbreviation}\` + Tab → ${symbol}`);
  }
});

test("eager mode waits out the greek prefix trap", async () => {
  await activateExtension();
  setConfig("input.eager", true);
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\a");
  focus(document, [cursor(0, 2)]);
  // `\a` is a prefix of `alpha`, `approx`, `and`…: it must NOT fire while typing.
  await drain();
  assert.strictEqual(document.getText(), "\\a", "a lone `\\a` must wait");
  // The separator closes the word (same rule as `\in ` → `∈ `).
  typeText(document, cursor(0, 2), " ");
  await drain();
  assert.strictEqual(document.getText(), "α ", "`\\a ` closes the word → `α `");

  // …and typing on towards `alpha` replaces with the same symbol, once.
  const spelled = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\al");
  focus(spelled, [cursor(0, 3)]);
  for (const [position, character] of [
    [cursor(0, 3), "p"],
    [cursor(0, 4), "h"],
    [cursor(0, 5), "a"],
  ]) {
    typeText(spelled, position, character);
    await drain();
  }
  assert.strictEqual(spelled.getText(), "α", "`\\alpha` completes → `α`");
  assert.strictEqual(spelled.__undoStack.length, 1, "only the complete word was replaced");
});

// ── 匿名构造子括号（设计 D6：Lean 的 `\<` / `\>`）────────────────────
// 全表唯一的**非字母**缩写。`\` 仍是 leader，孤立的 `\`（集合差）照样不替换。

test("Tab rewrites \\< and \\> to the anonymous-constructor brackets", async () => {
  await activateExtension();
  const left = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\<");
  focus(left, [cursor(0, 2)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(left.getText(), "⟨", "`\\<` + Tab must become `⟨`");

  const right = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\>");
  focus(right, [cursor(0, 2)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(right.getText(), "⟩", "`\\>` + Tab must become `⟩`");
});

test("the spelled-out bracket names work too, and eager fires at once", async () => {
  await activateExtension();
  const spelled = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\langle");
  focus(spelled, [cursor(0, 7)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(spelled.getText(), "⟨", "`\\langle` + Tab must become `⟨`");

  // `\<` 不是任何更长缩写的真前缀 ⇒ eager 模式敲完 `<` 就落定（`\` 本身不落定）。
  setConfig("input.eager", true);
  const eager = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\");
  focus(eager, [cursor(0, 1)]);
  typeText(eager, cursor(0, 1), "<");
  await drain();
  assert.strictEqual(eager.getText(), "⟨", "eager mode replaces `\\<` as soon as it is typed");
});

test("a lone \\ next to a comparison stays put", async () => {
  // 只认「`\` + 恰好一个 `<`/`>`」：`\` 后面跟别的东西（空格、字母）照旧不命中。
  await activateExtension();
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "A \\ B");
  focus(document, [cursor(0, 3)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(document.getText(), "A \\ B", "the lone backslash must survive untouched");
});

test("greek letters are identifiers, so their symbols survive as text", async () => {
  // D4：`α` 是**标识符**，不是记法符号 —— 替换出来的 `α` 就是一个普通标识符
  // （Rust 侧由 `notation_symbol: false` 保证它不被喂进词法；这里钉住 JS 侧
  // 替换出来的**文本**确实是那一个字符，不是别的码点）。
  await activateExtension();
  const document = fakeDocument("/repo/notes.sokonanoda", "sokonanoda", "\\mu");
  focus(document, [cursor(0, 3)]);
  await commandHandler(REPLACE_COMMAND)();
  assert.strictEqual(document.getText().codePointAt(0), 0x03bc, "`\\mu` must be U+03BC");
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

test("compile progress drives the status bar from idle to compiling and back", async () => {
await activateExtension();
focus(fakeDocument("/repo/playground.sokonanoda"));
await settle();
const notify = notificationHandlers["$/progress"];
assert.strictEqual(
  typeof notify,
  "function",
  "扩展必须**裸读** `$/progress`（P1→P2 的接缝；不是 `client.onProgress`，见 T5）",
);

notify({
  token: "sokonanoda/compile/repo/playground.sokonanoda",
  value: { kind: "begin", title: "sokonanoda", message: "编译 /repo/playground.sokonanoda" },
});
// **P7 展示延迟**（2026-10-01）：`begin` 只挂定时器，到点才亮 —— 慢编译
// （这次是真的慢，因为下面把定时器放行了）照旧看得见，只是**晚了 300ms**。
fireTimers();
assert.ok(
  String(statusBarStub().text).includes("编译中"),
  `\`begin\` 到点必须让状态栏说"编译中"（P2），实际 = ${JSON.stringify(statusBarStub().text)}`,
);

notify({
  token: "sokonanoda/compile/repo/playground.sokonanoda",
  value: { kind: "end" },
});
assert.ok(
  !String(statusBarStub().text).includes("编译中"),
  `\`end\` 必须退出"编译中"态（否则进度条永远转 ✗），实际 = ${JSON.stringify(statusBarStub().text)}`,
);
});

// ── P7 展示延迟（2026-10-01 用户反馈「一闪一闪」）────────────────────────
//
// 这两条是**确定性**的那一层判据（stub 宿主能控制定时器与"编译有多久"）：
// e2e 只能量到"共享 runner 上这次闪了几下"，这里能钉死**因果**。
// 背景：服务端**每个键都编一次**（实测 2.0 条 `$/progress`/键），立刻亮就是
// 「敲一个字闪一下」。真宿主实测 8 个键亮 16 次。
test("P7: a compile that finishes inside the show delay never touches the UI", async () => {
  await activateExtension();
  const editor = focus(
    fakeDocument("/repo/playground.sokonanoda", "sokonanoda", "theorem a : True := True.intro\n"),
  );
  await settle();
  const notify = notificationHandlers["$/progress"];
  const progressOf = () => vscodeStub.__infoview._lastProgress;
  const statusBefore = String(statusBarStub().text);

  // 快编译：`begin` 与 `end` 之间**不放行任何定时器**（服务端 0ms 编完就是这个形状）。
  notify({ value: { kind: "begin", message: "编译 x" } });
  notify({ value: { kind: "end" } });

  assert.strictEqual(
    editor.decorationCalls.length,
    0,
    "快编译**一次装饰都不许碰** —— 每个键碰两次正是用户看到的「一闪一闪」",
  );
  assert.strictEqual(
    String(statusBarStub().text),
    statusBefore,
    "快编译不许动状态栏",
  );
  assert.strictEqual(
    progressOf(),
    undefined,
    "快编译不许往 Infoview 推进度块（否则面板每键跳一下）",
  );
  assert.strictEqual(
    pendingTimers().length,
    0,
    "`end` 必须把还挂着的展示定时器撤掉（漏一个 ⇒ 300ms 后凭空亮一下 ✗）",
  );
});

test("P7: a compile that outlives the show delay still lights up, and lights up once", async () => {
  await activateExtension();
  const editor = focus(
    fakeDocument("/repo/playground.sokonanoda", "sokonanoda", "theorem a : True := True.intro\n"),
  );
  await settle();
  const notify = notificationHandlers["$/progress"];

  notify({ value: { kind: "begin", message: "编译 x" } });
  fireTimers();
  assert.strictEqual(editor.decorationCalls.length, 1, "慢编译必须亮（P1/P3/P4 的能力不许丢）");

  // 慢编译接着慢编译：**不许**亮一下灭一下（已经亮着就保持亮着）。
  notify({ value: { kind: "begin", message: "编译 x" } });
  notify({ value: { kind: "report", message: "编译 x", percentage: 50 } });
  assert.strictEqual(
    editor.decorationCalls.length,
    1,
    "连续慢编译之间不许重新计时（那会亮—灭—亮，正是要消灭的形状）",
  );

  notify({ value: { kind: "end" } });
  assert.strictEqual(editor.decorationCalls.length, 2, "`end` 必须清空装饰");
  assert.strictEqual(editor.decorationCalls[1].ranges.length, 0, "`end` 必须真的清空");
});

test("a burst of progress reports collapses into one refresh", async () => {
await activateExtension();
focus(fakeDocument("/repo/playground.sokonanoda"));
await settle();
const notify = notificationHandlers["$/progress"];
const progressOf = () => vscodeStub.__infoview._lastProgress;

notify({ value: { kind: "begin", message: "编译 x" } });
fireTimers(); // P7：先放行展示延迟，否则 `report` 一律不碰界面
assert.strictEqual(
  progressOf().phase,
  "begin",
  "`begin` **不许**被节流（它是成对的状态边界，漏一个界面就卡住 ✗）",
);

// 一串 `report`（长文件编译时会很密）⇒ 窗口内只该挂**一个**定时器 ✓。
const before = timers.length;
for (let i = 0; i < 5; i++) {
  notify({ value: { kind: "report", message: "编译 x", percentage: i * 10 } });
}
assert.strictEqual(
  timers.length - before,
  1,
  `5 条 \`report\` 只许挂 1 个定时器（P6 节流），实际多了 ${timers.length - before} 个`,
);
assert.strictEqual(
  progressOf().phase,
  "begin",
  "节流窗口内**先不刷**（还没到点）",
);
fireTimers();
assert.strictEqual(
  progressOf().percent,
  40,
  "窗口到点必须刷**最后一次**（不是第一次、也不是每一条都刷）",
);
});

test("compile progress marks the active document in the overview ruler", async () => {
  await activateExtension();
  const editor = focus(
    fakeDocument("/repo/playground.sokonanoda", "sokonanoda", "theorem a : True := True.intro\n"),
  );
  await settle();
  const notify = notificationHandlers["$/progress"];

  notify({ value: { kind: "begin", message: "编译 x" } });
  fireTimers(); // P7：展示延迟到点才真的画（快编译那一路在下面那条用例里钉住）
  assert.strictEqual(
    editor.decorationCalls.length,
    1,
    "`begin` 到点必须给当前文档加一层装饰（P4：状态栏只说在编，不说在哪编 ✗）",
  );
  assert.ok(
    editor.decorationCalls[0].ranges.length >= 1,
    "装饰必须落在**实际范围**上（空范围 = 概览尺上什么都没画 ✗）",
  );
  // **2026-10-01 用户反馈**：「每次修改代码，整个文件就会被高亮」⇒ 装饰**不许**
  // 染整篇：范围只能是**一行**（第一行），且**没有 backgroundColor** ✓。
  assert.strictEqual(
    editor.decorationCalls[0].ranges.length,
    1,
    "只许一个范围（第一行的小标记），不许铺满整份文档",
  );
  const marked = editor.decorationCalls[0].ranges[0];
  assert.strictEqual(marked.start.line, 0, "标记落在第一行");
  assert.strictEqual(marked.end.line, 0, "标记**只覆盖一行** —— 整篇染色就是用户报的那条 ✗");
  const options = vscodeStub.__decorationOptions;
  assert.ok(
    options && options.overviewRulerColor,
    "装饰必须带 `overviewRulerColor` —— 否则概览尺上看不见，P4 就白做了 ✗",
  );
  assert.strictEqual(
    options.overviewRulerLane,
    4,
    "概览尺要画在 **Right** 道（与 VS Code 自己的诊断同一侧 ✓）",
  );
  assert.ok(
    !options.backgroundColor,
    "**不许**有背景色：整篇背景高亮正是用户 2026-10-01 报的那条（`整个文件就会被高亮`）✗",
  );

  notify({ value: { kind: "end" } });
  assert.strictEqual(
    editor.decorationCalls.length,
    2,
    "`end` 必须再调一次 `setDecorations`",
  );
  assert.strictEqual(
    editor.decorationCalls[1].ranges.length,
    0,
    "`end` 必须把装饰**清空** —— 否则那条高亮会永远留在文件上 ✗",
  );
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
