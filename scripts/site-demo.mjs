#!/usr/bin/env node
// scripts/site-demo.mjs —— 给官网录一段**真 VS Code + 已发布插件**的 GIF。
//
// 为什么要录、而不是画
// --------------------
// 站点设计（docs/design/site-single-page.md §4）规定：演示图必须是"用户装完插件
// 真实会看到的东西"，不许 AI 生成/摆拍。头图已经按这条规矩用真 VS Code 截图
// （scripts/site-screenshot.mjs）；这里用同一条路录 GIF：打开仓库里真实存在的
// 课程单元 → 把 `sorry` 逐行换成真证明 → **等**内核算完 → 用 CDP screencast
// 录下应用自己渲染的画面。图里的定理名、代码、目标、🎉 都来自真实文件与内核，
// 没有一处是画上去的。
//
// 录的是哪道题
// ------------
// 卷 I 单元②练习 2 `subset_trans`（`A ⊆ C` 的传递性）：
//
//     intro x hx      -- 目标变成 x ∈ C，假设里出现 x、hx
//     apply h2        -- 目标变成 x ∈ B
//     exact h1 x hx   -- 闭合，面板显示 🎉 已无目标 ✓
//
// 选它是因为**单目标 + 三步**：每一步都在目标面板里留下一个可读的变化，
// 而不会像多目标证明那样把面板撑高。靶子行从真文件里读，不手写行号。
//
// 判据
// ----
//   · DOM：最后一条声明是 checked、面板里有「🎉 已无目标 ✓」；
//   · 像素：从**最终 GIF** 里抽首帧/末帧做系统 Vision OCR，断言首帧有
//     `subset_trans` / `sorry`，末帧有定理名与三行真代码；
//   · 产物：GIF 帧数 > 1、时长 > 3s（静态图不算演示）。
// 任何一条不过 ⇒ **删掉 GIF** 并退出码 1（不留坏图）。
//
// 用法
// ----
//   node scripts/site-demo.mjs                 # 亮色 ⇒ site/assets/demo-vscode.gif
//   node scripts/site-demo.mjs --theme dark    # 暗色 ⇒ site/assets/demo-vscode-dark.gif
//   node scripts/site-demo.mjs --out /tmp/demo.gif --keep   # 调试：保留帧与 VS Code
//   node scripts/site-demo.mjs --version 0.98.2 --no-verify # 免 gh / 免 OCR（仍查 DOM）
//
// 依赖：macOS + 本机 VS Code（可用 SOKO_VSCODE_BIN 覆盖）+ `gh`（下载已发布 VSIX；
// 也可 --vsix 指本地文件）+ `ffmpeg`/`ffprobe`（编码与取帧）+ `swift`（OCR）。
// **没有 npm 依赖。** 工作树脏就拒绝开工（`--allow-dirty` 覆盖）：站点写的是
// **已发布版本**的事实，拿未提交的课程文件录 GIF 会写出对不上的图。
//
// 两版配色必须**同一靶子、同一裁切几何**（与头图同一条规矩）：亮/暗只换
// `workbench.colorTheme`，尺寸不同 ⇒ check-site 判红。

import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const CACHE = join(tmpdir(), "soko-site-shot");
const log = (...a) => console.log("[demo]", ...a);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── 参数 ────────────────────────────────────────────────────────────────────
function usage() {
  console.log(readFileSync(fileURLToPath(import.meta.url), "utf8")
    .split("\n").slice(1, 45).map((l) => l.replace(/^\/\/ ?/, "")).join("\n"));
}

function parseArgs(argv) {
  const a = { theme: "light", port: 0, keep: false, verify: true, dirty: false, out: null };
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    if (k === "--out") a.out = resolve(argv[++i]);
    else if (k === "--theme") a.theme = argv[++i];
    else if (k === "--version") a.version = argv[++i];
    else if (k === "--vsix") a.vsix = resolve(argv[++i]);
    else if (k === "--port") a.port = Number(argv[++i]);
    else if (k === "--keep") a.keep = true;
    else if (k === "--no-verify") a.verify = false;
    else if (k === "--allow-dirty") a.dirty = true;
    else if (k === "-h" || k === "--help") { usage(); process.exit(0); }
    else { console.error(`未知参数：${k}（--help 看用法）`); process.exit(2); }
  }
  if (a.theme !== "light" && a.theme !== "dark") {
    console.error(`--theme 只认 light|dark（给了 ${a.theme}）`);
    process.exit(2);
  }
  if (!a.out) {
    a.out = join(REPO, a.theme === "dark" ? "site/assets/demo-vscode-dark.gif" : "site/assets/demo-vscode.gif");
  }
  return a;
}
const args = parseArgs(process.argv.slice(2));

