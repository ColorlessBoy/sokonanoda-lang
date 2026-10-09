#!/usr/bin/env node
// scripts/site-screenshot.mjs —— 给官网 HERO 截一张**真 VS Code + 已发布插件**的图。
//
// 为什么是 CDP 而不是系统截图
// ---------------------------
// macOS 的 `screencapture` 要「屏幕录制」权限，本机实测被拒（`could not create image
// from display`）⇒ 系统截图这条路在权限到手之前**不可复现**。这里改走 **Chrome
// DevTools Protocol**：VS Code 是 Electron 应用，`--remote-debugging-port` 之后
// `Page.captureScreenshot` 拿到的是**应用自己渲染器**的画面 —— 不需要任何系统权限、
// 可以按 DOM 量出来的矩形精确裁切，还能顺手用真 DOM 断言"面板里到底算了什么"。
//
// 图里必须是什么（硬约束，见 docs/design/site-single-page.md）
// ---------------------------------------------------------
// **只允许**出现真实文件里的定理文本、`sorry`、以及**内核算出来的目标状态**（`⊢ …`）。
// 于是本脚本：① 打开仓库里真实存在的课程单元（默认卷 I 单元④的 `inter_comm`）；
// ② 把光标停在它的 `sorry` 上，让 Infoview 显示内核算出的目标；③ **等**到面板里
// 出现 `⊢` 才继续（不是"等 3 秒"）；④ 按 DOM 量出来的锚点行位置滚动、再量一次确认；
// ⑤ 截完用系统 Vision OCR **回读像素**，断言图里真的有定理名、`sorry`、目标等式 ——
// 缺一条就退出码 1 **并删掉图**。判据落在**像素**上：数据对了 ≠ 用户看见了。
//
// 用法
// ----
//   node scripts/site-screenshot.mjs                 # 亮色，版本取 site/data/site.json（已发布版本）
//   node scripts/site-screenshot.mjs --theme dark    # 暗色 ⇒ site/assets/hero-vscode-dark.png
//   node scripts/site-screenshot.mjs --out /tmp/x.png --keep
//   node scripts/site-screenshot.mjs --vsix ~/Downloads/sokonanoda-darwin-arm64.vsix
//   node scripts/site-screenshot.mjs --version 0.87.0 --no-verify
//
// 两版主题（用户 2026-10-09）：站点跟随系统配色 ⇒ 头图也要两版，否则暗色页上贴一张
// 亮色截图会刺眼。**两张图必须同一版 VSIX、同一靶子、同一裁切几何**，只换
// `workbench.colorTheme` —— 这样两版只是"同一画面的两种配色"，不会各说各话。
//
// 依赖：macOS + 本机已装 VS Code（`/Applications/Visual Studio Code.app`，可用
// `SOKO_VSCODE_BIN` 覆盖）+ `gh`（只在需要下载 VSIX 时；也可用 `--vsix` 免下载）
// + `swift`（OCR 判据；`--no-verify` 可跳过，但那样就只剩 DOM 判据）。**没有 npm 依赖。**
//
// 工作树脏就拒绝开工（`--allow-dirty` 覆盖）：站点写的是**已发布版本**的事实，
// 拿未提交的课程文件截图会写出对不上的图（AGENTS.md 的同一条纪律）。

import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const CACHE = join(tmpdir(), "soko-site-shot");
const log = (...a) => console.log("[shot]", ...a);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── 参数 ────────────────────────────────────────────────────────────────────
function usage() {
  console.log(readFileSync(fileURLToPath(import.meta.url), "utf8")
    .split("\n").slice(1, 33).map((l) => l.replace(/^\/\/ ?/, "")).join("\n"));
}

