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
// Notation input (NI-2, docs/design/notation-input.md): `\and` + Tab → `∧`
// (src/abbreviation-rewriter.js); the Tab keybinding only fires while a
// `\`-abbreviation is being typed, so ordinary indentation is never swallowed.
const cp = require("child_process");
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const { LanguageClient, State, TransportKind } = require("vscode-languageclient/node");
const server = require("./server");
const projectTree = require("./project-tree");
// 记法输入法（NI-2，docs/design/notation-input.md）：`\and` + Tab → `∧`，表在
// `src/abbreviations.js`（front::notation_input 的镜像，契约测试钉住）。这里的接线
// 只有两处——命令注册（registerCommands）+ 一句 `notationInput.register`（监听器与
// Tab 的 context key）；状态机与表都在模块里（本文只管 VS Code 接线，规范 §1）。
const notationInput = require("./src/abbreviation-rewriter");

let client;
let serverOptions;
let extensionRoot;
let lastResolution; // {command, source} of the last successful resolveServerForStart

// 例行化 e2e 的诊断日志：`scripts/vscode-e2e.sh` 设 `SOKO_E2E_LOG` 时启用。
// 为什么需要它：扩展宿主的 `console` 在 vscode-test 的输出里**看不到**，
// output channel 的文件也拿不到，而"服务器到底起没起、用的是哪个二进制"是
// e2e 卡住时的第一现场（第一次例行跑就是靠这条线索定位的）。未设环境变量
// 时每处调用只做一次 `undefined` 判断，生产零成本。
const e2eLogPath = process.env.SOKO_E2E_LOG;
function e2eLog(line) {
  if (!e2eLogPath) return;
  try {
    fs.appendFileSync(e2eLogPath, `${new Date().toISOString()} ${line}\n`);
  } catch {
    // 诊断日志写不进去不影响任何功能
  }
}
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

// **未知声明状态不许伪装成"已解决"**（审计 #9，2026-09-25 ✗⇒✓）。
//
// 原来这两支都是"**兜底 = solved**"✗：`open`/`failed` 之外**一律**显示 `solved` +
// `check` 图标 ✓ ⇒ 只要 wire 上多出一个新状态（或字段缺失 ✓），学习者会看到
// **"已解决"而其实没解决** ✗✗ —— 这是**会骗人**的那类显示 bug ✓。
// 修法（AGENTS 验证纪律 §规则 4：「有就渲染」是反模式 ⇒ 缺失时给**可见信号** ✓）：
//   已知三态**逐个显式**映射 ✓；未知 ⇒ **原样回显** + 问号图标 + 一次性 `console.warn` ✓。
// 词表的唯一来源仍是 `query/mod.rs::status_str`（wire 上的 `status` ✓）——
// 这里只做 1:1 映射，不许再发明新词 ✗。
let warnedUnknownStatus = false; // 一次就够，别刷屏 ✓

function warnUnknownStatus(status) {
  if (warnedUnknownStatus) return;
  warnedUnknownStatus = true;
  // eslint-disable-next-line no-console
  console.warn(
    `sokonanoda: 未知的声明状态 ${JSON.stringify(status)} —— 已原样显示；` +
      `多半是扩展与 LSP 版本不一致（wire 上新增了状态词）。`,
  );
}

function statusIcon(status) {
  if (status === "open") return new vscode.ThemeIcon("circle-outline");
  if (status === "failed") return new vscode.ThemeIcon("error");
  if (status === "checked") return new vscode.ThemeIcon("check");
  warnUnknownStatus(status);
  return new vscode.ThemeIcon("question");
}

function statusLabel(status) {
  if (status === "open") return "open";
  if (status === "failed") return "failed";
  if (status === "checked") return "solved";
  warnUnknownStatus(status);
  return typeof status === "string" && status.length > 0 ? status : "(unknown)";
}

class GoalsTreeDataProvider {
  constructor() {
    this._emitter = new vscode.EventEmitter();
    this.onDidChangeTreeData = this._emitter.event;
    this.uri = undefined; // the active .sokonanoda document
    this.openCount = 0;
    this.cursorState = undefined; // {uri, state} from soko/stateAt
    this.cursorRequestSeq = 0; // discards stale soko/stateAt responses
    this._pendingDecls = undefined; // in-flight soko/goals load (concurrency merge)
    this.declItems = undefined; // cached decl TreeItems from the last soko/goals
    // **已经为哪个 URI 取过声明**（E6，计划 T-B10）：不能用 `!this.declItems`
    // 判断——`decls = []` 时 `declItems = []`，而 `![]` 是 false ⇒ 之后所有
    // `ensureDeclarations()` 都变成空转（面板永远停在「等待编译…」）。
    this.onDecls = undefined; // (decls, uri) => void — feeds the Infoview decls message
    this.onState = undefined; // (uri, state) => void — feeds the Infoview state message
    this.onStatus = undefined; // (status) => void — feeds the Infoview status message
  }

