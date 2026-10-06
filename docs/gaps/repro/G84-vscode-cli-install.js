// G-84 判据驱动器：**在用户动作级重放**「VS Code 命令安装 CLI」这条链。
//
// 用户报障（2026-10-02，**此前从未登记过**）：
//   「vscode 命令安装的 sokonanoda cli 版本不对，也缺少 uninstall 命令」
//
// 判据（四条，全部**机械重放** ✓；不看源码文本、只驱动真代码 + 问真二进制）：
//   ① 版本不匹配即重装（幂等）：往安装位**先塞一只旧版**（自述 0.0.1）⇒ 跑一次
//      `Sokonanoda: Install Command Line` ⇒ 安装位的 `--version` 必须 == `package.json` 版本 ✓
//   ② 有 `sokonanoda.uninstallCli`：跑一次 ⇒ 安装位（含 `.version` 标记与 `*.bak-*` 残留）
//      必须被清干净 ✓
//   ③ PATH 落地：跑完安装后，**新开登录 shell**（`bash -lc` / `zsh -lc`，HOME 指向临时家目录）
//      里 `command -v sokonanoda` 必须解析到**安装位那一只** ✓
//   ④ 真判据（**不许比刚拷的文件** ✗ —— 那是假绿）：上面 ③ 里那个二进制跑出来的
//      `--version` 必须 == `package.json` 版本 ✓
//
// 反向验证：①里"先塞旧版"那一步就是反向验证 —— 撤掉修复（不做版本比对/不覆盖）时
//   安装位会留下 0.0.1 ⇒ ①判红 ✓。`G84-vscode-cli-install.sh` 另外跑一遍
//   `SOKO_G84_LEGACY=1`（模拟旧实现：只拷不问）⇒ 判据必须判红 ✓。
//
// 退出码：0 = 缺口仍在（判据不成立）  1 = 判据全部成立（已修）  2 = 环境不满足
"use strict";
const Module = require("module");
const path = require("path");
const fs = require("fs");
const os = require("os");
const cp = require("child_process");

const ROOT = process.argv[2] || process.cwd();
const EXT_DIR = path.join(ROOT, "editor", "vscode");
const pkg = JSON.parse(fs.readFileSync(path.join(EXT_DIR, "package.json"), "utf8"));
const VERSION = pkg.version;
const LEGACY = process.env.SOKO_G84_LEGACY === "1";

const home = fs.mkdtempSync(path.join(os.tmpdir(), "g84-home-"));
process.env.HOME = home;
process.env.USERPROFILE = home;

const installDir = path.join(home, ".local", "share", "sokonanoda", "bin");
const destName = process.platform === "win32" ? "sokonanoda.exe" : "sokonanoda";
const dest = path.join(installDir, destName);

const notes = [];
const handlers = new Map();
function note(s) { notes.push(s); }