// ── 靶子：卷 I 单元②练习 2（单目标、三步）────────────────────────────────────
const TARGET = {
  file: join(REPO, "courses/set-theory/units/I.1/unit02-subsets-empty.sokonanoda"),
  workspace: join(REPO, "courses/set-theory"),
  theorem: "theorem subset_trans",
  proof: ["intro x hx", "apply h2", "exact h1 x hx"],
};

// 演示的裁切几何：与头图同一栏宽（头图的 843 来自 group + 目标面板的实测宽度），
// 高度取 390 —— 实测「目标 + 全部假设 + by 进度」在 390 内完整可见（少一档会
// 切掉进度行）。两版配色共用这个常数，尺寸才能一致。
const DEMO_CROP_HEIGHT = 390;
const DEMO_OUT_WIDTH = 843;

// ── 0. 图里那些内容必须是"已发布"的 ───────────────────────────────────────
function assertCleanWorktree() {
  const porcelain = execFileSync("git",
    ["status", "--porcelain", "--", TARGET.file, TARGET.workspace, "site/data/site.json"],
    { cwd: REPO, encoding: "utf8" }).trim();
  if (porcelain && !args.dirty) {
    console.error("[demo] 这些路径有未提交改动，拒绝录制（站点写的是已发布版本的事实）：\n"
      + porcelain + "\n  确认无所谓再加 --allow-dirty。");
    process.exit(3);
  }
  if (porcelain) log("⚠ --allow-dirty：" + porcelain.split("\n")[0] + " …");
}

function requireTool(name) {
  try {
    execFileSync("which", [name], { stdio: "ignore" });
  } catch {
    throw new Error(`找不到 ${name}（录 GIF 需要它；见本脚本头部的依赖）`);
  }
}

// ── 1. 已发布版本与 VSIX ────────────────────────────────────────────────────
function publishedVersion() {
  if (args.version) return args.version;
  const data = JSON.parse(readFileSync(join(REPO, "site/data/site.json"), "utf8"));
  if (!data.version) throw new Error("site/data/site.json 里没有 version");
  return data.version;
}

function platformVsix() {
  const table = { "darwin-arm64": "sokonanoda-darwin-arm64.vsix", "darwin-x64": "sokonanoda-darwin-x64.vsix" };
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

// ── 2. 真 VS Code（干净 profile：不碰用户自己的配置与扩展）───────────────────
function vscodeBin() {
  for (const c of [process.env.SOKO_VSCODE_BIN, "/Applications/Visual Studio Code.app/Contents/MacOS/Code"]) {
    if (c && existsSync(c)) return c;
  }
  throw new Error("找不到 VS Code 可执行文件（设 SOKO_VSCODE_BIN 指过去）");
}

function launch(version, extDir, cursorLine, port) {
  // ⚠ profile 路径不能长：VS Code 要在里面放一个 Unix socket（macOS 上限 103 字符）。
  // 实测 `demo-profile-light-<pid>-<port>` 会 EINVAL 直接退出 ⇒ 只用端口做短名。
  const profile = join(CACHE, `d${port}`);
  rmSync(profile, { recursive: true, force: true });
  mkdirSync(join(profile, "User"), { recursive: true });
  // 首帧就是 GIF 的一部分：把 UI 显式钉死（与头图同一套 + 录制专用三条）：
  //   · 主题跟着 --theme（亮/暗两版同一靶子）；
  //   · 放大一档（裁出来 843 CSS px，站点上正好铺满 832 的栏）；
  //   · 关掉面包屑/欢迎页/提示/Copilot/平滑滚动（噪声与不确定的滚动）；
  //   · **录制专用**：关自动保存（证明只写进内存缓冲，绝不碰工作树！）、
  //     关补全弹窗（会盖住正在敲的代码）、光标常亮（闪烁会污染帧）。
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
    "files.autoSave": "off",
    "editor.quickSuggestions": false,
    "editor.suggestOnTriggerCharacters": false,
    "editor.acceptSuggestionOnEnter": "off",
    "editor.parameterHints.enabled": false,
    "editor.inlineSuggest.enabled": false,
    "editor.cursorBlinking": "solid",
    // 关自动缩进：录制时我们**自己写**每行的两格缩进（否则 Enter 会按上一行
    // 再叠一层，实测三行变成 2/4/6 格的楼梯 ✗）。这不改变文件语义，只是把
    // 打字动作钉成确定的两格。
    "editor.autoIndent": "none",
  }, null, 2));

  const child = spawn(vscodeBin(), [
    `--user-data-dir=${profile}`,
    `--extensions-dir=${extDir}`,
    `--remote-debugging-port=${port}`,
    "--disable-gpu", "--disable-chromium-sandbox",
    "--no-first-run", "--disable-workspace-trust",
    "--disable-extension", "GitHub.copilot-chat",
    "--disable-extension", "GitHub.copilot",
    "--goto", `${TARGET.file}:${cursorLine}:3`,
    TARGET.workspace,
  ], { detached: true, stdio: ["ignore", "ignore", "ignore"] });
  child.unref();
  return { profile, child };
}

