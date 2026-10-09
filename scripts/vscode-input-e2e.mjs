#!/usr/bin/env node
// scripts/vscode-input-e2e.mjs —— **真 VS Code + 真按键**：记法输入法（`\alpha` → `α`）
// 到底会不会在用户手里生效。
//
// 为什么必须有这一条（而不是再加一条扩展宿主单测）
// ----------------------------------------------
// `editor/vscode/test-extension-host.js` 与 `src/test/extension.test.js` 都是
// **直接调命令**（`commandHandler(REPLACE_COMMAND)()` / `executeCommand(...)`）——
// 它们证明的是"状态机算得对"，**绕过了用户真正走的那条路**：`Tab` 键位 +
// `when: sokonanoda.input.abbreviationBeforeCursor` 这个 context key。
// 用户报的正是那条路（「输入 `\alpha` 未替换成 `α`」）。`AGENTS.md` 的
// 验证设计纪律 0(a)：**判据必须绑「用户动作」**——按下去 ⇒ 可见结果；
// **不许用"能跑通的位置"代替"用户实际点的位置"**。
//
// 做法：CDP 驱动真 VS Code（与 `scripts/site-screenshot.mjs` 同一套管道），
// 逐字符 **真 keydown**（走 Monaco 的输入 → 触发 `onDidChangeTextEditorSelection`
// ⇒ 扩展更新 context key），再发**真 `Tab` keydown**（走 VS Code 的键位解析 ⇒
// 命中的是 package.json 里那条 keybinding），最后读**编辑器 DOM 里的那一行**
// （用户屏幕上看到的东西）+ 存盘后的文件字节。
//
// ⚠ 这条脚本**不许**调 `vscode.*` API：那些 API 在扩展宿主里，不在渲染器里；
// 一调就变成"绕过用户动作"的第二条路 ✗ —— 选文件、清空、存盘全走真键位。
//
// 用法：
//   node scripts/vscode-input-e2e.mjs                 # 跑全部用例
//   node scripts/vscode-input-e2e.mjs --case alpha    # 只跑一条
//   node scripts/vscode-input-e2e.mjs --keep          # 留着 profile/工作区供排查
//   node scripts/vscode-input-e2e.mjs --json          # 机器可读结果（台账用）
//   node scripts/vscode-input-e2e.mjs --vsix <a.vsix> # 测**已发布的那份 VSIX**（默认测工作树）
//   SOKO_VSCODE_BIN=/path/to/Code node scripts/vscode-input-e2e.mjs
//
// 退出码：0 = 全绿；1 = 有用例失败；2 = 用法错误；3 = 前置缺失（找不到 VS Code /
// 扩展目录）。

import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const EXTENSION = join(REPO, "editor/vscode");
const CACHE = join(tmpdir(), "soko-input-e2e");
const IS_MAC = process.platform === "darwin";
// CDP 的 modifiers 位：Alt=1 · Ctrl=2 · Meta=4 · Shift=8。
const MOD = IS_MAC ? 4 : 2;
const log = (...a) => console.log("[input-e2e]", ...a);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── 参数 ────────────────────────────────────────────────────────────────────
const args = {
  port: 9417,
  keep: false,
  json: false,
  case: null,
  timeout: 300_000,
  vsix: null,
  ext: null,
  suggestOff: false,
};
for (let i = 2; i < process.argv.length; i++) {
  const k = process.argv[i];
  if (k === "--port") args.port = Number(process.argv[++i]);
  else if (k === "--keep") args.keep = true;
  else if (k === "--json") args.json = true;
  else if (k === "--case") args.case = process.argv[++i];
  else if (k === "--timeout") args.timeout = Number(process.argv[++i]);
  else if (k === "--vsix") args.vsix = resolve(process.argv[++i]);
  else if (k === "--ext") args.ext = resolve(process.argv[++i]);
  else if (k === "--suggest-off") args.suggestOff = true;
  else {
    console.error(`error: 未知参数 ${k}`);
    process.exit(2);
  }
}