// ── stub `vscode`（只提供扩展真正会碰到的面）─────────────────────────────
function noopDisposable() { return { dispose() {} }; }
const vscodeStub = {
  version: "1.85.0",
  window: {
    showInformationMessage: (t) => { note("info: " + t); return Promise.resolve(undefined); },
    showErrorMessage: (t) => { note("error: " + t); return Promise.resolve(undefined); },
    showWarningMessage: (t) => { note("warn: " + t); return Promise.resolve(undefined); },
    createOutputChannel: () => ({ appendLine() {}, append() {}, show() {}, hide() {}, clear() {}, dispose() {} }),
    createStatusBarItem: () => ({ show() {}, hide() {}, dispose() {}, text: "", tooltip: "", command: "" }),
    registerWebviewViewProvider: () => noopDisposable(),
    showQuickPick: () => Promise.resolve(undefined),
    showInputBox: () => Promise.resolve(undefined),
    showOpenDialog: () => Promise.resolve(undefined),
    withProgress: (_o, f) => f({ report() {} }, { isCancellationRequested: false }),
    onDidChangeActiveTextEditor: () => noopDisposable(),
    onDidChangeActiveColorTheme: () => noopDisposable(),
    activeColorTheme: { kind: 1 },
    onDidChangeTextEditorSelection: () => noopDisposable(),
    onDidChangeVisibleTextEditors: () => noopDisposable(),
    onDidChangeWindowState: () => noopDisposable(),
    activeTextEditor: undefined,
    visibleTextEditors: [],
    tabGroups: { all: [] },
    createTreeView: () => ({ dispose() {} }),
  },
  commands: {
    registerCommand: (id, fn) => { handlers.set(id, fn); return noopDisposable(); },
    executeCommand: () => Promise.resolve(undefined),
    getCommands: () => Promise.resolve([...handlers.keys()]),
  },
  workspace: {
    workspaceFolders: [],
    textDocuments: [],
    getConfiguration: () => ({ get: (_k, d) => d, has: () => false, update: () => Promise.resolve() }),
    onDidChangeConfiguration: () => noopDisposable(),
    onDidSaveTextDocument: () => noopDisposable(),
    onDidOpenTextDocument: () => noopDisposable(),
    onDidChangeTextDocument: () => noopDisposable(),
    onDidCloseTextDocument: () => noopDisposable(),
    createFileSystemWatcher: () => ({ onDidChange() {}, onDidCreate() {}, onDidDelete() {}, dispose() {} }),
    asRelativePath: (p) => p,
    openTextDocument: () => Promise.resolve({ getText: () => "", uri: { fsPath: "" } }),
    findFiles: () => Promise.resolve([]),
    fs: { readFile: () => Promise.resolve(Buffer.from("")), writeFile: () => Promise.resolve() },
  },
  languages: {
    createDiagnosticCollection: () => ({ set() {}, clear() {}, dispose() {}, forEach() {} }),
    registerHoverProvider: () => noopDisposable(),
    onDidChangeDiagnostics: () => noopDisposable(),
    getDiagnostics: () => [],
    registerCodeLensProvider: () => noopDisposable(),
    registerDocumentSymbolProvider: () => noopDisposable(),
  },
  Uri: {
    file: (p) => ({ fsPath: p, scheme: "file", toString: () => "file://" + p }),
    parse: (s) => ({ fsPath: s, toString: () => s }),
    joinPath: (a, b) => ({ fsPath: path.join(a && a.fsPath ? a.fsPath : a, b) }),
  },
  EventEmitter: class { constructor() { this.event = () => noopDisposable(); } fire() {} dispose() {} },
  Diagnostic: class { constructor(r, m, s) { this.range = r; this.message = m; this.severity = s; } },
  DiagnosticSeverity: { Error: 0, Warning: 1, Information: 2, Hint: 3 },
  Range: class { constructor(a, b, c, d) { this.start = a; this.end = b; void c; void d; } },
  Position: class { constructor(l, c) { this.line = l; this.character = c; } },
  ViewColumn: { One: 1, Two: 2, Active: -1 },
  TreeItem: class { constructor(l) { this.label = l; } },
  ThemeIcon: class { constructor(id) { this.id = id; } },
  TreeItemCollapsibleState: { None: 0, Collapsed: 1, Expanded: 2 },
  StatusBarAlignment: { Left: 1, Right: 2 },
  ProgressLocation: { Notification: 15, Window: 10 },
  RelativePattern: class { constructor(b, p) { this.base = b; this.pattern = p; } },
  CancellationTokenSource: class { constructor() { this.token = { isCancellationRequested: false, onCancellationRequested: () => noopDisposable() }; } cancel() {} dispose() {} },
};
const lspStub = {
  LanguageClient: class {
    constructor() {
      this.onNotification = () => noopDisposable();
      this.onRequest = () => noopDisposable();
      this.onDidChangeState = () => noopDisposable();
      this.onTelemetry = () => noopDisposable();
      this.state = 2;
    }
    start() { return Promise.resolve(); }
    stop() { return Promise.resolve(); }
    sendRequest() { return Promise.resolve({}); }
    dispose() { return Promise.resolve(); }
  },
  TransportKind: { stdio: 0, ipc: 1, pipe: 2 },
  State: { Stopped: 1, Running: 2, Starting: 3 },
  CloseAction: { DoNotRestart: 1, Restart: 2 },
  ErrorAction: { Continue: 1, Shutdown: 2 },
  RevealOutputChannelOn: { Never: 0, Error: 1 },
};
const realLoad = Module._load;
Module._load = function (request, parent, isMain) {
  if (request === "vscode") return vscodeStub;
  if (request === "vscode-languageclient/node" || request === "vscode-languageclient") return lspStub;
  return realLoad.call(this, request, parent, isMain);
};