function quit(proc) {
  if (!proc) return;
  try { proc.child.kill("SIGTERM"); } catch { /* already gone */ }
  try { execFileSync("pkill", ["-f", `user-data-dir=${proc.profile}`]); } catch { /* already gone */ }
}

// ── 3. CDP 客户端（零依赖；**每个调用都有超时**，绝不让失败变成挂死）────────
async function cdpTargets(port) {
  return (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
}

async function connect(url, timeoutMs = 4000) {
  const ws = new WebSocket(url);
  await new Promise((res, rej) => {
    const timer = setTimeout(() => { try { ws.close(); } catch { /* ignore */ } rej(new Error("CDP 连接超时：" + url)); }, timeoutMs);
    ws.addEventListener("open", () => { clearTimeout(timer); res(); }, { once: true });
    ws.addEventListener("error", (err) => { clearTimeout(timer); rej(err); }, { once: true });
  });
  let id = 0;
  const pending = new Map();
  const events = new Map();
  ws.addEventListener("message", (ev) => {
    const m = JSON.parse(ev.data);
    if (m.method) {
      for (const fn of events.get(m.method) || []) fn(m.params);
      return;
    }
    const p = pending.get(m.id);
    if (p) { pending.delete(m.id); clearTimeout(p.timer); p.res(m); }
  });
  ws.addEventListener("close", () => {
    for (const [, p] of pending) { clearTimeout(p.timer); p.rej(new Error("CDP 连接已关闭")); }
    pending.clear();
  });
  const call = (method, params = {}) => new Promise((res, rej) => {
    const n = ++id;
    const timer = setTimeout(() => {
      pending.delete(n);
      rej(new Error(`CDP 调用超时（${method}）`));
    }, 20000);
    pending.set(n, { res, rej, timer });
    ws.send(JSON.stringify({ id: n, method, params }));
  });
  const client = {
    close: () => { try { ws.close(); } catch { /* ignore */ } },
    on(method, fn) { const list = events.get(method) || []; list.push(fn); events.set(method, list); },
    callRaw: (method, params = {}) => call(method, params),
    async eval(expression) {
      const m = await call("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
      if (m.result?.exceptionDetails) throw new Error(JSON.stringify(m.result.exceptionDetails).slice(0, 300));
      return m.result?.result?.value;
    },
    async json(expression) { return JSON.parse(await client.eval(`JSON.stringify(${expression})`)); },
    async key(key, code, modifiers = 0) {
      const base = { key, code, modifiers, windowsVirtualKeyCode: 0, nativeVirtualKeyCode: 0 };
      await call("Input.dispatchKeyEvent", { type: "rawKeyDown", ...base });
      await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
    },
    async type(text) { await call("Input.insertText", { text }); },
    async click(x, y) {
      const base = { x, y, button: "left", clickCount: 1 };
      await call("Input.dispatchMouseEvent", { type: "mousePressed", ...base });
      await call("Input.dispatchMouseEvent", { type: "mouseReleased", ...base });
    },
    async wheel(x, y, deltaY) {
      await call("Input.dispatchMouseEvent", { type: "mouseWheel", x, y, deltaX: 0, deltaY, pointerType: "mouse" });
    },
    async shot(file) {
      const m = await call("Page.captureScreenshot", { format: "png" });
      writeFileSync(file, Buffer.from(m.result.data, "base64"));
    },
  };
  return client;
}

async function workbenchPage(port) {
  for (let i = 0; i < 60; i++) {
    await sleep(1000);
    try {
      const page = (await cdpTargets(port)).find((t) => t.type === "page" && t.url.includes("workbench"));
      if (page) {
        try { return await connect(page.webSocketDebuggerUrl); } catch { /* 还没起来 */ }
      }
    } catch { /* 还没起来 */ }
  }
  throw new Error("VS Code 没有暴露 CDP（--remote-debugging-port 没生效？）");
}

// Infoview 是**独立进程的 webview**：它自己在 /json/list 里是一个 target，真正的界面
// 又在它内部那个同源的 `#active-frame` 里 ⇒ "面板里现在是什么"只能进去问。
async function infoviewEval(port, expression, { tries = 8 } = {}) {
  for (let i = 0; i < tries; i++) {
    let list = [];
    try { list = await cdpTargets(port); } catch { /* retry */ }
    for (const t of list.filter((t) => t.type === "iframe" || (t.type === "page" && t.url.startsWith("vscode-webview:")))) {
      let c;
      try {
        c = await connect(t.webSocketDebuggerUrl, 1500);
        const v = await c.eval(`(() => { const f = document.querySelector('#active-frame'); const d = f && f.contentDocument; if (!d || !d.body) return null; return JSON.stringify(${expression}); })()`);
        if (v && v !== "null") return JSON.parse(v);
      } catch { /* 陈旧 target / 连接超时：换下一个 */ } finally { try { c?.close(); } catch { /* ignore */ } }
    }
    await sleep(500);
  }
  return null;
}

// ── 4. Vision OCR：判据落在**像素**上 ───────────────────────────────────────
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

// ── 5. 等内核/面板：不是"等 3 秒"，而是等**该出现的那句话** ─────────────────
async function goalText(port) {
  return (await infoviewEval(port,
    `(() => { const g = document.querySelector('#active-frame').contentDocument.querySelector('.goals'); return g ? g.innerText : null; })()`)) || "";
}

async function waitGoals(port, marker, label, timeout = 20000) {
  const start = Date.now();
  let last = "";
  let stableSince = 0;
  while (Date.now() - start < timeout) {
    const text = await goalText(port);
    if (text && text.includes(marker)) {
      if (text === last) {
        if (!stableSince) stableSince = Date.now();
        if (Date.now() - stableSince >= 300) return text;
      } else {
        last = text;
        stableSince = 0;
      }
    } else {
      last = text;
      stableSince = 0;
    }
    await sleep(150);
  }
  throw new Error(`等目标面板里的「${label}」超时（marker=${JSON.stringify(marker)}）；最后读到：${last.slice(0, 300)}`);
}

async function typeChunks(wb, text, { size = 4, delay = 35 } = {}) {
  for (let i = 0; i < text.length; i += size) {
    await wb.type(text.slice(i, i + size));
    await sleep(delay);
  }
}

// ── 6. 录、取时间线、编码 ───────────────────────────────────────────────────
function jpegSize(file) {
  const s = execFileSync("sips", ["-g", "pixelWidth", "-g", "pixelHeight", file], { encoding: "utf8" });
  return {
    w: Number(/pixelWidth:\s*(\d+)/.exec(s)?.[1]),
    h: Number(/pixelHeight:\s*(\d+)/.exec(s)?.[1]),
  };
}

function buildTimeline(frames, dir, fps = 10, targetSeconds = 5.0) {
  const sorted = [...frames].sort((a, b) => a.t - b.t);
  const t0 = sorted[0].t;
  const t1 = sorted[sorted.length - 1].t;
  const real = Math.max(0.1, t1 - t0);
  const speed = Math.max(1.0, real / targetSeconds);
  const sampled = Math.max(10, Math.round((real / speed) * fps));
  const sequence = [];
  // 开头停 0.8s：读者先看清「sorry + 目标」，再开始打字。
  for (let i = 0; i < Math.round(fps * 0.8); i++) sequence.push(sorted[0].file);
  for (let i = 0; i < sampled; i++) {
    const want = t0 + (i / fps) * speed;
    let pick = sorted[0];
    for (const f of sorted) {
      if (f.t <= want) pick = f;
      else break;
    }
    sequence.push(pick.file);
  }
  // 结尾停 1.2s：让「🎉 已无目标 ✓」留在屏幕上（GIF 循环播放，读者要有时间看）。
  for (let i = 0; i < Math.round(fps * 1.2); i++) sequence.push(sorted[sorted.length - 1].file);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  sequence.forEach((src, i) => {
    symlinkSync(src, join(dir, `f-${String(i).padStart(4, "0")}.jpg`));
  });
  return { count: sequence.length, fps, speed, real, duration: sequence.length / fps };
}

function encodeGif(timeline, crop, out) {
  const first = join(timeline.dir, "f-0000.jpg");
  const src = jpegSize(first);
  const k = src.w / crop.viewW;
  if (Math.abs(src.h - crop.viewH * k) > 3) {
    throw new Error(`screencast 帧尺寸 ${src.w}×${src.h} 与视口 ${crop.viewW}×${crop.viewH} 的倍率对不上（k=${k.toFixed(3)}）`);
  }
  const cw = Math.round(crop.w * k);
  const ch = Math.round(crop.h * k);
  const cx = Math.round(crop.x * k);
  const cy = Math.round(crop.y * k);
  if (cx < 0 || cy < 0 || cx + cw > src.w || cy + ch > src.h) {
    throw new Error(`裁切框 ${JSON.stringify({ cw, ch, cx, cy })} 超出帧 ${src.w}×${src.h}`);
  }
  mkdirSync(dirname(out), { recursive: true });
  // 64 色 + 不抖动：暗色版在 128 色时实测 255 KB、80 色 250 KB，都超过
  // **每张大图 240 KB** 的预算；64 色不抖动后暗色 ~230 KB、亮色更小，文字仍可读
  // （末帧 OCR 判据照样过）。VS Code 的 UI 本身就是窄色域，主要成本是帧而不是调色板。
  const vf = `crop=${cw}:${ch}:${cx}:${cy},scale=${DEMO_OUT_WIDTH}:-1:flags=lanczos,`
    + "split[a][b];[a]palettegen=max_colors=64:stats_mode=diff[p];"
    + "[b][p]paletteuse=dither=none";
  execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-framerate", String(timeline.fps),
    "-i", join(timeline.dir, "f-%04d.jpg"), "-vf", vf, "-loop", "0", out], { stdio: "inherit" });
  const gif = jpegSize(out);
  if (gif.w !== DEMO_OUT_WIDTH) throw new Error(`GIF 宽 ${gif.w}，应为 ${DEMO_OUT_WIDTH}`);
  log(`GIF ${gif.w}×${gif.h} · ${timeline.count} 帧 · ${timeline.duration.toFixed(1)}s · `
    + `${(statSync(out).size / 1024).toFixed(0)} KB（${timeline.speed.toFixed(2)}× 实时）`);
  return gif;
}