function vscodeBin() {
  for (const c of [
    process.env.SOKO_VSCODE_BIN,
    "/Applications/Visual Studio Code.app/Contents/MacOS/Code",
    "/usr/share/code/code",
  ]) {
    if (c && existsSync(c)) return c;
  }
  console.error("error: 找不到 VS Code 可执行文件（设 SOKO_VSCODE_BIN 指过去）");
  process.exit(3);
}

// ── 用例表 ──────────────────────────────────────────────────────────────────
// 每条 = 「在哪个文件里按什么键」⇒「那一行必须是什么」。
// `file` 决定语言：只有 `.sokonanoda` 归扩展管（对照臂钉住"别吞别的语言/别吞缩进"）。
const CASES = [
  {
    id: "alpha-space",
    why: "**用户 2026-10-09 拍板的那一条**：`\\alpha` 之后按【空格】⇒ α（对齐 Lean 4，不依赖 Tab）",
    file: "Input.sokonanoda",
    type: "\\alpha ",
    keys: [],
    expect: "α ",
    dom: true,
  },
  {
    id: "short-a-space",
    why: "短键同样按空格落定：`\\a ` ⇒ `α `（`a` 是 `alpha` 的前缀 ⇒ 空格就是「封口」那一刀）",
    file: "Input.sokonanoda",
    type: "\\a ",
    keys: [],
    expect: "α ",
    dom: true,
  },
  {
    id: "and-space",
    why: "逻辑符号同一条路：`\\and ` ⇒ `∧ `",
    file: "Input.sokonanoda",
    type: "\\and ",
    keys: [],
    expect: "∧ ",
    dom: true,
  },
  {
    id: "in-space",
    why: "前缀陷阱：`\\in ` ⇒ `∈ `（`in` ⊂ `inter`/`inv`，空格封口才落定）",
    file: "Input.sokonanoda",
    type: "\\in ",
    keys: [],
    expect: "∈ ",
    dom: true,
  },
  {
    id: "alpha-typed",
    why: "敲完就换（与 Lean 的 eager 默认同）：`\\alpha` 之后**什么都不按**也是 α",
    file: "Input.sokonanoda",
    type: "\\alpha",
    keys: [],
    expect: "α",
    dom: true,
  },
  {
    id: "in-tab",
    why: "`Tab` 仍是**内部**的显式路径（不再教、不再要求）：`\\in` 是 `\\inter` 的前缀 ⇒ Tab 立刻封口",
    file: "Input.sokonanoda",
    type: "\\in",
    keys: ["Tab"],
    expect: "∈",
    dom: true,
  },
  {
    id: "undo-single-step",
    why:
      "一次替换 = 一个 undo 单元：`\\in` + Tab ⇒ ∈ 之后**一次**撤销就回到 `\\in`" +
      "（用 Tab 那一刀当「最后一次编辑」——空格那刀之后撤销只会撤掉空格，属正确行为）",
    file: "Input.sokonanoda",
    type: "\\in",
    keys: ["Tab", "Undo"],
    expect: "\\in",
    dom: true,
  },
  {
    id: "plain-text-control",
    why: "对照臂：`.txt` 里空格/Tab 照旧，扩展不许动它",
    file: "Input.txt",
    type: "\\alpha ",
    keys: [],
    expect: "\\alpha ",
    dom: true,
  },
  {
    id: "tab-keeps-indent",
    why: "对照臂：**空行上** Tab 照旧是缩进（扩展不许吞 Tab）——不敲词，避开补全弹窗",
    file: "Indent.sokonanoda",
    type: "",
    keys: ["Tab"],
    expect: "\t",
    dom: false,
  },
  {
    id: "eager-off-space",
    why: "逃生门（`input.eager: false`）：空格**不许**自己换（老行为），文本原样",
    file: "Input.sokonanoda",
    type: "\\alpha ",
    keys: [],
    expect: "\\alpha ",
    dom: true,
    eager: false,
  },
  {
    id: "eager-off-tab",
    why: "逃生门（`input.eager: false`）：Tab 仍然是显式路径",
    file: "Input.sokonanoda",
    type: "\\alpha",
    keys: ["Tab"],
    expect: "α",
    dom: true,
    eager: false,
  },
];

