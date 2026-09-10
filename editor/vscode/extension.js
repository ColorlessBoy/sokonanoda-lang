// Minimal VS Code client for the sokonanoda language server.
// Server/CLI are resolved at runtime (server.js): bundled VSIX binaries →
// workspace `target/` builds → version-pinned release download. No cargo
// required for users; see docs/design-bundled-lsp.md.
// The server must never log to stdout; stdio carries the LSP stream.
// Goal view (I9): the "练习" tree consumes the server's `soko/goals` custom
// request; alt+n jumps between holes via `soko/nextHole` (server-side
// position logic — clients never re-derive hole positions).
// Cursor goal view (Phase 2): the tree's 「当前光标处」 group consumes
// `soko/stateAt` on (debounced) selection changes — tactic selection stays
// server-side; diagnostics refresh re-requests the caret state.
// Hint ladder: the tree's 「提示」 node reveals `soko/hints` one at a time;
// the reveal counter lives in workspaceState (the server stays stateless).
// Course map: the 「课程」 tree shells out to the CLI (`sokonanoda course
// <manifest> --json`) — cross-file aggregation is the CLI's job (the server
// stays single-document); the client only renders `course.unit` events.
const cp = require("child_process");
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const { LanguageClient, State, TransportKind } = require("vscode-languageclient/node");
const server = require("./server");

let client;
let extensionRoot;

function discoveryRoots() {
  return (vscode.workspace.workspaceFolders ?? [])
    .map((folder) => folder.uri.fsPath)
    .concat(path.join(__dirname, "..", "..")); // editor/vscode -> repo checkout
}

function extensionVersion(context) {
  return context?.extension?.packageJSON?.version;
}

// Server acquisition (bundled VSIX binary first, downloads only as a fallback)
// lives in server.js so plain Node can unit-test it; this module stays the
// VS Code wiring layer.
function resolveServerCommand(context) {
  return server.resolveServerCommand({
    setting: vscode.workspace.getConfiguration("sokonanoda").get("serverPath"),
    envBin: process.env.SOKONANODA_LSP_BIN,
    extensionPath: context.extensionPath,
    roots: discoveryRoots(),
    platform: process.platform,
    arch: process.arch,
    version: extensionVersion(context),
    log: (message) => console.warn(`[sokonanoda] ${message}`),
  });
}

// Same discovery pattern as the server, but for the `sokonanoda` CLI binary:
// bundled in the VSIX first (course map works with nothing installed), then
// workspace `target/{debug,release}` builds, then PATH.
function resolveCliCommand() {
  return server.resolveCliCommand({
    extensionPath: extensionRoot,
    roots: discoveryRoots(),
    platform: process.platform,
    arch: process.arch,
    log: (message) => console.warn(`[sokonanoda] ${message}`),
  });
}

function resolveCourseManifest() {
  const roots = (vscode.workspace.workspaceFolders ?? []).map((folder) => folder.uri.fsPath);
  return server.firstExisting(roots.map((root) => path.join(root, "course", "course.json")));
}

function isExplicitPath(command) {
  return path.isAbsolute(command) || command.includes(path.sep);
}

const stateNames = {
  [State.Starting]: "starting",
  [State.Running]: "running",
  [State.Stopped]: "stopped",
};

function statusIcon(status) {
  if (status === "open") return new vscode.ThemeIcon("circle-outline");
  if (status === "failed") return new vscode.ThemeIcon("error");
  return new vscode.ThemeIcon("check");
}

function statusLabel(status) {
  if (status === "open") return "open";
  if (status === "failed") return "failed";
  return "solved";
}

class GoalsTreeDataProvider {
  constructor() {
    this._emitter = new vscode.EventEmitter();
    this.onDidChangeTreeData = this._emitter.event;
    this.uri = undefined; // the active .sokonanoda document
    this.openCount = 0;
    this.cursorState = undefined; // {uri, state} from soko/stateAt
    this.cursorRequestSeq = 0; // discards stale soko/stateAt responses
  }