function parseArgs(argv) {
  const a = { out: null, theme: "light", port: 9411, keep: false, verify: true, dirty: false };
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    if (k === "--out") a.out = resolve(argv[++i]);
    else if (k === "--theme") a.theme = argv[++i];
    else if (k === "--version") a.version = argv[++i];
    else if (k === "--vsix") a.vsix = resolve(argv[++i]);
    else if (k === "--port") a.port = Number(argv[++i]);
    else if (k === "--file") a.file = resolve(argv[++i]);
    else if (k === "--anchor") a.anchor = argv[++i];
    else if (k === "--keep") a.keep = true;
    else if (k === "--no-verify") a.verify = false;
    else if (k === "--allow-dirty") a.dirty = true;
    else if (k === "-h" || k === "--help") { usage(); process.exit(0); }
    else { console.error(`未知参数：${k}（--help 看用法）`); process.exit(2); }
  }
  if (a.theme !== "light" && a.theme !== "dark") { console.error(`--theme 只认 light|dark（给了 ${a.theme}）`); process.exit(2); }
  // 默认输出名跟着主题走：亮色仍是历史那个 `hero-vscode.png`（既有引用不用改），
  // 暗色是 `hero-vscode-dark.png`（站点用 CSS 按配色换）。
  if (!a.out) a.out = join(REPO, a.theme === "dark" ? "site/assets/hero-vscode-dark.png" : "site/assets/hero-vscode.png");
  return a;
}
const args = parseArgs(process.argv.slice(2));

// 靶子：卷 I 单元④ 的 `inter_comm` —— 画布上带 `sorry` 的真练习，光标停在 sorry 上时
// 内核算出的目标就是 `⊢ (A ∩ B) = (B ∩ A)`。
const TARGET = {
  file: args.file || join(REPO, "courses/set-theory/units/I.1/unit04-extensionality-identities.sokonanoda"),
  workspace: join(REPO, "courses/set-theory"),
  anchor: args.anchor || "练习 2",
  theorem: "theorem inter_comm",
};

// ── 0. 图里那些内容必须是"已发布"的 ─────────────────────────────────────────
// 判据是**图里会出现什么**：课程文件（真源）与 site/data/site.json（版本号）。
// `crates/` 的未提交改动**不在判据里**：判卷用的 LSP 来自**已发布 VSIX 自带的那份二进制**，
// 仓库里的 Rust 工作树脏不脏都进不了这张图（本仓库常有人并行改 crates/，查它会误伤）。
function assertCleanWorktree() {
  const porcelain = execFileSync("git", ["status", "--porcelain", "--", TARGET.file, TARGET.workspace, "site/data/site.json"],
    { cwd: REPO, encoding: "utf8" }).trim();
  if (porcelain && !args.dirty) {
    console.error("[shot] 这些路径有未提交改动，拒绝截图（站点写的是已发布版本的事实）：\n" + porcelain + "\n  确认无所谓再加 --allow-dirty。");
    process.exit(3);
  }
  if (porcelain) log("⚠ --allow-dirty：" + porcelain.split("\n")[0] + " …");
}

// ── 1. 已发布版本：**唯一**来源是 site/data/site.json ────────────────────────
function publishedVersion() {
  if (args.version) return args.version;
  const data = JSON.parse(readFileSync(join(REPO, "site/data/site.json"), "utf8"));
  if (!data.version) throw new Error("site/data/site.json 里没有 version");
  return data.version;
}

// ── 2. 已发布 VSIX：本地 → 缓存 → gh 按 tag 下载（禁用 latest）──────────────
function platformVsix() {
  const table = {
    "darwin-arm64": "sokonanoda-darwin-arm64.vsix",
    "darwin-x64": "sokonanoda-darwin-x64.vsix",
  };
  const name = table[`${process.platform}-${process.arch}`];
  if (!name) throw new Error(`没有 ${process.platform}-${process.arch} 的 VSIX 名（用 --vsix 指一个本地文件）`);
  return name;
}

function vsixPath(version) {
  if (args.vsix) return args.vsix;
  const cached = join(CACHE, `v${version}`, platformVsix());
  if (existsSync(cached)) return cached;
  mkdirSync(dirname(cached), { recursive: true });
  log(`下载已发布 VSIX：v${version} / ${platformVsix()}`);
  execFileSync("gh", ["release", "download", `v${version}`, "--repo", "ColorlessBoy/sokonanoda-lang",
    "-p", platformVsix(), "--dir", dirname(cached), "--clobber"], { stdio: "inherit" });
  if (!existsSync(cached)) throw new Error("下载后仍找不到 " + cached);
  return cached;
}