const selected = args.case ? CASES.filter((c) => c.id === args.case) : CASES;
if (selected.length === 0) {
  console.error(`error: --case ${args.case} 不在用例表里（${CASES.map((c) => c.id).join(", ")}）`);
  process.exit(2);
}

// ── CDP 客户端（零依赖：Node 自带 WebSocket）─────────────────────────────────
/// 每组的调试端口 = `--port` + 组号：**组间绝不复用端口**（复用的后果实测过：
/// 上一组的窗口还没退干净，新窗口绑不上端口 ⇒ 脚本连到**上一组那个窗口**，
/// 于是"关掉 eager"那一组实际在跑默认档，结论全错 ✗）。
let debugPort = args.port;

async function cdpTargets() {
  return (await fetch(`http://127.0.0.1:${debugPort}/json/list`)).json();
}

async function connect(url) {
  const ws = new WebSocket(url);
  await new Promise((res, rej) => {
    ws.addEventListener("open", res, { once: true });
    ws.addEventListener("error", rej, { once: true });
  });
  let id = 0;
  const pending = new Map();
  ws.addEventListener("message", (ev) => {
    const m = JSON.parse(ev.data);
    const p = pending.get(m.id);
    if (p) {
      pending.delete(m.id);
      p(m);
    }
  });
  // ⚠ **必须带超时**：窗口被关掉/换掉之后 CDP 目标会失效，而失效的 socket
  // 不会回包 ⇒ 无超时的 await 会把整条脚本挂死（实测踩到：第二组启动后
  // 一直不返回，`timeout` 到点才结束，日志里什么都看不到 ✗）。
  const call = (method, params = {}, timeoutMs = 20_000) =>
    new Promise((res, rej) => {
      const n = ++id;
      const timer = setTimeout(() => {
        pending.delete(n);
        rej(new Error(`CDP ${method} 超时 ${timeoutMs}ms（目标失效？）`));
      }, timeoutMs);
      pending.set(n, (m) => {
        clearTimeout(timer);
        res(m);
      });
      ws.send(JSON.stringify({ id: n, method, params }));
    });
  const key = async (key_, code, vk, modifiers = 0) => {
    const base = {
      key: key_,
      code,
      modifiers,
      windowsVirtualKeyCode: vk,
      nativeVirtualKeyCode: vk,
    };
    await call("Input.dispatchKeyEvent", { type: "rawKeyDown", ...base });
    await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
  };
  return {
    close: () => ws.close(),
    async eval(expression) {
      const m = await call("Runtime.evaluate", {
        expression,
        returnByValue: true,
        awaitPromise: true,
      });
      if (m.result?.exceptionDetails) {
        throw new Error(JSON.stringify(m.result.exceptionDetails).slice(0, 300));
      }
      return m.result?.result?.value;
    },
    key,
    /// 一个**可打印字符**：真 keydown 带 `text`（Monaco 就是靠它插入字符的）。
    async char(ch) {
      const vk = virtualKey(ch);
      await call("Input.dispatchKeyEvent", {
        type: "keyDown",
        text: ch,
        unmodifiedText: ch,
        key: ch,
        code: codeOf(ch),
        windowsVirtualKeyCode: vk,
        nativeVirtualKeyCode: vk,
      });
      await call("Input.dispatchKeyEvent", {
        type: "keyUp",
        key: ch,
        code: codeOf(ch),
        windowsVirtualKeyCode: vk,
        nativeVirtualKeyCode: vk,
      });
    },
    /// 组合键（`Cmd/Ctrl + <letter>`）。
    async chord(letter, shift = false) {
      await key(
        letter,
        `Key${letter.toUpperCase()}`,
        letter.toUpperCase().charCodeAt(0),
        MOD | (shift ? 8 : 0),
      );
    },
    /// 在编辑器区域**真点一下**（`editorTextFocus` 得靠真焦点事件）。
    async clickEditor() {
      const rect = JSON.parse(
        await this.eval(
          `(() => { const r = document.querySelector('.monaco-editor')?.getBoundingClientRect();
             return JSON.stringify(r ? {x: r.x, y: r.y, w: r.width, h: r.height} : null); })()`,
        ),
      );
      if (!rect) throw new Error("找不到编辑器区域");
      const x = Math.round(rect.x + rect.w * 0.6);
      const y = Math.round(rect.y + rect.h * 0.4);
      for (const type of ["mousePressed", "mouseReleased"]) {
        await call("Input.dispatchMouseEvent", {
          type,
          x,
          y,
          button: "left",
          clickCount: 1,
          pointerType: "mouse",
        });
      }
    },
  };
}

