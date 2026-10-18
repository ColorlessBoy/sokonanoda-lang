#!/usr/bin/env node
// scripts/vscode-input-vim-e2e.mjs —— **真 VS Code + 真 vim 扩展 + 真按键**：
// 装了 VSCodeVim 之后，记法输入（`\alpha` → `α`）还对不对。
//
// 为什么必须有这一条（而不是再加一条 stub 宿主单测）
// ------------------------------------------------
// `.cache/ux/vim/REPORT.md`（只读取证，2026-10-11）用真宿主证明「全乱了」是**三个
// 叠加的问题**，而且**只在 vim 在场时**出现：
//   **A** 替换成功后 VSCodeVim 重设光标 ⇒ 光标落到自动配对的 `)` **右边** ⇒ 之后每个
//       键都错位（用户那段证明变成 `… (α) : Type) (A : Set α)) …`）；
//   **B** normal 模式的 `o`/`p`/`P`/`.` 做的是**纯插入**，改写器把它当"用户在打字"，
//       拿插入点**之前**的旧坐标回扫 ⇒ 把上一行的待定缩写补刀成符号；
//   **C** `\` 同时是 vim 的 leader（`vim.leader` 默认也是 `\`）⇒ normal 模式下
//       `\alpha` 整串被 vim 当命令吃掉。
// 这三条都活在**接缝**里（谁拥有光标 / 这次编辑是谁发起的），stub 宿主里没有 vim、
// 也没有"内容事件先于选区事件"的真时序 ⇒ 只有真宿主 + 真按键能判。
//
// 管道与 `scripts/vscode-input-e2e.mjs` 同一套（CDP 驱动真 VS Code、逐字符真 keydown、
// 读 `.view-line` DOM + 存盘字节），只是多装一个 vim 扩展、多摆一次 vim 模式。
// 那条脚本的**无 vim 对照臂**照旧是它自己（本脚本也跑 `arms: ["on","off"]` 的对照臂）。
//
// ⚠ 两条硬纪律（踩过的坑，见 REPORT §7）：
//   1. `--user-data-dir` 路径 >103 字符 ⇒ VS Code 起窗即退（IPC unix socket
//      `listen EINVAL`）⇒ profile 一律放 `$TMPDIR`，工作区留在仓库里；
//   2. 残留窗口会占着 CDP 端口 ⇒ 新窗口绑不上 ⇒ 脚本连到**上一组**窗口、读数全错
//      ⇒ 开跑前确认端口空闲，不空就**拒绝跑**（连到旧窗口的读数比没有读数更坏）。
//
// 用法：
//   node scripts/vscode-input-vim-e2e.mjs                    # 全部用例（vim 臂 + 对照臂）
//   node scripts/vscode-input-vim-e2e.mjs --list
//   node scripts/vscode-input-vim-e2e.mjs --case caret-after-rewrite
//   node scripts/vscode-input-vim-e2e.mjs --vim-only | --no-vim
//   node scripts/vscode-input-vim-e2e.mjs --json --keep
//   SOKO_VIM_EXT=~/.vscode/extensions/vscodevim.vim-1.32.4 node scripts/vscode-input-vim-e2e.mjs
//
// 退出码：0 = 全绿；1 = 有用例失败；2 = 用法错误；3 = 前置缺失（VS Code / vim 扩展）。

import { execFileSync, spawn } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  openSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const EXTENSION = join(REPO, "editor/vscode");
// ⚠ 工作区目录名必须**全局唯一**：它是这条脚本判断"连到的窗口是不是自己的"的
// 身份标记（`document.title` 里带工作区名）。仓库里同时可能有别的探针在跑，
// 端口撞车时**绝不能**把按键打进别人的窗口。
const WS_NAME = "soko-vim-fix-ws";
const WS = join(REPO, ".cache/ux/vim-fix-e2e", WS_NAME);
// ⚠ 短路径（>103 字符会让 VS Code 起窗即退）：工作区与产出留在仓库，profile 去 $TMPDIR。
const RUN = join(tmpdir(), "soko-vim-e2e");

// **陈旧 profile 清理**（2026-10-13 `SUB-probe-vscode-isolation`）：`prepare()` 只删
// **同名**的那一个，跑完**从不回收** ⇒ 实测 `TMPDIR/soko-vim-probe` 积了 **31 个
// profile / 640MB** ✗（每次运行按端口生成新 tag）。这里在**开工时**清掉**超过 6 小时**
// 的（用 mtime 判，正在跑的不会被误删 ✓；同一轮刚建的也不会 ✓）。
function pruneStaleProfiles() {
  let names;
  try {
    names = readdirSync(RUN);
  } catch {
    return;
  }
  const cutoff = Date.now() - 6 * 3600 * 1000;
  for (const n of names) {
    if (!n.startsWith("profile-") && !n.startsWith("p-")) continue;
    const p = join(RUN, n);
    try {
      if (statSync(p).mtimeMs < cutoff) rmSync(p, { recursive: true, force: true });
    } catch {
      /* 正被别的进程删/用 ⇒ 跳过 ✓ */
    }
  }
}

