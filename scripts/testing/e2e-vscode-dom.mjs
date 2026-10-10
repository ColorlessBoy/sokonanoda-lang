#!/usr/bin/env node
// e2e-vscode-dom.mjs —— CDP DOM 真机复现/验证通用驱动（docs/E2E.md §8）
// 在真 VS Code + 真扩展 + 真 Infoview 上,用真键盘驱动、只读渲染 DOM 复现 UI 行为。
//
// 读数**只走 DOM**（不截图、不 OCR）：Infoview webview 的 `#active-frame` 里
// `.messages .message` 的**实际份数**（`.message-kind` = `#check` / `#print`）。
// 帧的识别判据：那一帧必须同时有 `.decl` / `.status`（= 真 Infoview，不是别的 webview）。
//
// 阶段（`--phase=`，可对**已开着的窗口**续跑）：
//   launch  —— 启动 Extension Development Host，打开 set-theory 文件夹 + Infoview 面板
//   rebuild —— 命令面板跑 `Sokonanoda: Rebuild`，等产物落盘
//   steps   —— 第 3/4/5 步：加 `#check Set.subset` → 做 mem_of_subset → 光标回到 `#check`
//   (默认 all = 三段连着跑)
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const REPO = process.env.SOKO_REPO || "/Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang";
// ② 「vscode 打开 set-theory 文件夹」——默认就是仓库里那个；`SOKO_WS` 可指向
// **同一份内容的副本**（本仓库此刻有别的会话在跑门禁/重建二进制，改教材文件会打架）。
const WS = process.env.SOKO_WS || join(REPO, "courses/set-theory");
const UNIT_ABS = join(WS, "units/I.1/unit01-sets-membership.sokonanoda");
// ⚠ 环境变量可覆盖：换**已发布扩展**（自带成对的 CLI+LSP）跑同一套步骤。
const EXT_PATH = process.env.SOKO_EXT_PATH || join(REPO, "editor/vscode");
const LSP = process.env.SOKO_LSP === undefined ? join(REPO, "target/release/sokonanoda-lsp") : process.env.SOKO_LSP;
const ROOT = process.env.SOKO_OUT || "/tmp/soko-dom-repro";
const PROFILE = join(ROOT, "profile");
const SHOTS = join(ROOT, "shots");
const PORT = Number(process.env.PORT || 9470);
const ARTIFACTS = join(WS, ".sokonanoda", "compiled");
const phase = (process.argv.find((a) => a.startsWith("--phase=")) || "--phase=all").split("=")[1];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (...a) => console.log(`[${new Date().toISOString().slice(11, 19)}]`, ...a);