  refresh() {
    this._emitter.fire();
  }

  async trackEditor(editor) {
    const uri = editor && editor.document.languageId === "sokonanoda"
      ? editor.document.uri.toString()
      : undefined;
    if (uri !== this.uri) {
      this.uri = uri;
      this.refresh();
    }
  }

  async getTreeItem(element) {
    return element;
  }

  async getChildren(element) {
    if (!element) return this.getDeclarations();
    return element.children ?? [];
  }

  async requestGoals() {
    if (!client || !this.uri) return undefined;
    try {
      return await client.sendRequest("soko/goals", {
        textDocument: { uri: this.uri },
      });
    } catch (error) {
      client.outputChannel.appendLine(`[client] soko/goals failed: ${error?.message ?? error}`);
      return undefined;
    }
  }

  // Cursor goal view: the server owns position → tactic selection; the client
  // only forwards the caret and drops stale answers (newer request in flight
  // or active document changed).
  async requestCursorState(uriString, position) {
    if (!client) return undefined;
    const seq = ++this.cursorRequestSeq;
    try {
      const state = await client.sendRequest("soko/stateAt", {
        textDocument: { uri: uriString },
        position,
      });
      if (seq !== this.cursorRequestSeq) return undefined;
      if (uriString !== this.uri) return undefined;
      return state;
    } catch (error) {
      client.outputChannel.appendLine(`[client] soko/stateAt failed: ${error?.message ?? error}`);
      return undefined;
    }
  }

  async setCursorState(uriString, position) {
    const state = await this.requestCursorState(uriString, position);
    if (state === undefined || uriString !== this.uri) return;
    this.cursorState = { uri: uriString, state };
    this.refresh();
  }

  async getDeclarations() {
    const response = await this.requestGoals();
    const decls = response?.decls ?? [];
    this.openCount = decls.filter((d) => d.status === "open").length;
    updateStatusBar(this);
    const items = decls.map((decl) => {
      const item = new vscode.TreeItem(decl.name, decl.status === "open"
        ? vscode.TreeItemCollapsibleState.Expanded
        : vscode.TreeItemCollapsibleState.None);
      item.description = `${decl.kind} · ${statusLabel(decl.status)}`;
      item.iconPath = statusIcon(decl.status);
      if (decl.status === "open") {
        item.contextValue = "openExercise";
        item.children = buildOpenChildren(decl, this.uri);
        // soko/goals holes are `{range, id}` objects (docs/protocol.md) —
        // the client reads positions through hole.range, never bare.
        const hole = decl.holes?.[0];
        if (hole) {
          item.command = {
            command: "sokonanoda.revealRange",
            title: "",
            arguments: [this.uri, hole.range],
          };
        }
      }
      return item;
    });
    const cursor = this.cursorState !== undefined && this.cursorState.uri === this.uri
      ? this.cursorState.state
      : undefined;
    if (cursor?.decl) {
      const group = new vscode.TreeItem(
        "当前光标处",
        vscode.TreeItemCollapsibleState.Expanded,
      );
      group.iconPath = new vscode.ThemeIcon("target");
      group.children = buildCursorChildren(cursor, this.uri);
      items.unshift(group);
    }
    return items;
  }
}

function buildOpenChildren(decl, uriString) {
  const children = [];
  if (decl.goal) {
    const goal = new vscode.TreeItem("目标", vscode.TreeItemCollapsibleState.None);
    goal.description = decl.goal;
    children.push(goal);
  }
  for (const binder of decl.binders ?? []) {
    const item = new vscode.TreeItem(binder.name, vscode.TreeItemCollapsibleState.None);
    item.description = binder.ty;
    item.iconPath = new vscode.ThemeIcon("symbol-variable");
    children.push(item);
  }
  const hint = new vscode.TreeItem("提示", vscode.TreeItemCollapsibleState.None);
  hint.description = "逐条揭示";
  hint.iconPath = new vscode.ThemeIcon("lightbulb");
  hint.command = {
    command: "sokonanoda.revealHint",
    title: "揭示下一条提示",
    arguments: [uriString, decl.name, decl.range],
  };
  children.push(hint);
  return children;
}