function stageExtension(version, vsix) {
  const dir = join(CACHE, `v${version}`, "extensions");
  const installed = join(dir, `sokonanoda-lang.sokonanoda-${version}`);
  if (!existsSync(join(installed, "package.json"))) {
    const unpack = join(CACHE, `v${version}`, "unpack");
    rmSync(unpack, { recursive: true, force: true });
    mkdirSync(unpack, { recursive: true });
    mkdirSync(dir, { recursive: true });
    execFileSync("unzip", ["-q", "-o", vsix, "-d", unpack]);
    mkdirSync(installed, { recursive: true });
    execFileSync("cp", ["-R", join(unpack, "extension") + "/.", installed]);
  }
  const staged = JSON.parse(readFileSync(join(installed, "package.json"), "utf8")).version;
  if (staged !== version) throw new Error(`VSIX 里的版本是 ${staged}，期望 ${version}`);
  return dir;
}

// ── 3. 真 VS Code（干净 profile：不碰用户自己的配置与扩展）───────────────────
function vscodeBin() {
  for (const c of [process.env.SOKO_VSCODE_BIN, "/Applications/Visual Studio Code.app/Contents/MacOS/Code"]) {
    if (c && existsSync(c)) return c;
  }
  throw new Error("找不到 VS Code 可执行文件（设 SOKO_VSCODE_BIN 指过去）");
}

function launch(version, extDir, cursorLine) {
  const profile = join(CACHE, `v${version}`, `profile-${args.port}`);
  rmSync(profile, { recursive: true, force: true });
  mkdirSync(join(profile, "User"), { recursive: true });
  // 首帧状态就是**图的一部分**，所以这里把 UI 显式钉死（都是"让图更像教材"的选择）：
  //   · 主题跟着 `--theme`（亮色配站点那张"方格纸"；暗色给暗色页用）；
  //   · 放大一档 —— 裁出来 ~840 CSS px 宽，正好铺在站点 592–832px 的栏里还是可读字号；
  //   · 关掉面包屑 / 粘性滚动 / 欢迎页 / 提示 / Copilot 登录弹窗 / git 父目录询问 ——
  //     它们是噪声，会把"定理 + sorry + 目标"挤出画面；
  //   · 关掉平滑滚动 —— 滚动定位必须是确定的。
  writeFileSync(join(profile, "User", "settings.json"), JSON.stringify({
    "workbench.startupEditor": "none",
    "workbench.tips.enabled": false,
    "workbench.colorTheme": args.theme === "dark" ? "Default Dark Modern" : "Default Light Modern",
    "workbench.secondarySideBar.defaultVisibility": "visible",
    "window.commandCenter": false,
    "window.zoomLevel": 1,
    "breadcrumbs.enabled": false,
    "editor.stickyScroll.enabled": false,
    "git.openRepositoryInParentFolders": "never",
    "update.showReleaseNotes": false,
    "extensions.autoCheckUpdates": false,
    "extensions.autoUpdate": false,
    "telemetry.telemetryLevel": "off",
    "editor.minimap.enabled": false,
    "editor.fontSize": 14,
    "editor.lineHeight": 22,
    "editor.smoothScrolling": false,
    "workbench.list.smoothScrolling": false,
    "security.workspace.trust.enabled": false,
    "chat.disableAIFeatures": true,
  }, null, 2));

  const child = spawn(vscodeBin(), [
    `--user-data-dir=${profile}`,
    `--extensions-dir=${extDir}`,
    `--remote-debugging-port=${args.port}`,
    "--disable-gpu", "--disable-chromium-sandbox",
    "--no-first-run", "--disable-workspace-trust",
    "--disable-extension", "GitHub.copilot-chat",
    "--disable-extension", "GitHub.copilot",
    "--goto", `${TARGET.file}:${cursorLine}:3`,
    TARGET.workspace,
  ], { detached: true, stdio: ["ignore", "ignore", "ignore"] });
  child.unref();
  return profile;
}

function quit(profile) {
  try { execFileSync("pkill", ["-f", `user-data-dir=${profile}`]); } catch { /* 已经不在了 */ }
}

// ── 4. CDP 客户端（零依赖：Node 自带 WebSocket）──────────────────────────────
async function cdpTargets() {
  return (await fetch(`http://127.0.0.1:${args.port}/json/list`)).json();
}

