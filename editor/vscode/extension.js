// Minimal VS Code client for the sokonanoda language server.
// Server/CLI are resolved at runtime (server.js): bundled VSIX binaries →
// workspace `target/` builds → version-pinned release download. No cargo
// required for users; see docs/design/bundled-lsp.md.
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
// Infoview (方案 B, docs/design/webview-infoview.md): a WebviewViewProvider
// (`sokonanoda.infoview`) renders the same soko/stateAt snapshot and
// soko/goals declaration list in a dockable panel; it is a read-only
// presentation layer and the trees stay the default/fallback.
const cp = require("child_process");
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const { LanguageClient, State, TransportKind } = require("vscode-languageclient/node");
const server = require("./server");

let client;
let serverOptions;
let extensionRoot;
let lastResolution; // {command, source} of the last successful resolveServerForStart
let overrideNoticeShown = false; // one-time "your serverPath is ignored" notice
let doctorChannel; // shared read-only doctor output channel

function discoveryRoots() {
  return (vscode.workspace.workspaceFolders ?? [])
    .map((folder) => folder.uri.fsPath)
    .concat(path.join(__dirname, "..", "..")); // editor/vscode -> repo checkout
}

function extensionVersion(context) {
  return context?.extension?.packageJSON?.version;
}

// Version tuple compare (`1.2.3` vs `1.2.2`); >0 when `a` is newer.
function compareVersions(a, b) {
  const pa = String(a).split(".").map(Number);
  const pb = String(b).split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    const diff = (pa[i] || 0) - (pb[i] || 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

// Newest sokonanoda extension version installed on disk (sibling folders of
// the running extension). VS Code swaps extension *code* only on a window
// reload, so right after an upgrade this can be newer than the running host —
// worth surfacing instead of pretending a server restart fixed it
// (docs/vscode-dev-guide.md §5.6). `undefined` when it cannot be determined.
function newestInstalledExtensionVersion(context) {
  try {
    const dir = path.dirname(context.extensionPath);
    let best;
    for (const name of fs.readdirSync(dir)) {
      const match = /^sokonanoda-lang\.sokonanoda-(\d+\.\d+\.\d+)/.exec(name);
      if (!match) continue;
      if (best === undefined || compareVersions(match[1], best) > 0) {
        best = match[1];
      }
    }
    return best;
  } catch {
    return undefined;
  }
}

// Server acquisition (bundled VSIX binary first, downloads only as a fallback)
// lives in server.js so plain Node can unit-test it; this module stays the
// VS Code wiring layer. `serverOverride` defaults to false: the bundled server
// always wins and `serverPath` / `SOKONANODA_LSP_BIN` / workspace builds are
// ignored (docs/design/extension-server-policy.md §2).
function serverOverrideEnabled() {
  return vscode.workspace.getConfiguration("sokonanoda").get("serverOverride") === true;
}

function resolveServerCommand(context) {
  return server.resolveServerCommand({
    setting: vscode.workspace.getConfiguration("sokonanoda").get("serverPath"),
    envBin: process.env.SOKONANODA_LSP_BIN,
    extensionPath: context.extensionPath,
    roots: discoveryRoots(),
    platform: process.platform,
    arch: process.arch,
    version: extensionVersion(context),
    override: serverOverrideEnabled(),
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

// The user's explicit server choice (`sokonanoda.serverPath` /
// `SOKONANODA_LSP_BIN`), if any. With `serverOverride` off these are ignored
// (the bundled server wins), so this is used to report that instead of
// silently falling back to a download.
function requestedServerCommand() {
  const setting = vscode.workspace.getConfiguration("sokonanoda").get("serverPath");
  if (typeof setting === "string" && setting.trim() !== "") return setting.trim();
  if (process.env.SOKONANODA_LSP_BIN) return process.env.SOKONANODA_LSP_BIN;
  return undefined;
}

// One-time, non-blocking notice that an explicit server path is being ignored
// because `serverOverride` is off (docs/design/extension-server-policy.md §2).
// Never modifies the setting — the action just opens it.
function noticeIgnoredOverride() {
  if (overrideNoticeShown) return;
  overrideNoticeShown = true;
  const setting = vscode.workspace.getConfiguration("sokonanoda").get("serverPath");
  const which = typeof setting === "string" && setting.trim() !== ""
    ? "sokonanoda.serverPath"
    : "SOKONANODA_LSP_BIN";
  vscode.window
    .showWarningMessage(
      `sokonanoda：已忽略 ${which}，正在使用扩展内置的语言服务器（避免版本错配）。` +
        "要改用自定义服务器，请开启 sokonanoda.serverOverride。",
      "打开设置",
      "运行 doctor",
    )
    .then((choice) => {
      if (choice === "打开设置") {
        vscode.commands.executeCommand(
          "workbench.action.openSettings",
          "sokonanoda.serverOverride",
        );
      } else if (choice === "运行 doctor") {
        vscode.commands.executeCommand("sokonanoda.doctor");
      }
    });
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
    this.declItems = undefined; // cached decl TreeItems from the last soko/goals
    this.onDecls = undefined; // (decls) => void — feeds the Infoview decls message
    this.onState = undefined; // (uri, state) => void — feeds the Infoview state message
    this.treeView = undefined; // set by activate, for focusExercise reveal
  }

  // Full reload: the declarations themselves changed (new diagnostics or a
  // different active document), so drop the cached items and refetch.
  refresh() {
    this.declItems = undefined;
    this._emitter.fire();
  }

  // Cursor-only update: the declarations are unchanged, so reuse the cached
  // TreeItems and only rebuild the 「当前光标处」 group. This is what keeps
  // caret movement cheap — no `soko/goals` round trip and no rebuild of the
  // exercise nodes on every selection change (docs/design/goal-list.md).
  refreshCursor() {
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
    if (!element) return this.rootChildren();
    return element.children ?? [];
  }

  async rootChildren() {
    if (!this.declItems) await this.loadDeclarations();
    const items = [];
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
      items.push(group);
    }
    items.push(...(this.declItems ?? []));
    return items;
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
    // Fan the single soko/stateAt snapshot out to the Infoview webview too —
    // one request feeds both views, and cursor moves never trigger soko/goals.
    this.onState?.(uriString, state);
    this.refreshCursor();
  }

  // Load the declaration list once (per document/diagnostics version), or
  // reuse the cached items. The Infoview asks for this on `ready`/diagnostics;
  // cursor movement never calls it (docs/design/goal-list.md §2.4).
  async ensureDeclarations() {
    if (!this.declItems) await this.loadDeclarations();
  }

  // `focusExercise` from the Infoview: reveal + expand the declaration node in
  // the 练习 tree. Declarations come from `soko/goals` — never scan the source.
  async focusDeclaration(name) {
    if (!this.declItems) await this.ensureDeclarations();
    const item = (this.declItems ?? []).find((entry) => entry.label === name);
    if (!item || !this.treeView) return;
    await this.treeView.reveal(item, { select: true, focus: true, expand: true });
  }

  // Fetch `soko/goals` once per document/diagnostics version and cache the
  // built TreeItems; cursor movement reuses them (see `refreshCursor`).
  async loadDeclarations() {
    const response = await this.requestGoals();
    const decls = response?.decls ?? [];
    this.openCount = decls.filter((d) => d.status === "open").length;
    updateStatusBar(this);
    this.declItems = decls.map((decl) => {
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
    this.onDecls?.(decls);
  }
}

function binderItem(binder) {
  const item = new vscode.TreeItem(binder.name, vscode.TreeItemCollapsibleState.None);
  item.description = binder.ty;
  item.iconPath = new vscode.ThemeIcon("symbol-variable");
  return item;
}

function buildOpenChildren(decl, uriString) {
  const children = [];
  // soko/goals carries every open goal after the last tactic (`goals`,
  // current first); fall back to the single `goal` for older servers.
  const goals = Array.isArray(decl.goals) && decl.goals.length > 0
    ? decl.goals
    : (decl.goal ? [decl.goal] : []);
  goals.forEach((ty, index) => {
    const label = goals.length > 1 ? `目标 ${index + 1}/${goals.length}` : "目标";
    const goal = new vscode.TreeItem(label, vscode.TreeItemCollapsibleState.None);
    goal.description = ty;
    children.push(goal);
  });
  for (const binder of decl.binders ?? []) {
    children.push(binderItem(binder));
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

// Children of the 「当前光标处」 group (soko/stateAt): every goal selected by the
// cursor (current first), each with its own hypotheses, plus `by` progress.
// The server chose everything — the client only renders and wires the reveal
// command.
function buildCursorChildren(cursor, uriString) {
  const children = [];
  const goals = Array.isArray(cursor.goals) && cursor.goals.length > 0
    ? cursor.goals
    : (cursor.goal ? [{ goal: cursor.goal, binders: cursor.binders ?? [] }] : []);
  if (goals.length === 0) {
    const goal = new vscode.TreeItem("目标", vscode.TreeItemCollapsibleState.None);
    goal.description = "已无目标 ✓";
    goal.iconPath = new vscode.ThemeIcon("check");
    children.push(goal);
  } else if (goals.length === 1) {
    const goal = new vscode.TreeItem("目标", vscode.TreeItemCollapsibleState.None);
    goal.description = goals[0].goal;
    goal.iconPath = new vscode.ThemeIcon("circle-outline");
    if (cursor.span) {
      goal.command = {
        command: "sokonanoda.revealRange",
        title: "",
        arguments: [uriString, cursor.span],
      };
    }
    children.push(goal);
    for (const binder of goals[0].binders ?? []) {
      children.push(binderItem(binder));
    }
  } else {
    goals.forEach((state, index) => {
      const goal = new vscode.TreeItem(
        `目标 ${index + 1}/${goals.length}`,
        vscode.TreeItemCollapsibleState.Expanded,
      );
      goal.description = state.goal;
      goal.iconPath = new vscode.ThemeIcon("circle-outline");
      if (cursor.span) {
        goal.command = {
          command: "sokonanoda.revealRange",
          title: "",
          arguments: [uriString, cursor.span],
        };
      }
      goal.children = (state.binders ?? []).map(binderItem);
      children.push(goal);
    });
  }
  if (cursor.total > 0) {
    const progress = new vscode.TreeItem("by 进度", vscode.TreeItemCollapsibleState.None);
    progress.description = `${cursor.step + 1}/${cursor.total}`;
    progress.iconPath = new vscode.ThemeIcon("list-ordered");
    children.push(progress);
  }
  return children;
}

// Infoview webview (docs/design/webview-infoview.md, 方案 B): a read-only
// presentation of the same server data the trees consume. The extension host
// is the only holder of the LanguageClient, so it pushes structured JSON to
// the webview — never the other way around — and the webview renders it with
// textContent only (CSP nonce, local resources only, no innerHTML). Cursor
// moves push `state` only; `decls` follows diagnostics/file changes (§5).
const INFOVIEW_PROTOCOL = 1;

// Per-load CSP nonce (docs/design/webview-infoview.md §4): unpredictable,
// embedded in both the meta tag and the script tag.
function makeNonce() {
  return crypto.randomBytes(16).toString("base64");
}

function themeKindName() {
  switch (vscode.window.activeColorTheme.kind) {
    case vscode.ColorThemeKind.Light:
      return "light";
    case vscode.ColorThemeKind.HighContrast:
      return "high-contrast";
    case vscode.ColorThemeKind.HighContrastLight:
      return "high-contrast-light";
    default:
      return "dark";
  }
}

// Raw `soko/version` snapshot for the webview's `server` message (the restart
// receipt in `serverVersion` stays a human string).
async function requestServerInfo() {
  if (!client) return { running: false, source: lastResolution?.source };
  try {
    const result = await client.sendRequest("soko/version", {});
    return {
      running: true,
      version: result?.version,
      pid: result?.pid,
      source: lastResolution?.source,
    };
  } catch {
    return { running: false, source: lastResolution?.source };
  }
}

class InfoviewProvider {
  constructor(extensionUri, treeProvider) {
    this._extensionUri = extensionUri;
    this._treeProvider = treeProvider;
    this._view = undefined;
    this._ready = false;
    this._lastState = undefined; // {uri, state} — replayed when the view returns
    this._lastDecls = undefined; // last soko/goals decls (cursor moves don't touch it)
  }

  resolveWebviewView(view) {
    this._view = view;
    view.webview.options = {
      enableScripts: true,
      // Only media/ may be loaded — no remote assets (docs/design §4).
      localResourceRoots: [vscode.Uri.joinPath(this._extensionUri, "media")],
    };
    view.webview.html = this._buildHtml(view.webview);
    view.webview.onDidReceiveMessage((message) => {
      this._onMessage(message).catch(() => {});
    });
    view.onDidChangeVisibility(() => {
      if (view.visible) this._pushAll();
    });
  }

  _buildHtml(webview) {
    const mediaRoot = vscode.Uri.joinPath(this._extensionUri, "media");
    const nonce = makeNonce();
    const scriptUri = webview.asWebviewUri(vscode.Uri.joinPath(mediaRoot, "infoview.js"));
    const styleUri = webview.asWebviewUri(vscode.Uri.joinPath(mediaRoot, "infoview.css"));
    let template;
    try {
      template = fs.readFileSync(
        path.join(this._extensionUri.fsPath, "media", "infoview.html"),
        "utf8",
      );
    } catch {
      // Missing asset must not break activation: the tree stays the fallback
      // (docs/design §6); an empty document simply renders nothing.
      return "<!DOCTYPE html><html><body></body></html>";
    }
    return template
      .replaceAll("{{cspSource}}", webview.cspSource)
      .replaceAll("{{nonce}}", nonce)
      .replaceAll("{{styleUri}}", String(styleUri))
      .replaceAll("{{scriptUri}}", String(scriptUri));
  }

  // Every host message is stamped with the protocol so the webview can drop
  // anything it does not understand.
  _post(payload) {
    if (!this._view) return;
    this._view.webview.postMessage(Object.assign({ protocol: INFOVIEW_PROTOCOL }, payload));
  }

  setState(uri, state) {
    this._lastState = { uri, state };
    if (!this._view || !this._ready) return;
    this._post(Object.assign({ type: "state", uri }, state));
  }

  setDecls(decls) {
    this._lastDecls = decls;
    if (!this._view || !this._ready) return;
    this._post({ type: "decls", decls });
  }

  postTheme() {
    this._post({ type: "theme", kind: themeKindName() });
  }

  async _onMessage(message) {
    if (!message || message.protocol !== INFOVIEW_PROTOCOL) return;
    switch (message.type) {
      case "ready":
        this._ready = true;
        this._pushAll();
        break;
      case "reveal":
        if (typeof message.uri === "string" && message.range) {
          await vscode.commands.executeCommand(
            "sokonanoda.revealRange",
            message.uri,
            message.range,
          );
        }
        break;
      case "focusExercise":
        if (typeof message.name === "string") {
          await this._treeProvider.focusDeclaration(message.name);
          await vscode.commands.executeCommand("sokonanoda.goals.focus");
        }
        break;
    }
  }

  _pushAll() {
    if (!this._view) return;
    this.postTheme();
    this._pushDecls();
    this._pushState();
    this._pushServer();
  }

  _pushState() {
    if (!this._lastState) return;
    const { uri, state } = this._lastState;
    this._post(Object.assign({ type: "state", uri }, state));
  }

  _pushDecls() {
    if (this._lastDecls !== undefined) {
      this._post({ type: "decls", decls: this._lastDecls });
    } else {
      this._treeProvider.ensureDeclarations().catch(() => {});
    }
  }

  async _pushServer() {
    const info = await requestServerInfo();
    this._post(Object.assign({ type: "server" }, info));
  }
}

// `sokonanoda.openInfoview`: reveal the Infoview, which lives in the
// `sokonanoda` container on the right (secondary side bar,
// docs/design/goal-rendering.md §2.3). Focusing a view is enough — it renders
// on demand and the host replays the last snapshot on `ready` — so there is no
// handshake to wait for and nothing to report: an unavailable webview degrades
// silently to the tree's 「当前光标处」 group (§2.4).
async function openInfoview() {
  try {
    await vscode.commands.executeCommand("workbench.view.extension.sokonanoda");
  } catch {
    // Older/headless hosts may not expose the container command; the view
    // focus below still works when the view is already known.
  }
  try {
    await vscode.commands.executeCommand("sokonanoda.infoview.focus");
  } catch {
    // Best-effort: falling back to the tree is the documented behaviour.
  }
}

// Course map (docs/design/course-status.md §2): one node per unit, rendered
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

// Restart the language server in place: re-resolve the binary path first so a
// rebuilt or freshly downloaded server (or a changed `serverPath` setting)
// takes effect without reloading the window. Extension-code updates still need
// a window reload — a running extension host cannot swap itself.
// 问服务器自述（版本 + 进程号）。重启前后各问一次，旧 pid 消失、新 pid 出现，
// 「旧进程退出、新进程确实是新版本」就是可验证的事实而非口头保证——扩展更新
// 之后跑着旧版服务器是最常见的困惑（docs/vscode-dev-guide.md §5.6）。
async function serverVersion() {
  if (!client) return undefined;
  try {
    const result = await client.sendRequest("soko/version", {});
    return `${result?.version ?? "?"} (pid ${result?.pid ?? "?"})`;
  } catch {
    return undefined;
  }
}

// Resolve the language-server command for a fresh start, including the
// version-pinned download fallback for the universal package. Shared by
// activation and `sokonanoda: restart server` so a stale cache is never
// silently reused: re-resolution rejects a stale marker (server.js
// `cachedServerIsCurrent`), and this then fetches the release pinned to the
// extension version instead of leaving the old command in place.
//
// Returns `{command, source}`, or `undefined` after reporting why (missing
// explicit path / unsupported platform / download failure). `source` is one of
// `bundled | setting | env | workspace | cache | download` and feeds the
// restart receipt and the doctor report.
async function resolveServerForStart(context) {
  const override = serverOverrideEnabled();
  const requested = requestedServerCommand();
  // Bundled-first policy: a configured override is ignored (once) unless the
  // contributor explicitly opted in.
  if (!override && requested !== undefined) noticeIgnoredOverride();

  const resolved = await resolveServerCommand(context);
  if (resolved.command !== undefined) {
    // Under override, a typo'd path must be reported instead of silently
    // falling through to a download.
    if (
      (resolved.source === "setting" || resolved.source === "env") &&
      !fs.existsSync(resolved.command)
    ) {
      vscode.window.showErrorMessage(
        `sokonanoda：指定的语言服务器不存在：${resolved.command}（检查 sokonanoda.serverPath 或 SOKONANODA_LSP_BIN）`,
      );
      return undefined;
    }
    lastResolution = resolved;
    return resolved;
  }

  // Every platform package bundles the server; nothing resolved here means an
  // unsupported platform or the universal fallback package (no bundled bin).
  if (!server.platformTarget(process.platform, process.arch)) {
    vscode.window.showErrorMessage(
      `sokonanoda：当前平台（${process.platform}-${process.arch}）没有内置语言服务器。` +
        "请设置 sokonanoda.serverPath 指向本地二进制。",
    );
    return undefined;
  }
  vscode.window.showInformationMessage(
    `sokonanoda：未找到内置语言服务器（universal 包或安装损坏），` +
      `正在按 v${extensionVersion(context)} 回退下载…`,
  );
  try {
    const command = await server.downloadLspBinary({
      version: extensionVersion(context),
      platform: process.platform,
      arch: process.arch,
    });
    if (command && fs.existsSync(command)) {
      vscode.window.showInformationMessage("sokonanoda：语言服务器就绪 ✓");
      lastResolution = { command, source: "download" };
      return lastResolution;
    }
    throw new Error("download produced no binary");
  } catch (err) {
    vscode.window.showWarningMessage(
      "sokonanoda-lsp 回退下载失败。请安装对应平台的插件包（VS Code 会自动选择），" +
        "或设置 sokonanoda.serverPath 指向本地二进制。错误：" + (err?.message ?? err),
    );
    return undefined;
  }
}

// ── Doctor (docs/design/extension-server-policy.md §3) ─────────────────────
// A read-only self-check: it NEVER changes settings and NEVER deletes files.
// It reports physical facts (resolved source/command, server/host/cache
// versions, ignored overrides, stale installs) and, on an error-level finding,
// raises a non-blocking notification with a "运行 doctor" button. Returns the
// report text so callers/tests can assert on it.
const DOCTOR_CHECKS = [
  "resolution",
  "server-version",
  "host-version",
  "ignored-override",
  "download-cache",
  "obsolete-versions",
];

function doctorOutput(context) {
  if (!doctorChannel) {
    doctorChannel = vscode.window.createOutputChannel("sokonanoda doctor");
    context?.subscriptions?.push(doctorChannel);
  }
  return doctorChannel;
}

async function runDoctor(context, { notify = true, show = false } = {}) {
  const lines = [];
  const issues = [];
  let extVersion;
  try {
    extVersion = extensionVersion(context) ?? "?";
    lines.push(`sokonanoda doctor — 扩展 v${extVersion} (${DOCTOR_CHECKS.length} checks)`);

    // 1. resolution result and source.
    const override = serverOverrideEnabled();
    let dry = { command: undefined, source: undefined };
    try {
      dry = await resolveServerCommand(context);
    } catch (error) {
      lines.push(`[warn] 1/6 ${DOCTOR_CHECKS[0]}：解析失败 — ${error?.message ?? error}`);
    }
    const source = lastResolution?.source ?? dry.source;
    const command = lastResolution?.command ?? dry.command;
    if (command) {
      lines.push(`[ok] 1/6 ${DOCTOR_CHECKS[0]}：${source} — ${command} (source=${source ?? "?"})`);
    } else if (server.platformTarget(process.platform, process.arch)) {
      lines.push(
        `[info] 1/6 ${DOCTOR_CHECKS[0]}：无内置/缓存服务器，将按 v${extVersion} 下载 (source=download)`,
      );
    } else {
      const text = `1/6 ${DOCTOR_CHECKS[0]}：平台 ${process.platform}-${process.arch} 无内置服务器且无下载构建 (source=unsupported)`;
      lines.push(`[error] ${text}`);
      issues.push({ level: "error", text });
    }

    // 2. running server version vs the extension version.
    const info = await requestServerInfo();
    if (!info.running) {
      lines.push(`[warn] 2/6 ${DOCTOR_CHECKS[1]}：服务器未运行（打开一个 .sokonanoda 文件启动）`);
    } else if (info.version === extVersion) {
      lines.push(
        `[ok] 2/6 ${DOCTOR_CHECKS[1]}：${info.version} (pid ${info.pid}) == 扩展 v${extVersion} (source=${info.source ?? "?"})`,
      );
    } else {
      const hint = override
        ? "serverOverride 已开启"
        : "serverOverride 关闭，本应始终使用内置服务器";
      const text =
        `2/6 ${DOCTOR_CHECKS[1]}：运行 ${info.version} (pid ${info.pid}) != 扩展 v${extVersion}；` +
        `${hint}。请 “Developer: Reload Window” 或清空 sokonanoda.serverPath`;
      lines.push(`[error] ${text}`);
      issues.push({ level: "error", text });
    }

    // 3. extension host version vs newest installed on disk.
    const installed = newestInstalledExtensionVersion(context);
    if (installed && compareVersions(installed, extVersion) > 0) {
      const text = `3/6 ${DOCTOR_CHECKS[2]}：磁盘已安装 v${installed} > 运行 v${extVersion}；执行 “Developer: Reload Window”`;
      lines.push(`[warn] ${text}`);
      issues.push({ level: "warn", text });
    } else {
      lines.push(
        `[ok] 3/6 ${DOCTOR_CHECKS[2]}：运行宿主 v${extVersion}${installed ? ` == 最新安装 v${installed}` : ""}`,
      );
    }

    // 4. overrides that are being (or would be) ignored.
    const requested = requestedServerCommand();
    if (!override && requested !== undefined) {
      const text =
        `4/6 ${DOCTOR_CHECKS[3]}：已设置 ${requested}，但 serverOverride=false，已忽略；` +
        "要覆盖请开启 sokonanoda.serverOverride";
      lines.push(`[warn] ${text}`);
      issues.push({ level: "warn", text });
    } else if (override && requested !== undefined) {
      lines.push(`[ok] 4/6 ${DOCTOR_CHECKS[3]}：serverOverride=true，使用 ${requested}`);
    } else {
      lines.push(`[ok] 4/6 ${DOCTOR_CHECKS[3]}：没有被忽略的 serverPath/env 覆盖`);
    }

    // 5. download cache marker (CLI/universal fallback only).
    let markerVersion;
    try {
      const marker = server.serverVersionMarker();
      markerVersion = fs.existsSync(marker)
        ? String(fs.readFileSync(marker, "utf8")).trim()
        : undefined;
    } catch (error) {
      markerVersion = `unreadable: ${error?.message ?? error}`;
    }
    if (markerVersion === extVersion) {
      lines.push(`[ok] 5/6 ${DOCTOR_CHECKS[4]}：缓存 marker = 扩展 v${extVersion}`);
    } else if (markerVersion === undefined) {
      lines.push(
        `[info] 5/6 ${DOCTOR_CHECKS[4]}：无下载缓存（仅影响 CLI/universal 兜底，不影响内置服务器）`,
      );
    } else {
      lines.push(
        `[info] 5/6 ${DOCTOR_CHECKS[4]}：缓存 marker ${markerVersion} != 扩展 v${extVersion}；可运行 sokonanoda update`,
      );
    }

    // 6. old extension versions piling up (informational).
    try {
      const dir = path.dirname(context.extensionPath);
      let versions = 0;
      let obsolete = 0;
      for (const name of fs.readdirSync(dir)) {
        if (/^sokonanoda-lang\.sokonanoda-\d+\.\d+\.\d+/.test(name)) versions++;
        if (name.includes(".obsolete")) obsolete++;
      }
      lines.push(
        `[info] 6/6 ${DOCTOR_CHECKS[5]}：检测到 ${versions} 个扩展版本快照、${obsolete} 个 .obsolete 条目；完整重启 VS Code 会自动清理`,
      );
    } catch (error) {
      lines.push(`[info] 6/6 ${DOCTOR_CHECKS[5]}：无法读取扩展目录 — ${error?.message ?? error}`);
    }
  } catch (error) {
    const text = `doctor 运行失败：${error?.message ?? error}`;
    lines.push(`[error] ${text}`);
    issues.push({ level: "error", text });
  }

  const report = lines.join("\n");
  try {
    doctorOutput(context).appendLine(report);
    if (show) doctorOutput(context).show(true);
  } catch {
    // Output channel is best-effort; never fail the command.
  }

  const errors = issues.filter((entry) => entry.level === "error");
  if (notify && errors.length > 0) {
    try {
      vscode.window
        .showWarningMessage(
          `sokonanoda doctor 发现 ${errors.length} 个问题：${errors[0].text}`,
          "运行 doctor",
        )
        .then((choice) => {
          if (choice === "运行 doctor") {
            vscode.commands.executeCommand("sokonanoda.doctor");
          }
        });
    } catch {
      // notification is best-effort
    }
  }
  return report;
}

async function restartServer(context) {
  if (!client) {
    vscode.window.showWarningMessage(
      "sokonanoda language server is not running; open a .sokonanoda file to start it.",
    );
    return;
  }
  // Extension-code upgrades only take effect on a window reload, so the
  // bundled server of a freshly installed version cannot be picked up by a
  // server restart. Tell the user instead of silently staying on the old one.
  const current = extensionVersion(context);
  const installed = newestInstalledExtensionVersion(context);
  if (installed && current && compareVersions(installed, current) > 0) {
    vscode.window.showWarningMessage(
      `sokonanoda: 已安装扩展 v${installed}，但当前窗口仍运行 v${current}；` +
        "扩展本体升级需要 “Developer: Reload Window”（restart server 只能重解析服务器二进制）。",
    );
  }
  // 重启前先记录旧进程：restart() 会 stop() 旧客户端（2s 宽限后 SIGTERM/SIGKILL
  // 旧子进程），再从重新解析出的命令启动新进程。
  const before = await serverVersion();
  // Re-resolve through the same path as activation so a rebuilt/updated server
  // takes effect — and so a stale cache triggers a version-pinned download
  // instead of silently restarting the old binary.
  const next = await resolveServerForStart(context);
  if (next === undefined) {
    // resolveServerForStart already surfaced the reason; never restart with the
    // stale command still in `serverOptions`.
    return;
  }
  if (serverOptions) {
    serverOptions.run.command = next.command;
    serverOptions.debug.command = next.command;
  }
  try {
    await client.restart();
  } catch (error) {
    vscode.window.showErrorMessage(
      `sokonanoda: restart server failed — ${error?.message ?? error}`,
    );
    return;
  }
  const after = await serverVersion();
  vscode.window.showInformationMessage(
    `sokonanoda: server restarted${before ? ` — ${before}` : ""} → ${
      after ?? next.command ?? "?"
    }, source=${next.source ?? "?"}`,
  );
  // Post-restart self-check: surface version/source problems automatically.
  await runDoctor(context, { notify: true, show: false });
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
    vscode.commands.registerCommand("sokonanoda.openInfoview", () => openInfoview()),
    vscode.commands.registerCommand("sokonanoda.revealRange", revealRange),
    vscode.commands.registerCommand(
      "sokonanoda.revealHint",
      (uri, declName, declRange) => revealHint(context, uri, declName, declRange),
    ),
    vscode.commands.registerCommand(
      "sokonanoda.restartServer",
      () => restartServer(context),
    ),
    vscode.commands.registerCommand(
      "sokonanoda.doctor",
      () => runDoctor(context, { notify: true, show: true }),
    ),
  );
}

async function activate(context) {
  extensionRoot = context.extensionPath;

  // Resolve the server (bundled bin first; explicit path only with
  // `serverOverride` on; version-pinned download fallback). Same path as
  // `restart server`.
  const resolution = await resolveServerForStart(context);
  if (resolution === undefined) return;
  const command = resolution.command;

  serverOptions = {
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

  // Infoview webview (方案 B): the tree fans its single soko/stateAt snapshot
  // and its soko/goals declaration list out to the webview. Hidden context is
  // released (`retainContextWhenHidden: false`); the provider caches the last
  // state/decls and replays them on the next `ready` (docs/design §5).
  const infoviewProvider = new InfoviewProvider(context.extensionUri, provider);
  provider.onDecls = (decls) => infoviewProvider.setDecls(decls);
  provider.onState = (uriString, state) => infoviewProvider.setState(uriString, state);
  provider.treeView = tree;
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider("sokonanoda.infoview", infoviewProvider, {
      webviewOptions: { retainContextWhenHidden: false },
    }),
    vscode.window.onDidChangeActiveColorTheme(() => infoviewProvider.postTheme()),
  );

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
      // decls follow diagnostics/file changes only (never cursor moves): the
      // webview gets the fresh declaration list through the onDecls hook.
      provider.ensureDeclarations().catch(() => {});
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
  client.outputChannel.appendLine(
    `[client] ready — server synchronized over stdio: ${command} (source=${resolution.source ?? "?"})`,
  );
  // Auto-run the read-only doctor once, after the first soko/version exchange,
  // so version/source problems surface without the user hunting for them.
  await runDoctor(context, { notify: true, show: false });
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
