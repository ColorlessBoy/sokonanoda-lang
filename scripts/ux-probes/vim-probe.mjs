#!/usr/bin/env node
// scripts/ux-probes/vim-probe.mjs —— **真 VS Code + 真 vim 扩展 + 真按键**：vim 模式下的
// 记法输入（`\alpha` → `α`）与键位冲突复现件。
//
// 它抄的是 `scripts/vscode-input-e2e.mjs` 的管道（CDP 驱动真 VS Code、逐字符真
// keydown、读编辑器 DOM 的 `.view-line`、读存盘字节）——**不改那个脚本**，只是
// 多装一个 `vscodevim.vim` 扩展、多读一个 vim 模式指示器。
//
// 两条臂（对照臂是判据的一部分）：
//   * `--arm off` = 不装 vim（同一按键序列的对照组）
//   * `--arm on`  = 装 vim（`--extensions-dir` 里放 vscodevim.vim-1.32.4）
// 每个 case 自带 `vim` 默认臂；`--arm` 可强制覆盖（这就是"同一按键序列两条臂"）。
//
// 退出码（与 `scripts/vscode-input-e2e.mjs` 同口径）：
//   0 = 该 case 的 DOM 与存盘字节都等于声明的期望
//   1 = 不一致（有 case 失败 / 或 `--pair` 的"无 vim 绿 + 有 vim 红"不成立）
//   2 = 用法错误 · 3 = 前置缺失
//
// 用法：
//   node scripts/ux-probes/vim-probe.mjs --list
//   node scripts/ux-probes/vim-probe.mjs --case nota-space            # 该 case 的默认臂
//   node scripts/ux-probes/vim-probe.mjs --case nota-space --arm off  # 对照臂
//   node scripts/ux-probes/vim-probe.mjs --pair vim-nm-o-prefix       # 判据：off 绿 + on 红
//   node scripts/ux-probes/vim-probe.mjs --all --json
//   node scripts/ux-probes/vim-probe.mjs --all --json --keep
//
// 纪律：独立 CDP 端口（默认 9501）、独立 profile / 工作区（`.cache/ux/vim/`）、
// 跑完 `pkill -f user-data-dir=<profile>`；**不碰仓库里那份课程文件**。

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

const HERE = dirname(fileURLToPath(import.meta.url)); // <repo>/scripts/ux-probes
const REPO = resolve(HERE, "..", "..");               // <repo>
const EXTENSION = join(REPO, "editor/vscode");
// 工作区落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏（路径与旧版 `.cache/ux/vim/` 一致）
const STATE = join(REPO, ".cache", "ux", "vim");
const WS = join(STATE, "ws");
// ⚠ **profile 必须在短路径下**：VS Code 的实例 IPC 是 `--user-data-dir` 里的
// unix socket，路径 >103 字符 ⇒ `listen EINVAL` ⇒ 窗口**起来就退**（实测踩到：
// 仓库路径 + `.cache/ux/vim/run/profile-9501-pair-off/1.14-main.sock` = 118 字符）。
// `scripts/vscode-input-e2e.mjs` 用 `tmpdir()` 正是这个原因。工作区/产出仍在仓库里。
const RUN = join(tmpdir(), "soko-vim-probe");

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

const VIM_SRC = join(homedir(), ".vscode", "extensions", "vscodevim.vim-1.32.4");
const IS_MAC = process.platform === "darwin";
const MOD = IS_MAC ? 4 : 2; // CDP modifiers：Alt=1 Ctrl=2 Meta=4 Shift=8
const log = (...a) => console.log("[vim-probe]", ...a);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── 参数 ────────────────────────────────────────────────────────────────────
const args = { port: 9501, case: null, pair: null, all: false, list: false,
  json: false, keep: false, arm: null, timeout: 240_000, suggestOff: false,
  stepDom: true, file: "Scratch.sokonanoda", charDelay: 60 };
for (let i = 2; i < process.argv.length; i++) {
  const k = process.argv[i];
  if (k === "--port") args.port = Number(process.argv[++i]);
  else if (k === "--case") args.case = (args.case ? args.case + "," : "") + process.argv[++i];
  else if (k === "--pair") args.pair = process.argv[++i];
  else if (k === "--all") args.all = true;
  else if (k === "--list") args.list = true;
  else if (k === "--json") args.json = true;
  else if (k === "--keep") args.keep = true;
  else if (k === "--arm") args.arm = process.argv[++i];
  else if (k === "--file") args.file = process.argv[++i];
  else if (k === "--timeout") args.timeout = Number(process.argv[++i]);
  else if (k === "--suggest-off") args.suggestOff = true;
  else if (k === "--no-step-dom") args.stepDom = false;
  else if (k === "--char-delay") args.charDelay = Number(process.argv[++i]);
  else { console.error(`error: 未知参数 ${k}`); process.exit(2); }
}