function frameCount(file) {
  const n = execFileSync("ffprobe", ["-v", "error", "-count_frames", "-select_streams", "v:0",
    "-show_entries", "stream=nb_read_frames", "-of", "csv=p=0", file], { encoding: "utf8" }).trim();
  return Number(n);
}

function extractFrame(file, index, out) {
  execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-i", file,
    "-vf", `select=eq(n\\,${index})`, "-frames:v", "1", out], { stdio: "inherit" });
}

// ── 主流程 ──────────────────────────────────────────────────────────────────
assertCleanWorktree();
requireTool("ffmpeg");
requireTool("ffprobe");
requireTool("sips");
requireTool("pbpaste");
if (args.verify) requireTool("swift");
const version = publishedVersion();

const source = readFileSync(TARGET.file, "utf8").split("\n");
const theoremLine = source.findIndex((l) => l.includes(TARGET.theorem)) + 1;
if (!theoremLine) throw new Error(`${TARGET.file} 里找不到「${TARGET.theorem}」`);
const cursorLine = source.findIndex((l, i) => i > theoremLine - 1 && l.trim() === "sorry") + 1;
if (!cursorLine) throw new Error(`${TARGET.theorem} 下面没有 sorry`);
log(`靶子 ${TARGET.file.replace(REPO + "/", "")}（定理 L${theoremLine} · 光标 L${cursorLine}）· 插件 v${version} · 证明 ${JSON.stringify(TARGET.proof)}`);