// Children of the 「当前光标处」 group (soko/stateAt): the goal selected by the
// cursor, its hypotheses, and `by` progress. The server chose everything —
// the client only renders and wires the reveal command.
function buildCursorChildren(cursor, uriString) {
  const children = [];
  const goal = new vscode.TreeItem("目标", vscode.TreeItemCollapsibleState.None);
  goal.description = cursor.goal ?? "已无目标 ✓";
  goal.iconPath = new vscode.ThemeIcon(cursor.goal ? "circle-outline" : "check");
  if (cursor.span) {
    goal.command = {
      command: "sokonanoda.revealRange",
      title: "",
      arguments: [uriString, cursor.span],
    };
  }
  children.push(goal);
  for (const binder of cursor.binders ?? []) {
    const item = new vscode.TreeItem(binder.name, vscode.TreeItemCollapsibleState.None);
    item.description = binder.ty;
    item.iconPath = new vscode.ThemeIcon("symbol-variable");
    children.push(item);
  }
  if (cursor.total > 0) {
    const progress = new vscode.TreeItem("by 进度", vscode.TreeItemCollapsibleState.None);
    progress.description = `${cursor.step + 1}/${cursor.total}`;
    progress.iconPath = new vscode.ThemeIcon("list-ordered");
    children.push(progress);
  }
  return children;
}

// Course map (docs/design-course-status.md §2): one node per unit, rendered
// from `course.unit` events emitted by the CLI subprocess. The client never
// re-derives unit status — the CLI is the single data source.
const COURSE_TIMEOUT_MS = 10000;

function courseUnitIcon(unit) {
  if (unit.error !== undefined || (unit.failed ?? 0) > 0) {
    return new vscode.ThemeIcon("circle-filled", new vscode.ThemeColor("charts.red"));
  }
  if ((unit.open ?? 0) > 0) {
    return new vscode.ThemeIcon("circle-filled", new vscode.ThemeColor("charts.yellow"));
  }
  return new vscode.ThemeIcon("circle-filled", new vscode.ThemeColor("charts.green"));
}

function courseUnitItem(unit, manifestDir) {
  const item = new vscode.TreeItem(
    `unit ${unit.unit} ${unit.title ?? ""}`.trim(),
    vscode.TreeItemCollapsibleState.None,
  );
  item.description = `${unit.checked ?? 0} checked · ${unit.open ?? 0} open · ${unit.failed ?? 0} failed`;
  item.iconPath = courseUnitIcon(unit);
  item.tooltip = unit.error !== undefined
    ? `单元加载失败：${unit.error}`
    : `checked ${unit.checked ?? 0} · open ${unit.open ?? 0} · failed ${unit.failed ?? 0}`;
  if (manifestDir && typeof unit.file === "string" && unit.file !== "") {
    // Unit paths resolve relative to the manifest's directory (docs/protocol.md).
    const abs = path.isAbsolute(unit.file) ? unit.file : path.join(manifestDir, unit.file);
    item.command = {
      command: "vscode.open",
      title: "打开单元文件",
      arguments: [vscode.Uri.file(abs)],
    };
  }
  return item;
}

function parseCourseEvents(stdout) {
  const units = [];
  for (const line of stdout.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed === "") continue;
    try {
      const event = JSON.parse(trimmed);
      if (event && event.type === "course.unit") units.push(event);
    } catch {
      // not JSON Lines — ignore (stderr is dropped too)
    }
  }
  return units;
}

// Subprocess discipline: JSON Lines from stdout, stderr ignored, kill after
// the timeout and fall back to an empty tree (progress is never an error).
function runCourseCommand(command, manifestPath) {
  return new Promise((resolve) => {
    let stdout = "";
    let settled = false;
    let child;
    let timer;
    const finish = (units) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      child.kill();
      resolve(units);
    };
    try {
      child = cp.spawn(command, ["course", manifestPath, "--json"]);
    } catch {
      resolve([]);
      return;
    }
    timer = setTimeout(() => finish([]), COURSE_TIMEOUT_MS);
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.resume();
    child.on("error", () => finish([]));
    child.on("close", () => finish(parseCourseEvents(stdout)));
  });
}