function codeOf(ch) {
  if (ch === "\\") return "Backslash";
  if (ch === " ") return "Space";
  if (/[a-zA-Z]/.test(ch)) return `Key${ch.toUpperCase()}`;
  return "";
}

function virtualKey(ch) {
  if (ch === "\\") return 220;
  if (ch === " ") return 32;
  if (/[a-zA-Z]/.test(ch)) return ch.toUpperCase().charCodeAt(0);
  return 0;
}

async function workbenchPage(deadline) {
  while (Date.now() < deadline) {
    await sleep(700);
    try {
      const page = (await cdpTargets()).find(
        (t) => t.type === "page" && t.url.includes("workbench"),
      );
      if (page) return await connect(page.webSocketDebuggerUrl);
    } catch {
      /* 还没起来 */
    }
  }
  throw new Error("VS Code 没有暴露 CDP（--remote-debugging-port 没生效？）");
}

// ── 编辑器 DOM：用户屏幕上那些行 ─────────────────────────────────────────────
// Monaco 把每行画成 `.view-line`；这里读的是**渲染结果**，不是模型对象。
const READ_LINES = `(() => {
  const lines = Array.from(document.querySelectorAll('.view-lines .view-line'));
  return JSON.stringify(lines.map((l) => l.textContent));
})()`;
const ACTIVE_TAB = `(() => {
  const t = document.querySelector('.tabs-container .tab.active');
  return t ? t.textContent : null;
})()`;

async function waitForEditor(client, deadline) {
  while (Date.now() < deadline) {
    try {
      const raw = await client.eval(READ_LINES);
      if (raw && raw !== "[]" && raw !== "null") return JSON.parse(raw);
    } catch {
      /* 页面还在换 */
    }
    await sleep(400);
  }
  throw new Error("编辑器没有渲染出任何行（窗口没起来 / 文件没打开？）");
}

/// 用**真键位**切到某个文件（`Cmd/Ctrl+P` 快速打开 ⇒ 打名字 ⇒ Enter）。
async function openFile(client, name) {
  await client.chord("p");
  await sleep(500);
  for (const ch of name) {
    await client.char(ch);
    await sleep(30);
  }
  await sleep(800);
  await client.key("Enter", "Enter", 13);
  await sleep(1000);
  const tab = await client.eval(ACTIVE_TAB);
  if (!String(tab ?? "").includes(name)) {
    throw new Error(`快速打开没能切到 ${name}（当前标签页：${tab}）`);
  }
}

/// 清空当前文件（`Cmd/Ctrl+A` + `Delete`）—— 同样走真键位。
async function clearFile(client) {
  await client.chord("a");
  await sleep(200);
  await client.key("Delete", "Delete", 46);
  await sleep(300);
}