  // Full reload: the declarations themselves changed (new diagnostics or a
  // different active document), so drop the cached items and refetch.
  refresh() {
    this.declItems = undefined;
    this._declsUri = undefined;
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
      // 切换活动文档时必须**主动**把声明列表推给 Infoview，不能只靠 `refresh()`
      // 触发的树重建：VS Code 只在 `sokonanoda.goals` 视图**可见**时才会调
      // `getChildren`，而常见的布局是只开着 Infoview 面板（树收在侧栏里）——
      // 那时 `onDecls` 根本没人调，卡片就停在上一份文档上（实测：单文件之间切
      // focus 不更新；项目文件打开也因为不发诊断而完全不更新）。`onDecls` 是
      // Infoview 拿声明的唯一入口，所以在源头补这一下。
      // `loadDeclarations` 有并发合并（`_pendingDecls`），与树自己的那次合并。
      this.ensureDeclarations().catch(() => {});
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
    // **走 `ensureDeclarations()`**，不要在这里再写一遍真值判断：E6 的同一个
    // 错误在这里也有一份（`decls = []` ⇒ `declItems = []` ⇒ `![]` 是 false
    // ⇒ 树永远空着）。一处判据、两处引用。
    await this.ensureDeclarations();
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

  async requestGoals(uri = this.uri) {
    if (!client || !uri) return undefined;
    try {
      const response = await client.sendRequest("soko/goals", {
        textDocument: { uri },
      });
      // 服务端回显了它答的是哪份文档：不是我们问的那份 ⇒ 当作过期答案丢弃
      // （快速切换文件时，树上不能出现另一份文件的声明）。
      if (response && response.uri !== undefined && response.uri !== uri) {
        return undefined;
      }
      return response;
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
      // 同一份身份回显也用在 stateAt 上（目标面板/当前光标处）。
      if (state && state.uri !== undefined && state.uri !== uriString) {
        return undefined;
      }
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
    // 「取过没有」看 **URI**，不看 `declItems` 的真值（E6：空数组是真值）。
    if (this._declsUri !== this.uri) await this.loadDeclarations();
  }

  // Fetch `soko/goals` once per document/diagnostics version and cache the
  // built TreeItems; cursor movement reuses them (see `refreshCursor`).
  //
  // 并发合并：一次诊断事件里 `refresh()` 之后树会自己 resolve 一次，同时
  // `ensureDeclarations()` 也会来一次——原先是两次 `soko/goals`（单文件模式下
  // 每次还带请求期内核探针）。这里把并发调用合并成同一个 promise。
  async loadDeclarations() {
    // **合并按 URI 分键**（E5，计划 T-B09）：合并的初衷是"同一次诊断事件里树和
    // Infoview 各要一次、别发两趟"。但以前只存**一个**在飞的 promise ⇒ A 在飞时
    // 切到 B，`trackEditor(B)` 拿回的是 A 的 promise，而 A 随后因过期提前返回
    // ——**B 的取数从未发生**（面板停在上一份文档上）。按 URI 分键之后，B 有自己的
    // 那一趟；同一个 URI 的并发调用仍然合并。
    const uri = this.uri;
    if (!(this._pendingDecls instanceof Map)) this._pendingDecls = new Map();
    const pending = this._pendingDecls.get(uri);
    if (pending) return pending;
    const started = this._loadDeclarations().finally(() => {
      // 只删自己那一把键：期间可能已经有别的 URI 在飞。
      if (this._pendingDecls instanceof Map && this._pendingDecls.get(uri) === started) {
        this._pendingDecls.delete(uri);
      }
    });
    this._pendingDecls.set(uri, started);
    return started;
  }

  async _loadDeclarations() {
    // 请求发起时就把文档钉住：`soko/goals` 是异步的，期间用户切到别的文件时
    // `this.uri` 会变；若拿响应回来后的 `this.uri` 建节点，树上的行会是 A 的
    // 声明、点击却 `revealRange(B, A 的洞)`（实测复现过）。答案属于旧文档 ⇒ 丢弃。
    const requestedUri = this.uri;
    // Feedback UX: tell the Infoview a fetch is in flight (host → webview
    // `status`), then the resulting declaration count (or `idle` without a
    // document). `soko/goals` is the only slow step in this path.
    if (requestedUri === undefined) this.onStatus?.({ state: "idle" });
    else this.onStatus?.({ state: "loading" });
    const response = await this.requestGoals(requestedUri);
    if (requestedUri !== this.uri) {
      // 过期答案：**丢弃**（不能拿它建节点，否则树上是 A 的声明、点击却跳到 B）。
      // 但**不能就此什么都不做**：用户已经切到别的文档了，而那份文档的声明
      // 还没人取——面板会一直停在上一份的卡片上（E4）。这里补一次取数；
      // 若用户又切走了，下一次同样会补，链条由 `loadDeclarations` 的合并收敛。
      this.loadDeclarations().catch(() => {});
      return;
    }
    const decls = response?.decls ?? [];
    this.openCount = decls.filter((d) => d.status === "open").length;
    updateStatusBar(this);
    // **取数失败不算"取过了"**：`requestGoals` 在客户端还没起来 / 请求出错时
    // 返回 `undefined`。激活时活动编辑器已经是 `.sokonanoda` 就是这种情况——
    // 若把它记成"取过了"，E6 的空转就换个形式回来了（面板停在「等待编译…」，
    // 直到下一次诊断事件）。
    if (response === undefined) {
      // 取数没成功：**不能假装"没有声明"**（T-B12 的三态）。区分两种原因——
      // 客户端还没起来是正常的启动中间态（"编译中"），否则是真的失败了。
      this.onStatus?.({ state: client ? "error" : "loading" });
      return;
    }
    this._declsUri = requestedUri;
    this.declItems = decls.map((decl) => {
      const item = new vscode.TreeItem(decl.name, decl.status === "open"
        ? vscode.TreeItemCollapsibleState.Expanded
        : vscode.TreeItemCollapsibleState.None);
      item.description = `${decl.kind} · ${statusLabel(decl.status)}`;
      item.iconPath = statusIcon(decl.status);
      // **声明的值**（T-D52 / 用户第 8 条反馈）：`def` 的 `:=` 之后那一行。
      // Infoview 卡片里那一行是**主**展示（`media/infoview.js`）；树这边放
      // tooltip——树的行高是固定的，多一行会把"一行一声明"的节奏打散。
      if (typeof decl.value === "string" && decl.value !== "") {
        item.tooltip = codeMarkdown(`${decl.ty ?? ""}\n:= ${decl.value}`);
      }
      if (decl.status === "open") {
        item.contextValue = "openExercise";
        // 请求发起时钉住的 URI（`requestedUri`），**不是** `this.uri`：
        // 切文件竞态的修复（docs/vscode-dev-guide.md 坑 15）在 0.57.0；另一条线
        // （0.56.2）这里还是旧的 `this.uri`。合并时保留修好的那版。
        item.children = buildOpenChildren(decl, requestedUri);
        // soko/goals holes are `{range, id, redundant}` objects
        // (docs/protocol.md) — the client reads positions through
        // hole.range, never bare.
        const hole = decl.holes?.[0];
        if (hole) {
          item.command = {
            command: "sokonanoda.revealRange",
            title: "",
            arguments: [requestedUri, hole.range],
          };
        }
      }
      return item;
    });
    this.onDecls?.(decls, requestedUri);
    this.onStatus?.({
      state: requestedUri === undefined ? "idle" : "ready",
      decls: decls.length,
    });
  }
}

// 诊断事件全窗口共享；只有这些 URI 属于本扩展。
function isSokonanodaFile(uri) {
  return uri?.scheme === "file" && String(uri.fsPath ?? "").endsWith(".sokonanoda");
}

// `decls` 是整表重建的输入；用一个便宜且稳定的指纹判断"内容真的变了吗"。
// 字段顺序固定，避免同一份数据因键序不同而误判为新内容。
function declsFingerprint(decls) {
  if (!Array.isArray(decls)) return "[]";
  // **类型文本与行号也要进指纹**（E8，计划 T-B11）：只含
  // `name/kind/status/holes[0].id` 时，"只改了类型"的更新会被判成"没变"
  // ⇒ 卡片上的 `ty` 与 `L<n>` 停在旧值（而树里的行已经重建过了）。
  //
  // 为什么用 `ty_runs` 的长度 + 首尾文本而不是整条 `ty`：一次诊断事件里
  // 50 条声明的完整类型串起来是几十 KB，而这个指纹每次 `setDecls` 都要算。
  // 首尾 + 长度足以发现"类型变了"（改一个标识符/一层箭头都会动到首或尾），
  // 又不会把每次按键都变成一次大字符串比较。
  const tyDigest = (decl) => {
    const runs = Array.isArray(decl?.ty_runs) ? decl.ty_runs : [];
    if (runs.length > 0) {
      const first = String(runs[0]?.text ?? "");
      const last = String(runs[runs.length - 1]?.text ?? "");
      return [runs.length, first, last];
    }
    const ty = String(decl?.ty ?? "");
    return [0, ty.slice(0, 24), ty.slice(-24)];
  };
  return JSON.stringify(decls.map((decl) => [
    decl?.name ?? null,
    decl?.kind ?? null,
    decl?.status ?? null,
    decl?.holes?.[0]?.id ?? null,
    // 行号提示（`L<n>`）也跟着 span 走。
    decl?.range?.start?.line ?? null,
    tyDigest(decl),
  ]));
}

// Every place the extension renders `.sokonanoda` text uses the same language
// id, so tooltips are highlighted by the same grammar as hover/completion/
// Infoview (docs/design/goal-rendering.md §7).
function codeMarkdown(text) {
  const md = new vscode.MarkdownString();
  md.appendCodeblock(String(text ?? ""), "sokonanoda");
  return md;
}

function goalTooltip(state) {
  const lines = [];
  for (const binder of state?.binders ?? []) {
    lines.push(`${binder.name} : ${binder.ty}`);
  }
  lines.push(`⊢ ${state?.goal ?? ""}`);
  return codeMarkdown(lines.join("\n"));
}

function binderItem(binder) {
  const item = new vscode.TreeItem(binder.name, vscode.TreeItemCollapsibleState.None);
  item.description = binder.ty;
  item.tooltip = codeMarkdown(`${binder.name} : ${binder.ty}`);
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
    goal.tooltip = codeMarkdown(ty);
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
    title: "Reveal Hint (揭示下一条提示)",
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
    goal.tooltip = goalTooltip(goals[0]);
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
      goal.tooltip = goalTooltip(state);
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
    this._lastStatus = undefined; // last {state, decls?} progress snapshot
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

  // **Infoview 字号**（2026-09-26 用户反馈「字太小太暗」）：配置**每条消息都带**
  // ⇒ 改设置后下一条状态就生效，不用重开面板 ✓。webview 侧把它落到
  // `--soko-font-scale` 上（`media/infoview.js`）✓。
  _fontScale() {
    const value = vscode.workspace
      .getConfiguration("sokonanoda")
      .get("infoview.fontScale");
    return typeof value === "number" && isFinite(value) && value > 0 ? value : 1;
  }

  setState(uri, state) {
    this._lastState = { uri, state };
    if (!this._view || !this._ready) return;
    this._post(
      Object.assign({ type: "state", uri, fontScale: this._fontScale() }, state),
    );
  }

  // ---- 测试可见的只读访问器（计划 T-016）--------------------------------
  //
  // 为什么需要它们：VS Code 的测试 API **拿不到 webview 的 DOM**，所以 e2e 没法
  // 直接断言"面板上画了几个卡片"。分工是：
  //   * e2e 断言**载荷**（真 LSP → 真扩展 → 这里存的这两个字段）；
  //   * `test-extension-host.js` / `test-webview.js` 断言**渲染**（载荷 → DOM）。
  // 两者合起来才是"面板真的显示了"。生产路径零行为变化（只是把已有字段读出来）。
  lastDecls() {
    return Array.isArray(this._lastDecls) ? this._lastDecls : [];
  }

  lastState() {
    return this._lastState ? this._lastState.state : null;
  }

  setDecls(decls) {
    this._lastDecls = decls;
    // Webview 的 `decls` 是整表重建（50 条 ≈ 1200 个 DOM 节点）：内容没变就别发。
    // 一次诊断事件原本会发两遍（树 + ensureDeclarations 各一次）。
    const fingerprint = declsFingerprint(decls);
    if (fingerprint === this._lastDeclsFingerprint) return;
    this._lastDeclsFingerprint = fingerprint;
    if (!this._view || !this._ready) return;
    this._post({ type: "decls", decls });
  }

  // Progress/feedback snapshot (host → webview `status`): the webview shows a
  // skeleton until the first one arrives, so the panel is never silently blank.
  setStatus(status) {
    this._lastStatus = status;
    if (!this._view || !this._ready) return;
    this._post(Object.assign({ type: "status" }, status));
  }

  // **编译进度**（P3 主机侧，2026-09-26）：与 `setStatus` 同款 —— **先存再发**，
  // 面板重开（`retainContextWhenHidden` 默认 false ⇒ webview 会被重建）时由
  // `_pushAll` 回放 ✓，否则进度块在重开后**凭空消失** ✗（S2 调研的 T8）。
  setProgress(progress) {
    this._lastProgress = progress;
    if (!this._view || !this._ready) return;
    this._post(Object.assign({ type: "progress" }, progress));
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
    }
  }

  _pushAll() {
    if (!this._view) return;
    this.postTheme();
    this._pushDecls();
    this._pushState();
    this.postStatus();
    this.postProgress();
    this.postServer();
  }

  postProgress() {
    if (!this._lastProgress) return;
    this._post(Object.assign({ type: "progress" }, this._lastProgress));
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

  // Replay the last status snapshot; before any snapshot the webview keeps its
  // own `正在渲染…` skeleton.
  postStatus() {
    if (this._lastStatus === undefined) return;
    this._post(Object.assign({ type: "status" }, this._lastStatus));
  }

  // Public (fire-and-forget) server snapshot, used by activation to move the
  // panel's server line from "启动中…" to the resolved {version, pid}.
  postServer() {
    this._pushServer();
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
  // The Infoview lives in the `sokonanoda` container on the right (secondary
  // side bar). Revealing it takes three best-effort steps: make sure the
  // auxiliary bar is shown, reveal the container, then focus the view.
  for (const command of [
    "workbench.action.focusAuxiliaryBar",
    "workbench.view.extension.sokonanoda",
    "sokonanoda.infoview.focus",
  ]) {
    try {
      await vscode.commands.executeCommand(command);
    } catch {
      // Older/headless hosts may not expose some of these; the tree's
      // 「当前光标处」 group remains the fallback.
    }
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

// Course map v2 (ledger G-07, docs/design/course-manifest-v2.md §4.3): a v2
// manifest (`soko.course/2`) makes the CLI add `volume`/`chapter`/`tags` to
// every `course.unit` event, and the tree then renders 卷 → 章 → 单元.
//
// The switch is **data-driven, not manifest-driven**: the client only ever
// reads events (docs/design/course-status.md §0 — aggregation lives in the
// CLI), so "does any event carry a volume?" *is* the CLI's verdict. A v1
// manifest produces no such field and keeps the flat tree byte for byte.
function courseHasVolumes(units) {
  return units.some((unit) => unit && typeof unit.volume === "object" && unit.volume !== null);
}

function courseVolumeItem(volume, children) {
  const item = new vscode.TreeItem(
    `卷 ${volume.id ?? ""} ${volume.title ?? ""}`.trim(),
    vscode.TreeItemCollapsibleState.Collapsed,
  );
  const chapters = new Set(
    children.map((child) => child.__chapterId).filter((id) => id !== undefined),
  );
  item.description = `${chapters.size} 章 · ${children.length} 单元`;
  item.iconPath = new vscode.ThemeIcon("book");
  item.tooltip = `${volume.title ?? volume.id ?? "卷"} · ${chapters.size} 章 · ${children.length} 单元`;
  item.children = children;
  return item;
}

function courseChapterItem(chapter, children) {
  const item = new vscode.TreeItem(
    `${chapter.id ?? ""} ${chapter.title ?? ""}`.trim(),
    vscode.TreeItemCollapsibleState.Collapsed,
  );
  const tags = Array.isArray(chapter.tags) ? chapter.tags.join(" · ") : "";
  const prereqs = Array.isArray(chapter.prereqs) && chapter.prereqs.length
    ? `先修 ${chapter.prereqs.join("、")}`
    : "";
  item.description = [tags, prereqs].filter(Boolean).join(" · ");
  item.iconPath = new vscode.ThemeIcon("bookmark");
  item.tooltip = item.description
    ? `${chapter.title ?? chapter.id ?? "章"}（${item.description}）`
    : `${chapter.title ?? chapter.id ?? "章"}`;
  item.children = children;
  return item;
}

// 有单元读不出来（`error`）时它仍要在树上有个位置：v2 分组下挂在一个兜底节点，
// v1 平铺下照旧直接是根的子节点。
function courseErrorItem(count) {
  const item = new vscode.TreeItem(
    `无法分组（${count}）`,
    vscode.TreeItemCollapsibleState.Collapsed,
  );
  item.description = "清单/单元读不出来";
  item.iconPath = new vscode.ThemeIcon("warning");
  item.children = [];
  return item;
}

// 卷 → 章 → 单元：按事件的**文档序**分组（CLI 按清单顺序发事件）。
function courseVolumeGroups(units, manifestDir) {
  const volumes = [];
  const byVolume = new Map();
  const ungrouped = [];
  for (const unit of units) {
    if (!unit || typeof unit.volume !== "object" || unit.volume === null) {
      ungrouped.push(unit);
      continue;
    }
    const volumeId = String(unit.volume.id ?? "");
    if (!byVolume.has(volumeId)) {
      const volume = { ref: unit.volume, chapters: [], byChapter: new Map() };
      byVolume.set(volumeId, volume);
      volumes.push(volume);
    }
    const volume = byVolume.get(volumeId);
    const chapter = unit.chapter && typeof unit.chapter === "object" ? unit.chapter : null;
    const chapterId = chapter ? String(chapter.id ?? "") : "";
    if (!volume.byChapter.has(chapterId)) {
      const entry = { ref: chapter, units: [] };
      volume.byChapter.set(chapterId, entry);
      volume.chapters.push(entry);
    }
    volume.byChapter.get(chapterId).units.push(unit);
  }

  const roots = volumes.map((volume) =>
    courseVolumeItem(
      volume.ref,
      volume.chapters.map((entry) => {
        const chapter = courseChapterItem(
          entry.ref ?? {},
          entry.units.map((unit) => courseUnitItem(unit, manifestDir)),
        );
        chapter.__chapterId = entry.ref ? String(entry.ref.id ?? "") : "";
        return chapter;
      }),
    ),
  );
  if (ungrouped.length) {
    const fallback = courseErrorItem(ungrouped.length);
    fallback.children = ungrouped.map((unit) => courseUnitItem(unit, manifestDir));
    roots.push(fallback);
  }
  return roots;
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

// 课程树的一次完整刷新 = 一个 CLI 进程编译 11 个单元（实测 release 热缓存
// ~320ms）。树在结果回来前是空的，而 VS Code 会在展开/可见性变化时重新 resolve
// 根节点——没有缓存就会反复起进程、反复空白。30s 内的重复 resolve 直接复用结果，
// `sokonanoda.courseRefresh`（refresh()）与激活时强制重跑。
const COURSE_CACHE_MS = 30000;

class CourseTreeDataProvider {
  constructor() {
    this._emitter = new vscode.EventEmitter();
    this.onDidChangeTreeData = this._emitter.event;
    this.manifestDir = undefined;
    this.units = [];
    this._pending = undefined;
    this._loadedAt = 0;
  }

  refresh() {
    this._loadedAt = 0; // 显式刷新（命令/激活）⇒ 丢掉缓存
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
    if (this._pending) return this._pending;
    if (this._loadedAt !== 0 && Date.now() - this._loadedAt < COURSE_CACHE_MS) {
      return this.units;
    }
    this._pending = this.runCourse().finally(() => {
      this._pending = undefined;
      this._loadedAt = Date.now();
    });
    return this._pending;
  }

  async runCourse() {
    this.units = [];
    const manifest = resolveCourseManifest();
    if (!manifest) return this.units; // no course manifest -> empty tree
    const units = await runCourseCommand(resolveCliCommand(), manifest);
    this.manifestDir = path.dirname(manifest);
    // v2 (any event carries a volume) ⇒ 卷 → 章 → 单元；v1 ⇒ 平铺（不许回归）。
    this.units = courseHasVolumes(units)
      ? courseVolumeGroups(units, this.manifestDir)
      : units.map((unit) => courseUnitItem(unit, this.manifestDir));
    return this.units;
  }
}

let statusBar;
let projectProvider;
let goalProvider;
/// Infoview provider（`activate()` 里建，模块级留一个引用）：编译进度要推给它 ✓
/// （`goalProvider` 同款的理由：进度是**模块级**回调在推，不是 `activate` 的局部）✓。
let infoviewProvider;
// 状态栏 tooltip 里的项目那一行（由 `soko/project` 的答案派生）。
let projectStatusLine;

/// 项目状态摘要：`项目：<根>（清单/零配置）· N 模块 · M 失败`。
function projectStatusText(answer) {
  const project = answer?.project;
  if (!project) {
    const reason = answer?.reason;
    if (reason === "parse-error") return "项目：先修语法错误";
    if (reason === "no-path") return "项目：有 import，但没有入口路径";
    return "项目：单文件（无 import）";
  }
  const manifest = projectTree.manifestSource(project);
  return `项目：${project.root}（${manifest}）· ${projectTree.projectSummary(project)}`;
}

/// 取回项目视图（只读请求；服务器不可用时保留上一次的视图，不假装"没有项目"）。
async function loadProject() {
  if (!projectProvider || !client) {
    e2eLog(`loadProject skipped (provider=${!!projectProvider} client=${!!client})`);
    return;
  }
  const uri = projectProvider.uri;
  if (!uri) {
    e2eLog("loadProject: no active .sokonanoda document");
    projectStatusLine = undefined;
    updateStatusBar(goalProvider);
    return;
  }
  try {
    const answer = await client.sendRequest("soko/project", {
      textDocument: { uri },
    });
    if (!projectProvider.setAnswer(uri, answer)) {
      e2eLog(`soko/project answer for another document: ${answer?.uri}`);
      return;
    }
    projectStatusLine = projectStatusText(answer);
    e2eLog(
      `soko/project ok: project=${answer?.project ? `${answer.project.entry} (${answer.project.counts?.modules} modules)` : "null"} reason=${answer?.reason ?? "null"}`,
    );
  } catch (error) {
    e2eLog(`soko/project failed for ${uri}: ${error?.message ?? error}`);
    return;
  }
  updateStatusBar(goalProvider);
}

/// `$/progress` 的载荷 → webview 的消息（`{phase, label, percent}`）。
///
/// **必须显式翻译，不许把 `kind` 原样传**：webview 侧 `renderProgress` 认不出
/// `phase` 就**静默删块**（"有就渲染"的反模式 ⇒ 键名写错=看不见 ✗，S2 调研的 T6）
/// ⇒ 键名由这一处**唯一**决定，并由宿主层判据钉住 ✓。
function translateProgress(value) {
  const kind = value && value.kind;
  const percent =
    value && typeof value.percentage === "number" ? value.percentage : null;
  if (kind === "begin") {
    return { phase: "begin", label: value.message || value.title || "编译中…", percent };
  }
  if (kind === "report") {
    return { phase: "report", label: value.message || "编译中…", percent };
  }
  if (kind === "end") {
    return { phase: "end", label: value.message || null, percent: null };
  }
  return null;
}

/// **P6 节流**：`report` 可能很密（长文件编译）⇒ 窗口内只刷**最后一次** ✓。
/// `begin`/`end` **绝不节流**：它们是成对的状态边界，漏一个界面就卡住 ✗。
function progressThrottleMs() {
  const value = vscode.workspace
    .getConfiguration("sokonanoda")
    .get("progress.throttleMs");
  return typeof value === "number" && isFinite(value) && value >= 0 ? value : 250;
}

let progressTimer = null;
let pendingProgress = null;

function onCompileProgress(value) {
  const info = translateProgress(value);
  if (!info) return;
  if (info.phase === "report") {
    pendingProgress = info;
    if (progressTimer) return;
    progressTimer = setTimeout(() => {
      progressTimer = null;
      const latest = pendingProgress;
      pendingProgress = null;
      if (latest) applyProgress(latest);
    }, progressThrottleMs());
    return;
  }
  applyProgress(info);
}

/// **P4（空间进度，2026-09-26 用户需求）**：状态栏只说"在编"，不说"**在哪编**" ✓。
/// 编译期间给当前文档加一层**整行装饰** ⇒ 编辑器右侧**概览尺**（overview ruler）
/// 与行背景同时亮起 ✓。
///
/// **刻意不用 `gutterIconPath`**：那要往扩展里打包一张 svg ✗；`overviewRulerColor`
/// 与 `backgroundColor` 都是纯声明式的，**零资源** ✓。
/// 范围 = **整个文档** —— 与当前的进度粒度（"这一份在编"）一致 ✓，别假装更精确 ✗。
let compileDecorationType = null;

function ensureCompileDecoration() {
  if (compileDecorationType) return compileDecorationType;
  compileDecorationType = vscode.window.createTextEditorDecorationType({
    isWholeLine: true,
    // `OverviewRulerLane.Right` —— 与 VS Code 自己的诊断同一侧 ✓。
    overviewRulerColor: new vscode.ThemeColor("progressBar.background"),
    overviewRulerLane: vscode.OverviewRulerLane.Right,
    backgroundColor: new vscode.ThemeColor("editor.wordHighlightBackground"),
  });
  return compileDecorationType;
}

function setCompileDecorations(on) {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !editor.document) return;
  // **成对**：`end` 必须把装饰**清空**（`setDecorations(type, [])`）—— 否则那条
  // 高亮会永远留在文件上 ✗（进度通知最常见的第二类 bug ✓）。
  const text = editor.document.getText();
  const ranges = on
    ? [new vscode.Range(editor.document.positionAt(0), editor.document.positionAt(text.length))]
    : [];
  if (typeof editor.setDecorations === "function") {
    editor.setDecorations(ensureCompileDecoration(), ranges);
  }
}

function applyProgress(info) {
  if (info.phase === "begin") {
    statusBarCompiling = true;
    updateStatusBar(goalProvider);
    setCompileDecorations(true);
  } else if (info.phase === "end") {
    statusBarCompiling = false;
    updateStatusBar(goalProvider);
    setCompileDecorations(false);
    // 收工时把进度块**清干净**（webview 侧 `end` 会删块 ✓）；同时别让
    // 下一次面板重开回放出一块**已经结束**的进度 ✗（T8 的边界）。
    infoviewProvider?.setProgress({ phase: "end", label: null, percent: null });
    return;
  }
  infoviewProvider?.setProgress(info);
}

/// **编译中**（P2，2026-09-26 用户需求）：慢文件编译时状态栏先动起来 ✓。
/// **必须排在"没文档就 hide"之前**：编译可能在文档刚关掉时还在跑，这时
/// 状态栏该说"在编"，不是消失 ✓。`end` 一到就退回原来的两态 ✓。
let statusBarCompiling = false;

function updateStatusBar(provider) {
  if (!statusBar) return;
  if (statusBarCompiling) {
    statusBar.text = "$(sync~spin) Sokonanoda: 编译中…";
    statusBar.tooltip = new vscode.MarkdownString(
      "正在编译当前文件所在的 import 闭包（长文件第一次编会比较久）。",
    );
    statusBar.show();
    return;
  }
  if (!provider || provider.uri === undefined) {
    // 顺手清文本：隐藏项留着上一次的文本，会让"状态栏现在说什么"变成假象 ✗
    // （测试与人都可能读到它）✓。
    statusBar.text = "";
    statusBar.hide();
    return;
  }
  // **两个"N 练习"必须各自说出范围**（审计 #21，2026-09-25 ✗⇒✓）：
  // 这一个数是**当前文件**的（`decls.filter(status === "open")` ✓），
  // 而项目树/`projectStatusLine` 那个是**整个闭包**的（`counts.open_exercises` ✓）——
  // 原来两处都只写"练习"✗ ⇒ 同一窗口两个数、**用户无法分辨** ✓。这里把范围写进
  // **文本**（不是只写进 tooltip ✓：状态栏本来就只有一瞥的时间 ✓）。
  statusBar.text = `$(circle-outline) 本文件 ${provider.openCount}`;
  const lines = [
    `sokonanoda：**本文件**还有 ${provider.openCount} 个练习未完成（点击查看练习面板）`,
  ];
  if (projectStatusLine) {
    lines.push(`下面这行是**整个项目**的：`);
    lines.push(projectStatusLine);
  }
  statusBar.tooltip = new vscode.MarkdownString(lines.join("\n\n"));
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
// activation and `Sokonanoda: Restart Server (重启服务器)` so a stale cache is never
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

// ── build / rebuild（编译缓存预热；docs/design/compile-cache.md）─────────────
// `sokonanoda build [--clean] [<file>|<dir> ...]` 预热**共享的持久编译缓存**，
// 让随后的 check/course/LSP 都是命中（crates/cli/src/build.rs，事件契约见
// docs/protocol.md 的 build.file / build.clean / build.summary）。
// 为什么值得做成命令：① 第一次按键慢、② 在编辑器外改了依赖文件后面板像是
// "没反应"，这两件事的答案都是"把项目 build 一遍"；而 CLI 一直有这个子命令，
// 只是扩展没把它接出来（用户报的就是这个缺口）。`rebuild` = `build --clean`
// （清缓存）+ `build`（重新预热），也就是"从头重编译一遍"。
// 作用域：有活动 .sokonanoda 文件就编它（CLI 会顺着 import 编整个闭包），
// 否则编第一个工作区文件夹（CLI 递归遍历目录）。
const BUILD_TIMEOUT_MS = 300000;
let buildChannel;

function buildOutput(context) {
  if (!buildChannel) {
    buildChannel = vscode.window.createOutputChannel("sokonanoda build");
    context?.subscriptions?.push(buildChannel);
  }
  return buildChannel;
}

/// build 的目标路径：活动 .sokonanoda 文件 → 第一个工作区文件夹 → undefined。
function buildTarget() {
  const editor = vscode.window.activeTextEditor;
  if (
    editor &&
    editor.document.languageId === "sokonanoda" &&
    editor.document.uri.scheme === "file"
  ) {
    return editor.document.uri.fsPath;
  }
  const folder = (vscode.workspace.workspaceFolders ?? [])[0];
  return folder?.uri?.fsPath;
}

/// 子进程纪律与课程树同款：stdout 是 JSON Lines、stderr 进输出面板、
/// 有界运行（超时就 kill），**永不阻塞**（build 失败不是异常路径）。
function runBuildProcess(command, args, channel) {
  return new Promise((resolve) => {
    let stdout = "";
    let child;
    try {
      child = cp.spawn(command, args);
    } catch (error) {
      resolve({ code: -1, stdout: "", error: String(error?.message ?? error) });
      return;
    }
    const timer = setTimeout(() => {
      channel?.appendLine(`[timeout] ${command} ${args.join(" ")} (> ${BUILD_TIMEOUT_MS}ms)`);
      child.kill();
      resolve({ code: -1, stdout, error: `timeout after ${BUILD_TIMEOUT_MS}ms` });
    }, BUILD_TIMEOUT_MS);
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
      for (const line of chunk.split("\n")) {
        if (line.trim()) channel?.appendLine(line.trimEnd());
      }
    });
    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk) => channel?.appendLine(String(chunk).trimEnd()));
    child.on("error", (error) => {
      clearTimeout(timer);
      resolve({ code: -1, stdout, error: String(error?.message ?? error) });
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({ code: code ?? -1, stdout });
    });
  });
}

/// 只认 JSON Lines：非 JSON 行是噪声，不是 build 结果。
function parseBuildEvents(stdout) {
  const events = [];
  for (const line of String(stdout).split("\n")) {
    const text = line.trim();
    if (!text.startsWith("{")) continue;
    try {
      events.push(JSON.parse(text));
    } catch {
      // 非 JSON 行：忽略（CLI 的诊断走 stderr）
    }
  }
  return events;
}

/// T-A52：`sokonanoda.warmCacheOnOpen`（**默认关**）——激活时后台把工作区根
/// build 一遍，让"第一次打开某个单元"直接命中缓存（T-A10/T-A11 的预热）。
///
/// **为什么默认关**（用户 2026-09-21 拍板「做，但默认关」）：它占 CPU/IO，而
/// "打开编辑器"本身会因此变慢——那正是另一面的抱怨。默认关 = 想要的人自己开。
///
/// 三条纪律：**不弹输出面板**（后台活不该抢注意力，进度写进 channel）、
/// **不报错**（失败只是没预热，`sokonanoda: doctor` 能查）、**不阻塞**
/// （`activate()` 不等它；`build` 是独立子进程，超时由 `runBuildProcess` 兜）。
async function warmCacheOnOpen(context) {
  const enabled = vscode.workspace
    .getConfiguration("sokonanoda")
    .get("warmCacheOnOpen", false);
  if (enabled !== true) return;
  const target = (vscode.workspace.workspaceFolders ?? [])[0]?.uri?.fsPath;
  if (!target) return;
  let command;
  try {
    command = resolveCliCommand();
  } catch {
    command = undefined;
  }
  if (!command) return;
  const channel = buildOutput(context);
  channel.appendLine(`> ${command} build --json ${target}（warmCacheOnOpen）`);
  const result = await runBuildProcess(command, ["build", "--json", target], channel);
  channel.appendLine(`[warmCacheOnOpen] exit=${result.code}`);
  // 记账（`SOKO_E2E_LOG`）：真宿主用例靠这一行证明"接线真的跑了"，而不用去猜
  // 缓存里那条目是谁写的。
  e2eLog(`warmCacheOnOpen: exit=${result.code}`);
}

/// 跑一次 build/rebuild；返回给用户/测试看的摘要文本（跑不起来时 undefined）。
/// 会刷新三个视图：build 改变了缓存状态，"面板像是没反应"正是它要回答的问题。
async function runBuild(context, { clean = false, courseProvider } = {}) {
  const channel = buildOutput(context);
  channel.show(true);
  const target = buildTarget();
  if (!target) {
    vscode.window.showWarningMessage(
      "sokonanoda: 先打开一个 .sokonanoda 文件或一个工作区文件夹，再 build。",
    );
    return undefined;
  }
  let command;
  try {
    command = resolveCliCommand();
  } catch {
    command = undefined;
  }
  if (!command) {
    vscode.window.showErrorMessage(
      "sokonanoda: 找不到 CLI（build 需要它；VSIX 自带，其他情况见 sokonanoda doctor）。",
    );
    return undefined;
  }

  const started = Date.now();
  channel.appendLine(`> ${command} build ${clean ? "--clean " : ""}${target}`);
  let removed;
  if (clean) {
    const cleaned = await runBuildProcess(command, ["build", "--json", "--clean"], channel);
    for (const event of parseBuildEvents(cleaned.stdout)) {
      if (event.type === "build.clean") removed = event.removed ?? 0;
    }
    if (cleaned.error) channel.appendLine(`[error] clean: ${cleaned.error}`);
  }
  const result = await runBuildProcess(command, ["build", "--json", target], channel);
  if (result.error) {
    const text = `sokonanoda: build 失败 — ${result.error}`;
    channel.appendLine(text);
    vscode.window.showErrorMessage(text);
    return undefined;
  }
  const summary = parseBuildEvents(result.stdout).find((event) => event.type === "build.summary");
  const elapsed = Date.now() - started;
  const counts = summary
    ? `${summary.files} 个文件 · 编译 ${summary.compiled} · 命中 ${summary.hit} · 失败 ${summary.failed}`
    : "完成";
  const text =
    `sokonanoda ${clean ? "rebuild" : "build"}：${counts}` +
    `${clean && removed !== undefined ? ` · 清掉 ${removed} 条缓存` : ""}（${elapsed}ms）`;
  channel.appendLine(text);
  const notify = summary && summary.failed > 0
    ? vscode.window.showWarningMessage
    : vscode.window.showInformationMessage;
  notify.call(vscode.window, text, "显示输出").then((pick) => {
    if (pick) channel.show(true);
  });

  // 缓存状态变了 ⇒ 让读 CLI/LSP 状态的视图重算（目标树/练习树/课程树）。
  goalProvider?.refresh?.();
  projectProvider?.refresh?.();
  loadProject();
  courseProvider?.refresh?.();
  return text;
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
    vscode.commands.registerCommand("sokonanoda.project.refresh", () => {
      projectProvider.refresh();
      loadProject();
    }),
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
    vscode.commands.registerCommand("sokonanoda.build", () =>
      runBuild(context, { courseProvider }),
    ),
    vscode.commands.registerCommand("sokonanoda.rebuild", () =>
      runBuild(context, { clean: true, courseProvider }),
    ),
    // 记法缩写改写器（NI-2）：键位 Tab，`when` 子句由 abbreviation-rewriter.js
    // 置位的 context key 把关（普通 Tab 照旧缩进）。命令注册在这里、状态机在
    // src/abbreviation-rewriter.js —— 与开发规范 §1 的文件职责一致。
    vscode.commands.registerCommand(
      "sokonanoda.input.replaceAbbreviation",
      notationInput.replaceAbbreviation,
    ),
  );
}

async function activate(context) {
  extensionRoot = context.extensionPath;

  // ORDER IS LOAD-BEARING — every view/command is registered *before the
  // first `await`. VS Code registers extension view containers with
  // `hideIfEmpty`: while `sokonanoda.infoview` has no provider the container
  // has no visible view and is hidden, so the panel is blank and
  // `openInfoview` does nothing. Resolving the server first (filesystem
  // probing, and a download on unbundled platforms) used to delay
  // `registerWebviewViewProvider`, and that late registration is what made the
  // panel "suddenly appear" on a side-bar toggle. The slow server startup now
  // runs in the async continuation at the bottom of this function, which posts
  // `status` to the Infoview as it progresses.
  const provider = new GoalsTreeDataProvider();
  const tree = vscode.window.createTreeView("sokonanoda.goals", {
    treeDataProvider: provider,
    showCollapseAll: true,
  });
  statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
  statusBar.command = "sokonanoda.goals.focus";
  context.subscriptions.push(tree, statusBar);

  // 项目树（0.58.0）：只读呈现服务端 `soko/project` 的答案——模块根、清单来源、
  // 闭包模块与各自状态。注册同样在第一次 await 之前（见上面的 ORDER 注释）。
  projectProvider = new projectTree.ProjectTreeProvider();
  const projectView = vscode.window.createTreeView("sokonanoda.project", {
    treeDataProvider: projectProvider,
    showCollapseAll: true,
  });
  context.subscriptions.push(projectView);

  // Infoview webview (方案 B): the tree fans its single soko/stateAt snapshot
  // and its soko/goals declaration list out to the webview. Hidden context is
  // released (`retainContextWhenHidden: false`); the provider caches the last
  // state/decls/status and replays them on the next `ready` (docs/design §5).
  infoviewProvider = new InfoviewProvider(context.extensionUri, provider);
  provider.onDecls = (decls) => infoviewProvider.setDecls(decls);
  provider.onState = (uriString, state) => infoviewProvider.setState(uriString, state);
  provider.onStatus = (status) => infoviewProvider.setStatus(status);
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
  let diagnosticsTimer;
  const DIAGNOSTICS_DEBOUNCE_MS = 150;
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      provider.trackEditor(editor);
      if (projectProvider.trackEditor(editor)) {
        e2eLog(`active document changed: ${projectProvider.uri ?? "(none)"}`);
        loadProject();
      }
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
    vscode.languages.onDidChangeDiagnostics((event) => {
      // 只理 `.sokonanoda`：诊断事件是**全窗口**的，别的扩展（TS/ESLint/
      // rust-analyzer）报一次错不该让我们跑 soko/goals 并重建树/面板。
      if (!event.uris.some(isSokonanodaFile)) return;
      // 事件会连着来（项目模式一次编辑可能发多份文档的诊断）⇒ 合并成一次刷新。
      clearTimeout(diagnosticsTimer);
      diagnosticsTimer = setTimeout(() => {
        provider.refresh();
        // 项目视图描述的是"刚编译过的闭包"：诊断到来说明服务器已经重编译，
        // 重取一次即可（含"改依赖 ⇒ 入口状态变了"的场景）。
        projectProvider.refresh();
        loadProject();
        // decls follow diagnostics/file changes only (never cursor moves): the
        // webview gets the fresh declaration list through the onDecls hook.
        provider.ensureDeclarations().catch(() => {});
        requestCursorForEditor(); // the document may have been re-checked
      }, DIAGNOSTICS_DEBOUNCE_MS);
    }),
    {
      dispose: () => {
        clearTimeout(selectionTimer);
        clearTimeout(diagnosticsTimer);
      },
    },
  );
  provider.trackEditor(vscode.window.activeTextEditor);
  goalProvider = provider;
  projectProvider.trackEditor(vscode.window.activeTextEditor);
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
  // 记法缩写改写器（NI-2）：命令 + Tab 键位的 context key + 即时替换监听
  // （`sokonanoda.input.eager`，默认关）。注册同样在第一次 await 之前。
  notationInput.register(context);
  courseProvider.refresh();