const port = args.port || (9300 + (process.pid % 200));
const proc = launch(version, stageExtension(version, vsixPath(version)), cursorLine, port);
if (!args.keep) process.on("exit", () => quit(proc));

let wb = null;
let frameDir = null;
let timelineDir = null;
try {
  wb = await workbenchPage(port);
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
    const text = await infoviewEval(port, `document.querySelector('#active-frame').contentDocument.body.innerText`);
    if (text && text.includes("⊢")) { panel = text; break; }
  }
  if (!panel) throw new Error("等不到目标面板里的 ⊢（内核没算出目标？）");
  log("面板就绪：" + panel.split("\n").map((s) => s.trim()).filter(Boolean).slice(0, 8).join(" · "));

  // 面板算完后扩展会自己 reveal 一次（把声明滚到视野里）⇒ 先等滚动停下来。
  const readScroll = () => wb.eval(`(() => { const lc = document.querySelector('.monaco-editor .lines-content'); const m = /top: (-?[\\d.]+)px/.exec(lc.getAttribute('style') || ''); return m ? Math.round(-Number(m[1])) : 0; })()`);
  let prevScroll = -1;
  for (let i = 0; i < 12; i++) {
    const cur = await readScroll();
    if (cur === prevScroll) break;
    prevScroll = cur;
    await sleep(800);
  }

  const geo = await wb.json(`(() => {
    const R = (el) => { const r = el.getBoundingClientRect(); return [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)]; };
    return { aux: R(document.querySelector('.part.auxiliarybar')), group: R(document.querySelector('.editor-group-container')), frame: R(document.querySelector('iframe.webview')), view: [innerWidth, innerHeight] };
  })()`);
  const wheelX = geo.group[0] + 200;
  const wheelY = geo.group[1] + 300;
  const wantAnchorY = geo.group[1] + 40;   // 定理行放顶部：下面三步证明都留在画面里
  const lineY = () => wb.eval(`(() => {
    const hits = [...document.querySelectorAll('.editor-group-container .view-line')]
      .filter((e) => e.textContent.replace(/\\u00a0/g, ' ').includes(${JSON.stringify(TARGET.theorem)}))
      .map((e) => Math.round(e.getBoundingClientRect().y));
    return hits.length ? Math.min(...hits) : null;
  })()`);
  const visibleLines = () => wb.json(`(() => {
    const n = [...document.querySelectorAll('.editor-group-container .line-numbers')].map((e) => Number(e.textContent)).filter((v) => v > 0);
    return n.length ? [Math.min(...n), Math.max(...n)] : null;
  })()`);

  // 滚轮 deltaY → 像素的换算随机器/系统设置变（本机实测 100 → 42px）。**先量再算**。
  let gain = 0.42;
  {
    const y0 = await lineY();
    if (y0 !== null) {
      await wb.wheel(wheelX, wheelY, 100);
      await sleep(300);
      const y1 = await lineY();
      if (y1 !== null && Math.abs(y1 - y0) > 2) gain = Math.abs(y1 - y0) / 100;
      await wb.wheel(wheelX, wheelY, -100);
      await sleep(300);
    }
  }
  const wheelPerLine = 22 / gain;
  log(`滚轮换算 1 单位 ≈ ${gain.toFixed(3)} px（一行 ≈ ${wheelPerLine.toFixed(0)} 单位）`);

  let anchorY = null;
  for (let i = 0; i < 24; i++) {
    const y = await lineY();
    if (y === null) {
      const range = await visibleLines();
      const dir = !range || theoremLine > range[1] ? 1 : -1;
      await wb.wheel(wheelX, wheelY, dir * 300);
      await sleep(250);
      continue;
    }
    anchorY = y;
    if (Math.abs(y - wantAnchorY) <= 14) break;
    const notches = Math.max(-6, Math.min(6, Math.round((y - wantAnchorY) / 22)));
    await wb.wheel(wheelX, wheelY, notches * wheelPerLine);
    await sleep(250);
  }
  if (anchorY === null || Math.abs(anchorY - wantAnchorY) > 40) {
    throw new Error(`定理行没滚到位（y=${anchorY}，想要 ${wantAnchorY}±14）`);
  }
  log(`定理行在 y=${anchorY}（想要 ${wantAnchorY}±14）`);

  // ── 开录 ────────────────────────────────────────────────────────────────
  frameDir = join(CACHE, `demo-frames-${args.theme}-${process.pid}`);
  rmSync(frameDir, { recursive: true, force: true });
  mkdirSync(frameDir, { recursive: true });
  const frames = [];
  wb.on("Page.screencastFrame", (params) => {
    try {
      const file = join(frameDir, `frame-${String(frames.length).padStart(5, "0")}.jpg`);
      writeFileSync(file, Buffer.from(params.data, "base64"));
      frames.push({ file, t: params.metadata?.timestamp ?? Date.now() / 1000 });
    } catch (error) {
      log("screencast 帧写盘失败：" + error.message);
    }
    wb.callRaw("Page.screencastFrameAck", { sessionId: params.sessionId }).catch(() => {});
  });
  await wb.callRaw("Page.startScreencast", { format: "jpeg", quality: 85, maxWidth: 3000, maxHeight: 2000, everyNthFrame: 1 });
  await sleep(1200);   // 开头停一下：先看清 sorry 与目标

  const lineGeo = await wb.json(`(() => {
    const node = [...document.querySelectorAll('.editor-group-container .line-numbers')]
      .find((e) => e.textContent === ${JSON.stringify(String(cursorLine))});
    if (!node) return null;
    const r = node.getBoundingClientRect();
    return { x: Math.round(r.x), y: Math.round(r.y), h: Math.round(r.height) };
  })()`);
  if (!lineGeo) throw new Error(`编辑器里看不到第 ${cursorLine} 行（sorry）`);
  await wb.click(geo.group[0] + 100, lineGeo.y + lineGeo.h / 2);
  await sleep(250);
  // ⚠ macOS 上 Cmd+Left 先到**词首**、再到**行首**（实测一次只到 "sorry" 的词首，
  // 于是只替掉了 "rry"，留下 "sointro x hx" ✗）⇒ 连按两次再到行首。
  await wb.key("ArrowLeft", "ArrowLeft", 4);
  await sleep(80);
  await wb.key("ArrowLeft", "ArrowLeft", 4);
  await sleep(80);
  await wb.key("ArrowRight", "ArrowRight", 12);  // Meta+Shift+Right = 选中整行
  await sleep(100);

  const markers = ["x ∈ C", "x ∈ B", "🎉 已无目标 ✓"];
  for (let i = 0; i < TARGET.proof.length; i++) {
    const line = TARGET.proof[i];
    // 整行都选中了（含原来的两格缩进）⇒ 第一行把 `  ` 也一起写回；之后每行
    // 用 `Input.insertText("\n")` 换行、再写两格（autoIndent 已关，不会叠成楼梯）。
    // ⚠ 不用 `Input.dispatchKeyEvent("Enter")`：实测它进不了 Monaco 的换行路径
    // （两行被拼成一行 `intro x hx  apply h2` ✗）。
    if (i > 0) {
      await wb.type("\n");
      await sleep(80);
    }
    await typeChunks(wb, "  " + line);
    const text = await waitGoals(port, markers[i], line);
    log(`第 ${i + 1}/${TARGET.proof.length} 步「${line}」已进面板：`
      + text.split("\n").map((s) => s.trim()).filter(Boolean).slice(-3).join(" · "));
    await sleep(900);   // 让这一步在 GIF 里停得下来
  }
  await sleep(1800);    // 末尾多留一点时间给 🎉
  await wb.callRaw("Page.stopScreencast");
  await sleep(300);

  if (frames.length < 8) throw new Error(`只录到 ${frames.length} 帧（screencast 没工作？）`);
  log(`录到 ${frames.length} 帧，真实时长 ${(frames[frames.length - 1].t - frames[0].t).toFixed(2)}s`);

  // ── 编码 ────────────────────────────────────────────────────────────────
  timelineDir = join(CACHE, `demo-timeline-${args.theme}-${process.pid}`);
  const timeline = buildTimeline(frames, timelineDir);
  timeline.dir = timelineDir;
  const crop = {
    x: geo.group[0],
    y: geo.group[1],
    w: geo.aux[0] + geo.aux[2] - geo.group[0],
    h: Math.min(DEMO_CROP_HEIGHT, geo.group[1] + geo.group[3] - geo.group[1]),
    viewW: geo.view[0],
    viewH: geo.view[1],
  };
  const gif = encodeGif(timeline, crop, args.out);

  // ── 判据：DOM 的真相 + 最终 GIF 的像素 ──────────────────────────────────
  const finalGoals = await goalText(port);
  if (!finalGoals.includes("🎉 已无目标 ✓") || !finalGoals.includes("checked")) {
    throw new Error("最后不是 checked + 🎉：\n" + finalGoals.slice(0, 500));
  }
  // ⚠ 不用 `.view-line` 做这条判据：Monaco 会把可见行分片，且代码镜/虚拟化会
  // 让同一个文档在 DOM 里出现**重复的旧行**（实测把证明行数成了"跑到文件末尾"✗）。
  // 走用户的**全选 + 复制**，读剪贴板 —— 那是文档真正的文本（真 UI 动作）。
  await wb.click(geo.group[0] + 220, geo.group[1] + 300);
  await sleep(200);
  await wb.key("a", "KeyA", 4);
  await sleep(200);
  await wb.key("c", "KeyC", 4);
  await sleep(400);
  const editorText = execFileSync("pbpaste", { encoding: "utf8" });
  const missingProof = TARGET.proof.filter((line) => !editorText.includes(line));
  if (missingProof.length) throw new Error("编辑器里找不到这些证明行：" + JSON.stringify(missingProof));

  const n = frameCount(args.out);
  if (n < 10) throw new Error(`GIF 只有 ${n} 帧（静态图不算演示）`);
  if (n / 10 < 3) throw new Error(`GIF 只有 ${(n / 10).toFixed(1)}s（<3s，太快）`);

  if (args.verify) {
    const first = join(CACHE, `demo-first-${process.pid}.png`);
    const last = join(CACHE, `demo-last-${process.pid}.png`);
    extractFrame(args.out, 0, first);
    extractFrame(args.out, n - 1, last);
    const firstText = ocr(first).map((m) => m.text).join("\n");
    const lastText = ocr(last).map((m) => m.text).join("\n");
    const checks = [
      ["首帧有定理名 subset_trans", /subset_trans/.test(firstText)],
      ["首帧有 sorry", /sorry/.test(firstText)],
      ["末帧有定理名 subset_trans", /subset_trans/.test(lastText)],
      ["末帧有 intro x hx", /intro/.test(lastText) && /hx/.test(lastText)],
      ["末帧有 apply h2", /(apply|oply|app1y)/i.test(lastText) && /h2/.test(lastText)],
      ["末帧有 exact h1", /exact/.test(lastText) && /h1/.test(lastText)],
    ];
    const failed = checks.filter(([, ok]) => !ok).map(([name]) => name);
    if (failed.length) {
      rmSync(args.out, { force: true });
      throw new Error(`像素判据不过：缺 ${JSON.stringify(failed)}（GIF 已删除，不留错图）`);
    }
    log(`像素判据 ${checks.length} 条全过：${checks.map(([name]) => name).join(" / ")}`);
  }

  log(`落图 ${args.out.replace(REPO + "/", "")} · ${gif.w}×${gif.h} · ${n} 帧 · `
    + `${(statSync(args.out).size / 1024).toFixed(0)} KB`);
} catch (error) {
  // 失败时不留半张图（与头图脚本同一条纪律）。
  try { rmSync(args.out, { force: true }); } catch { /* ignore */ }
  throw error;
} finally {
  try { wb?.close(); } catch { /* ignore */ }
  if (!args.keep) {
    quit(proc);
    if (frameDir) rmSync(frameDir, { recursive: true, force: true });
    if (timelineDir) rmSync(timelineDir, { recursive: true, force: true });
  }
}