// ── 启动 ────────────────────────────────────────────────────────────────────
function prepareWorkspace(tag, eager) {
  const ws = join(CACHE, `ws-${tag}`);
  rmSync(ws, { recursive: true, force: true });
  mkdirSync(ws, { recursive: true });
  for (const c of CASES) writeFileSync(join(ws, c.file), "\n", "utf8");
  const profile = join(CACHE, `profile-${tag}`);
  rmSync(profile, { recursive: true, force: true });
  mkdirSync(join(profile, "User"), { recursive: true });
  writeFileSync(
    join(profile, "User", "settings.json"),
    JSON.stringify(
      {
        "workbench.startupEditor": "none",
        "workbench.tips.enabled": false,
        "workbench.colorTheme": "Default Dark Modern",
        "window.commandCenter": false,
        "breadcrumbs.enabled": false,
        "editor.minimap.enabled": false,
        "editor.stickyScroll.enabled": false,
        ...(args.suggestOff
          ? {
              "editor.suggestOnTriggerCharacters": false,
              "editor.quickSuggestions": false,
              "editor.tabCompletion": "off",
              "editor.acceptSuggestionOnEnter": "off",
            }
          : {}),
        "editor.detectIndentation": false,
        "editor.insertSpaces": false,
        "editor.tabSize": 4,
        "git.openRepositoryInParentFolders": "never",
        "update.showReleaseNotes": false,
        "extensions.autoCheckUpdates": false,
        "extensions.autoUpdate": false,
        "telemetry.telemetryLevel": "off",
        "security.workspace.trust.enabled": false,
        // **默认那一组不写这个键**：要判的就是"清单里的默认值"本身（真宿主会
        // 拿 `package.json` 的 default 顶上）；逃生门那组才显式钉 `false`。
        ...(eager === undefined ? {} : { "sokonanoda.input.eager": eager }),
      },
      null,
      2,
    ),
  );
  return { ws, profile };
}

/// `--vsix` ⇒ 把**已发布的那份**装进 profile 的扩展目录（不碰用户自己的扩展）。
/// 装完返回 `null`（走已安装路径），否则返回工作树路径（走 `--extensionDevelopmentPath`）。
function stageVsix(profile) {
  if (args.ext) return args.ext;
  if (!args.vsix) return EXTENSION;
  if (!existsSync(args.vsix)) {
    console.error(`error: 找不到 VSIX：${args.vsix}`);
    process.exit(3);
  }
  const dir = join(profile, "extensions");
  const unpack = join(profile, "unpack");
  rmSync(unpack, { recursive: true, force: true });
  mkdirSync(unpack, { recursive: true });
  execFileSync("unzip", ["-q", "-o", args.vsix, "-d", unpack]);
  const manifest = join(unpack, "extension", "package.json");
  if (!existsSync(manifest)) {
    console.error("error: VSIX 里没有 extension/package.json");
    process.exit(3);
  }
  const version = JSON.parse(readFileSync(manifest, "utf8")).version;
  const installed = join(dir, `sokonanoda-lang.sokonanoda-${version}`);
  rmSync(installed, { recursive: true, force: true });
  mkdirSync(installed, { recursive: true });
  execFileSync("cp", ["-R", join(unpack, "extension") + "/.", installed]);
  log(`已把 VSIX ${version} 装进 profile`);
  return null;
}

function launch(ws, profile, file, devPath) {
  const child = spawn(
    vscodeBin(),
    [
      `--user-data-dir=${profile}`,
      `--extensions-dir=${join(profile, "extensions")}`,
      ...(devPath ? [`--extensionDevelopmentPath=${devPath}`] : []),
      `--remote-debugging-port=${debugPort}`,
      "--disable-gpu",
      "--disable-chromium-sandbox",
      "--no-first-run",
      "--disable-workspace-trust",
      "--disable-extension",
      "GitHub.copilot-chat",
      "--disable-extension",
      "GitHub.copilot",
      "--goto",
      `${join(ws, file)}:1:1`,
      ws,
    ],
    { detached: true, stdio: ["ignore", "ignore", "ignore"] },
  );
  child.unref();
}

async function quit(profile) {
  try {
    execFileSync("pkill", ["-f", `user-data-dir=${profile}`]);
  } catch {
    /* 已经不在了 */
  }
  // 等端口真的空出来（最多 15s）：下一组要换端口，但旧窗口残留会占着 CDP，
  // 让"新窗口还没起来"与"旧窗口还在答"分不清。
  for (let i = 0; i < 30; i++) {
    try {
      await cdpTargets();
    } catch {
      return;
    }
    await sleep(500);
  }
}