const CRITERIA = process.env.SOKO_G84_CRITERIA === "1"; // --criteria：判据不成立 ⇒ 退出码非 0
const failures = [];
function fail(msg) { failures.push(msg); console.log("✗ " + msg); }
function settle() {
  if (failures.length) { console.log(`\n✗ G-84：${failures.length} 条判据不成立`); process.exit(CRITERIA ? 1 : 0); }
  console.log(`\n✓ G-84 四条判据全部成立（插件 ${VERSION}）`);
  process.exit(CRITERIA ? 0 : 1);
}
function env(msg) { console.error("G-84: " + msg); process.exit(2); }   // 2 = 环境不满足

function cliVersion(file) {
  try {
    const out = cp.execFileSync(file, ["--version"], { encoding: "utf8", timeout: 20000 });
    const m = String(out).match(/\d+\.\d+\.\d+/);
    return m ? m[0] : "";
  } catch (e) {
    return "";
  }
}

function loginShellFinds() {
  // **新开登录 shell**（判据 ③④）：HOME 指向临时家目录 ⇒ 只可能看见"真落地"的那个。
  for (const shell of ["zsh", "bash"]) {
    const bin = cp.execFileSync("bash", ["-c", `command -v ${shell} || true`], { encoding: "utf8" }).trim();
    if (!bin) continue;
    const script = `command -v sokonanoda || true`;
    let out = "";
    try {
      out = cp.execFileSync(bin, shell === "zsh" ? ["-lc", script] : ["-lc", script], {
        encoding: "utf8", env: { ...process.env, HOME: home }, timeout: 20000,
      }).trim();
    } catch (e) { out = ""; }
    // **诊断（2026-10-03 ✓）**：CI（ubuntu）在 ③④ 判红 ✗，而本地（macOS）zsh 与 bash **两条都绿** ✓
    // ⇒ 差异不在"回退到 bash"本身 ✓。必须把**用了哪个 shell、它看到的 PATH 是什么**打出来 ✓ ——
    // 本文件的输出会原样进 `cargo test` 的 panic 文本 ✓，下一轮 CI 就能指名机制 ✓（不再靠猜 ✗）。
    let pathSeen = "";
    try {
      pathSeen = cp.execFileSync(bin, ["-lc", 'printf %s "$PATH"'], {
        encoding: "utf8", env: { ...process.env, HOME: home }, timeout: 20000,
      }).trim();
    } catch (e) { pathSeen = "(跑不出 PATH)"; }
    console.log(
      `  · 诊断：${shell} @ ${bin} ⇒ command -v = ${out || "(空)"} · 该登录 shell 看到的 PATH = ${pathSeen}`
    );
    if (out) return { shell, cmd: out, version: cliVersion(out) };
  }
  return undefined;
}