function vscodeBin() {
  for (const c of [
    process.env.SOKO_VSCODE_BIN,
    "/Applications/Visual Studio Code.app/Contents/MacOS/Code",
    "/usr/share/code/code",
  ]) if (c && existsSync(c)) return c;
  console.error("error: 找不到 VS Code（设 SOKO_VSCODE_BIN）");
  process.exit(3);
}

// ── 构建身份（每条读数都要能归属到一份构建）─────────────────────────────────
function buildIdentity() {
  let head = "unknown";
  try { head = execFileSync("git", ["-C", REPO, "rev-parse", "--short", "HEAD"], { encoding: "utf8" }).trim(); } catch {}
  const ext = join(EXTENSION, "extension.js");
  let mtime = "missing", bytes = 0;
  if (existsSync(ext)) { const st = execFileSync("stat", ["-f", "%m %z", ext], { encoding: "utf8" }).trim(); mtime = st; bytes = readFileSync(ext).length; }
  return { head, extension_js_mtime: mtime, extension_js_bytes: bytes, repo: REPO };
}

// ── 用例表 ──────────────────────────────────────────────────────────────────
// `keys` 语言：字面字符串 = 逐字符真 keydown（`\n` = Enter）；
// `<Esc>`/`<Tab>`/`<CR>`/`<BS>`/`<Del>`/`<C-a>`/`<C-s>`/`<C-z>` = 具名键。
// `expect` = **整个文档**（DOM 与存盘字节都要等于它）。
// `note` = 这条为什么在表里（判据绑用户动作）。
const P = (...l) => l; // 可读性：expect 就是行数组
const CASES = [
  // ── 0. 诊断：**逐字符**（每个字符后记一次 DOM）───────────────────────────
  { id: "diag-alpha-chars", vim: true, initial: [""], mode: "fresh",
    keys: ["\\", "a", "l", "p", "h", "a"], expect: null,
    note: "诊断：`\\alpha` 逐字符（哪一步 DOM 变了？）" },
  { id: "diag-in-chars", vim: true, initial: [""], mode: "fresh",
    keys: ["\\", "i", "n"], expect: null,
    note: "诊断：`\\in` 逐字符（对照：`in` 是未完成前缀，靠空格封口）" },
  { id: "diag-alpha-space", vim: true, initial: [""], mode: "fresh",
    keys: ["\\", "a", "l", "p", "h", "a", " "], expect: null,
    note: "诊断：`\\alpha` 逐字符 + 空格" },

  { id: "diag-caret-shift", vim: true, mode: "insert", initial: [""],
    keys: ["(", "\\", "a", "l", "p", "h", "a", " ", "x"], expect: P("(α x)"),
    note: "**最小复现（光标错位）**：`(` 自动配对出 `)`，再打 `\\alpha` ⇒ α —— 光标该停在 "
      + "`α` 与 `)` 之间；有 vim 时它**跳到 `)` 右边**，于是空格/后续字符全落在括号外" },

  // ── 0.9 **对照臂**：同一串键，无 vim 必须绿、有 vim 必须红 ────────────────
  { id: "pair-alpha-space", vim: true, initial: [""], mode: "fresh",
    keys: ["<Esc>", "\\alpha "], expect: P("α "),
    note: "**对照臂**：`\\alpha ` 一串键——无 vim ⇒ `α `（绿）；有 vim ⇒ 文件开在 "
      + "NORMAL 模式，`\\` 是 vim 的 leader、`a` 是 append、`l/p/h` 是动作 ⇒ 文本没进去（红）" },

  // ── 1. 记法输入：insert 模式逐字符 + 空格封口 ────────────────────────────
  { id: "nota-space", vim: true, initial: [""], mode: "insert",
    keys: ["\\alpha ", "\\in ", "\\sub "], expect: P("α ∈ ⊆ "),
    note: "空格封口三连：`\\alpha `/`\\in `/`\\sub ` ⇒ `α `/`∈ `/`⊆ `" },
  { id: "nota-and", vim: true, initial: [""], mode: "insert",
    keys: ["\\and"], expect: P("∧"),
    note: "敲完即换：`\\and` ⇒ `∧`（不需要分隔符）" },
  { id: "nota-tab", vim: true, initial: [""], mode: "insert",
    keys: ["\\in", "<Tab>"], expect: P("∈"),
    note: "Tab 键位：`\\in` + Tab ⇒ `∈`（`in` ⊂ `inter` ⇒ 只有 Tab/分隔符能封口）" },
  { id: "nota-eager-off-tab", vim: true, initial: [""], eager: false, mode: "insert",
    keys: ["\\alpha", "<Tab>"], expect: P("α"),
    note: "逃生门（eager=false）：Tab 是唯一的路（根因判据用）" },
  { id: "nota-eager-off-space", vim: true, initial: [""], eager: false, mode: "insert",
    keys: ["\\alpha "], expect: P("\\alpha "),
    note: "逃生门（eager=false）：空格**不许**自己换（根因判据用）" },

  // ── 2. vim 模式切换：normal 模式里扩展会不会偷偷改文本 ──────────────────
  { id: "vim-esc-then-hl", vim: true, initial: [""], mode: "insert",
    keys: ["\\alpha", "<Esc>", "h", "l", "0", "$"],
    expect: P("α"),
    note: "纯光标移动（`h`/`l`/`0`/`$`）**不许**改文本" },
  { id: "vim-nm-o-prefix", vim: true, initial: [""], mode: "insert",
    keys: ["\\an", "<Esc>", "o"], expect: P("\\an", ""),
    note: "**头号嫌疑**：`\\an` 是待定前缀（未落定）⇒ normal 模式按 `o` 开新行，"
      + "上一行行尾的 `\\an` 会不会被改写器补刀成 `∧`？" },
  { id: "vim-nm-O-prefix", vim: true, initial: ["", "x"], mode: "insert",
    keys: ["\\an", "<Esc>", "k", "O"], expect: P("", "\\an", "x"),
    note: "同上，`O` 版本（在上一行之上开新行）" },
  { id: "vim-nm-p-prefix", vim: true, initial: [""], mode: "insert",
    keys: ["\\an", "<Esc>", "o", "<Esc>", "p"], expect: P("\\an", ""),
    note: "`p` 粘贴 = normal 模式的纯插入 ⇒ 同样会喂给改写器吗？" },
  { id: "vim-nm-enter-prefix", vim: true, initial: [""], mode: "insert",
    keys: ["\\an", "<Esc>", "<CR>"], expect: P("\\an", ""),
    note: "normal 模式 `<CR>`（vim 只移光标）**不许**改文本" },
  { id: "vim-nm-i-prefix", vim: true, initial: [""], mode: "insert",
    keys: ["\\an", "<Esc>", "i"], expect: P("\\an"),
    note: "`Esc` + `i` 回 insert：**不许**在回到 insert 的瞬间补刀" },
  { id: "vim-nm-a-prefix", vim: true, initial: [""], mode: "insert",
    keys: ["\\an", "<Esc>", "a"], expect: P("\\an"),
    note: "`Esc` + `a` 回 insert（追加位）" },

  // ── 3. 缩进 / 换行 ─────────────────────────────────────────────────────
  { id: "vim-o-indent", vim: true, initial: ["theorem t : Prop := by", "  trivial"], mode: "normal",
    keys: ["<Esc>", "o", "exact h", "<Esc>"], expect: P("theorem t : Prop := by", "  trivial", "  exact h"),
    note: "`o` 的自动缩进：新行是否与上一行同缩进、且没被改写器吞掉/加倍" },
  { id: "vim-O-indent", vim: true, initial: ["theorem t : Prop := by", "  trivial"], mode: "normal",
    keys: ["<Esc>", "j", "O", "exact h", "<Esc>"], expect: P("theorem t : Prop := by", "  exact h", "  trivial"),
    note: "`O` 的自动缩进（向上开行）" },
  { id: "vim-enter-indent", vim: true, initial: ["theorem t : Prop := by", "  trivial"], mode: "normal",
    keys: ["<Esc>", "A", "<CR>", "exact h", "<Esc>"],
    expect: P("theorem t : Prop := by", "  trivial", "  exact h"),
    note: "insert 模式回车后的自动缩进（`A` 到行尾再回车）" },
  { id: "ctl-enter-indent", vim: false, initial: ["theorem t : Prop := by", "  trivial"],
    keys: ["<Esc>", "A", "<CR>", "exact h", "<Esc>"],
    expect: P("theorem t : Prop := by", "  trivialA", "exact h"),
    note: "**对照臂**：同一串键在无 vim 下 —— `A` 只是插入字母 A（没有模式），回车后也没有 vim 那份自动缩进" },

  // ── 4. 撤销 / 存盘 / 一个 undo 单元 ─────────────────────────────────────
  { id: "vim-undo-one-step", vim: true, initial: [""], mode: "insert",
    keys: ["\\in", "<Tab>", "<Esc>", "u"], expect: P(""),
    note: "一次替换 = 一个 undo 单元：`u` 一次回到 `\\in`（不是回到 `\\i`）" },
  { id: "vim-colon-w", vim: true, initial: [""], mode: "insert",
    keys: ["\\alpha ", "<Esc>", ":w", "<CR>"], expect: P("α "),
    note: "`:w` 存盘（命令行走真键位）" },
  { id: "vim-undo-after-o", vim: true, initial: [""], mode: "insert",
    keys: ["\\alpha", "<Esc>", "o", "<Esc>", "u"], expect: P("α"),
    note: "`o` 之后一次 `u` 只该撤掉 `o` 那一刀" },

  // ── 5. dd / gg / G ─────────────────────────────────────────────────────
  { id: "vim-dd", vim: true, initial: ["aaa", "bbb", "ccc"], mode: "normal",
    keys: ["<Esc>", "gg", "j", "dd"], expect: P("aaa", "ccc"),
    note: "`dd` 删一行（normal 模式删除不该触发改写器）" },
  { id: "vim-gg-G", vim: true, initial: ["aaa", "\\an", "ccc"], mode: "normal",
    keys: ["<Esc>", "G", "gg"], expect: P("aaa", "\\an", "ccc"),
    note: "`gg`/`G` 纯光标移动：`\\an` 必须原样留着" },
  { id: "vim-dollar-append", vim: true, initial: ["abc"], mode: "normal",
    keys: ["<Esc>", "$", "a", "X", "<Esc>"], expect: P("abcX"),
    note: "`$` + `a` 追加" },

  // ── 6. 补全 / Tab 冲突 ─────────────────────────────────────────────────
  { id: "vim-tab-completion", vim: true, initial: ["", ""], mode: "insert",
    keys: ["theo", "<Tab>"], expect: null, // 只记录，不判分（见 note）
    note: "Tab 在补全弹窗与记法键位之间的归属——只取证不判分（`expect: null`）" },
  { id: "vim-tab-plain-indent", vim: true, initial: ["x", ""], mode: "normal",
    keys: ["<Esc>", "j", "i", "<Tab>"], expect: P("x", "\t"), domCheck: false,
    note: "对照臂：空行上 Tab 照旧是缩进（扩展不许吞 Tab）" },

  // ── 6.5 **project 模式**（用户 2026-10-11 追问：是不是 project 模式有奇葩问题）──
  //     同一串键在「裸文件」与「project 单元」里各跑一次 = 干净的 A/B。
  { id: "proj-nota-space", vim: true, initial: [""], file: "Scratch.sokonanoda", mode: "insert",
    keys: ["\\alpha ", "\\in ", "\\sub "], expect: P("α ∈ ⊆ "),
    note: "A/B 的 A：**裸文件**（不在任何模块根下）里的记法输入" },
  { id: "proj-unit-nota-space", vim: true, initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda", mode: "insert",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    keys: ["<Esc>", "G", "o", "\\alpha \\in \\sub ", "<Esc>"], expect: null,
    note: "A/B 的 B：**project 单元**（有 sokonanoda.toml + import lib.Set）里同一串键" },

  { id: "diag-proof-line1", vim: true, mode: "insert", initial: [""],
    keys: "theorem subset_of_mem_singleton (\\alpha : Type) :".split(""), expect: null,
    note: "**找第一处错乱**：用户证明第一行逐字符（DOM 每步一记）" },

  // ── 7. 用户那段证明（修前证据）─────────────────────────────────────────
  { id: "proof-in-unit-vim", vim: true, proof: "unit", mode: "insert",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    file: "units/I.1/unit01-sets-membership.sokonanoda", keys: null, expect: null,
    note: "**用户原话那条**：打开 set-theory 教材（project 模式）做题目——在单元末尾把那段证明打一遍" },
  { id: "proof-full-vim", vim: true, initial: [""], proof: true, mode: "insert",
    keys: null, expect: null,
    note: "用户那段 set-theory 证明的完整输入过程（DOM + 存盘字节，只取证）" },
  { id: "proof-full-novim", vim: false, initial: [""], proof: true,
    keys: null, expect: null,
    note: "同一段证明在无 vim 下的输入过程（对照）" },
];