async function connect(url) {
  const ws = new WebSocket(url);
  await new Promise((res, rej) => { ws.addEventListener("open", res, { once: true }); ws.addEventListener("error", rej, { once: true }); });
  let id = 0;
  const pending = new Map();
  ws.addEventListener("message", (ev) => {
    const m = JSON.parse(ev.data);
    const p = pending.get(m.id);
    if (p) { pending.delete(m.id); p(m); }
  });
  const call = (method, params = {}) => new Promise((res) => { const n = ++id; pending.set(n, res); ws.send(JSON.stringify({ id: n, method, params })); });
  const client = {
    close: () => ws.close(),
    async eval(expression) {
      const m = await call("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
      if (m.result?.exceptionDetails) throw new Error(JSON.stringify(m.result.exceptionDetails).slice(0, 200));
      return m.result?.result?.value;
    },
    async json(expression) { return JSON.parse(await client.eval(`JSON.stringify(${expression})`)); },
    async key(key, code, modifiers = 0) {
      const base = { key, code, modifiers, windowsVirtualKeyCode: 0, nativeVirtualKeyCode: 0 };
      await call("Input.dispatchKeyEvent", { type: "rawKeyDown", ...base });
      await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
    },
    async type(text) { await call("Input.insertText", { text }); },
    async wheel(x, y, deltaY) { await call("Input.dispatchMouseEvent", { type: "mouseWheel", x, y, deltaX: 0, deltaY, pointerType: "mouse" }); },
    // **整窗截图，绝不用 clip**：带 clip 的 captureScreenshot 会按裁切框重排页面
    // （实测：panel 内容整体漂了 80+ px、还混进了编辑器文字），裁切一律交给 sips。
    async shot(file) {
      const m = await call("Page.captureScreenshot", { format: "png" });
      writeFileSync(file, Buffer.from(m.result.data, "base64"));
    },
  };
  return client;
}

async function workbenchPage() {
  for (let i = 0; i < 60; i++) {
    await sleep(1000);
    try {
      const page = (await cdpTargets()).find((t) => t.type === "page" && t.url.includes("workbench"));
      if (page) return await connect(page.webSocketDebuggerUrl);
    } catch { /* 还没起来 */ }
  }
  throw new Error("VS Code 没有暴露 CDP（--remote-debugging-port 没生效？）");
}

// Infoview 是**独立进程的 webview**：它自己在 /json/list 里是一个 target，真正的界面
// 又在它内部那个同源的 `#active-frame` 里 ⇒ "面板里现在是什么"只能进去问。每次重连，
// 因为面板会随文档重渲染换 target（实测：连接期间旧 target 会失效）。
async function infoviewEval(expression, { tries = 8 } = {}) {
  for (let i = 0; i < tries; i++) {
    let list = [];
    try { list = await cdpTargets(); } catch { /* retry */ }
    for (const t of list.filter((t) => t.type === "iframe" || (t.type === "page" && t.url.startsWith("vscode-webview:")))) {
      let c;
      try {
        c = await connect(t.webSocketDebuggerUrl);
        const v = await c.eval(`(() => { const f = document.querySelector('#active-frame'); const d = f && f.contentDocument; if (!d || !d.body) return null; return JSON.stringify(${expression}); })()`);
        if (v && v !== "null") return JSON.parse(v);
      } catch { /* 陈旧 target */ } finally { try { c?.close(); } catch { /* ignore */ } }
    }
    await sleep(700);
  }
  return null;
}

// ── 5. Vision OCR：判据落在**像素**上 ───────────────────────────────────────
const OCR_SWIFT = `
import Foundation
import Vision
import AppKit
let path = CommandLine.arguments[1]
guard let img = NSImage(contentsOfFile: path), let cg = img.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
  FileHandle.standardError.write("cannot read \\(path)\\n".data(using: .utf8)!); exit(2)
}
let req = VNRecognizeTextRequest()
req.recognitionLevel = .accurate
req.recognitionLanguages = ["en-US", "zh-Hans"]
req.usesLanguageCorrection = false
try VNImageRequestHandler(cgImage: cg, options: [:]).perform([req])
for obs in (req.results ?? []) {
  guard let c = obs.topCandidates(1).first else { continue }
  let b = obs.boundingBox
  print(String(format: "%.4f %.4f %.4f %.4f\\t%@", b.origin.x, b.origin.y, b.size.width, b.size.height, c.string))
}
`;

function ocr(file) {
  const swift = join(CACHE, "ocr.swift");
  if (!existsSync(swift)) { mkdirSync(CACHE, { recursive: true }); writeFileSync(swift, OCR_SWIFT); }
  const mcache = join(CACHE, "mcache");
  mkdirSync(mcache, { recursive: true });
  const out = execFileSync("swift", ["-module-cache-path", mcache, swift, file], {
    env: { ...process.env, TMPDIR: CACHE, CLANG_MODULE_CACHE_PATH: mcache, SWIFT_MODULE_CACHE_PATH: mcache },
    encoding: "utf8",
  });
  return out.trim().split("\n").filter(Boolean).map((line) => {
    const [box, ...rest] = line.split("\t");
    const [x, y, w, h] = box.split(" ").map(Number);
    return { x, y, w, h, text: rest.join("\t") };
  });
}

// ── 主流程 ──────────────────────────────────────────────────────────────────
assertCleanWorktree();
const version = publishedVersion();

// 靶子行从**真文件**里读出来（行号不许手写：课程会长）。
const source = readFileSync(TARGET.file, "utf8").split("\n");
const anchorLine = source.findIndex((l) => l.includes(TARGET.anchor)) + 1;
if (!anchorLine) throw new Error(`${TARGET.file} 里找不到锚点「${TARGET.anchor}」`);
const theoremLine = source.findIndex((l) => l.includes(TARGET.theorem)) + 1;
if (!theoremLine) throw new Error(`${TARGET.file} 里找不到「${TARGET.theorem}」`);
const cursorLine = source.findIndex((l, i) => i > theoremLine && l.trim() === "sorry") + 1;
if (!cursorLine) throw new Error(`${TARGET.theorem} 下面没有 sorry`);
log(`靶子 ${TARGET.file.replace(REPO + "/", "")}（锚点 L${anchorLine} · 定理 L${theoremLine} · 光标 L${cursorLine}）· 插件 v${version} · 文件里不许有假内容`);

const profile = launch(version, stageExtension(version, vsixPath(version)), cursorLine);
if (!args.keep) process.on("exit", () => quit(profile));

const wb = await workbenchPage();
try {
  await sleep(4000);

  // 首启的登录弹窗会盖住编辑器 —— 点掉它（真 UI 动作，不做假）。
  await wb.eval(`(() => {
    const click = (root) => { for (const el of root.querySelectorAll('*')) {
      if (el.shadowRoot && click(el.shadowRoot)) return true;
      if (el.children.length === 0 && /^(Continue without Signing In|Not now|Skip|No, Thanks)$/i.test((el.textContent || '').trim())) { el.click(); return true; }
    } return false; };
    return click(document);
  })()`);
  await sleep(1500);

  // 打开目标面板：走**命令面板**（与用户按键是同一条路，不是内部 API）。
  await wb.key("P", "KeyP", 12);
  await sleep(1200);
  await wb.type("Sokonanoda: Infoview");
  await sleep(1500);
  await wb.key("Enter", "Enter");

  // **等内核算完**：面板里出现 `⊢` 才算就绪。
  let panel = null;
  for (let i = 0; i < 60; i++) {
    await sleep(1000);
    const text = await infoviewEval(`document.querySelector('#active-frame').contentDocument.body.innerText`);
    if (text && text.includes("⊢")) { panel = text; break; }
  }
  if (!panel) throw new Error("等不到目标面板里的 ⊢（内核没算出目标？）");
  log("面板就绪：" + panel.split("\n").map((s) => s.trim()).filter(Boolean).slice(0, 7).join(" · "));

  // 目标面板底部（DOM 量，OCR 兜底）：图的下边界就是它。
  const panelBottomFrame = await infoviewEval(`(() => {
    const hits = [...document.querySelectorAll('*')].filter((e) => (e.textContent || '').includes('进度'));
    const el = hits[hits.length - 1];
    return el ? Math.round(el.getBoundingClientRect().bottom) : null;
  })()`);

  // 扩展在算完目标后会**自己 reveal 一次**（把声明滚到视野里）⇒ 先等布局静下来，
  // 再按"锚点行的实测 y"滚动 —— 别用"行号 × 行高"推算：代码镜（exercise: open）
  // 会让行高不均匀，推算出来的位置实测差十几行。
  const readScroll = () => wb.eval(`(() => { const lc = document.querySelector('.monaco-editor .lines-content'); const m = /top: (-?[\\d.]+)px/.exec(lc.getAttribute('style') || ''); return m ? Math.round(-Number(m[1])) : 0; })()`);
  let prev = -1;
  for (let i = 0; i < 12; i++) {
    const cur = await readScroll();
    if (cur === prev) break;
    prev = cur;
    await sleep(1500);
  }

  const geo = await wb.json(`(() => {
    const R = (el) => { const r = el.getBoundingClientRect(); return [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)]; };
    return { aux: R(document.querySelector('.part.auxiliarybar')), group: R(document.querySelector('.editor-group-container')), frame: R(document.querySelector('iframe.webview')), view: [innerWidth, innerHeight] };
  })()`);
  const wheelX = geo.group[0] + 200;
  const wheelY = geo.group[1] + 300;
  const wantAnchorY = geo.group[1] + 72;
  // 锚点行：按**文本**找，不按"行号 × 行高"推算 —— 扩展的代码镜（exercise: open）
  // 会让行高不均匀，推算出来的位置实测差十几行。
  const lineY = () => wb.eval(`(() => {
    const hits = [...document.querySelectorAll('.editor-group-container .view-line')]
      .filter((e) => e.textContent.replace(/\\u00a0/g, ' ').includes(${JSON.stringify(TARGET.anchor)}))
      .map((e) => Math.round(e.getBoundingClientRect().y));
    return hits.length ? Math.min(...hits) : null;
  })()`);
  const visibleLines = () => wb.json(`(() => {
    const n = [...document.querySelectorAll('.editor-group-container .line-numbers')].map((e) => Number(e.textContent)).filter((v) => v > 0);
    return n.length ? [Math.min(...n), Math.max(...n)] : null;
  })()`);
  // 滚轮 deltaY → 像素的换算随机器/系统设置变（本机实测 100 → 42px）。**先量再算**。
  // 但 VS Code 还会把滚轮**量化到整行**（小 delta 干脆不动）⇒ "精确落在 105px" 做不到，
  // 判据改成"落在标签栏下面那一行的窗口里"（±半行）。
  let gain = 0.42;
  {
    const y0 = await lineY();
    if (y0 !== null) {
      await wb.wheel(wheelX, wheelY, 100);
      await sleep(350);
      const y1 = await lineY();
      if (y1 !== null && Math.abs(y1 - y0) > 2) gain = Math.abs(y1 - y0) / 100;
      await wb.wheel(wheelX, wheelY, -100);
      await sleep(350);
    }
  }
  const wheelPerLine = 22 / gain;
  log(`滚轮换算 1 单位 ≈ ${gain.toFixed(3)} px（一行 ≈ ${wheelPerLine.toFixed(0)} 单位）`);
  const lowY = wantAnchorY - 12;
  const highY = wantAnchorY + 12;
  let anchorY = null;
  for (let i = 0; i < 30; i++) {
    const y = await lineY();
    if (y === null) {
      // 不在视野里：用行号槽（.line-numbers）判断该往上还是往下找。
      const range = await visibleLines();
      const dir = !range || anchorLine > range[1] ? 1 : -1;
      await wb.wheel(wheelX, wheelY, dir * 300);
      await sleep(300);
      continue;
    }
    anchorY = y;
    if (y >= lowY && y <= highY) break;
    const notches = Math.max(-6, Math.min(6, Math.round((y - wantAnchorY) / 22)));
    await wb.wheel(wheelX, wheelY, notches * wheelPerLine);
    await sleep(300);
  }
  if (anchorY === null || anchorY < lowY - 14 || anchorY > highY + 14) {
    throw new Error(`锚点行没滚到位（y=${anchorY}，想要 ${lowY}–${highY}）`);
  }
  log(`锚点「${TARGET.anchor}」在 y=${anchorY}（想要 ${lowY}–${highY}）`);

  // 下边界：目标面板的"进度"行底 → 映射回工作台 CSS px（frame 与工作台 1:1）。
  const bottom = panelBottomFrame === null ? geo.group[1] + 300
    : Math.min(geo.group[1] + geo.group[3], Math.round(geo.frame[1] + (panelBottomFrame + 12) * (geo.frame[3] / 613)));
  const rect = { x: geo.group[0], y: geo.group[1], w: geo.aux[0] + geo.aux[2] - geo.group[0], h: bottom - geo.group[1] };
  log("裁切框（CSS px）" + JSON.stringify(rect));

  // 整窗截图 → 按 CSS px × 倍率 用 sips 裁出来（倍率从图宽/视口宽算，不写死）。
  mkdirSync(dirname(args.out), { recursive: true });
  const full = join(CACHE, "full.png");
  await wb.shot(full);
  const imgW = Number(/pixelWidth:\s*(\d+)/.exec(execFileSync("sips", ["-g", "pixelWidth", full], { encoding: "utf8" }))[1]);
  const k = imgW / geo.view[0];
  execFileSync("sips", ["-c", String(Math.round(rect.h * k)), String(Math.round(rect.w * k)),
    "--cropOffset", String(Math.round(rect.y * k)), String(Math.round(rect.x * k)), full, "--out", args.out]);
  log(`整窗 ${imgW}px / 视口 ${geo.view[0]}CSS px ⇒ 倍率 ${k.toFixed(3)}`);

  // ── 判据 ────────────────────────────────────────────────────────────────
  // ① DOM：面板里那行**就是内核算的**（`⊢` + 目标等式），且面板显示的是 inter_comm；
  const domGoal = panel.split("\n").map((s) => s.trim()).find((s) => s.startsWith("⊢")) || "";
  if (!domGoal.includes("A") || !domGoal.includes("B")) throw new Error("面板里的目标行不对：" + domGoal);
  // ② 像素：图里**真读得到**定理名、sorry、目标等式（Vision 把 `∩` 读成 `n`，
  //    所以判据用 OCR 稳定可读的片段，而不是拿 ∣/∩ 这类符号去赌 OCR）。
  if (args.verify) {
    const marks = ocr(args.out);
    const texts = marks.map((m) => m.text).join("\n");
    // 左右两半的分界由**实测几何**给出（面板左沿在裁切框里的比例），不写死：
    // 面板起点随缩放/主题/版本变，写死 0.75 会把面板里的字判成"不在面板里"（实测踩过）。
    const panelFrac = (geo.aux[0] - rect.x) / rect.w;
    const left = (m) => m.x + m.w < panelFrac;
    const right = (m) => m.x > panelFrac - 0.03;
    const checks = [
      ["定理名 inter_comm", () => marks.some((m) => left(m) && m.text.includes("inter_comm"))],
      ["sorry 行", () => marks.some((m) => left(m) && /sorry/.test(m.text))],
      ["定理签名里的 Set α", () => marks.some((m) => left(m) && /Set/.test(m.text))],
      ["面板里的目标等式 (A ∩ B) = (B ∩ A)", () => marks.some((m) => right(m) && /A\s*n\s*B.*=.*B\s*n\s*A|A\s*∩\s*B.*=.*B\s*∩\s*A/.test(m.text))],
      ["面板里的假设 α : Type", () => marks.some((m) => right(m) && /Type/.test(m.text))],
    ];
    const failed = checks.filter(([, fn]) => !fn()).map(([name]) => name);
    // 反面：欢迎页/设置页/无关英文混进来 ⇒ 这张图不合格
    const junk = ["Welcome", "Settings", "Extensions", "Get Started", "lorem ipsum"].filter((j) => texts.includes(j));
    if (failed.length || junk.length) {
      rmSync(args.out, { force: true });
      throw new Error(`像素判据不过：缺 ${JSON.stringify(failed)} · 混入 ${JSON.stringify(junk)}（图已删除，不留错图）`);
    }
    log(`像素判据 ${checks.length} 条全过：${checks.map(([n]) => n).join(" / ")}`);
  }
  log(`落图 ${args.out.replace(REPO + "/", "")} · ${rect.w}×${rect.h} CSS px · ${(statSync(args.out).size / 1024).toFixed(0)} KB`);
} finally {
  wb.close();
  if (!args.keep) quit(profile);
}