// ── 一条用例 ────────────────────────────────────────────────────────────────
async function runCase(client, testCase, ws) {
  const startedAt = Date.now();
  const path = join(ws, testCase.file);
  writeFileSync(path, "\n", "utf8");
  await openFile(client, testCase.file);
  await client.clickEditor();
  await sleep(300);
  await clearFile(client);

  for (const ch of testCase.type) {
    await client.char(ch);
    await sleep(60); // 真人的打字节奏：给 selection 事件与 context key 留出时间
  }
  await sleep(350);

  const before = (JSON.parse(await client.eval(READ_LINES))[0] ?? "").replace(/\u00a0/g, " ");
  for (const key of testCase.keys) {
    if (key === "Undo") {
      await client.chord("z");
    } else {
      const [code, vk] = { Tab: ["Tab", 9], Enter: ["Enter", 13] }[key] ?? [key, 0];
      await client.key(key, code, vk);
    }
    await sleep(250);
  }
  await sleep(500);

  const domLine = (JSON.parse(await client.eval(READ_LINES))[0] ?? "").replace(/\u00a0/g, " ");
  await client.chord("s");
  await sleep(700);
  const diskLine = readFileSync(path, "utf8").split("\n")[0];

  const domOk = testCase.dom === false ? true : domLine === testCase.expect;
  const pass = domOk && diskLine === testCase.expect;
  return {
    id: testCase.id,
    ms: Date.now() - startedAt,
    why: testCase.why,
    typed: testCase.type,
    keys: testCase.keys,
    typed_line: before,
    dom_line: domLine,
    disk_line: diskLine,
    expect: testCase.expect,
    dom_checked: testCase.dom !== false,
    pass,
  };
}

/// 一组同 eager 设置的用例 = 一次真 VS Code 启动（设置改了要重启窗口才生效）。
async function runGroup(cases, eager, tag, port) {
  debugPort = port;
  const { ws, profile } = prepareWorkspace(tag, eager);
  const devPath = stageVsix(profile);
  launch(ws, profile, cases[0].file, devPath);
  log(
    `真 VS Code 起来了（profile ${profile} · eager=${eager === undefined ? "默认（清单）" : eager}）`,
  );
  const out = [];
  let client = null;
  try {
    client = await workbenchPage(Date.now() + args.timeout);
    await waitForEditor(client, Date.now() + args.timeout);
    for (const testCase of cases) {
      const result = await runCase(client, testCase, ws);
      out.push(result);
      log(
        `${result.pass ? "PASS" : "FAIL"} ${result.id} (${(result.ms / 1000).toFixed(1)}s): ` +
          `打「${result.typed}」+ [${result.keys.join(" ")}] ` +
          `⇒ DOM「${result.dom_line}」/ 磁盘「${result.disk_line}」/ 期望「${result.expect}」`,
      );
    }
  } finally {
    try {
      client?.close();
    } catch {
      /* ignore */
    }
    if (!args.keep) await quit(profile);
  }
  return out;
}

// ── 主流程 ──────────────────────────────────────────────────────────────────
if (!existsSync(join(EXTENSION, "package.json"))) {
  console.error(`error: 扩展目录不完整：${EXTENSION}`);
  process.exit(3);
}

const results = [];
let failure = null;
try {
  const groups = [
    { eager: undefined, cases: selected.filter((c) => c.eager === undefined) },
    { eager: false, cases: selected.filter((c) => c.eager === false) },
    { eager: true, cases: selected.filter((c) => c.eager === true) },
  ].filter((g) => g.cases.length > 0);
  for (const [index, group] of groups.entries()) {
    results.push(
      ...(await runGroup(group.cases, group.eager, `${args.port}-${index}`, args.port + index)),
    );
  }
} catch (e) {
  failure = String(e?.message ?? e);
  log(`错误：${failure}`);
}

const failed = results.filter((r) => !r.pass);
if (args.json) {
  console.log(
    JSON.stringify(
      { schema: "soko.vscode-input-e2e/1", results, failed: failed.length, error: failure },
      null,
      2,
    ),
  );
}
if (failure || failed.length > 0) {
  console.error(`[input-e2e] ${failed.length} 条失败${failure ? "（外加一条异常）" : ""}`);
  process.exit(1);
}
log(`全部 ${results.length} 条通过`);