class CourseTreeDataProvider {
  constructor() {
    this._emitter = new vscode.EventEmitter();
    this.onDidChangeTreeData = this._emitter.event;
    this.manifestDir = undefined;
    this.units = [];
    this._pending = undefined;
  }

  refresh() {
    this._emitter.fire();
  }

  async getTreeItem(element) {
    return element;
  }

  async getChildren(element) {
    if (element) return element.children ?? [];
    return this.loadUnits();
  }

  async loadUnits() {
    if (!this._pending) {
      this._pending = this.runCourse().finally(() => {
        this._pending = undefined;
      });
    }
    return this._pending;
  }

  async runCourse() {
    this.units = [];
    const manifest = resolveCourseManifest();
    if (!manifest) return this.units; // no course manifest -> empty tree
    const units = await runCourseCommand(resolveCliCommand(), manifest);
    this.manifestDir = path.dirname(manifest);
    this.units = units.map((unit) => courseUnitItem(unit, this.manifestDir));
    return this.units;
  }
}

let statusBar;

function updateStatusBar(provider) {
  if (!statusBar) return;
  if (provider.uri === undefined) {
    statusBar.hide();
    return;
  }
  statusBar.text = `$(circle-outline) ${provider.openCount}`;
  statusBar.tooltip = new vscode.MarkdownString(
    `sokonanoda：${provider.openCount} 个练习未完成（点击查看练习面板）`,
  );
  statusBar.show();
}

async function revealRange(uriString, range) {
  const uri = typeof uriString === "string" ? vscode.Uri.parse(uriString) : uriString;
  const editor = vscode.window.activeTextEditor?.document.uri.toString() === uri.toString()
    ? vscode.window.activeTextEditor
    : (await vscode.workspace.openTextDocument(uri), await vscode.window.showTextDocument(uri));
  if (editor && range) {
    editor.revealRange(range, vscode.TextEditorRevealType.InCenter);
    editor.selection = new vscode.Selection(range.start, range.end);
  }
}

async function nextHole(backward) {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== "sokonanoda") {
    vscode.window.showInformationMessage("请打开一个 .sokonanoda 文件再跳洞。");
    return;
  }
  if (!client) {
    vscode.window.showWarningMessage("sokonanoda 语言服务器没有运行。");
    return;
  }
  const position = editor.selection.active;
  const ask = (pos, forward) =>
    client.sendRequest("soko/nextHole", {
      textDocument: { uri: editor.document.uri.toString() },
      position: pos,
      forward,
    });
  let range = await ask(
    { line: position.line, character: position.character },
    !backward,
  );
  if (!range && !backward) {
    // 环绕：从头再找第一个洞。
    range = await ask({ line: 0, character: 0 }, true);
  }
  if (!range) {
    vscode.window.showInformationMessage("没有更多的洞。");
    return;
  }
  await revealRange(editor.document.uri.toString(), range);
}

function hintStateKey(uriString, declName) {
  return `sokonanoda.hintRevealed:${uriString}:${declName}`;
}

function positionInRange(range, position) {
  if (!range || !position) return false;
  if (position.line < range.start.line || position.line > range.end.line) return false;
  if (position.line === range.start.line && position.character < range.start.character) return false;
  if (position.line === range.end.line && position.character > range.end.character) return false;
  return true;
}