const IS_MAC = process.platform === "darwin";
const MOD = IS_MAC ? 4 : 2; // CDP modifiers：Alt=1 Ctrl=2 Meta=4 Shift=8
const log = (...a) => console.log("[vim-e2e]", ...a);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── 参数 ────────────────────────────────────────────────────────────────────
const args = {
  port: 9701,  // 私有端口段（避开别的探针：实测撞过一次，读数全错）
  case: null,
  json: false,
  keep: false,
  list: false,
  vimExt: process.env.SOKO_VIM_EXT ?? null,
  noVim: false,
  vimOnly: false,
  charDelay: 60,
  timeout: 300_000,
};
for (let i = 2; i < process.argv.length; i++) {
  const k = process.argv[i];
  if (k === "--port") args.port = Number(process.argv[++i]);
  else if (k === "--case") args.case = process.argv[++i];
  else if (k === "--json") args.json = true;
  else if (k === "--keep") args.keep = true;
  else if (k === "--list") args.list = true;
  else if (k === "--vim-ext") args.vimExt = resolve(process.argv[++i]);
  else if (k === "--no-vim") args.noVim = true;
  else if (k === "--vim-only") args.vimOnly = true;
  else if (k === "--char-delay") args.charDelay = Number(process.argv[++i]);
  else if (k === "--timeout") args.timeout = Number(process.argv[++i]);
  else {
    console.error(`error: 未知参数 ${k}`);
    process.exit(2);
  }
}
if (args.noVim && args.vimOnly) {
  console.error("error: --no-vim 与 --vim-only 不能同时给");
  process.exit(2);
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

/// vim 扩展：显式给路径，或在 `~/.vscode/extensions/` 里挑版本最高的
/// `vscodevim.vim-*`。找不到 ⇒ exit 3（**绝不**静默跳过 vim 臂——那样这条脚本就
/// 退化成"又跑了一遍无 vim 的 e2e"，判据空转）。
function vimExtension() {
  if (args.vimExt) {
    if (existsSync(join(args.vimExt, "package.json"))) return args.vimExt;
    console.error(`error: --vim-ext 里没有 package.json：${args.vimExt}`);
    process.exit(3);
  }
  const dir = join(homedir(), ".vscode", "extensions");
  let best = null;
  try {
    for (const name of readdirSync(dir)) {
      if (!name.startsWith("vscodevim.vim-")) continue;
      if (!existsSync(join(dir, name, "package.json"))) continue;
      if (!best || name.localeCompare(best, undefined, { numeric: true }) > 0) best = name;
    }
  } catch {
    /* 没有那个目录 */
  }
  if (!best) {
    console.error(
      "error: 找不到 vim 扩展（装一个，或用 --vim-ext / SOKO_VIM_EXT 指过去）",
    );
    process.exit(3);
  }
  return join(dir, best);
}

// 用户那段证明（`.cache/ux/vim/REPORT.md` §2.1 的原串；缩进显式打出来，两条臂同序列）。
// **判据是"配对"**：vim 臂与无 vim 臂必须逐行逐字节相同 —— 长串连打里自动配对的
// `)` 漏跳（多出一个 `)`）就是靠这条抓的，期望值不写死（省得跟宿主的缩进口径绑死）。
const PROOF = [
  "theorem subset_of_mem_singleton (\\alpha : Type) (A : Set \\alpha) (a : \\alpha) (h : a \\in A) :",
  "    {a} \\sub A := by",
  "  intro x hx",
  "  unfold Set.singleton Set.mem at hx",
  "  apply Eq.subst \\alpha (fun (y : \\alpha) => y \\in A) a x",
  "  apply Eq.symm \\alpha x a hx",
  "  exact h",
].join("\n");

// ── 用例表 ──────────────────────────────────────────────────────────────────
// `keys` 语言：字符串 = 逐字符真 keydown（`\n` = Enter）；`<Esc>` / `<Tab>` / `<CR>`
// = 具名键。`expect` = **整个文档**（DOM 与存盘字节都要等于它，行尾空行按宿主口径
// 归一——所以期望里**不要**写行尾空行）。
// `arms`：`on` = 装 vim，`off` = 不装（同一串键的对照臂）。`settings` 进 profile。
const CASES = [
  {
    id: "caret-after-rewrite",
    why:
      "**A（头号）**：insert 模式里 `(` 自动配对出 `)`，再打 `\\alpha` ⇒ α —— 替换之后光标必须" +
      "钉在 `α` 与 `)` 之间。有 vim 时 VSCodeVim 把光标推到 `)` **右边** ⇒ 空格/`x` 全落在括号外",
    arms: ["on", "off"],
    mode: "insert",
    initial: [""],
    // `)` **显式打出来**：自动配对开着时它是"跳过自动配对的 `)`"，关着时就是普通输入
    // ⇒ 期望 `(α x)` 在两种档位下都成立（判据不绑宿主的 autoClose 设置）。
    keys: ["(\\alpha x)"],
    expect: ["(α x)"],
  },
  {
    id: "normal-o-keeps-pending-prefix",
    why:
      "**B**：`\\an` 是待定前缀（故意不落定）+ `Esc` + `o`（normal 模式开新行）——" +
      "`o` 做的是纯插入，但插入点在行尾而光标停在 `n` **上** ⇒ 不许把上一行的 `\\an` 补刀成 `∧`",
    arms: ["on"],
    mode: "insert",
    initial: [""],
    keys: ["\\an", "<Esc>", "o", "Z"],
    expect: ["\\an", "Z"],
  },
  {
    id: "normal-put-keeps-complete-word",
    why:
      "**B（同族）**：normal 模式 `x` 删一个字符再 `p` 放回来——那是 put，不是打字；" +
      "文档里本来就有的 `\\and` 不许被改写（`p` 插的是**单字符**，所以这条专门判光标守卫）",
    arms: ["on"],
    mode: "normal",
    initial: ["\\and"],
    keys: ["$", "x", "p"],
    expect: ["\\and"],
  },
  {
    id: "insert-space-still-seals",
    why:
      "**B 的反向**：守卫不许把正常打字也挡掉——insert 模式下 `\\in` + 空格照样封口 ⇒ `∈ `（vim 在场也一样）",
    arms: ["on", "off"],
    mode: "insert",
    initial: [""],
    keys: ["\\in "],
    expect: ["∈ "],
  },
  {
    id: "insert-enter-still-seals",
    why:
      "**B 的反向（换行那一刀）**：insert 模式回车仍然是分隔符 ⇒ `\\in` + Enter 封口成 `∈`" +
      "（vim 的 `o` 之所以被挡，是**光标不匹配**，不是因为「插入里带换行」）",
    arms: ["on", "off"],
    mode: "insert",
    initial: [""],
    keys: ["\\in", "<CR>"],
    expect: ["∈"],
  },
  {
    id: "proof-full-vim",
    why:
      "**用户原话那条（整段证明）**：同一串键在 vim / 无 vim 两条臂下必须**逐行逐字节相同**" +
      "（自动配对的 `)` 在长串连打里漏跳一次就会多出一个 `)`）。⚠ 这条**判定抗间歇**：" +
      "同一条用例连跑 ≥3 次全绿才算过 —— flaky = 失败",
    arms: ["on", "off"],
    mode: "insert",
    initial: [""],
    // 逐行按（每行末尾补一个 Enter）⇒ `steps` 里能看出**哪一行开始多出 `)`**。
    keys: PROOF.split("\n").map((line, index, all) => (index < all.length - 1 ? `${line}\n` : line)),
    pair: true,
  },
  {
    id: "leader-comma-insert",
    why:
      "**C**：leader 可配置（`sokonanoda.input.leader: \",\"`，对齐 Lean 4 的 `lean4.input.leader`）" +
      "——vim 用户把两边 leader 都改成 `,` 之后，`,alpha` + 空格 ⇒ `α `",
    arms: ["on", "off"],
    mode: "insert",
    initial: [""],
    keys: [",alpha "],
    expect: ["α "],
    settings: { "sokonanoda.input.leader": ",", "vim.leader": "," },
  },
  {
    id: "leader-comma-tab",
    why: "**C**：`Tab` 键位的 `when`（context key）也跟着配置的 leader 走：`,in` + Tab ⇒ `∈`",
    arms: ["on"],
    mode: "insert",
    initial: [""],
    keys: [",in", "<Tab>"],
    expect: ["∈"],
    settings: { "sokonanoda.input.leader": ",", "vim.leader": "," },
  },
  {
    id: "leader-comma-old-leader-is-text",
    why:
      "**C（反向）**：leader 改成 `,` 之后 `\\alpha ` 就是普通文本，**不许**再替换" +
      "（否则说明设置只改了一半：还有地方写死 `\\`）",
    arms: ["on"],
    mode: "insert",
    initial: [""],
    keys: ["\\alpha "],
    expect: ["\\alpha "],
    settings: { "sokonanoda.input.leader": ",", "vim.leader": "," },
  },
];

// ── 构建身份（每条读数都要能归属到一份构建）─────────────────────────────────
function buildIdentity() {
  const short = (p) => {
    try {
      const st = execFileSync("stat", ["-f", "%m %z", p], { encoding: "utf8" }).trim();
      return st.split(" ").join("B@");
    } catch {
      return "missing";
    }
  };
  let head = "unknown";
  try {
    head = execFileSync("git", ["-C", REPO, "rev-parse", "--short", "HEAD"], {
      encoding: "utf8",
    }).trim();
  } catch {
    /* 不是 git 工作区 */
  }
  return {
    head,
    extension_js: short(join(EXTENSION, "extension.js")),
    // ⚠ 改写器**不在** extension.js 里（那边 `require("./src/abbreviation-rewriter")`）
    // ⇒ 身份必须带上它，否则「改了 src 却没生效」看不出来。
    abbreviation_rewriter: short(join(EXTENSION, "src/abbreviation-rewriter.js")),
  };
}

// ── CDP 客户端（零依赖：Node 自带 WebSocket）─────────────────────────────────
let debugPort = args.port;

async function cdpTargets() {
  return (await fetch(`http://127.0.0.1:${debugPort}/json/list`)).json();
}

const CODE = {
  "\\": "Backslash",
  " ": "Space",
  "\n": "Enter",
  "{": "BracketLeft",
  "}": "BracketRight",
  "(": "Digit9",
  ")": "Digit0",
  ":": "Semicolon",
  ".": "Period",
  "_": "Minus",
  "'": "Quote",
  ">": "Period",
  "<": "Comma",
  "=": "Equal",
  "|": "Backslash",
  ",": "Comma",
  "-": "Minus",
  "/": "Slash",
};
const VK = {
  "\\": 220,
  " ": 32,
  "\n": 13,
  "{": 219,
  "}": 221,
  "(": 57,
  ")": 48,
  ":": 186,
  ".": 190,
  "_": 189,
  "'": 222,
  "<": 188,
  ">": 190,
  "=": 187,
  "|": 220,
  ",": 188,
  "-": 189,
  "/": 191,
};
const SHIFTED = new Set(["{", "}", "(", ")", ":", "_", "<", ">", "|", "?"]);
function codeOf(ch) {
  if (CODE[ch]) return CODE[ch];
  if (/[a-zA-Z]/.test(ch)) return `Key${ch.toUpperCase()}`;
  if (/[0-9]/.test(ch)) return `Digit${ch}`;
  return "";
}
function vkOf(ch) {
  if (VK[ch] !== undefined) return VK[ch];
  if (/[a-zA-Z]/.test(ch)) return ch.toUpperCase().charCodeAt(0);
  if (/[0-9]/.test(ch)) return ch.charCodeAt(0);
  return 0;
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
  // ⚠ 必须带超时：目标失效的 socket 不会回包，无超时的 await 会把整条脚本挂死。
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
    /// ⚠ **读 DOM 之前必须把窗口提到前台**：macOS 会节流后台窗口的渲染 ⇒
    /// `.view-line` 停在旧内容，而文档/磁盘已经是对的（实测：一条用例 DOM 显示上一个
    /// 用例的文本 ⇒ **假红**）。`Page.bringToFront` 是 CDP 的激活手段。
    async bringToFront() {
      try {
        await call("Page.bringToFront", {}, 5_000);
      } catch {
        /* 老宿主没有这个方法就算了 */
      }
    },
    /// ⭐ **按"已聚焦"渲染**（2026-10-18 实测的第二个探针缺陷 ✗→✓）：
    /// `Page.bringToFront` 只切**页签**、**不把 OS 窗口提到前台** ⇒ 窗口被挡在后面时
    /// macOS 会把渲染**节流**，`.view-line` 可以是**永久陈旧**的 —— 实测：磁盘已经是
    /// `["(α x)"]`（对的 ✓）而 DOM **整条用例**都停在 `[""]` ✗ ⇒ 那是**假红**，
    /// 且 `stableLines()` 的"连读两次一样就算数"**挡不住它**（陈旧画面**本身**很稳 ✗）。
    /// ⇒ 用 CDP 的 `Emulation.setFocusEmulationEnabled`（DevTools 那条 "Emulate a
    /// focused page" ✓）把渲染与"窗口在不在前台"**解耦** ✓。
    async focusEmulation() {
      try {
        await call("Emulation.setFocusEmulationEnabled", { enabled: true }, 5_000);
      } catch {
        /* 老宿主没有这个方法就算了（照 `bringToFront` 的先例 ✓） */
      }
    },
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
    /// 一个可打印字符：真 keydown 带 `text`（Monaco 靠它插入字符）。
    async char(ch) {
      const base = {
        key: ch,
        code: codeOf(ch),
        windowsVirtualKeyCode: vkOf(ch),
        nativeVirtualKeyCode: vkOf(ch),
        modifiers: SHIFTED.has(ch) ? 8 : 0,
      };
      await call("Input.dispatchKeyEvent", {
        type: "keyDown",
        text: ch,
        unmodifiedText: ch,
        ...base,
      });
      await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
    },
    /// 真正的 Enter：`key` 必须是 "Enter"（Monaco 的 keyCode 判定），**同时**带
    /// `text: "\r"`（插入换行靠它）。少任一个都插不出换行。
    async enter() {
      const base = { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 };
      await call("Input.dispatchKeyEvent", {
        type: "keyDown",
        text: "\r",
        unmodifiedText: "\r",
        ...base,
      });
      await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
    },
    async chord(letter, shift = false) {
      await key(
        letter,
        `Key${letter.toUpperCase()}`,
        letter.toUpperCase().charCodeAt(0),
        MOD | (shift ? 8 : 0),
      );
    },
    async clickEditor() {
      const rect = JSON.parse(
        await this.eval(
          `(() => { const r = document.querySelector('.monaco-editor')?.getBoundingClientRect();
             return JSON.stringify(r ? {x:r.x,y:r.y,w:r.width,h:r.height} : null); })()`,
        ),
      );
      if (!rect) throw new Error("找不到编辑器区域");
      const x = Math.round(rect.x + rect.w * 0.55);
      const y = Math.round(rect.y + rect.h * 0.35);
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

// ── 具名键 ──────────────────────────────────────────────────────────────────
const NAMED = {
  "<Esc>": ["Escape", "Escape", 27, 0],
  "<Tab>": ["Tab", "Tab", 9, 0],
  "<BS>": ["Backspace", "Backspace", 8, 0],
  "<Del>": ["Delete", "Delete", 46, 0],
};
async function press(client, token) {
  if (token === "<CR>") {
    await client.enter();
    return;
  }
  if (NAMED[token]) {
    await client.key(...NAMED[token]);
    return;
  }
  for (const ch of token) {
    if (ch === "\n") await client.enter();
    else await client.char(ch);
    await sleep(args.charDelay);
  }
}

// ── 编辑器 DOM：用户屏幕上那些行 ─────────────────────────────────────────────
const READ_LINES = `(() => {
  const lines = Array.from(document.querySelectorAll('.view-lines .view-line'));
  return JSON.stringify(lines.map((l) => l.textContent));
})()`;
const READ_MODE = `(() => {
  const items = Array.from(document.querySelectorAll('.statusbar-item'));
  const t = items.map((i) => i.textContent || '').filter((s) => /NORMAL|INSERT|VISUAL|REPLACE|--/.test(s));
  return t.length ? t[t.length - 1] : null;
})()`;
const ACTIVE_TAB = `(() => { const t = document.querySelector('.tabs-container .tab.active'); return t ? t.textContent : null; })()`;

function cleanLines(raw) {
  const lines = JSON.parse(raw).map((s) => s.replace(/\u00a0/g, " ").replace(/\u200b/g, ""));
  while (lines.length > 1 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

/// **窗口身份判据**：连上 CDP 之后第一件事是确认这个窗口是自己的工作区。
/// 起因（实测）：仓库里同时有别的探针/子 agent 在跑 VS Code，端口一撞，脚本会
/// 连到**别人的窗口**——读数全错是轻的，重的是按键会打进别人的工作区。
async function assertOwnWindow(client) {
  const deadline = Date.now() + 30_000;
  let title = "";
  while (Date.now() < deadline) {
    title = String((await client.eval("document.title")) ?? "");
    if (title.includes(WS_NAME)) return;
    await sleep(500); // 刚连上时标题可能还没写（空串）——等它写完再判
  }
  throw new Error(
    `CDP 连到的不是本脚本的窗口（title「${title}」里没有 ${WS_NAME}）——` +
      "端口被别的窗口占着，**不跑**",
  );
}

/// **稳定读 DOM**：macOS 会节流后台窗口的渲染 ⇒ 单次读可能拿到**上一个用例**的内容
///（实测：磁盘已经是对的、DOM 还是旧的 ⇒ 假红）。连续两次读到一样的才算数。
async function stableLines(client) {
  // ⭐ **先让渲染按"已聚焦"跑**（2026-10-18 实测的第二个探针缺陷 ✗→✓，理由见
  //   `focusEmulation()` 的注释）：不设它，被挡在后面的窗口可以**整条用例**都停在
  //   陈旧画面 ⇒ "连读两次一样"照样返回**假红**的 `[""]` ✗。
  await client.focusEmulation();
  let previous = null;
  for (let i = 0; i < 5; i++) {
    await client.bringToFront();
    const lines = cleanLines(await client.eval(READ_LINES));
    if (previous && same(previous, lines)) return lines;
    previous = lines;
    await sleep(250);
  }
  return previous;
}

async function waitForEditor(client, deadline) {
  while (Date.now() < deadline) {
    try {
      const raw = await client.eval(READ_LINES);
      if (raw && raw !== "[]" && raw !== "null") return cleanLines(raw);
    } catch {
      /* 页面还在换 */
    }
    await sleep(400);
  }
  throw new Error("编辑器没有渲染出任何行（窗口没起来 / 文件没打开？）");
}

/// 命令面板 / 快速打开**真的打开了吗**。实测踩到两次：面板没开（窗口忙/焦点没到），
/// 后面那几个字符就**打进编辑器**（DOM 变成 `evert` / `cratch.sokonanoda`），读数全错。
/// ⇒ 开面板要**验**，验不过就重试；再不过就**报错停下**，绝不盲打字。
const QUICK_INPUT_OPEN = `(() => {
  const w = document.querySelector('.quick-input-widget');
  if (!w) return false;
  const s = getComputedStyle(w);
  return s.display !== 'none' && s.visibility !== 'hidden' && w.offsetHeight > 0;
})()`;

async function openPalette(client, shift) {
  for (let attempt = 0; attempt < 3; attempt++) {
    // ⚠ 起手先 `Escape`：新窗口有时把焦点放在 Welcome/onboarding 上
    //（实测 `activeElement` = `onboarding-a-signin-btn`），此时 `Cmd+P` 不生效 ——
    // 面板没开、后面那串字符就**打进编辑器**（DOM 变成 `evert`/`cratch.sokonanoda`）。
    await client.key("Escape", "Escape", 27);
    await sleep(150);
    await client.chord("p", shift);
    const deadline = Date.now() + 5000;
    while (Date.now() < deadline) {
      if (await client.eval(QUICK_INPUT_OPEN)) {
        await sleep(300); // 输入框拿到焦点再打字
        return true;
      }
      await sleep(150);
    }
  }
  // 失败要说清楚现场（面板在不在 DOM 里 / display 是什么 / 标题是什么），
  // 否则只剩一句"没打开"，下一次还是不知道从哪查。
  const state = await client.eval(`(() => {
    const w = document.querySelector('.quick-input-widget');
    return JSON.stringify({
      widget: w ? getComputedStyle(w).display + "/" + w.offsetHeight : "absent",
      title: document.title,
      focused: document.hasFocus(),
      active: (document.activeElement && document.activeElement.className) || "",
    });
  })()`);
  log(`快速打开没打开：${state}`);
  return false;
}

/// 用**真键位**切到某个文件（`Cmd/Ctrl+P` 快速打开 ⇒ 打名字 ⇒ Enter）。
async function openFile(client, name) {
  if (!(await openPalette(client, false))) throw new Error("快速打开没打开（不盲打字）");
  await sleep(200);
  for (const ch of name) {
    await client.char(ch);
    await sleep(25);
  }
  await sleep(800);
  await client.key("Enter", "Enter", 13);
  await sleep(900);
  const tab = await client.eval(ACTIVE_TAB);
  if (!String(tab ?? "").includes(name)) {
    throw new Error(`快速打开没能切到 ${name}（当前标签页：${tab}）`);
  }
}

/// `File: Revert File`（命令面板）—— 每个用例的干净起点（磁盘上刚写下的内容）。
/// ⚠ 不用 `Cmd+A`+`Delete`：vim 在场时那是 normal 模式的动作，会跟模式摆位打架。
async function revertFile(client) {
  if (!(await openPalette(client, true))) throw new Error("命令面板没打开（不盲打字）");
  await sleep(200);
  for (const ch of "revert") {
    await client.char(ch);
    await sleep(35);
  }
  await sleep(700);
  await client.key("Enter", "Enter", 13);
  await sleep(900);
}

// ── 工作区 / profile / 启动 ─────────────────────────────────────────────────
function baseSettings() {
  return {
    "workbench.startupEditor": "none",
    "workbench.tips.enabled": false,
    "workbench.colorTheme": "Default Dark Modern",
    "window.commandCenter": false,
    "breadcrumbs.enabled": false,
    "editor.minimap.enabled": false,
    "editor.stickyScroll.enabled": false,
    "editor.wordWrap": "off",
    "editor.detectIndentation": false,
    "editor.insertSpaces": false,
    "editor.tabSize": 4,
    "editor.autoIndent": "full",
    // ⚠ **不要**在这里写 `editor.autoClosingBrackets`：它会把扩展给 `[sokonanoda]` 的
    // `configurationDefaults`（= 用户真实拿到的那一档）压掉，判据就测不到产品默认值了。
    // （本轮实测：`languageDefined` 下整段证明会多出 `)` —— 见 README「Vim users」。）
    // Enter **必须是换行**：`\in` 会拉起补全弹窗，而 Enter 的默认行为是"接受补全"
    // ⇒ `insert-enter-still-seals` 会变成 `\inductive`（实测踩到，两条臂都踩）。
    "editor.acceptSuggestionOnEnter": "off",
    "git.openRepositoryInParentFolders": "never",
    "update.showReleaseNotes": false,
    "extensions.autoCheckUpdates": false,
    "extensions.autoUpdate": false,
    "telemetry.telemetryLevel": "off",
    "security.workspace.trust.enabled": false,
  };
}

function prepare(tag, arm, settings, vimExt) {
  const profile = join(RUN, `profile-${tag}`);
  rmSync(profile, { recursive: true, force: true });
  mkdirSync(join(profile, "User"), { recursive: true });
  mkdirSync(join(profile, "extensions"), { recursive: true });
  writeFileSync(
    join(profile, "User", "settings.json"),
    JSON.stringify({ ...baseSettings(), ...settings }, null, 2),
  );
  if (arm === "on") {
    const name = `vscodevim.vim-${JSON.parse(readFileSync(join(vimExt, "package.json"), "utf8")).version}`;
    cpSync(vimExt, join(profile, "extensions", name), { recursive: true });
  }
  mkdirSync(WS, { recursive: true });
  return profile;
}

function launch(profile, file, arm) {
  // stderr 落盘（诊断用）：窗口起不来时唯一能看的东西。
  const errLog = join(profile, "vscode-stderr.log");
  const fd = openSync(errLog, "a");
  const child = spawn(
    vscodeBin(),
    [
      `--user-data-dir=${profile}`,
      `--extensions-dir=${join(profile, "extensions")}`,
      `--shared-data-dir=${join(profile, "shared")}`,
      `--remote-debugging-port=${debugPort}`,
      `--extensionDevelopmentPath=${EXTENSION}`,
    // **隔离：不加载用户/内置扩展**（2026-10-13 `SUB-probe-vscode-isolation`）——
    // 官方 e2e（`editor/vscode/.vscode-test.mjs`）一直带着它，而四个探针都漏了 ⇒
    // 每次起窗都会加载 VS Code 的**内置**扩展（GitHub 认证 / Settings Sync 一类）⇒
    // ① 弹 GitHub 登录并拉起浏览器；② 副作用一直在（用户看得见）✗。
    // ⚠ `--disable-extensions` **不会**关掉被测扩展：我们同时传了
    // `--extensionDevelopmentPath`（开发模式）⇒ 被测的那个照常加载 ✓。
    // ⚠ ⭐ **但它会关掉 vim 扩展** ✗（2026-10-18 实测 —— **"vim 臂"此前一直是假的**）：
    //   本脚本把 VSCodeVim 拷进 `--extensions-dir`（`prepare()` ✓），而
    //   `--disable-extensions` 把**装在那里的**扩展一起禁用 ⇒ **vim 臂里根本没有 vim**
    //   ✗ ⇒ 摆模式的 `<Esc>` + `i` 里那个 `i` **被当正文打进去**（实测 DOM = `["i(α x)"]`）
    //   ⇒ 9 用例 × 2 臂 = **14 条读数全是假的**（"vim 臂"退化成"又跑一遍无 vim 臂"、
    //   还多一个 `i` ✗ = 判据**空转** ✗）。⚠ 它在 2026-10-13 被加进来，而这条判据
    //   **从没接线**（真接线 0/2 ✗）⇒ **没人发现** —— 用户 2026-10-17「修了不守 ⇒ 会漂」✓。
    //   ⇒ **修法**：vim 臂**不传** `--disable-extensions`，改成**逐个点名**关掉内置噪声
    //   （`vscode.github-authentication` / `vscode.microsoft-authentication` /
    //   `vscode.github` / `github.copilot-chat` —— **正是**那条队列条目点名的几个 ✓）。
    //   ⚠ **用户自己装的扩展仍然一个都不加载** ✓：本脚本**总是**传
    //   `--extensions-dir=<profile>/extensions`（只有被测扩展 + 拷进去的 vim ✓）
    //   ⇒ 隔离没有丢，丢的只是"连 vim 一起关掉" ✗。
    //   ⚠ 为什么不用 `--enable-extension=vscodevim.vim`（**实测否掉** ✗）：VS Code
    //   1.141.0 的 app bundle 里**没有** `enable-extension` 这个开关
    //   （`grep -o enable-extension Contents/Resources/app/out/*.js` ⇒ **0** 处 ✓），
    //   传了也不生效（实测：vim 臂照样把 `i` 打进正文 ✗）。
    //   ⚠ A/B 读数（同一条用例 / 同一天 / 同一份构建，见 `.cache/ux/fix-v1/VERDICT.md`）：
    //   修前磁盘「""」（`i` 在正文里、vim 不在）；修后 **磁盘 = 期望「(α x)」** ✓。
    ...(arm === "on" ? [] : ["--disable-extensions"]),
      "--disable-gpu",
      "--disable-chromium-sandbox",
      "--no-first-run",
      "--disable-workspace-trust",
      // ⚠ 两条臂**都**关这几个（`arm === "on"` 那一臂靠的就是这一串当隔离 ✓）。
      "--disable-extension",
      "vscode.github-authentication",
      "--disable-extension",
      "vscode.microsoft-authentication",
      "--disable-extension",
      "vscode.github",
      "--disable-extension",
      "GitHub.copilot-chat",
      "--disable-extension",
      "GitHub.copilot",
      "--goto",
      `${join(WS, file)}:1:1`,
      WS,
    ],
    { detached: true, stdio: ["ignore", "ignore", fd] },
  );
  child.unref();
  return errLog;
}

/// 端口卫生（判据能不能信的前提）：残留窗口占着 CDP 端口 ⇒ 新窗口绑不上 ⇒ 连到
/// 上一组窗口，读数全错。
function freePort(port) {
  for (const pat of [
    `remote-debugging-port=${port}`,
    `user-data-dir=${join(RUN, `profile-${port}`)}`,
  ]) {
    try {
      execFileSync("pkill", ["-f", pat]);
    } catch {
      /* 本来就没有 */
    }
  }
}

async function portFree(port, deadline) {
  while (Date.now() < deadline) {
    let busy = false;
    try {
      const r = execFileSync("lsof", ["-nP", `-iTCP:${port}`, "-sTCP:LISTEN"], {
        encoding: "utf8",
      });
      busy = r.trim().split("\n").length > 1;
    } catch {
      busy = false;
    }
    if (!busy) return true;
    await sleep(500);
  }
  return false;
}

async function quit(profile) {
  for (const pattern of [`user-data-dir=${profile}`, `remote-debugging-port=${debugPort}`]) {
    try {
      execFileSync("pkill", ["-f", pattern]);
    } catch {
      /* 已经不在了 */
    }
  }
  for (let i = 0; i < 30; i++) {
    try {
      await cdpTargets();
    } catch {
      return;
    }
    await sleep(500);
  }
  // 没退干净要说出来：残留窗口会占着 CDP 端口，让下一组的读数归属到旧窗口。
  log(`⚠ 窗口没退干净（profile ${profile}）——下一组换端口，但请留意`);
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

// ── 一条用例 ────────────────────────────────────────────────────────────────
async function runCase(client, testCase, arm) {
  const startedAt = Date.now();
  const file = testCase.file ?? "Scratch.sokonanoda";
  const path = join(WS, file);
  // ⚠ 不写结尾换行：`[""]` 要是**一行**空文档。写成 `"\n"` 就是两行，而
  // `clickEditor()` 点在编辑器中部 ⇒ 光标落在第 2 行，用例全跑到下一行去（实测踩到）。
  writeFileSync(path, testCase.initial.join("\n"), "utf8");
  await openFile(client, file.split("/").pop());
  await client.clickEditor();
  await sleep(300);
  await revertFile(client);

  // **模式卫生**：vim 模式会跨用例残留（上一个停在 INSERT，下一个就在 INSERT 里跑
  // ⇒ 结果依赖顺序）。每个用例显式摆模式：insert ⇒ `<Esc>` + `i`；normal ⇒ `<Esc>`；
  // fresh ⇒ 不摆（刚开窗的模式）。无 vim 臂没有模式，一步都不摆。
  const setupKeys =
    arm === "on" && testCase.mode !== "fresh"
      ? testCase.mode === "normal"
        ? ["<Esc>"]
        : ["<Esc>", "i"]
      : [];
  // 每一步记一次 DOM（诊断用：失败时能看出"哪一步开始变坏"，报告里也贴这个）。
  const steps = [];
  const record = async (label) => {
    steps.push({ step: label, dom: await stableLines(client) });
  };
  for (const token of setupKeys) {
    await press(client, token);
    await sleep(220);
  }
  await record("(摆好模式)");

  for (const token of testCase.keys) {
    await press(client, token);
    await sleep(token.startsWith("<") ? 260 : 160);
    await record(token);
  }
  await sleep(400);

  const domLines = await stableLines(client);
  const modeFinal = await client.eval(READ_MODE);
  await client.chord("s");
  await sleep(800);
  const diskRaw = readFileSync(path, "utf8").replace(/\r\n/g, "\n");
  const diskLines = diskRaw.split("\n");
  while (diskLines.length > 1 && diskLines[diskLines.length - 1] === "") diskLines.pop();

  if (testCase.pair === true) {
    return {
      id: testCase.id, arm, ms: Date.now() - startedAt, why: testCase.why,
      keys: ["PROOF（整段）"], mode: testCase.mode, setup_keys: setupKeys,
      mode_final: modeFinal, steps, dom_lines: domLines, disk_lines: diskLines,
      expect: null, dom_ok: null, disk_ok: null, pass: null,
    };
  }
  const domOk = same(domLines, testCase.expect);
  const diskOk = same(diskLines, testCase.expect);
  return {
    id: testCase.id,
    arm,
    ms: Date.now() - startedAt,
    why: testCase.why,
    keys: testCase.keys,
    mode: testCase.mode,
    setup_keys: setupKeys,
    mode_final: modeFinal,
    steps,
    dom_lines: domLines,
    disk_lines: diskLines,
    expect: testCase.expect,
    dom_ok: domOk,
    disk_ok: diskOk,
    pass: domOk && diskOk,
  };
}

/// 一组 =（臂 × 设置）⇒ 一次真 VS Code 启动（改设置要重启窗口才生效）。
async function runGroup(cases, arm, settings, tag, port, vimExt) {
  debugPort = port;
  const profile = prepare(tag, arm, settings, vimExt);
  freePort(port);
  if (!(await portFree(port, Date.now() + 20_000))) {
    throw new Error(
      `端口 ${port} 被占着（残留窗口没退干净）——**不跑**，否则读数会归属到旧窗口`,
    );
  }
  launch(profile, cases[0].file ?? "Scratch.sokonanoda", arm);
  log(`真 VS Code 起来了（arm=${arm} · port=${port} · ${JSON.stringify(settings)}）`);
  const out = [];
  let client = null;
  try {
    client = await workbenchPage(Date.now() + args.timeout);
    await assertOwnWindow(client);
    await waitForEditor(client, Date.now() + args.timeout);
    for (const testCase of cases) {
      const result = await runCase(client, testCase, arm);
      out.push(result);
      log(
        `${result.pass === null ? "PAIR" : result.pass ? "PASS" : "FAIL"} ${result.id} [vim=${arm}] (${(result.ms / 1000).toFixed(1)}s): ` +
          `按「${result.keys.join(" · ")}」⇒ DOM「${JSON.stringify(result.dom_lines)}」/ ` +
          `磁盘「${JSON.stringify(result.disk_lines)}」/ 期望「${JSON.stringify(result.expect)}」`,
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
pruneStaleProfiles();  // 陈旧 profile 回收（SUB-probe-vscode-isolation）✓
if (!existsSync(join(EXTENSION, "package.json"))) {
  console.error(`error: 扩展目录不完整：${EXTENSION}`);
  process.exit(3);
}
const identity = buildIdentity();
if (args.list) {
  for (const c of CASES) {
    console.log(`${c.arms.join("/").padEnd(7)} ${c.id.padEnd(34)} ${c.why}`);
  }
  process.exit(0);
}

const wantedArms = args.noVim ? ["off"] : args.vimOnly ? ["on"] : ["on", "off"];
const selected = (args.case ? CASES.filter((c) => c.id === args.case) : CASES).filter((c) =>
  c.arms.some((arm) => wantedArms.includes(arm)),
);
if (selected.length === 0) {
  console.error(
    args.case
      ? `error: --case ${args.case} 不在用例表里（${CASES.map((c) => c.id).join(", ")}）`
      : "error: 没有可跑的用例（--vim-only/--no-vim 过滤掉了全部？）",
  );
  process.exit(2);
}
const vimExt = wantedArms.includes("on") ? vimExtension() : null;

const results = [];
let failure = null;
try {
  // 分组键 = 臂 + 设置（同一次启动能跑完的用例）
  const groups = new Map();
  for (const testCase of selected) {
    for (const arm of testCase.arms.filter((a) => wantedArms.includes(a))) {
      const settings = testCase.settings ?? {};
      const key = `${arm}|${JSON.stringify(settings)}`;
      if (!groups.has(key)) groups.set(key, { arm, settings, cases: [] });
      groups.get(key).cases.push(testCase);
    }
  }
  let index = 0;
  for (const [, group] of groups) {
    results.push(
      ...(await runGroup(
        group.cases,
        group.arm,
        group.settings,
        `${args.port}-${index}-${group.arm}`,
        args.port + index,
        vimExt,
      )),
    );
    index += 1;
  }
} catch (e) {
  failure = String(e?.message ?? e);
  log(`错误：${failure}`);
}

// 配对用例（`pair: true`）：两条臂逐行逐字节比 —— 不等就是**失败**（这就是判据）。
for (const testCase of selected.filter((c) => c.pair === true)) {
  const arms = results.filter((r) => r.id === testCase.id);
  if (arms.length !== 2) continue;
  const [a, b] = arms;
  const domSame = same(a.dom_lines, b.dom_lines);
  const diskSame = same(a.disk_lines, b.disk_lines);
  const pass = domSame && diskSame;
  for (const r of arms) {
    r.pass = pass;
    r.pair_note = pass
      ? `两臂一致：DOM/磁盘都等于 ${JSON.stringify(a.dom_lines)}`
      : `两臂**不一致**：DOM ${JSON.stringify(a.dom_lines)} vs ${JSON.stringify(b.dom_lines)} · ` +
        `磁盘 ${JSON.stringify(a.disk_lines)} vs ${JSON.stringify(b.disk_lines)}`;
  }
}

const failed = results.filter((r) => !r.pass);
if (args.json) {
  console.log(
    JSON.stringify(
      {
        schema: "soko.vscode-input-vim-e2e/1",
        identity: { ...identity, vim: vimExt },
        results,
        failed: failed.length,
        error: failure,
      },
      null,
      2,
    ),
  );
} else {
  log(
    `构建身份：HEAD ${identity.head} · extension.js ${identity.extension_js} · ` +
      `abbreviation-rewriter.js ${identity.abbreviation_rewriter} · vim ${vimExt ?? "（未装）"}`,
  );
}
if (failure || failed.length > 0) {
  console.error(`[vim-e2e] ${failed.length} 条失败${failure ? "（外加一条异常）" : ""}`);
  process.exit(1);
}
log(`全部 ${results.length} 条通过`);