// 用户那段证明：逐字符打（`\n` = Enter）。缩进**显式**打出来（两条臂同序列）。
const PROOF = [
  "theorem subset_of_mem_singleton (\\alpha : Type) (A : Set \\alpha) (a : \\alpha) (h : a \\in A) :",
  "    {a} \\sub A := by",
  "  intro x hx",
  "  unfold Set.singleton Set.mem at hx",
  "  apply Eq.subst \\alpha (fun (y : \\alpha) => y \\in A) a x",
  "  apply Eq.symm \\alpha x a hx",
  "  exact h",
];
for (const c of CASES) {
  if (c.proof === "unit") c.keys = ["<Esc>", "G", "o", PROOF.join("\n"), "<Esc>"];
  else if (c.proof) c.keys = [PROOF.join("\n")];
}

// ── CDP 客户端（零依赖：Node 自带 WebSocket）───────────────────────────────
let debugPort = args.port;

async function cdpTargets() {
  return (await fetch(`http://127.0.0.1:${debugPort}/json/list`)).json();
}

const CODE = { "\\": "Backslash", " ": "Space", "\n": "Enter", "{": "BracketLeft",
  "}": "BracketRight", "(": "Digit9", ")": "Digit0", ":": "Semicolon",
  ".": "Period", "_": "Minus", "'": "Quote", ">": "Period", "<": "Comma",
  "=": "Equal", "|": "Backslash", ",": "Comma", "-": "Minus", "/": "Slash" };