async function revealHint(context, uriArg, declName, declRange) {
  if (!client) {
    vscode.window.showWarningMessage("sokonanoda 语言服务器没有运行。");
    return;
  }
  let uriString = typeof uriArg === "string" ? uriArg : uriArg?.toString();
  let position = declRange?.start;
  if (!position) {
    // 命令面板回退：没有传入声明 range 时，用活动编辑器的光标位置定位。
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "sokonanoda") {
      vscode.window.showInformationMessage("请打开一个 .sokonanoda 文件再获取提示。");
      return;
    }
    uriString = editor.document.uri.toString();
    position = editor.selection.active;
  }
  let name = declName;
  if (!name) {
    // 声明定位走服务端数据（客户端不扫文本找声明）。
    const goals = await client.sendRequest("soko/goals", {
      textDocument: { uri: uriString },
    }).catch(() => undefined);
    name = (goals?.decls ?? []).find((d) => positionInRange(d.range, position))?.name
      ?? `line:${position.line}:${position.character}`;
  }
  let response;
  try {
    response = await client.sendRequest("soko/hints", {
      textDocument: { uri: uriString },
      position,
    });
  } catch (error) {
    vscode.window.showErrorMessage(`sokonanoda: 提示请求失败 — ${error?.message ?? error}`);
    return;
  }
  const hints = response?.hints ?? [];
  if (hints.length === 0) {
    vscode.window.showInformationMessage("这个声明还没有挂提示。");
    return;
  }
  const key = hintStateKey(uriString, name);
  const revealed = context.workspaceState.get(key, 0);
  if (revealed >= hints.length) {
    vscode.window.showInformationMessage("这个练习的提示都给你了，试试写一步吧。");
    return;
  }
  await context.workspaceState.update(key, revealed + 1);
  vscode.window.showInformationMessage(hints[revealed]);
}

function registerCommands(context, provider, courseProvider) {
  const showStatus = async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "sokonanoda") {
      vscode.window.showInformationMessage("请打开一个 .sokonanoda 文件再查看练习状态。");
      return;
    }
    if (!client) {
      vscode.window.showWarningMessage("sokonanoda 语言服务器没有运行。");
      return;
    }
    let response;
    try {
      response = await client.sendRequest("textDocument/documentSymbol", {
        textDocument: { uri: editor.document.uri.toString() },
      });
    } catch (error) {
      vscode.window.showErrorMessage(`sokonanoda: 状态请求失败 — ${error?.message ?? error}`);
      return;
    }
    const items = (response ?? []).map((symbol) => ({
      label: symbol.name,
      description: symbol.detail ?? "",
      range: symbol.range ?? symbol.location?.range,
    }));
    if (items.length === 0) {
      vscode.window.showInformationMessage("sokonanoda: 没有找到声明（可能存在解析错误）。");
      return;
    }
    const pick = await vscode.window.showQuickPick(items, {
      placeHolder: "练习状态 — 名称 · 类型/进度（alt+s 随时呼出）",
      matchOnDescription: true,
    });
    if (pick?.range) {
      editor.revealRange(pick.range, vscode.TextEditorRevealType.InCenter);
      editor.selection = new vscode.Selection(pick.range.start, pick.range.start);
    }
  };

  context.subscriptions.push(
    vscode.commands.registerCommand("sokonanoda.showStatus", showStatus),
    vscode.commands.registerCommand("sokonanoda.status", showStatus), // code lens target
    vscode.commands.registerCommand("sokonanoda.nextHole", () => nextHole(false)),
    vscode.commands.registerCommand("sokonanoda.previousHole", () => nextHole(true)),
    vscode.commands.registerCommand("sokonanoda.goals.refresh", () => provider.refresh()),
    vscode.commands.registerCommand("sokonanoda.courseRefresh", () => courseProvider.refresh()),
    vscode.commands.registerCommand("sokonanoda.revealRange", revealRange),
    vscode.commands.registerCommand(
      "sokonanoda.revealHint",
      (uri, declName, declRange) => revealHint(context, uri, declName, declRange),
    ),
  );
}