(async () => {
  // ── **环境前提**（2026-10-06 补 ✓）：判据 ①③④ 要的是"扩展把**自带** CLI 装出去"
  // ⇒ `editor/vscode/bin/<target>/` 里**必须先有**那份自带 CLI ✓。它是**构建产物**
  // （`.gitignore`）⇒ CI 的 `test` lane **不 stage** ⇒ 不查这一条的话，安装动作
  // 什么也拷不出来 ⇒ ①②③④ 全部"判据不成立" ⇒ **假红** ✗（实测：CI run
  // `37431004826` 的 `test (sokonanoda-cli, tests)` 腿就是这么红的，而本机 stage 过
  // ⇒ 全绿）。⚠ 这里**不复制**扩展的平台→target 映射（那正是"抄第二份"✗）——
  // 只问"**有没有**暂存的 CLI" ✓。⇒ 没有就 **exit 2（环境不满足）**：Rust 侧
  // `the_installed_cli_resolves_in_a_fresh_login_shell` 对 2 的处理是**响亮跳过**
  // （不判绿也不判红 ✓，与它自己的文档一致 ✓）。
  const binRoot = path.join(EXT_DIR, "bin");
  const staged = fs.existsSync(binRoot)
    ? fs.readdirSync(binRoot).filter((d) => fs.existsSync(path.join(binRoot, d, destName)))
    : [];
  if (staged.length === 0) {
    env(`没有暂存的自带 CLI（editor/vscode/bin/*/${destName} 不存在）⇒ 判据的环境前提不成立`);
  }

  let activate;
  try {
    ({ activate } = require(path.join(EXT_DIR, "extension.js")));
  } catch (e) {
    env("装不进 stub 宿主（require extension.js 失败）：" + (e && e.message));
  }
  const context = {
    extensionPath: EXT_DIR,
    extensionUri: { fsPath: EXT_DIR },
    subscriptions: [],
    extension: { id: "sokonanoda.sokonanoda", packageJSON: pkg },
    globalState: { get: () => undefined, update: () => Promise.resolve(), keys: () => [] },
    workspaceState: { get: () => undefined, update: () => Promise.resolve(), keys: () => [] },
    secrets: { get: () => Promise.resolve(undefined), store: () => Promise.resolve(), delete: () => Promise.resolve() },
    environmentVariableCollection: { replace() {}, append() {}, prepend() {}, clear() {} },
    asAbsolutePath: (p) => path.join(EXT_DIR, p),
    storageUri: { fsPath: path.join(home, ".storage") },
    globalStorageUri: { fsPath: path.join(home, ".globalStorage") },
    logUri: { fsPath: path.join(home, ".log") },
  };
  try {
    await activate(context);
  } catch (e) {
    env("activate() 在 stub 宿主里抛错：" + (e && e.message));
  }

  const install = handlers.get("sokonanoda.installCli");
  const uninstall = handlers.get("sokonanoda.uninstallCli");
  if (!install) env("扩展没注册 sokonanoda.installCli（形状变了）");

  // ── 反向验证的前置：安装位先塞一只**旧版**（自述 0.0.1）+ 一个 `*.bak-*` 残留 ──
  fs.mkdirSync(installDir, { recursive: true });
  const fake = "#!/bin/sh\necho 'sokonanoda 0.0.1'\n";
  fs.writeFileSync(dest, fake, { mode: 0o755 });
  fs.writeFileSync(dest + ".version", "0.0.1 darwin-arm64\n");
  fs.writeFileSync(path.join(installDir, destName + "-lsp.bak-0.0.1"), "leftover\n");
  if (!LEGACY) {
    // 正常路径：跑用户的动作
    await install();
  } // LEGACY：模拟旧实现 —— 只当没看见（判据必须因此判红 ✓）

  // ── 判据 ①：安装位的 `--version` == package.json ──
  const v1 = cliVersion(dest);
  if (v1 !== VERSION) fail(`① 版本不匹配即重装：安装位自述 ${v1 || "(跑不起来)"} ✗，插件 ${VERSION}`);
  else console.log(`  ✓ ① 安装位 --version = ${v1} == package.json ${VERSION}`);

  // ── 判据 ②：清 `*.bak-*` ──
  const leftovers = fs.readdirSync(installDir).filter((n) => n.includes(".bak-"));
  if (leftovers.length) fail(`② 安装位仍有 *.bak-* 残留：${leftovers.join(", ")} ✗`);
  else console.log("  ✓ ② 无 *.bak-* 残留");

  // ── 判据 ③④：新开登录 shell 里拿到的是**安装位那一只**且版本正确 ──
  const found = loginShellFinds();
  if (!found) {
    fail("③④ 新开登录 shell 里 `command -v sokonanoda` 找不到 ⇒ PATH 没落地 ✗");
  } else {
    if (path.resolve(found.cmd) !== path.resolve(dest)) fail(`③ 登录 shell 解析到 ${found.cmd} ✗，不是安装位 ${dest}`);
    else console.log(`  ✓ ③ ${found.shell} -lc ⇒ command -v sokonanoda = ${found.cmd}`);
    if (found.version !== VERSION) fail(`④ 登录 shell 里那只自述 ${found.version || "(空)"} ✗，插件 ${VERSION}`);
    else console.log(`  ✓ ④ 登录 shell 里那只 --version = ${found.version} == 插件 ${VERSION}`);
  }

  // ── 判据 ②（命令存在 + 真能清干净）──
  if (!uninstall) {
    fail("② 没有 sokonanoda.uninstallCli 命令 ✗");
  } else {
    await uninstall();
    const stillThere = fs.existsSync(dest) || fs.existsSync(dest + ".version");
    if (stillThere) fail("② 跑完 uninstallCli 后安装位仍在 ✗");
    const after = fs.existsSync(installDir) ? fs.readdirSync(installDir).filter((n) => n.includes(".bak-")) : [];
    if (after.length) fail(`② uninstallCli 没清 *.bak-*：${after.join(", ")} ✗`);
    if (!stillThere && !after.length) console.log("  ✓ ② uninstallCli 清干净（二进制 + 版本标记 + *.bak-*）");
  }
  settle();
})().catch((e) => env("驱动器异常：" + (e && e.stack ? e.stack.split("\n")[0] : e)));