  // Test-only surface: when the real VS Code host runs the integration suite
  // (`vscode-test` sets `ExtensionMode.Test`), `activate()` returns the tree
  // providers so the suite can assert **real** rows built from **real**
  // `soko/project` answers. Production hosts ignore the return value.
  //
  // NOTE: return it at the **end** of activate() — returning here would skip
  // the async continuation below and the language server would never start
  // (the whole suite then times out with no diagnostics; cost me one debug
  // cycle, hence this comment).
  const testMode = vscode.ExtensionMode ? vscode.ExtensionMode.Test : undefined;
  const testApi =
    testMode !== undefined && context.extensionMode === testMode
      ? {
          goals: provider,
          project: projectProvider,
          course: courseProvider,
          // T-016：e2e 通过它断言"面板收到了什么"（见上面 lastDecls/lastState）。
          infoview: infoviewProvider,
        }
      : undefined;

  // Async continuation (fire-and-forget): resolve + start the server without
  // blocking the view. This is the only slow path — the UI is already live and
  // the Infoview shows progress until the first snapshot arrives.
  // T-A52：预热缓存（默认关，见 `warmCacheOnOpen`）。**不 await**：它是后台活，
  // 打开了设置的用户也不该为它多等一次激活。
  void warmCacheOnOpen(context).catch(() => {});
  (async () => {
    infoviewProvider.setStatus({ state: "loading" });
    infoviewProvider.postServer(); // server not running yet -> "启动中"
    const resolution = await resolveServerForStart(context);
    if (resolution === undefined) {
      e2eLog("server resolution failed (no command)");
      infoviewProvider.setStatus({ state: "idle" });
      infoviewProvider.postServer();
      return;
    }
    const command = resolution.command;
    serverOptions = {
      run: { command, transport: TransportKind.stdio },
      debug: { command, transport: TransportKind.stdio },
    };
    client = new LanguageClient("sokonanoda", "sokonanoda", serverOptions, {
      documentSelector: [{ language: "sokonanoda", scheme: "file" }],
      synchronize: { fileEvents: vscode.workspace.createFileSystemWatcher("**/*.sokonanoda") },
    });
    // 扩展宿主控制台（测试/开发可见）：例行 e2e 卡住时，第一现场是"服务器到底
    // 起没起、用的是哪个二进制"——output channel 在测试里读不到，控制台能。
    console.log(`[sokonanoda] server command: ${command} (source=${resolution.source ?? "?"})`);
    e2eLog(`server command: ${command} (source=${resolution.source ?? "?"})`);
    client.onDidChangeState((event) => {
      const name = stateNames[event.newState] ?? event.newState;
      client.outputChannel.appendLine(`[client] ${name}`);
      console.log(`[sokonanoda] client state: ${name}`);
      e2eLog(`client state: ${name}`);
    });
    // **编译进度**（P1→P2/P3 的接缝，2026-09-26）：**裸读** `$/progress` 自己
    // 翻译并渲染（状态栏 + Infoview）✓。
    //
    // 为什么不用 `client.onProgress(type, token, handler)`：它要求**事先知道令牌**，
    // 而令牌是按 uri 定的（`sokonanoda/compile<path>`）⇒ ① 切文档要重注册
    // ② `compile_one` 的**下游扇出**编的不是当前文档 ⇒ 那一路**必然被丢** ✗。
    // 裸读一次拿到所有令牌，"在编什么"才完整 ✓（S2 调研的 T5）。
    //
    // ⚠ **与它绑定的一条**：服务端**刻意不发** `window/workDoneProgress/create`
    // —— jsonrpc 的通知处理表按方法名**唯一**，裸读会覆盖内建分发器；两者同时上
    // 会让 `ProgressPart` 造出来却收不到 `begin` ⇒ **每次编译泄漏一个** ✗（T3）。
    client.onNotification("$/progress", (params) => {
      onCompileProgress(params && params.value);
    });
    context.subscriptions.push(client);
    await client.start();
    client.outputChannel.appendLine(
      `[client] ready — server synchronized over stdio: ${command} (source=${resolution.source ?? "?"})`,
    );
    // Server is up: refresh the panel's server line and warm the decl list.
    infoviewProvider.postServer();
    provider.ensureDeclarations().catch(() => {});
    loadProject();
    // Auto-run the read-only doctor once, after the first soko/version
    // exchange, so version/source problems surface without the user hunting.
    await runDoctor(context, { notify: true, show: false });
  })().catch((error) => {
    e2eLog(`activation failed: ${error?.stack ?? error}`);
    console.error(`[sokonanoda] activation failed: ${error?.stack ?? error}`);
    try {
      infoviewProvider.setStatus({ state: "idle" });
    } catch {
      // the view may not exist in a headless host — best-effort only
    }
  });

  return testApi;
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