const VK = { "\\": 220, " ": 32, "\n": 13, "{": 219, "}": 221, "(": 57, ")": 48,
  ":": 186, ".": 190, "_": 189, "'": 222, "<": 188, ">": 190, "=": 187, "|": 220,
  ",": 188, "-": 189, "/": 191 };
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
    if (p) { pending.delete(m.id); p(m); }
  });
  const call = (method, params = {}, timeoutMs = 20_000) =>
    new Promise((res, rej) => {
      const n = ++id;
      const timer = setTimeout(() => { pending.delete(n); rej(new Error(`CDP ${method} 超时`)); }, timeoutMs);
      pending.set(n, (m) => { clearTimeout(timer); res(m); });
      ws.send(JSON.stringify({ id: n, method, params }));
    });
  const key = async (key_, code, vk, modifiers = 0) => {
    const base = { key: key_, code, modifiers, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk };
    await call("Input.dispatchKeyEvent", { type: "rawKeyDown", ...base });
    await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
  };
  return {
    close: () => ws.close(),
    call,
    async eval(expression) {
      const m = await call("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
      if (m.result?.exceptionDetails) throw new Error(JSON.stringify(m.result.exceptionDetails).slice(0, 300));
      return m.result?.result?.value;
    },
    key,
    async char(ch) {
      const shift = SHIFTED.has(ch) ? 8 : 0;
      const base = { key: ch, code: codeOf(ch), windowsVirtualKeyCode: vkOf(ch),
        nativeVirtualKeyCode: vkOf(ch), modifiers: shift };
      await call("Input.dispatchKeyEvent", { type: "keyDown", text: ch, unmodifiedText: ch, ...base });
      await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
    },
    /// 真正的 Enter：`key` 必须是 "Enter"（Monaco 的 keyCode 判定），
    /// **同时**带 `text: "\r"`（插入换行靠它）。少任一个都插不出换行
    /// （实测：只发 rawKeyDown ⇒ `A`+Enter+`exact h` 变成 `trivialexact h`）。
    async enter() {
      const base = { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 };
      await call("Input.dispatchKeyEvent", { type: "keyDown", text: "\r", unmodifiedText: "\r", ...base });
      await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
    },
    async chord(letter, shift = false) {
      await key(letter, `Key${letter.toUpperCase()}`, letter.toUpperCase().charCodeAt(0), MOD | (shift ? 8 : 0));
    },
    async clickEditor() {
      const rect = JSON.parse(await this.eval(
        `(() => { const r = document.querySelector('.monaco-editor')?.getBoundingClientRect();
           return JSON.stringify(r ? {x:r.x,y:r.y,w:r.width,h:r.height} : null); })()`));
      if (!rect) throw new Error("找不到编辑器区域");
      const x = Math.round(rect.x + rect.w * 0.55), y = Math.round(rect.y + rect.h * 0.35);
      for (const type of ["mousePressed", "mouseReleased"])
        await call("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1, pointerType: "mouse" });
    },
  };
}

// ── 具名键 ──────────────────────────────────────────────────────────────────
const NAMED = {
  "<Esc>": ["Escape", "Escape", 27, 0], "<Tab>": ["Tab", "Tab", 9, 0],
  "<CR>": ["\r", "Enter", 13, 0], "<BS>": ["Backspace", "Backspace", 8, 0],
  "<Del>": ["Delete", "Delete", 46, 0], "<Space>": [" ", "Space", 32, 0],
  "<Up>": ["ArrowUp", "ArrowUp", 38, 0], "<Down>": ["ArrowDown", "ArrowDown", 40, 0],
  "<Left>": ["ArrowLeft", "ArrowLeft", 37, 0], "<Right>": ["ArrowRight", "ArrowRight", 39, 0],
  "<End>": ["End", "End", 35, 0],
};
const CHORDED = { "<Cmd+Down>": ["ArrowDown", "ArrowDown", 40, MOD] };
const CTRL = { "<C-a>": "a", "<C-s>": "s", "<C-z>": "z", "<C-p>": "p", "<C-w>": "w", "<C-r>": "r" };

/// 一个 token → 真按键。返回人类可读的"这一步按了什么"。
async function press(client, token) {
  if (token === "<CR>") { await client.enter(); return token; }
  if (NAMED[token]) { await client.key(...NAMED[token]); return token; }
  if (CHORDED[token]) { await client.key(...CHORDED[token]); return token; }
  if (CTRL[token]) { await client.chord(CTRL[token]); return token; }
  for (const ch of token) {
    // ⚠ Enter 必须带 `text: "\r"`：只发 rawKeyDown 的话 Monaco 不会插入换行
    // （实测：`A` + Enter + `exact h` 变成 `trivialexact h` —— 那是探针的锅，不是产品）。
    if (ch === "\n") await client.enter();
    else await client.char(ch);
    await sleep(args.charDelay);
  }
  return token;
}

// ── 编辑器 DOM：用户屏幕上那些行 ────────────────────────────────────────────
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

function cleanDom(raw) {
  const lines = JSON.parse(raw).map((s) => s.replace(/\u00a0/g, " ").replace(/\u200b/g, ""));
  while (lines.length > 1 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}
const norm = (l) => l.replace(/\u00a0/g, " ").replace(/\r\n/g, "\n");
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

async function waitForEditor(client, deadline) {
  while (Date.now() < deadline) {
    try {
      const raw = await client.eval(READ_LINES);
      if (raw && raw !== "[]" && raw !== "null") return cleanDom(raw);
    } catch { /* 页面还在换 */ }
    await sleep(400);
  }
  throw new Error("编辑器没有渲染出任何行");
}

async function openFile(client, name) {
  await client.chord("p");
  await sleep(500);
  for (const ch of name) { await client.char(ch); await sleep(25); }
  await sleep(800);
  await client.key("Enter", "Enter", 13);
  await sleep(900);
  const tab = await client.eval(ACTIVE_TAB);
  if (!String(tab ?? "").includes(name)) throw new Error(`快速打开没能切到 ${name}（当前：${tab}）`);
}

/// `File: Revert File` —— 走真命令面板，把缓冲区拉回磁盘（每个 case 的干净起点）。
async function revertFile(client) {
  await client.chord("p", true); // Cmd+Shift+P
  await sleep(600);
  for (const ch of "revert") { await client.char(ch); await sleep(35); }
  await sleep(700);
  await client.key("Enter", "Enter", 13);
  await sleep(900);
}

// ── 工作区 / profile / 启动 ─────────────────────────────────────────────────
function settingsFor(testCase, eager) {
  return {
    "workbench.startupEditor": "none", "workbench.tips.enabled": false,
    "workbench.colorTheme": "Default Dark Modern", "window.commandCenter": false,
    "breadcrumbs.enabled": false, "editor.minimap.enabled": false,
    "editor.stickyScroll.enabled": false, "editor.wordWrap": "off",
    "editor.renderWhitespace": "none", "editor.detectIndentation": false,
    "editor.insertSpaces": false, "editor.tabSize": 4,
    "editor.autoIndent": "full", "editor.autoClosingBrackets": "languageDefined",
    "git.openRepositoryInParentFolders": "never", "update.showReleaseNotes": false,
    "extensions.autoCheckUpdates": false, "extensions.autoUpdate": false,
    "telemetry.telemetryLevel": "off", "security.workspace.trust.enabled": false,
    ...(args.suggestOff ? { "editor.suggestOnTriggerCharacters": false,
      "editor.quickSuggestions": false, "editor.tabCompletion": "off",
      "editor.acceptSuggestionOnEnter": "off" } : {}),
    ...(eager === undefined ? {} : { "sokonanoda.input.eager": eager }),
  };
}

function prepare(tag, arm, eager) {
  const profile = join(RUN, `profile-${tag}`);
  rmSync(profile, { recursive: true, force: true });
  mkdirSync(join(profile, "User"), { recursive: true });
  mkdirSync(join(profile, "extensions"), { recursive: true });
  writeFileSync(join(profile, "User", "settings.json"),
    JSON.stringify(settingsFor(null, eager), null, 2));
  if (arm === "on") {
    if (!existsSync(join(VIM_SRC, "package.json"))) {
      console.error(`error: 找不到 vim 扩展：${VIM_SRC}`);
      process.exit(3);
    }
    cpSync(VIM_SRC, join(profile, "extensions", "vscodevim.vim-1.32.4"), { recursive: true });
  }
  mkdirSync(WS, { recursive: true });
  return profile;
}

/// 工作区自举：从 `courses/set-theory/` 镜像模块根（`sokonanoda.toml` + `lib/` + `units/`）。
/// **为什么**：工作区原先靠上一轮会话手工复制 ⇒ clone 之后探针跑不起来；探针必须自足。
function ensureWorkspace() {
  mkdirSync(WS, { recursive: true });
  if (existsSync(join(WS, "sokonanoda.toml"))) return;
  for (const rel of ["sokonanoda.toml", "lib", "units"]) {
    cpSync(join(REPO, "courses", "set-theory", rel), join(WS, rel), { recursive: true });
  }
  log(`工作区自举：${WS}（从 courses/set-theory/ 镜像）`);
}

function launch(profile, file) {
  // stderr 落盘（诊断用）：窗口起不来时唯一能看的东西。
  const errLog = join(profile, "vscode-stderr.log");
  const fd = openSync(errLog, "a");
  const child = spawn(vscodeBin(), [
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
    "--disable-extensions",
    "--disable-gpu", "--disable-chromium-sandbox", "--no-first-run",
    "--disable-workspace-trust",
    "--disable-extension", "GitHub.copilot-chat",
    "--disable-extension", "GitHub.copilot",
    "--goto", `${join(WS, file)}:1:1`, WS,
  ], { detached: true, stdio: ["ignore", "ignore", fd] });
  child.unref();
  return errLog;
}

/// 端口卫生（**这是判据能不能信的前提**）：残留窗口会占着 CDP 端口 ⇒ 新窗口
/// 绑不上 ⇒ 脚本连到**上一组那个窗口**，于是"换了 profile/工作区"根本没生效，
/// 读数全错（`scripts/vscode-input-e2e.mjs` 的注释里记过同款事故）。
function freePort(port) {
  for (const pat of [`remote-debugging-port=${port}`, `user-data-dir=${join(RUN, `profile-${port}`)}`]) {
    try { execFileSync("pkill", ["-f", pat]); } catch { /* 本来就没有 */ }
  }
}

/// 等到端口真的空出来（最多 20s）。返回 false = 还有人在答 ⇒ 调用方必须**停**，
/// 不许硬跑（连到旧窗口的读数比没有读数更坏）。
async function portFree(port, deadline) {
  while (Date.now() < deadline) {
    let busy = false;
    try {
      const r = execFileSync("lsof", ["-nP", `-iTCP:${port}`, "-sTCP:LISTEN"], { encoding: "utf8" });
      busy = r.trim().split("\n").length > 1;
    } catch { busy = false; }
    if (!busy) return true;
    await sleep(500);
  }
  return false;
}

async function quit(profile) {
  try { execFileSync("pkill", ["-f", `user-data-dir=${profile}`]); } catch { /* 已经不在了 */ }
  for (let i = 0; i < 30; i++) {
    try { await cdpTargets(); } catch { return; }
    await sleep(500);
  }
}

async function workbenchPage(deadline) {
  while (Date.now() < deadline) {
    await sleep(700);
    try {
      const page = (await cdpTargets()).find((t) => t.type === "page" && t.url.includes("workbench"));
      if (page) return await connect(page.webSocketDebuggerUrl);
    } catch { /* 还没起来 */ }
  }
  throw new Error("VS Code 没有暴露 CDP");
}

// ── 一条用例 ────────────────────────────────────────────────────────────────
async function runCase(client, testCase, arm) {
  const startedAt = Date.now();
  const path = join(WS, testCase.file ?? args.file);
  const initial = testCase.initialFrom
    ? readFileSync(join(REPO, testCase.initialFrom), "utf8").replace(/\n$/, "").split("\n")
    : testCase.initial;
  writeFileSync(path, initial.join("\n"), "utf8");
  await openFile(client, (testCase.file ?? args.file).split("/").pop());
  await client.clickEditor();
  await sleep(300);
  await revertFile(client);

  const steps = [];
  const record = async (label) => {
    if (!args.stepDom) return;
    const dom = cleanDom(await client.eval(READ_LINES));
    const mode = await client.eval(READ_MODE);
    steps.push({ step: label, dom, mode });
  };
  // **模式卫生**：vim 模式会跨 case 残留（实测：上一个 case 停在 INSERT，下一个
  // 就在 INSERT 里跑 ⇒ 结果依赖顺序，不可复现）。所以每个 case 显式把模式摆好。
  //   mode: "insert" ⇒ 先 <Esc> 再 i；mode: "normal" ⇒ 先 <Esc>；
  //   mode: "fresh"  ⇒ 不摆（用刚开窗的模式 = vim 的 NORMAL / 无 vim 的"永远 insert"）
  const setupKeys = arm === "on" && testCase.mode !== "fresh"
    ? (testCase.mode === "normal" ? ["<Esc>"] : ["<Esc>", "i"]) : [];
  for (const token of setupKeys) { await press(client, token); await sleep(200); }
  await record("(摆好模式)");

  for (const token of testCase.keys) {
    await press(client, token);
    await sleep(token.startsWith("<") ? 260 : 160);
    await record(token);
  }
  await sleep(400);

  const dom = cleanDom(await client.eval(READ_LINES));
  const mode = await client.eval(READ_MODE);
  await client.chord("s");
  await sleep(800);
  const disk = norm(readFileSync(path, "utf8"));
  const diskLines = disk.split("\n");
  while (diskLines.length > 1 && diskLines[diskLines.length - 1] === "") diskLines.pop();

  let pass = null, verdict = "recorded";
  if (testCase.expect !== null) {
    const domOk = testCase.domCheck === false ? true : same(dom, testCase.expect);
    const diskOk = same(diskLines, testCase.expect);
    pass = domOk && diskOk;
    verdict = pass ? "match" : `dom=${domOk ? "ok" : "MISMATCH"} disk=${diskOk ? "ok" : "MISMATCH"}`;
  }
  return { id: testCase.id, arm, ms: Date.now() - startedAt, note: testCase.note,
    keys: testCase.keys, typed: testCase.proof ? "PROOF" : testCase.keys.join(" · "),
    steps, setup_keys: setupKeys, dom_lines: dom, disk_lines: diskLines,
    disk_bytes: readFileSync(path).length, file: testCase.file ?? args.file,
    mode_final: mode, expect: testCase.expect, verdict, pass };
}

async function runArm(cases, arm, tag, port) {
  debugPort = port;
  const eager = cases[0]?.eager; // 一组 = 一次启动（改设置要重启窗口）
  const profile = prepare(tag, arm, eager);
  freePort(port);
  if (!(await portFree(port, Date.now() + 20_000))) {
    throw new Error(`端口 ${port} 被占着（残留窗口没退干净）——**不跑**，否则读数会归属到旧窗口`);
  }
  launch(profile, args.file);
  log(`真 VS Code 起来了（arm=${arm} · eager=${eager === undefined ? "默认（清单）" : eager} · port=${port}）`);
  const out = [];
  let client = null;
  try {
    client = await workbenchPage(Date.now() + args.timeout);
    await waitForEditor(client, Date.now() + args.timeout);
    for (const c of cases) {
      const r = await runCase(client, c, arm);
      out.push(r);
      log(`${r.pass === false ? "FAIL" : r.pass ? "PASS" : "REC "} ${r.id} [${arm}] (${(r.ms / 1000).toFixed(1)}s): ` +
        `DOM「${JSON.stringify(r.dom_lines)}」/ 磁盘「${JSON.stringify(r.disk_lines)}」` +
        (r.expect ? ` / 期望「${JSON.stringify(r.expect)}」` : ""));
    }
  } finally {
    try { client?.close(); } catch { /* ignore */ }
    if (!args.keep) await quit(profile);
  }
  return out;
}

// ── 主流程 ──────────────────────────────────────────────────────────────────
if (!existsSync(join(EXTENSION, "package.json"))) {
  console.error(`error: 扩展目录不完整：${EXTENSION}`); process.exit(3);
}
ensureWorkspace();
pruneStaleProfiles();  // 陈旧 profile 回收（SUB-probe-vscode-isolation）✓
const identity = buildIdentity();
if (args.list) {
  for (const c of CASES) console.log(`${c.vim ? "vim " : "novim"} ${c.id.padEnd(24)} ${c.note}`);
  process.exit(0);
}

const wanted = args.case ? args.case.split(",") : args.pair ? [args.pair] : null;
const selected = wanted ? CASES.filter((c) => wanted.includes(c.id)) : args.all ? CASES : [];
if (selected.length === 0) {
  console.error("error: 用 --case <id> / --pair <id> / --all（--list 看清单）"); process.exit(2);
}
for (const id of wanted ?? []) if (!CASES.some((c) => c.id === id)) {
  console.error(`error: --case ${id} 不在表里`); process.exit(2);
}
if (args.arm && !["on", "off"].includes(args.arm)) { console.error("error: --arm 只能是 on/off"); process.exit(2); }

const results = [];
let failure = null;
try {
  if (args.pair) {
    // **对照臂判据**：同一按键序列 —— 无 vim 必须绿、有 vim 必须红。
    const c = selected[0];
    results.push(...(await runArm([c], "off", `${args.port}-pair-off`, args.port)));
    results.push(...(await runArm([c], "on", `${args.port}-pair-on`, args.port + 1)));
  } else {
    // 按 (arm, eager) 分组：一次启动跑完同组（启动一次 ~15s，别浪费）
    const groups = new Map();
    for (const c of selected) {
      const arm = args.arm ?? (c.vim ? "on" : "off");
      const key = `${arm}|${c.eager === undefined ? "default" : c.eager}`;
      if (!groups.has(key)) groups.set(key, { arm, cases: [] });
      groups.get(key).cases.push(c);
    }
    let i = 0;
    for (const [key, g] of groups) {
      results.push(...(await runArm(g.cases, g.arm, `${args.port}-${i}-${key.replace(/[^a-z0-9]/gi, "")}`, args.port + i)));
      i += 1;
    }
  }
} catch (e) {
  failure = String(e?.message ?? e);
  log(`错误：${failure}`);
}

const graded = results.filter((r) => r.pass !== null);
const failed = graded.filter((r) => !r.pass);
let pairOk = null;
if (args.pair && !failure && graded.length === 2) {
  const off = results.find((r) => r.arm === "off");
  const on = results.find((r) => r.arm === "on");
  pairOk = off?.pass === true && on?.pass === false;
}
if (args.json) {
  console.log(JSON.stringify({ schema: "soko.vim-probe/1", identity, arm: args.arm ?? null,
    case: args.case ?? args.pair ?? null, pair_ok: pairOk, results, failed: failed.length,
    error: failure }, null, 2));
} else {
  log(`构建身份：HEAD ${identity.head} · extension.js ${identity.extension_js_bytes}B @ ${identity.extension_js_mtime}`);
  if (pairOk !== null) log(pairOk ? "对照臂成立：无 vim 绿 + 有 vim 红" : "对照臂**不成立**");
}
if (failure) process.exit(1);
if (args.pair) process.exit(pairOk ? 0 : 1);
process.exit(failed.length > 0 ? 1 : 0);