// ── 真键盘（CDP Input，和用户的手一样）──────────────────────────────────────
const META = 4, CTRL = 2, SHIFT = 8;
const VK = { Enter: 13, Home: 36, End: 35, ArrowRight: 39, ArrowLeft: 37, KeyG: 71, KeyP: 80, KeyS: 83 };
async function key(c, k, code, vk, modifiers = 0) {
  const base = { key: k, code, modifiers, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk };
  await c.call("Input.dispatchKeyEvent", { type: "rawKeyDown", ...base });
  await c.call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
}
async function typeChars(c, text) {
  for (const ch of text) {
    const letter = /[a-zA-Z]/.test(ch);
    const vk = letter ? ch.toUpperCase().charCodeAt(0) : ch === " " ? 32 : 0;
    const code = letter ? `Key${ch.toUpperCase()}` : ch === " " ? "Space" : "";
    await c.call("Input.dispatchKeyEvent", {
      type: "keyDown", text: ch, unmodifiedText: ch, key: ch, code,
      windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk,
    });
    await c.call("Input.dispatchKeyEvent", {
      type: "keyUp", key: ch, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk,
    });
    await sleep(45);
  }
}
/// 编辑器里的**换行**必须走 `keyDown` + `text:"\r"`：⚠ 实测 `rawKeyDown` 不带 text
/// 时，Monaco 的 Enter 动作**不插行**（命令面板/转到行的输入框却认它）——
/// 上一轮就是这么把 `#check Set.subsettheorem …` 焊在一行上的 ✗。
async function enterEditor(c) {
  const b = { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13, text: "\r", unmodifiedText: "\r" };
  await c.call("Input.dispatchKeyEvent", { type: "keyDown", ...b });
  await c.call("Input.dispatchKeyEvent", { type: "keyUp", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
  await sleep(200);
}
/// 编辑器里的**文本**用 `Input.insertText`（走 VS Code 的输入通道）。
async function insert(c, text) { await c.call("Input.insertText", { text }); await sleep(150); }
/// 「转到行」：**所有平台都是 Ctrl+G**（macOS 上 Cmd+G 是「查找下一个」）。
async function gotoLine(c, line) { return gotoLineColumn(c, line, null); }
/// ⚠ **必须用 `行:列` 精确定位**：macOS 上 `Cmd+←` / `Home` 都是 `cursorHome`
/// = **首个非空白列**（实测 `  sorry` 落在 Col 3），不是列 1 ✗ ——
/// 上一轮靠 Cmd+← + 两次右移去"跳缩进"，结果只选中了 `rry`、
/// 把 `sorry` 写成了 `sointro ha` ✗。
async function gotoLineColumn(c, line, col) {
  await key(c, "g", "KeyG", VK.KeyG, CTRL);
  await sleep(650);
  await typeChars(c, col ? `${line}:${col}` : String(line));
  await key(c, "Enter", "Enter", VK.Enter);
  await sleep(1400);
  const es = await editorState(c);
  if (col && (es.line !== line || es.col !== col)) {
    log(`   ⚠ 光标没到位：要 ${line}:${col}，实到 ${es.line}:${es.col}`);
  }
  return es;
}
/// 命令面板：**先确认 quick input 真的开了**再打字 —— 否则字符会落进编辑器里 ✗
/// （上一轮 `Sokonanoda: Infoview` 就被插到了文件第 1 行）。
async function commandPalette(c, text) {
  for (let attempt = 1; attempt <= 3; attempt++) {
    await key(c, "Escape", "Escape", 27); // 关掉可能抢焦点的通知 toast
    await sleep(300);
    await c.call("Page.bringToFront", {}).catch(() => {});
    await key(c, "P", "KeyP", VK.KeyP, META | SHIFT);
    await sleep(1100);
    const open = await c.eval(`(() => { const w = document.querySelector('.quick-input-widget'); return !!w && getComputedStyle(w).display !== 'none'; })()`);
    if (!open) { log(`   ⚠ 第 ${attempt} 次命令面板没开，重试`); continue; }
    await typeChars(c, text);
    await sleep(1300);
    const value = await c.eval(`(document.querySelector('.quick-input-box input') || {}).value || null`);
    // ⚠ 命令面板的输入框自带 `>` 前缀（`>` = 命令模式）⇒ 判据是**后缀**相等。
    if (typeof value !== "string" || !value.endsWith(text)) { log(`   ⚠ 命令面板输入成了 ${JSON.stringify(value)}（期望后缀 ${JSON.stringify(text)}），重试`); continue; }
    await key(c, "Enter", "Enter", VK.Enter);
    await sleep(1600);
    return true;
  }
  throw new Error(`命令面板打不开/打字不进去：${text}`);
}
const save = async (c) => { await key(c, "s", "KeyS", VK.KeyS, META); await sleep(2200); };

// ── CDP ────────────────────────────────────────────────────────────────────
async function connect(url) {
  const ws = new WebSocket(url);
  await new Promise((res, rej) => {
    const t = setTimeout(() => rej(new Error("ws 连接超时")), 15000);
    ws.addEventListener("open", () => { clearTimeout(t); res(); }, { once: true });
    ws.addEventListener("error", (e) => { clearTimeout(t); rej(e); }, { once: true });
  });
  let id = 0;
  const pending = new Map();
  ws.addEventListener("message", (ev) => {
    const m = JSON.parse(ev.data);
    const p = pending.get(m.id);
    if (p) { pending.delete(m.id); p(m); }
  });
  const call = (method, params = {}) => new Promise((res, rej) => {
    const n = ++id;
    const t = setTimeout(() => { pending.delete(n); rej(new Error(`CDP ${method} 超时`)); }, 25000);
    pending.set(n, (m) => { clearTimeout(t); res(m); });
    ws.send(JSON.stringify({ id: n, method, params }));
  });
  const c = { ws, call, close: () => ws.close() };
  c.eval = async (expression) => {
    const m = await c.call("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    if (m.result?.exceptionDetails) throw new Error(JSON.stringify(m.result.exceptionDetails).slice(0, 300));
    return m.result?.result?.value;
  };
  return c;
}
async function targets() { return await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json(); }
async function workbenchPage(deadlineMs = 90000) {
  const until = Date.now() + deadlineMs;
  for (;;) {
    try {
      const p = (await targets()).find((t) => t.type === "page" && t.url.includes("workbench"));
      if (p) {
        const c = await connect(p.webSocketDebuggerUrl);
        // **窗口可能不在最前**（同时开着好几个 dev host）⇒ 真键位会打空。
        // `Page.bringToFront` + 焦点模拟把输入通道打开（实测：不调用时命令面板
        // 根本不出现，`Input.dispatchKeyEvent` 石沉大海）。
        await c.call("Page.bringToFront", {}).catch(() => {});
        await c.call("Emulation.setFocusEmulationEnabled", { enabled: true }).catch(() => {});
        await sleep(400);
        return c;
      }
    } catch { /* 还没起来 */ }
    if (Date.now() > until) throw new Error("VS Code 没有暴露 workbench 页");
    await sleep(800);
  }
}
/// **只认**有 `.decl` / `.status` / `.messages` 的那一帧（= 真 Infoview）。
const MARK = `!!(d.querySelector('.decl') || d.querySelector('.status') || d.querySelector('.messages'))`;
async function infoviewEval(expression, { tries = 12, requireInfoview = true } = {}) {
  for (let i = 0; i < tries; i++) {
    let list = [];
    try { list = await targets(); } catch { /* retry */ }
    for (const t of list.filter((x) => x.type === "iframe" || (x.type === "page" && x.url.startsWith("vscode-webview:")))) {
      let c;
      try {
        c = await connect(t.webSocketDebuggerUrl);
        const mark = await c.eval(`(() => { const f = document.querySelector('#active-frame'); const d = f && f.contentDocument; if (!d || !d.body) return null; return ${MARK}; })()`);
        if (requireInfoview && mark !== true) continue;
        const v = await c.eval(`(() => { const f = document.querySelector('#active-frame'); const d = f && f.contentDocument; if (!d || !d.body) return null; return JSON.stringify((() => (${expression}))()); })()`);
        if (v && v !== "null") return JSON.parse(v);
      } catch { /* 陈旧 target */ } finally { try { c?.close(); } catch { /* ignore */ } }
    }
    await sleep(800);
  }
  return null;
}
/// ⚠ 快速探测：`tries: 1`（默认 12 次 × 800ms = 10s，轮询里等不起）。
const panelOpen = async () => (await infoviewEval(`({ ok: true })`, { tries: 1 })) !== null;
/// Infoview 渲染 DOM 的读数：命令输出**实际几份**。
const readPanel = async () => await infoviewEval(`({
  url: location.href.slice(0, 32),
  hasDecl: !!d.querySelector('.decl'), hasStatus: !!d.querySelector('.status'),
  status: (d.querySelector('.status') || {}).textContent || null,
  server: (d.querySelector('.server') || {}).textContent || null,
  decls: d.querySelectorAll('.decl').length,
  messagesHead: (d.querySelector('.messages-head') || {}).textContent || null,
  messages: Array.from(d.querySelectorAll('.messages .message')).map(function (e) {
    return {
      kind: (e.querySelector('.message-kind') || {}).textContent || '?',
      text: ((e.querySelector('.message-text') || {}).textContent || '').replace(/\\s+/g, ' ').trim(),
    };
  }),
})`);
/// 等面板读数**稳定**（两次相邻读数一致）——命令输出是服务端往返后才画的。
async function stablePanel(timeoutMs = 9000) {
  const until = Date.now() + timeoutMs;
  let prev = null;
  while (Date.now() < until) {
    const cur = await readPanel();
    if (cur && prev && JSON.stringify(cur.messages) === JSON.stringify(prev.messages)) return cur;
    prev = cur;
    await sleep(1100);
  }
  return prev;
}
/// 编辑器真相（DOM）：状态栏 `Ln x, Col y` + 可见行的**行号↔文本**映射。
async function editorState(wb) {
  const raw = await wb.eval(`(() => {
    const pos = Array.from(document.querySelectorAll('.statusbar-item'))
      .map(function (e) { return (e.textContent || '').trim(); })
      .find(function (t) { return /^(Ln|行)\\b/.test(t); }) || null;
    const nums = Array.from(document.querySelectorAll('.margin-view-overlays > .line-numbers'))
      .map(function (e) { return { top: parseFloat(e.style.top), n: (e.textContent || '').trim() }; });
    const lines = Array.from(document.querySelectorAll('.view-lines > .view-line'))
      .map(function (e) { return { top: parseFloat(e.style.top), text: e.textContent }; });
    const cur = document.querySelector('.monaco-editor .cursor');
    return JSON.stringify({ pos: pos, nums: nums, lines: lines,
      cursor: cur ? { top: cur.style.top, left: cur.style.left } : null });
  })()`);
  const s = JSON.parse(raw);
  const m = /(\d+)\D+(\d+)/.exec(s.pos || "");
  s.line = m ? Number(m[1]) : null;
  s.col = m ? Number(m[2]) : null;
  // 当前行的**渲染文本**：`.line-numbers` 没有 `top`（不能按坐标配对 ✗），
  // 而 `.cursor` 的 `top` 与所在 `.view-line` 的 `top` **同一坐标系**（实测都 837px）
  // ⇒ 用光标 top 去认那一行 ✓。
  const curTop = s.cursor ? parseFloat(s.cursor.top) : NaN;
  s.cursorLineText = Number.isFinite(curTop)
    ? (s.lines.find((l) => Math.abs(l.top - curTop) < 1)?.text ?? null)
    : null;
  s.visible = { nums: s.nums.length, lines: s.lines.length };
  return s;
}
const fileLines = () => readFileSync(UNIT_ABS, "utf8").split("\n");
function snapshot(tag, extra = {}) {
  mkdirSync(ROOT, { recursive: true });
  const lines = fileLines();
  const state = { tag, at: new Date().toISOString(), file: UNIT_ABS, lines: lines.length, ...extra };
  writeFileSync(join(ROOT, `snapshot-${tag}.json`), JSON.stringify(state, null, 2));
  return state;
}

// ── ① launch：真 VS Code（dev host）+ 真扩展 + 真 LSP ───────────────────────
function writeProfile() {
  mkdirSync(join(PROFILE, "User"), { recursive: true });
  writeFileSync(join(PROFILE, "User", "settings.json"), JSON.stringify({
    "workbench.startupEditor": "none",
    "workbench.tips.enabled": false,
    "workbench.colorTheme": "Default Dark Modern",
    "window.commandCenter": false,
    "breadcrumbs.enabled": false,
    "editor.minimap.enabled": false,
    "editor.stickyScroll.enabled": false,
    "editor.tabSize": 2,
    "editor.insertSpaces": true,
    "editor.autoIndent": "full",
    "editor.quickSuggestions": false,
    "editor.suggestOnTriggerCharacters": false,
    "editor.acceptSuggestionOnEnter": "off",
    "editor.parameterHints.enabled": false,
    "editor.wordBasedSuggestions": "off",
    "editor.inlineSuggest.enabled": false,
    "editor.formatOnType": false,
    "editor.formatOnSave": false,
    "files.autoSave": "off",
    "git.openRepositoryInParentFolders": "never",
    "update.showReleaseNotes": false,
    "extensions.autoCheckUpdates": false,
    "extensions.autoUpdate": false,
    "telemetry.telemetryLevel": "off",
    "security.workspace.trust.enabled": false,
    // `SOKO_LSP` 给了就覆盖服务器；**不给**（空串）就用扩展自带的那个
    // （成对的 CLI+LSP = 用户现场的那一份）。
    ...(LSP ? { "sokonanoda.serverOverride": true, "sokonanoda.serverPath": LSP } : {}),
  }, null, 2));
}
function launchVscode() {
  mkdirSync(SHOTS, { recursive: true });
  const bin = "/Applications/Visual Studio Code.app/Contents/MacOS/Code";
  const child = spawn(bin, [
    `--user-data-dir=${PROFILE}`,
    `--extensions-dir=${join(PROFILE, "extensions")}`,
    `--extensionDevelopmentPath=${EXT_PATH}`,
    `--remote-debugging-port=${PORT}`,
    "--disable-gpu", "--disable-chromium-sandbox", "--no-first-run", "--disable-workspace-trust",
    "--disable-extension", "GitHub.copilot-chat", "--disable-extension", "GitHub.copilot",
    "--goto", `${UNIT_ABS}:1:1`,
    WS,
  ], { detached: true, stdio: ["ignore", "ignore", "ignore"] });
  child.unref();
  return child.pid;
}

// ── ② rebuild：命令面板跑 `Sokonanoda: Rebuild` ────────────────────────────
function artifactsMtime() {
  try {
    const files = readdirSync(ARTIFACTS).filter((f) => f.endsWith(".json"));
    let newest = 0;
    for (const f of files) newest = Math.max(newest, statSync(join(ARTIFACTS, f)).mtimeMs);
    return { count: files.length, newest };
  } catch { return { count: 0, newest: 0 }; }
}
async function waitArtifacts(wb, timeoutMs = 900000) {
  const until = Date.now() + timeoutMs;
  let last = artifactsMtime();
  let stableSince = 0;
  while (Date.now() < until) {
    await sleep(2000);
    const cur = artifactsMtime();
    if (cur.count > 1 && cur.count === last.count && cur.newest === last.newest) {
      if (!stableSince) stableSince = Date.now();
      else if (Date.now() - stableSince > 6000) return cur;
    } else { stableSince = 0; last = cur; }
    const prog = await wb.eval(`(() => {
      const items = Array.from(document.querySelectorAll('.statusbar-item, .notification-list-item'))
        .map(function (e) { return (e.textContent || '').trim(); })
        .filter(function (t) { return /文件|编译|build|Sokonanoda/.test(t); });
      return JSON.stringify(items.slice(0, 4));
    })()`).catch(() => "[]");
    const p = JSON.parse(prog || "[]");
    if (p.length) log("   进度：", p.join(" · ").slice(0, 160));
  }
  throw new Error("Rebuild 超时（产物没落盘）");
}

// ── 第 3/4/5 步 ────────────────────────────────────────────────────────────
function findLine(pred, from = 0) {
  const lines = fileLines();
  for (let i = from; i < lines.length; i++) if (pred(lines[i])) return i + 1;
  return -1;
}
async function probeAt(wb, line, label) {
  await gotoLine(wb, line);
  await sleep(1500); // 让 `soko/stateAt` 往返 + webview 重画
  const es = await editorState(wb);
  const panel = await stablePanel();
  const row = {
    label, asked_line: line, statusbar: es.pos, cursor_line: es.line, cursor_col: es.col,
    cursor_line_text: es.cursorLineText, file_line_text: fileLines()[line - 1],
    has_decl: panel?.hasDecl ?? null, has_status: panel?.hasStatus ?? null,
    status: panel?.status ?? null, messages_head: panel?.messagesHead ?? null,
    messages: panel?.messages ?? null,
    n_check: (panel?.messages ?? []).filter((m) => m.kind === "#check").length,
    n_total: (panel?.messages ?? []).length,
  };
  log(`  [${label}] 状态栏=${row.statusbar} · 渲染行文本=${JSON.stringify((row.cursor_line_text || "").slice(0, 48))}`);
  log(`  [${label}] Infoview(.decl=${row.has_decl} .status=${row.has_status}) 「${row.messages_head || "(无命令输出块)"}」 ⇒ ${row.n_total} 条：` +
    JSON.stringify((row.messages || []).map((m) => `${m.kind} ${m.text.slice(0, 52)}`)));
  return row;
}

/// 看**模块根产物**里那条 `#check` 在不在（复现的充要条件：产物带着它，
/// 然后被**回放**）。0.87.2 的 CLI 写出来的条目**没有 `cmd`**（`#[serde(skip)]`）
/// ⇒ 回放后 `cmd` 归零 ⇒ 与 fresh 的真相号对不上 ⇒ 拼接时同一条算两遍。
function unitArtifact() {
  try {
    const meta = JSON.parse(readFileSync(join(WS, ".sokonanoda", "meta.json"), "utf8"));
    const key = Object.keys(meta.entries || {}).find((k) => k.endsWith("unit01-sets-membership.sokonanoda"));
    if (!key) return { found: false };
    const p = join(WS, ".sokonanoda", "compiled", meta.entries[key] + ".json");
    const rep = JSON.parse(readFileSync(p, "utf8"));
    return {
      found: true, compiler: meta.compiler, mtime: statSync(p).mtime.toTimeString().slice(0, 8),
      shape: rep.shape, checks: (rep.report?.checks || []).length,
      check_cmds: (rep.report?.checks || []).map((c) => c.cmd ?? null),
      prints: (rep.report?.prints || []).length,
    };
  } catch (e) { return { error: String(e).slice(0, 100) }; }
}

async function main() {
  const rows = [];
  const report = { phase, port: PORT, extension: EXT_PATH, lsp: LSP || "(扩展自带)", workspace: WS, file: UNIT_ABS, rows };
  let wb;
  // ── 验收阶段：`#check` / `#print` 每次操作都只显示一条（2026-10-10 用户实测）──
  //
  // 对**已开着**的窗口续跑（先 `--phase=all` 把窗口与产物准备好）。判据是**用户
  // 看得见的那一层**：Infoview 渲染 DOM 里 `.messages .message` 的**实际份数**
  // （只读 DOM，不截图/OCR）—— 用户原话「每编辑一下 `#check`/`#print` 就多一份」。
  //
  // 动作序列（用户的稳定序列）：命令行在**被编辑的声明之前** ⇒ 它们落进信任前缀
  // ⇒ 靠拼接回填；每编辑一刀读一次两条命令行。**任何一次 ≠ 1 条 ⇒ exit 1**。
  if (phase === "accept") {
    wb = await workbenchPage(20000);
    let cLine = findLine((l) => l.trim() === "#check Set.subset");
    if (cLine < 0) throw new Error("窗口里没有 `#check Set.subset` —— 先跑 `--phase=all`");
    // 补一条 `#print Set.subset`（`all` 只加了 `#check`；用户报的是**两条**都重复）。
    let pLine = findLine((l) => l.trim() === "#print Set.subset");
    if (pLine < 0) {
      log("① 在 `#check Set.subset` 下面补一行 `#print Set.subset`");
      await gotoLineColumn(wb, cLine + 1, 1);
      await insert(wb, "#print Set.subset");
      await enterEditor(wb);
      await sleep(600);
      await save(wb);
      await sleep(2000);
      pLine = findLine((l) => l.trim() === "#print Set.subset");
      if (pLine < 0) throw new Error("`#print Set.subset` 没落成独立一行");
    }
    /// **声明列表名字的计算样式**（2026-10-11 用户第三条：去掉行号 + 名字改**普通粗体**）。
    ///
    /// 判据 = **真宿主里算出来的样式**（`getComputedStyle`，不是我们自己写的 CSS 规则
    /// 文本）：必须**粗体**（font-weight ≥ 700）、背景必须透明、**不许**有下划线
    /// （链接式文字正是用户这次点名要去掉的 ✗）；顺带把计算出来的颜色打进读数
    /// （人读：它应当是正文前景色，而不是主题的链接蓝 ✓）。
    const declNameStyle = async () => {
      return await infoviewEval(`({
        fontWeight: (function () {
          const el = d.querySelector('.decl-name');
          if (!el) return null;
          return getComputedStyle(el).fontWeight;
        })(),
        background: (function () {
          const el = d.querySelector('.decl-name');
          if (!el) return null;
          return getComputedStyle(el).backgroundColor;
        })(),
        decoration: (function () {
          const el = d.querySelector('.decl-name');
          if (!el) return null;
          return getComputedStyle(el).textDecorationLine;
        })(),
        color: (function () {
          const el = d.querySelector('.decl-name');
          if (!el) return null;
          return getComputedStyle(el).color;
        })(),
        lineHints: d.querySelectorAll('.decl-line-hint').length,
        artifacts: (function () {
          const el = d.querySelector('.project-artifacts');
          return el ? (el.textContent || '').trim() : null;
        })(),
      })`);
    };
    const read = async (tag) => {
      const a = await probeAt(wb, cLine, `${tag} · #check`);
      const b = await probeAt(wb, pLine, `${tag} · #print`);
      const art = unitArtifact();
      const style = await declNameStyle();
      return { tag, check: a.n_total, print: b.n_total, artifact: art, style };
    };
    /// 用户动作：在**命令之后**敲一刀 —— 文件末尾追加一条**真命令**
    /// （`theorem accept_N : True := True.intro`）。
    ///
    /// 为什么追加而不是改已有的 `sorry`：这一支要**任何现场都能跑**
    /// （重复跑、前一轮已经改过题都不影响），而且它同时满足复现的**充要条件** ——
    /// 命令行在改动点**之前** ⇒ 它们落进**信任前缀**（靠拼接/回放回填），
    /// 而这是一次**命令级**的真改动（不是"零重编译"的注释改动 ✓）。
    const editOnce = async (round) => {
      const last = fileLines().length;
      log(`②.${round} 文件末尾追加一条真命令（第 ${last} 行之后）`);
      await gotoLineColumn(wb, last, 1);
      await key(wb, "End", "End", VK.End);
      await enterEditor(wb);
      await insert(wb, `theorem accept_${round} : True := True.intro`);
      await sleep(500);
      await save(wb);
      await sleep(2500);
    };
    /// 用户动作：**重启服务器**（= 用户现场"重开这一份文档"）—— 让命令行那一段
    /// 真正走**产物回放**那条路（0.87.2 的 `cmd` 不进序列化 ⇒ 回放后归零 ⇒
    /// 与 fresh 的真命令号对不上 ⇒ 两条都留下 ✗）。这一步正是"当初没发现此 bug"
    /// 缺的那一步（此前 e2e 只做同会话连续编辑）。
    const restartServer = async (tag) => {
      log(`①b ${tag}：命令面板 → 「Sokonanoda: Restart Server」（走产物回放）`);
      await commandPalette(wb, "Sokonanoda: Restart Server");
      await sleep(12000);
      for (let i = 0; i < 20; i++) { if (await panelOpen()) break; await sleep(1500); }
    };
    rows.push(await read("① 起点（冷编之后）"));
    await restartServer("重启服务器");
    rows.push(await read("①b 产物回放之后"));
    for (let round = 1; round <= 2; round += 1) {
      await editOnce(round);
      rows.push(await read(`②.${round} 第 ${round} 刀之后`));
    }
    log("──────── 验收读数（DOM 里 `.messages .message` 的实际份数）────────");
    for (const r of rows) {
      log(`  ${r.tag}：#check=${r.check} · #print=${r.print} · 产物(checks=${r.artifact?.checks ?? "?"} prints=${r.artifact?.prints ?? "?"})`);
    }
    const bad = rows.filter((r) => r.check !== 1 || r.print !== 1);
    log("──────── 声明列表名字的**计算样式**（真宿主）────────");
    for (const r of rows) {
      log(`  ${r.tag}：font-weight=${r.style?.fontWeight} · color=${r.style?.color} · ` +
        `background=${r.style?.background} · text-decoration=${r.style?.decoration} · ` +
        `.decl-line-hint=${r.style?.lineHints}`);
    }
    const style0 = rows.at(-1)?.style || {};
    const weight = Number(style0.fontWeight || 400);
    const bold = weight >= 700; // **普通粗体**（用户 2026-10-11 点名要的那个样式 ✓）
    const opaque = !(style0.background === "rgba(0, 0, 0, 0)" || style0.background === "transparent");
    const underlined = String(style0.decoration || "").includes("underline");
    const hints = Number(style0.lineHints || 0);
    const styleBad = !bold || opaque || underlined || hints !== 0;
    log(`  产物行：${style0.artifacts || "(没有 .project-artifacts)"}`);
    report.verdict = bad.length === 0 && !styleBad
      ? "SINGLE_EVERY_OPERATION + PLAIN_BOLD_NAME"
      : (bad.length ? "REPRODUCED" : "NAME_STILL_LOUD");
    log(bad.length === 0
      ? "  ✅ 每次操作都只显示一条（达标）"
      : `  ✗ 有 ${bad.length} 次读数不是一条：${JSON.stringify(bad.map((r) => [r.tag, r.check, r.print]))}`);
    log(styleBad
      ? `  ✗ 声明名样式不符合「普通粗体」：bold=${bold} opaqueBackground=${opaque} ` +
        `underlined=${underlined} lineHints=${hints}`
      : "  ✅ 声明名是普通粗体（加粗 / 背景透明 / 无下划线 / 无 L<n> 行号）");
    const problems = rows.reduce((n, r) => n + (r.check === 1 && r.print === 1 ? 0 : 1), 0) + (styleBad ? 1 : 0);
    snapshot("accept", report);
    wb.close();
    process.exitCode = problems === 0 ? 0 : 1;
    return report;
  }

  if (phase === "panel") {
    wb = await workbenchPage(20000);
    log("①b 对**已开着**的窗口发命令：Sokonanoda: Infoview（真键位）");
    await commandPalette(wb, "Sokonanoda: Infoview");
    for (let i = 0; i < 30; i++) { if (await panelOpen()) break; await sleep(1500); }
    const p0 = await stablePanel(20000);
    log(`   面板：.decl=${p0?.hasDecl} .status=${p0?.hasStatus} · ${p0?.status} · ${p0?.server} · 命令输出 ${p0?.messages?.length ?? "?"} 条`);
    report.panel_initial = p0;
    wb.close();
    return report;
  }
  if (phase === "reloadread") {
    wb = await workbenchPage(20000);
    const read = async (tag) => {
      const cLine = findLine((l) => l.trim() === "#check Set.subset");
      const pLine = findLine((l) => l.trim() === "#print Set.subset");
      const a = await probeAt(wb, cLine, `${tag} · #check`);
      const b = await probeAt(wb, pLine, `${tag} · #print`);
      const art = unitArtifact();
      log(`   [产物] checks=${art.checks} prints=${art.prints} mtime=${art.mtime}`);
      return { tag, check: a.n_total, print: b.n_total };
    };
    log("→ Developer: Reload Window（让新会话开档，走产物回放）");
    await commandPalette(wb, "Developer: Reload Window");
    await sleep(30000);
    wb = await workbenchPage(60000);
    for (let i = 0; i < 40; i++) { if (await panelOpen()) break; await sleep(1500); }
    rows.push(await read("Reload 之后"));
    for (const r of rows) log(`  ${r.tag}：#check=${r.check} · #print=${r.print}`);
    snapshot("reloadread", report);
    wb.close();
    return report;
  }
  if (phase === "domk") {
    wb = await workbenchPage(20000);
    const read = async (tag) => {
      const cLine = findLine((l) => l.trim() === "#check Set.subset");
      const pLine = findLine((l) => l.trim() === "#print Set.subset");
      const a = await probeAt(wb, cLine, `${tag} · #check`);
      const b = await probeAt(wb, pLine, `${tag} · #print`);
      const art = unitArtifact();
      log(`   [模块根产物] checks=${art.checks} prints=${art.prints} mtime=${art.mtime}`);
      return { tag, check: a.n_total, print: b.n_total };
    };
    const reload = async (tag) => {
      log(`   → Developer: Reload Window（${tag}）`);
      await commandPalette(wb, "Developer: Reload Window");
      await sleep(30000);
      wb = await workbenchPage(60000);
      for (let i = 0; i < 40; i++) { if (await panelOpen()) break; await sleep(1500); }
    };
    log("① 开档（模块根产物里已经躺着 **2 份** 报告）");
    rows.push(await read("① 回放 2 份"));
    log("② 逐键编辑一次（在 eq 的 `sorry` 后加一行注释）+ 存盘");
    const thm = findLine((l) => l.startsWith("theorem eq_of_same_elements"));
    const sorry = findLine((l) => l.trim() === "sorry", thm - 1);
    await gotoLineColumn(wb, sorry, fileLines()[sorry - 1].length + 1);
    await enterEditor(wb);
    await typeChars(wb, "-- one-more-edit");
    await save(wb);
    await sleep(3500);
    rows.push(await read("② 编辑一次后（bug = 3 份）"));
    log("③ 命令面板 → 「Sokonanoda: Rebuild」：把**干净**的 k=1 产物写回模块根");
    const before = artifactsMtime();
    await commandPalette(wb, "Sokonanoda: Rebuild");
    await sleep(3000);
    if (artifactsMtime().newest === before.newest) { await key(wb, "Enter", "Enter", VK.Enter); await sleep(2000); }
    await waitArtifacts(wb);
    log(`   产物：${JSON.stringify(unitArtifact())}`);
    await reload("Rebuild 之后");
    rows.push(await read("③ Rebuild + Reload 之后（应回到 1）"));
    log("──────── 读数表 ────────");
    for (const r of rows) log(`  ${r.tag}：#check=${r.check} · #print=${r.print}`);
    snapshot("domk", report);
    wb.close();
    return report;
  }
  if (phase === "grow2") {
    wb = await workbenchPage(20000);
    const read = async (tag) => {
      const cLine = findLine((l) => l.trim() === "#check Set.subset");
      const pLine = findLine((l) => l.trim() === "#print Set.subset");
      const a = await probeAt(wb, cLine, `${tag} · #check`);
      const b = await probeAt(wb, pLine, `${tag} · #print`);
      const art = unitArtifact();
      log(`   [产物] checks=${art.checks} prints=${art.prints} mtime=${art.mtime}`);
      return { tag, check: a.n_total, print: b.n_total };
    };
    const reload = async (tag) => {
      log(`   → Developer: Reload Window（${tag}）`);
      await commandPalette(wb, "Developer: Reload Window");
      await sleep(30000);
      wb = await workbenchPage(60000);
      for (let i = 0; i < 40; i++) { if (await panelOpen()) break; await sleep(1500); }
    };
    // 真·命令文本改动（逐键）：把 eq_of_same_elements 的 `sorry` 换成证明
    const eqProof = async () => {
      const thm = findLine((l) => l.startsWith("theorem eq_of_same_elements"));
      const sorry = findLine((l) => l.trim() === "sorry", thm - 1);
      const col = fileLines()[sorry - 1].indexOf("sorry") + 1;
      log(`   → 逐键：第 ${sorry} 行 \`sorry\` → \`apply Set.ext\` + \`exact h\``);
      await gotoLineColumn(wb, sorry, col);
      await key(wb, "End", "End", VK.End, SHIFT);
      await typeChars(wb, "apply Set.ext");
      await key(wb, "End", "End", VK.End); await sleep(200);
      await enterEditor(wb);
      await typeChars(wb, "exact h");
      await save(wb);
      await sleep(2500);
    };
    const eqSorry = async () => {
      const thm = findLine((l) => l.startsWith("theorem eq_of_same_elements"));
      log(`   → 逐键：把证明换回 \`sorry\`（第 ${thm} 行的定理体）`);
      await gotoLineColumn(wb, thm + 2, 1);
      await key(wb, "ArrowDown", "ArrowDown", 40, SHIFT);
      await sleep(250);
      await insert(wb, "  sorry");
      await save(wb);
      await sleep(2500);
      log("     定理体：" + JSON.stringify(fileLines().slice(thm - 1, thm + 3)));
    };
    rows.push(await read("① 起点（Reload 之后）"));
    await eqProof();
    rows.push(await read("② eq 证完（证明完整）"));
    await reload("第一轮");
    rows.push(await read("③ Reload 之后"));
    await eqSorry();
    rows.push(await read("④ 把 sorry 加回来（证明不完整）"));
    await reload("第二轮");
    rows.push(await read("⑤ 再 Reload 之后"));
    await eqProof();
    rows.push(await read("⑥ 再证完一次"));
    await reload("第三轮");
    rows.push(await read("⑦ 第三次 Reload 之后"));
    log("──────── 读数表 ────────");
    for (const r of rows) log(`  ${r.tag}：#check=${r.check} · #print=${r.print}`);
    snapshot("grow2", report);
    wb.close();
    return report;
  }
  if (phase === "grow") {
    wb = await workbenchPage(20000);
    const read = async (tag) => {
      const cLine = findLine((l) => l.trim() === "#check Set.subset");
      const pLine = findLine((l) => l.trim() === "#print Set.subset");
      const a = await probeAt(wb, cLine, `${tag} · #check`);
      const b = await probeAt(wb, pLine, `${tag} · #print`);
      log(`   [产物] ${JSON.stringify(unitArtifact())}`);
      return { tag, check: a.n_total, print: b.n_total };
    };
    const reload = async (tag) => {
      log(`   → Developer: Reload Window（${tag}）`);
      await commandPalette(wb, "Developer: Reload Window");
      await sleep(30000);
      wb = await workbenchPage(60000);
      for (let i = 0; i < 40; i++) { if (await panelOpen()) break; await sleep(1500); }
    };
    const editOnce = async (tag) => {
      log(`   → 逐键编辑一次：文件末尾加一行注释 + 存盘（${tag}）`);
      const last = fileLines().length;
      await gotoLineColumn(wb, last, 1);
      await key(wb, "End", "End", VK.End);
      await enterEditor(wb);
      await typeChars(wb, `-- t${tag}`);
      await save(wb);
      await sleep(2500);
    };
    log("① 命令面板 → 「Sokonanoda: Rebuild」：把**当前文本**的 k=1 产物写进模块根");
    const before = artifactsMtime();
    await commandPalette(wb, "Sokonanoda: Rebuild");
    await sleep(3000);
    if (artifactsMtime().newest === before.newest) { await key(wb, "Enter", "Enter", VK.Enter); await sleep(2000); }
    await waitArtifacts(wb);
    log(`   产物：${JSON.stringify(unitArtifact())}`);
    await reload("回到干净会话");
    rows.push(await read("① Rebuild + Reload 之后"));
    for (let i = 1; i <= 4; i++) {
      await editOnce(String(i));
      rows.push(await read(`②.${i} 第 ${i} 次编辑后`));
      await reload(`第 ${i} 轮之后`);
      rows.push(await read(`③.${i} 第 ${i} 次 Reload 之后`));
    }
    log("──────── 读数表 ────────");
    for (const r of rows) log(`  ${r.tag}：#check=${r.check} · #print=${r.print}`);
    snapshot("grow", report);
    wb.close();
    return report;
  }
  if (phase === "userflow3") {
    wb = await workbenchPage(20000);
    const readBoth = async (tag) => {
      const cLine = findLine((l) => l.trim() === "#check Set.subset");
      const pLine = findLine((l) => l.trim() === "#print Set.subset");
      const a = await probeAt(wb, cLine, `${tag} · #check 行`);
      const b = await probeAt(wb, pLine, `${tag} · #print 行`);
      log(`   [产物] ${JSON.stringify(unitArtifact())}`);
      return { tag, check: a.n_total, print: b.n_total };
    };
    log("① 现状（产物里已经带两条命令）→ 先读一次");
    rows.push(await readBoth("① 起点"));
    log("② Restart Server（回放产物；cmd 归零进内存）");
    await commandPalette(wb, "Sokonanoda: Restart Server");
    await sleep(12000);
    rows.push(await readBoth("② 回放之后"));

    log("③ 逐键做 eq_of_same_elements（删掉 `sorry` ⇒ 证明完整）");
    let thmLine = findLine((l) => l.startsWith("theorem eq_of_same_elements"));
    let sorryLine = findLine((l) => l.trim() === "sorry", thmLine - 1);
    let col = fileLines()[sorryLine - 1].indexOf("sorry") + 1;
    await gotoLineColumn(wb, sorryLine, col);
    await key(wb, "End", "End", VK.End, SHIFT);
    await typeChars(wb, "apply Set.ext");
    await key(wb, "End", "End", VK.End); await sleep(200);
    await enterEditor(wb);
    await typeChars(wb, "exact h");
    await sleep(800);
    await save(wb);
    log("   定理现在：" + JSON.stringify(fileLines().slice(thmLine - 1, thmLine + 4)));
    rows.push(await readBoth("③ eq 证完（证明完整）"));

    log("④ 逐键把 `sorry` 加回来（证明不完整）");
    thmLine = findLine((l) => l.startsWith("theorem eq_of_same_elements"));
    for (let i = 0; i < 2; i++) {
      await gotoLineColumn(wb, thmLine + 2, 1);
      await key(wb, "End", "End", VK.End, SHIFT);
      await key(wb, "K", "KeyK", 75, META | SHIFT);
      await sleep(500);
    }
    await gotoLineColumn(wb, thmLine + 2, 1);
    await typeChars(wb, "  sorry");
    await sleep(800);
    await save(wb);
    log("   定理现在：" + JSON.stringify(fileLines().slice(thmLine - 1, thmLine + 3)));
    rows.push(await readBoth("④ sorry 加回来"));

    log("⑤ 再补一次注释改动（零重编译编辑）看还会不会 +1");
    await gotoLineColumn(wb, fileLines().length, 1);
    await enterEditor(wb);
    await typeChars(wb, "-- zz");
    await save(wb);
    rows.push(await readBoth("⑤ 注释改动之后"));

    log("⑥ Developer: Reload Window");
    await commandPalette(wb, "Developer: Reload Window");
    await sleep(28000);
    wb = await workbenchPage(60000);
    for (let i = 0; i < 40; i++) { if (await panelOpen()) break; await sleep(1500); }
    rows.push(await readBoth("⑥ Reload 之后"));

    log("──────── 读数表 ────────");
    for (const r of rows) log(`  ${r.tag}：#check=${r.check} · #print=${r.print}`);
    snapshot("userflow3", report);
    wb.close();
    return report;
  }
  if (phase === "userflow2") {
    wb = await workbenchPage(20000);
    const readBoth = async (tag) => {
      const cLine = findLine((l) => l.trim() === "#check Set.subset");
      const pLine = findLine((l) => l.trim() === "#print Set.subset");
      const a = await probeAt(wb, cLine, `${tag} · #check 行`);
      const b = await probeAt(wb, pLine, `${tag} · #print 行`);
      log(`   [产物] ${JSON.stringify(unitArtifact())}`);
      return { tag, check: a.n_total, print: b.n_total };
    };
    log("① 第 1 次 Restart Server（冷编一趟 ⇒ LSP 把带两条命令的产物写下来）");
    await commandPalette(wb, "Sokonanoda: Restart Server");
    await sleep(12000);
    log(`   产物 ${JSON.stringify(unitArtifact())}`);
    log("② 第 2 次 Restart Server（文本没变 ⇒ **回放命中**，cmd 归零进内存缓存）");
    await commandPalette(wb, "Sokonanoda: Restart Server");
    await sleep(12000);
    log(`   产物 ${JSON.stringify(unitArtifact())}`);
    rows.push(await readBoth("② 回放之后"));

    log("③ 逐键做 eq_of_same_elements（删掉 `sorry` ⇒ 证明完整）");
    let thmLine = findLine((l) => l.startsWith("theorem eq_of_same_elements"));
    let sorryLine = findLine((l) => l.trim() === "sorry", thmLine - 1);
    let col = fileLines()[sorryLine - 1].indexOf("sorry") + 1;
    await gotoLineColumn(wb, sorryLine, col);
    await key(wb, "End", "End", VK.End, SHIFT);
    await typeChars(wb, "apply Set.ext");
    await key(wb, "End", "End", VK.End); await sleep(200);
    await enterEditor(wb);
    await typeChars(wb, "exact h");
    await sleep(800);
    await save(wb);
    log("   定理现在：" + JSON.stringify(fileLines().slice(thmLine - 1, thmLine + 4)));
    rows.push(await readBoth("③ eq 证完（证明完整）"));

    log("④ 逐键把 `sorry` 加回来（证明不完整）");
    thmLine = findLine((l) => l.startsWith("theorem eq_of_same_elements"));
    for (let i = 0; i < 2; i++) {
      await gotoLineColumn(wb, thmLine + 2, 1);
      await key(wb, "End", "End", VK.End, SHIFT);
      await key(wb, "K", "KeyK", 75, META | SHIFT);
      await sleep(500);
    }
    await gotoLineColumn(wb, thmLine + 2, 1);
    await typeChars(wb, "  sorry");
    await sleep(800);
    await save(wb);
    log("   定理现在：" + JSON.stringify(fileLines().slice(thmLine - 1, thmLine + 3)));
    rows.push(await readBoth("④ sorry 加回来（证明不完整）"));

    log("⑤ Developer: Reload Window（用户说这能清掉重复）");
    await commandPalette(wb, "Developer: Reload Window");
    await sleep(28000);
    wb = await workbenchPage(60000);
    for (let i = 0; i < 40; i++) { if (await panelOpen()) break; await sleep(1500); }
    rows.push(await readBoth("⑤ Reload Window 之后"));

    log("──────── 读数表 ────────");
    for (const r of rows) log(`  ${r.tag}：#check=${r.check} · #print=${r.print}`);
    snapshot("userflow2", report);
    wb.close();
    return report;
  }
  if (phase === "userflow") {
    wb = await workbenchPage(20000);
    const inCount = (line) => 0;
    const readBoth = async (tag) => {
      const cLine = findLine((l) => l.trim() === "#check Set.subset");
      const pLine = findLine((l) => l.trim() === "#print Set.subset");
      const a = await probeAt(wb, cLine, `${tag} · #check 行`);
      const b = await probeAt(wb, pLine, `${tag} · #print 行`);
      log(`   [产物] ${JSON.stringify(unitArtifact())}`);
      return { tag, check: a.n_total, print: b.n_total, check_msgs: a.messages, print_msgs: b.messages };
    };
    log("① 逐键输入两条命令（真键盘，一个字符一个 keydown）");
    let thmLine = findLine((l) => l.startsWith("theorem mem_of_subset"));
    await gotoLineColumn(wb, thmLine, 1);
    await typeChars(wb, "#check Set.subset");
    await sleep(500);
    await enterEditor(wb);
    await typeChars(wb, "#print Set.subset");
    await sleep(500);
    await enterEditor(wb);
    await save(wb);
    log(`   命令已键入：盘上第 ${findLine((l) => l.trim() === "#check Set.subset")} 行 = #check，第 ${findLine((l) => l.trim() === "#print Set.subset")} 行 = #print`);
    rows.push(await readBoth("① 加完两条命令"));

    log("② 逐键做 mem_of_subset（`sorry` 选中后逐字打）");
    thmLine = findLine((l) => l.startsWith("theorem mem_of_subset"));
    let sorryLine = findLine((l) => l.trim() === "sorry", thmLine - 1);
    let col = fileLines()[sorryLine - 1].indexOf("sorry") + 1;
    await gotoLineColumn(wb, sorryLine, col);
    await key(wb, "End", "End", VK.End, SHIFT);
    await typeChars(wb, "intro ha");
    await key(wb, "End", "End", VK.End); await sleep(200);
    await enterEditor(wb);
    await typeChars(wb, "apply h");
    await key(wb, "End", "End", VK.End); await sleep(200);
    await enterEditor(wb);
    await typeChars(wb, "exact ha");
    await sleep(600);
    await save(wb);
    log("   定理现在：" + JSON.stringify(fileLines().slice(thmLine - 1, thmLine + 4)));
    rows.push(await readBoth("② 做完 mem_of_subset"));

    log("③ 逐键做 eq_of_same_elements（删掉它的 `sorry`，证明完整）");
    thmLine = findLine((l) => l.startsWith("theorem eq_of_same_elements"));
    sorryLine = findLine((l) => l.trim() === "sorry", thmLine - 1);
    col = fileLines()[sorryLine - 1].indexOf("sorry") + 1;
    await gotoLineColumn(wb, sorryLine, col);
    await key(wb, "End", "End", VK.End, SHIFT);
    await typeChars(wb, "apply Set.ext");
    await key(wb, "End", "End", VK.End); await sleep(200);
    await enterEditor(wb);
    await typeChars(wb, "exact h");
    await sleep(600);
    await save(wb);
    log("   定理现在：" + JSON.stringify(fileLines().slice(thmLine - 1, thmLine + 4)));
    rows.push(await readBoth("③ eq 证完（证明完整）"));

    log("④ 逐键把 `sorry` 加回来（证明又不完整）");
    thmLine = findLine((l) => l.startsWith("theorem eq_of_same_elements"));
    await gotoLineColumn(wb, thmLine + 2, 1);
    await key(wb, "End", "End", VK.End, SHIFT); // 选中 `  apply Set.ext`
    await key(wb, "K", "KeyK", 75, META | SHIFT); // Cmd+Shift+K 删行（macOS 绑定）
    await sleep(400);
    await gotoLineColumn(wb, thmLine + 2, 1);
    await key(wb, "End", "End", VK.End, SHIFT);
    await key(wb, "K", "KeyK", 75, META | SHIFT);
    await sleep(400);
    await gotoLineColumn(wb, thmLine + 2, 1);
    await typeChars(wb, "  sorry");
    await sleep(600);
    await save(wb);
    log("   定理现在：" + JSON.stringify(fileLines().slice(thmLine - 1, thmLine + 4)));
    rows.push(await readBoth("④ 把 sorry 加回来"));

    log("⑤ 额外操作：注释改动 + 再存盘 + 光标往返（看还会不会 +1）");
    await gotoLineColumn(wb, fileLines().length, 1);
    await enterEditor(wb);
    await typeChars(wb, "-- comment-only");
    await save(wb);
    rows.push(await readBoth("⑤ 注释改动之后"));
    await commandPalette(wb, "Developer: Reload Window");
    await sleep(25000);
    wb = await workbenchPage(60000);
    for (let i = 0; i < 40; i++) { if (await panelOpen()) break; await sleep(1500); }
    rows.push(await readBoth("⑥ Reload Window 之后"));

    log("──────── 读数表 ────────");
    for (const r of rows) log(`  ${r.tag}：#check=${r.check} · #print=${r.print}`);
    snapshot("userflow", report);
    wb.close();
    return report;
  }
  if (phase === "variant6") {
    wb = await workbenchPage(20000);
    const checkLine = findLine((l) => l.trim() === "#check Set.subset");
    const mtime = () => unitArtifact().mtime ?? "?";
    log(`① 现状：产物 ${mtime()}`);
    log("② 第 1 次 Restart：冷编一趟 ⇒ LSP 会把**当前文本**的产物写下来");
    await commandPalette(wb, "Sokonanoda: Restart Server");
    await sleep(12000);
    const afterFirst = mtime();
    log(`   产物 ${afterFirst}（应已更新）`);
    log("③ 第 2 次 Restart：文本没变 ⇒ 应当**命中并回放**（产物不该被重写）");
    await commandPalette(wb, "Sokonanoda: Restart Server");
    await sleep(12000);
    const afterSecond = mtime();
    log(`   产物 ${afterSecond} ${afterSecond === afterFirst ? "（没变 ⇒ 回放命中 ✓）" : "（又变了 ⇒ 还是没命中 ✗）"}`);
    rows.push(await probeAt(wb, checkLine, "③ 回放之后（期望 1 条）"));
    const thm = findLine((l) => l.startsWith("theorem mem_of_subset"));
    const exactLine = findLine((l) => l.trim() === "exact ha", thm - 1);
    log("④ 编辑一次（bug 触发点）");
    await gotoLineColumn(wb, exactLine, fileLines()[exactLine - 1].length + 1);
    await enterEditor(wb);
    await insert(wb, "-- v6");
    await save(wb);
    await sleep(4000);
    rows.push(await probeAt(wb, checkLine, "④ 编辑一次后（bug = 2 条）"));
    const n = rows.at(-1)?.n_total ?? 0;
    log(`${n >= 2 ? "⚠ 复现到重复输出" : "（仍没读到重复）"}：${n} 条 = ` +
      JSON.stringify((rows.at(-1)?.messages || []).map((m) => `${m.kind} ${m.text}`)));
    report.verdict = n >= 2 ? "REPRODUCED" : "NOT_REPRODUCED";
    report.artifact = unitArtifact();
    report.artifact_after_first_restart = afterFirst;
    snapshot("variant6", report);
    wb.close();
    return report;
  }
  if (phase === "variant5") {
    wb = await workbenchPage(20000);
    const checkLine = findLine((l) => l.trim() === "#check Set.subset");
    const mtime = () => { const a = unitArtifact(); return `${a.mtime ?? "?"} checks=${a.checks ?? "?"}`; };
    log(`① 现状：产物 ${mtime()}`);
    const thm = findLine((l) => l.startsWith("theorem mem_of_subset"));
    const exactLine = findLine((l) => l.trim() === "exact ha", thm - 1);
    const editOnce = async (text, tag) => {
      await gotoLineColumn(wb, exactLine, fileLines()[exactLine - 1].length + 1);
      await enterEditor(wb);
      await insert(wb, text);
      await save(wb);
      await sleep(4000);
      log(`   ${tag}：产物 ${mtime()}`);
    };
    log("② 编辑一次（文件判绿 ⇒ LSP 这趟会**自己写产物**，键是它自己的摘要）");
    await editOnce("-- v5-a", "② 之后");
    log("③ 命令面板 → 「Sokonanoda: Restart Server」");
    await commandPalette(wb, "Sokonanoda: Restart Server");
    await sleep(9000);
    log(`   重启之后：产物 ${mtime()}（**没变** = 回放命中、没重编）`);
    rows.push(await probeAt(wb, checkLine, "③ 重启后（期望 1 条）"));
    log("④ 再编辑一次（bug 触发点）");
    await editOnce("-- v5-b", "④ 之后");
    await sleep(2000);
    rows.push(await probeAt(wb, checkLine, "④ 再编辑一次后（bug = 2 条）"));
    const n = rows.at(-1)?.n_total ?? 0;
    log(`${n >= 2 ? "⚠ 复现到重复输出" : "（仍没读到重复）"}：${n} 条 = ` +
      JSON.stringify((rows.at(-1)?.messages || []).map((m) => `${m.kind} ${m.text}`)));
    report.verdict = n >= 2 ? "REPRODUCED" : "NOT_REPRODUCED";
    report.artifact = unitArtifact();
    snapshot("variant5", report);
    wb.close();
    return report;
  }
  if (phase === "variant3") {
    wb = await workbenchPage(20000);
    const thmLine = findLine((l) => l.startsWith("theorem mem_of_subset"));
    const sorryLine = findLine((l) => l.trim() === "sorry", thmLine - 1);
    const col = fileLines()[sorryLine - 1].indexOf("sorry") + 1;
    log(`① 把第 ${sorryLine} 行那个**多余**的 \`sorry\` 改成注释（它让 `+"`by`"+` 块无目标 ⇒ 判红 ⇒ 产物根本不写）`);
    await gotoLineColumn(wb, sorryLine, col);
    await key(wb, "End", "End", VK.End, SHIFT);
    await insert(wb, "-- 证完了");
    await save(wb);
    await sleep(2500);
    log("   定理现在：", JSON.stringify(fileLines().slice(thmLine - 1, thmLine + 6)));
    const checkLine = findLine((l) => l.trim() === "#check Set.subset");
    log("② 命令面板 → 「Sokonanoda: Rebuild」（此时文件判绿 ⇒ 产物带上 `#check`）");
    const before = artifactsMtime();
    await commandPalette(wb, "Sokonanoda: Rebuild");
    await sleep(3000);
    if (artifactsMtime().newest === before.newest) { await key(wb, "Enter", "Enter", VK.Enter); await sleep(2000); }
    await waitArtifacts(wb);
    const art = unitArtifact();
    log(`   产物：${JSON.stringify(art)}`);
    if (!art.checks) throw new Error("产物里还是没有 #check");
    log("③ 命令面板 → 「Sokonanoda: Restart Server」（把产物回放进内存缓存：cmd 归零）");
    await commandPalette(wb, "Sokonanoda: Restart Server");
    await sleep(9000);
    rows.push(await probeAt(wb, checkLine, "③ 回放之后（期望 1 条）"));
    const exactLine = findLine((l) => l.trim() === "exact ha", thmLine - 1);
    log(`④ 编辑一次：第 ${exactLine} 行后加一行注释`);
    await gotoLineColumn(wb, exactLine, fileLines()[exactLine - 1].length + 1);
    await enterEditor(wb);
    await insert(wb, "-- touch");
    await save(wb);
    await sleep(3500);
    rows.push(await probeAt(wb, checkLine, "④ 编辑一次后（bug = 2 条）"));
    const n = rows.at(-1)?.n_total ?? 0;
    log(`${n >= 2 ? "⚠ 复现到重复输出" : "（仍没读到重复）"}：${n} 条 = ` +
      JSON.stringify((rows.at(-1)?.messages || []).map((m) => `${m.kind} ${m.text}`)));
    report.verdict = n >= 2 ? "REPRODUCED" : "NOT_REPRODUCED";
    report.artifact = art;
    snapshot("variant3", report);
    wb.close();
    return report;
  }
  if (phase === "variant2") {
    wb = await workbenchPage(20000);
    // ① 让 mem_of_subset **真的证完**：删掉尾部多余的 `sorry`（它是 `elab-tactic-failed`，
    //    会让整个文件判红 ⇒ 产物根本不写 ⇒ 没有可回放的产物）。我上一轮加的 `-- touch` 一并删。
    const thmLine = findLine((l) => l.startsWith("theorem mem_of_subset"));
    const sorryLine = findLine((l) => l.trim() === "sorry", thmLine - 1);
    log(`① 删掉第 ${sorryLine} 行的多余 \`sorry\`（Ctrl+Shift+K 整行删）`);
    await gotoLineColumn(wb, sorryLine, 1);
    await key(wb, "K", "KeyK", 75, CTRL | SHIFT);
    await sleep(600);
    const touchLine = findLine((l) => l.trim() === "-- touch");
    if (touchLine > 0) {
      log(`   再删掉第 ${touchLine} 行的 \`-- touch\``);
      await gotoLineColumn(wb, touchLine, 1);
      await key(wb, "K", "KeyK", 75, CTRL | SHIFT);
      await sleep(600);
    }
    await save(wb);
    await sleep(2000);
    const body = fileLines().slice(thmLine - 1, thmLine + 4);
    log("   定理现在：", JSON.stringify(body));
    const checkLine = findLine((l) => l.trim() === "#check Set.subset");
    // ② Rebuild：**文件此刻是干净的** ⇒ 产物会带上这条 `#check`
    log("② 命令面板 → 「Sokonanoda: Rebuild」（清缓存 + 全项目重编；产物这次带 `#check`）");
    const before = artifactsMtime();
    await commandPalette(wb, "Sokonanoda: Rebuild");
    await sleep(3000);
    if (artifactsMtime().newest === before.newest) { await key(wb, "Enter", "Enter", VK.Enter); await sleep(2000); }
    await waitArtifacts(wb);
    const art = unitArtifact();
    log(`   产物：${JSON.stringify(art)}`);
    if (!art.checks) throw new Error("产物里还是没有 #check —— 文件仍判红？");
    // ③ 重启服务器 ⇒ 产物被**回放**（cmd 归零的那一份进内存缓存）
    log("③ 命令面板 → 「Sokonanoda: Restart Server」（回放产物）");
    await commandPalette(wb, "Sokonanoda: Restart Server");
    await sleep(9000);
    rows.push(await probeAt(wb, checkLine, "③ 回放之后（期望 1 条）"));
    // ④ 编辑一次
    const exactLine = findLine((l) => l.trim() === "exact ha", thmLine - 1);
    log(`④ 编辑一次：第 ${exactLine} 行后加一行注释`);
    await gotoLineColumn(wb, exactLine, fileLines()[exactLine - 1].length + 1);
    await enterEditor(wb);
    await insert(wb, "-- touch");
    await save(wb);
    await sleep(3500);
    rows.push(await probeAt(wb, checkLine, "④ 编辑一次后（bug = 2 条）"));
    const n = rows.at(-1)?.n_total ?? 0;
    log(`${n >= 2 ? "⚠ 复现到重复输出" : "（仍没读到重复）"}：${n} 条 = ` +
      JSON.stringify((rows.at(-1)?.messages || []).map((m) => `${m.kind} ${m.text}`)));
    report.verdict = n >= 2 ? "REPRODUCED" : "NOT_REPRODUCED";
    report.artifact = art;
    snapshot("variant2", report);
    wb.close();
    return report;
  }
  if (phase === "variant") {
    wb = await workbenchPage(20000);
    const checkLine = findLine((l) => l.trim() === "#check Set.subset");
    if (checkLine < 0) throw new Error("文件里没有 #check Set.subset");
    log("(a) 命令面板 → 「Sokonanoda: Rebuild」——**这次文件里已经有 `#check`**，产物会带上它");
    const before = artifactsMtime();
    await commandPalette(wb, "Sokonanoda: Rebuild");
    await sleep(3000);
    if (artifactsMtime().newest === before.newest) { await key(wb, "Enter", "Enter", VK.Enter); await sleep(2000); }
    await waitArtifacts(wb);
    const art = unitArtifact();
    log(`    产物：${JSON.stringify(art)}`);
    if (!art.checks) throw new Error("产物里没有 #check —— 条件不成立，不用往下走");
    log("(b) 命令面板 → 「Sokonanoda: Restart Server」——重启会把产物**回放**进内存缓存");
    await commandPalette(wb, "Sokonanoda: Restart Server");
    await sleep(9000);
    rows.push(await probeAt(wb, checkLine, "(b) 重启回放后（期望 1 条）"));
    const thm = findLine((l) => l.startsWith("theorem mem_of_subset"));
    const exactLine = findLine((l) => l.trim() === "exact ha", thm - 1);
    log(`(c) 编辑一次（第 ${exactLine} 行后加注释）`);
    await gotoLineColumn(wb, exactLine, fileLines()[exactLine - 1].length + 1);
    await enterEditor(wb);
    await insert(wb, "  -- touch");
    await save(wb);
    await sleep(3500);
    rows.push(await probeAt(wb, checkLine, "(c) 编辑一次后（bug = 2 条）"));
    const n = rows.at(-1)?.n_total ?? 0;
    log(`${n >= 2 ? "⚠ 复现到重复输出" : "（仍没读到重复）"}：${n} 条 = ` +
      JSON.stringify((rows.at(-1)?.messages || []).map((m) => `${m.kind} ${m.text}`)));
    report.verdict = n >= 2 ? "REPRODUCED" : "NOT_REPRODUCED";
    report.artifact = art;
    snapshot("variant", report);
    wb.close();
    return report;
  }
  if (phase === "retest") {
    wb = await workbenchPage(20000);
    const checkLine = findLine((l) => l.trim() === "#check Set.subset");
    if (checkLine < 0) throw new Error("文件里没有 #check Set.subset");
    log("④a 命令面板 → 「Sokonanoda: Restart Server」（重启会**回放项目产物**）");
    await commandPalette(wb, "Sokonanoda: Restart Server");
    await sleep(9000);
    rows.push(await probeAt(wb, checkLine, "④a 重启服务器后（期望 1 条）"));
    const thm = findLine((l) => l.startsWith("theorem mem_of_subset"));
    const exactLine = findLine((l) => l.trim() === "exact ha", thm - 1);
    log(`④b 再编辑一处：第 ${exactLine} 行后加一行注释`);
    await gotoLineColumn(wb, exactLine, fileLines()[exactLine - 1].length + 1);
    await enterEditor(wb);
    await insert(wb, "  -- touch");
    await save(wb);
    await sleep(3000);
    rows.push(await probeAt(wb, checkLine, "④b 编辑一次后（bug = 2 条）"));
    const n = rows.at(-1)?.n_total ?? 0;
    log(`${n >= 2 ? "⚠ 复现到重复输出" : "（仍没读到重复）"}：${n} 条 = ` +
      JSON.stringify((rows.at(-1)?.messages || []).map((m) => `${m.kind} ${m.text}`)));
    report.verdict = n >= 2 ? "REPRODUCED" : "NOT_REPRODUCED";
    snapshot("retest", report);
    wb.close();
    return report;
  }
  if (phase === "check") {
    wb = await workbenchPage(20000);
    const es = await gotoLineColumn(wb, Number(process.env.CHECK_AT || 45), Number(process.env.CHECK_COL || 3));
    const panel = await stablePanel();
    log(`   编辑器：${es.pos} · 该行渲染文本=${JSON.stringify(es.cursorLineText)}`);
    log(`   面板：.decl=${panel?.hasDecl} .status=${panel?.hasStatus} · ${panel?.status} · 命令输出 ${panel?.messages?.length ?? "?"} 条`);
    log(`   盘上第 45 行=${JSON.stringify(fileLines()[44])}`);
    report.check = { pos: es.pos, cursor_line_text: es.cursorLineText, panel };
    wb.close();
    return report;
  }
  if (phase === "launch" || phase === "all") {
    rmSync(PROFILE, { recursive: true, force: true });
    writeProfile();
    log("① 启动真 VS Code（Extension Development Host）· 工作区 =", WS);
    log("   LSP（serverPath 覆盖）=", LSP, existsSync(LSP) ? "(存在)" : "(缺失 ✗)");
    report.vscode_pid = launchVscode();
    wb = await workbenchPage();
    log("   workbench 已就绪；等 Infoview 面板…");
    for (let i = 0; i < 40; i++) { if (await panelOpen()) break; await sleep(1500); }
    if (!(await panelOpen())) { log("   → 发命令：Sokonanoda: Infoview"); await commandPalette(wb, "Sokonanoda: Infoview"); }
    for (let i = 0; i < 60; i++) { if (await panelOpen()) break; await sleep(1500); }
    const p0 = await stablePanel(20000);
    log(`   面板：.decl=${p0?.hasDecl} .status=${p0?.hasStatus} · ${p0?.status} · ${p0?.server}`);
    report.panel_initial = p0;
    const es0 = await editorState(wb);
    log(`   编辑器：${es0.pos} · 光标行文本=${JSON.stringify((es0.cursorLineText || "").slice(0, 60))}`);
    log(`   文件：${fileLines().length} 行 · 产物 ${artifactsMtime().count} 条`);
    if (phase === "launch") { wb.close(); return report; }
  } else {
    wb = await workbenchPage(20000);
  }

  if (phase === "rebuild" || phase === "all") {
    log("② 命令面板 → 「Sokonanoda: Rebuild」（真键位）");
    const before = artifactsMtime();
    await commandPalette(wb, "Sokonanoda: Rebuild");
    await sleep(3000);
    if (artifactsMtime().newest === before.newest) { await key(wb, "Enter", "Enter", VK.Enter); await sleep(2000); }
    const after = await waitArtifacts(wb);
    log(`   产物：${before.count} → ${after.count} 条（clean + 全项目重编完成）`);
    report.artifacts_after_rebuild = after;
    await sleep(2500);
    if (phase === "rebuild") { snapshot("rebuild", report); wb.close(); return report; }
  }

  // ③ 在 `theorem mem_of_subset` **上方**加一行 `#check Set.subset`，先看到**一行**
  log("③ 在 theorem mem_of_subset 上方加 `#check Set.subset`");
  const thmLine = findLine((l) => l.startsWith("theorem mem_of_subset"));
  if (thmLine < 0) throw new Error("找不到 theorem mem_of_subset（文件不是初始状态？）");
  log(`   theorem mem_of_subset 在第 ${thmLine} 行（盘上原文：${JSON.stringify(fileLines()[thmLine - 1].slice(0, 60))}）`);
  await gotoLineColumn(wb, thmLine, 1);
  await insert(wb, "#check Set.subset");
  await enterEditor(wb); // 上方插入新行（自动缩进 = 0）
  await sleep(600);
  await save(wb);
  const checkLine = findLine((l) => l.trim() === "#check Set.subset");
  if (checkLine < 0) throw new Error("`#check Set.subset` 没落到独立一行（打字没进编辑器？）");
  log(`   \`#check Set.subset\` 落在第 ${checkLine} 行（thm 现在第 ${findLine((l) => l.startsWith("theorem mem_of_subset"))} 行）`);
  report.check_line = checkLine;
  await sleep(2500);
  rows.push(await probeAt(wb, checkLine, "③ 刚加完 #check（期望 1 条）"));

  // ④ 做题目：theorem mem_of_subset 的 `sorry` 换成 `intro ha / apply h / exact ha / sorry`
  log("④ 做题目 mem_of_subset：`intro ha` → `apply h` → `exact ha` → `sorry`");
  const thmLine2 = findLine((l) => l.startsWith("theorem mem_of_subset"));
  const sorryLine = findLine((l) => l.trim() === "sorry" && l.startsWith("  "), thmLine2 - 1);
  if (sorryLine < 0) throw new Error("找不到 mem_of_subset 的 sorry 行");
  const sorryCol = fileLines()[sorryLine - 1].indexOf("sorry") + 1;
  log(`   sorry 在第 ${sorryLine} 行第 ${sorryCol} 列`);
  await gotoLineColumn(wb, sorryLine, sorryCol);
  await key(wb, "End", "End", VK.End, SHIFT); // 选中 `sorry`（保留两格缩进）
  await sleep(200);
  await insert(wb, "intro ha");
  await key(wb, "End", "End", VK.End); await sleep(150);
  await enterEditor(wb);
  await insert(wb, "apply h");
  await enterEditor(wb);
  await insert(wb, "exact ha");
  await enterEditor(wb);
  await insert(wb, "sorry");
  await sleep(500);
  await save(wb);
  const thmLine3 = findLine((l) => l.startsWith("theorem mem_of_subset"));
  const block = fileLines().slice(thmLine3 - 1, thmLine3 + 4);
  log("   盘上定理现在的样子：", JSON.stringify(block));
  if (!block.some((l, i) => i > 0 && l.trim() === "intro ha")) throw new Error("证明体没写进去（打字没进编辑器？）");
  await sleep(3000);

  // ⑤ 光标移回 `#check Set.subset` ⇒ 期望**两行**
  const checkLine2 = findLine((l) => l.trim() === "#check Set.subset");
  log(`⑤ 光标移回 \`#check Set.subset\`（第 ${checkLine2} 行）——看**两份**`);
  rows.push(await probeAt(wb, checkLine2, "⑤ 编辑完定理后回到 #check（bug = 2 条）"));

  const dup = rows.at(-1)?.n_total ?? 0;
  log("──────── 读数表 ────────");
  for (const r of rows) log(`  ${r.label}：#check=${r.n_check} 总条数=${r.n_total}`);
  log(`${dup >= 2 ? "⚠ 复现到重复输出" : "（本轮没读到重复）"}：最后一次 ${dup} 条 = ` +
    JSON.stringify((rows.at(-1)?.messages || []).map((m) => `${m.kind} ${m.text}`)));
  report.verdict = dup >= 2 ? "REPRODUCED" : "NOT_REPRODUCED";
  snapshot("final", report);
  wb.close();
  return report;
}

main().then((r) => { console.log("[done]", JSON.stringify({ verdict: r.verdict ?? null, check_line: r.check_line ?? null })); process.exit(0); })
  .catch((e) => { console.error("[error]", e && e.message ? e.message : e); process.exit(2); });