async function activate(context) {
  extensionRoot = context.extensionPath;
  let command = await resolveServerCommand(context);
  if (command === undefined || (isExplicitPath(command) && !fs.existsSync(command))) {
    // No bundled binary for this platform (universal VSIX / unsupported arch):
    // fall back to the version-pinned GitHub Release download.
    if (!server.platformTarget(process.platform, process.arch)) {
      vscode.window.showErrorMessage(
        `sokonanoda：当前平台（${process.platform}-${process.arch}）没有内置语言服务器。` +
          "请设置 sokonanoda.serverPath 指向本地编译的 sokonanoda-lsp。",
      );
      return;
    }
    vscode.window.showInformationMessage(
      "sokonanoda：正在获取语言服务器（内置包缺失，回退下载）…",
    );
    try {
      command = await server.downloadLspBinary({
        version: extensionVersion(context),
        platform: process.platform,
        arch: process.arch,
      });
      if (command && fs.existsSync(command)) {
        vscode.window.showInformationMessage("sokonanoda：语言服务器就绪 ✓");
      } else {
        throw new Error("download produced no binary");
      }
    } catch (err) {
      vscode.window.showWarningMessage(
        "sokonanoda-lsp 获取失败。请安装对应平台的插件包，或联网后重试；也可用 sokonanoda.serverPath 指向本地二进制。错误：" + (err?.message ?? err)
      );
      return;
    }
  }

  const serverOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };
  client = new LanguageClient("sokonanoda", "sokonanoda", serverOptions, {
    documentSelector: [{ language: "sokonanoda", scheme: "file" }],
    synchronize: { fileEvents: vscode.workspace.createFileSystemWatcher("**/*.sokonanoda") },
  });
  client.onDidChangeState((event) => {
    client.outputChannel.appendLine(`[client] ${stateNames[event.newState] ?? event.newState}`);
  });
  context.subscriptions.push(client);

  // 练习面板 + 状态栏（消费 soko/goals；诊断更新即刷新）。
  const provider = new GoalsTreeDataProvider();
  const tree = vscode.window.createTreeView("sokonanoda.goals", {
    treeDataProvider: provider,
    showCollapseAll: true,
  });
  statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
  statusBar.command = "sokonanoda.goals.focus";
  context.subscriptions.push(tree, statusBar);

  // 光标目标视图：选区变化去抖 ~200ms 后请求 soko/stateAt（位置选取在服务端）。
  const requestCursorForEditor = (editor = vscode.window.activeTextEditor) => {
    if (!editor || editor.document.languageId !== "sokonanoda") return;
    const uriString = editor.document.uri.toString();
    if (uriString !== provider.uri) return;
    provider.setCursorState(uriString, editor.selection.active);
  };
  let selectionTimer;
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      provider.trackEditor(editor);
      requestCursorForEditor(editor);
    }),
    vscode.window.onDidChangeTextEditorSelection((event) => {
      const editor = event.textEditor;
      if (!editor || editor.document.languageId !== "sokonanoda") return;
      const uriString = editor.document.uri.toString();
      if (uriString !== provider.uri) return;
      const position = editor.selection.active;
      clearTimeout(selectionTimer);
      selectionTimer = setTimeout(
        () => provider.setCursorState(uriString, position),
        200,
      );
    }),
    vscode.languages.onDidChangeDiagnostics(() => {
      provider.refresh();
      requestCursorForEditor(); // the document may have been re-checked
    }),
    { dispose: () => clearTimeout(selectionTimer) },
  );
  provider.trackEditor(vscode.window.activeTextEditor);
  requestCursorForEditor();

  // 课程面板：数据来自 CLI 子进程（跨文件聚合）；不挂诊断刷新——诊断是
  // 单文档事件，课程地图只需激活时与手动刷新（sokonanoda.courseRefresh）。
  const courseProvider = new CourseTreeDataProvider();
  const courseTree = vscode.window.createTreeView("sokonanoda.courseMap", {
    treeDataProvider: courseProvider,
    showCollapseAll: true,
  });
  context.subscriptions.push(courseTree);

  registerCommands(context, provider, courseProvider);
  courseProvider.refresh();

  await client.start();
  client.outputChannel.appendLine(`[client] ready — server synchronized over stdio: ${command}`);
}

async function deactivate() {
  if (!client) return;
  try {
    await client.stop();
  } finally {
    client = undefined;
  }
}

module.exports = { activate, deactivate };
