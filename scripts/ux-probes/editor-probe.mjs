#!/usr/bin/env node
// scripts/ux-probes/editor-probe.mjs —— **真 VS Code + 真按键 + 真判卷**：在编辑器里做
// set-theory unit01 的全流程体验探针（目标视图 / 洞与导航 / 补全 / 诊断 / 命令 /
// 性能 / vim 新模式）。
//
// 管道抄 `scripts/ux-probes/vim-probe.mjs`（CDP 驱动真 VS Code、逐字符真 keydown、读
// `.view-line` DOM、读存盘字节）——**不改那个文件**。新增的读法：
//   * Infoview（secondary sidebar 的 webview）⇒ CDP `Page.createIsolatedWorld`
//     进那个 iframe 的 JS 上下文读 DOM（跨源 iframe 只能这样读）
//   * 诊断 ⇒ `.view-line` 里的 `squiggly-*` span + Problems 面板（真文案）
//   * 补全 ⇒ `.suggest-widget` / `.suggest-details`；quick fix ⇒ `.context-view`
//   * 判卷真相 ⇒ 直接 exec `scripts/soko query check|holes|goals`
//     （`SOKONANODA_BIN` 钉在扩展自带的 CLI 上 ⇒ 与编辑器同一个内核版本）
//
// 用法：
//   node scripts/ux-probes/editor-probe.mjs --list
//   node scripts/ux-probes/editor-probe.mjs --case <id> [--json] [--keep]
//   node scripts/ux-probes/editor-probe.mjs --all --json
//   node scripts/ux-probes/editor-probe.mjs --case <id> --arm on     # 装 vim 扩展
//
// 退出码：0 = 该 case 与声明的期望一致（`expect: null` 的取证 case 记 0）
//         1 = 不一致 · 2 = 用法错误 · 3 = 前置缺失
//
// 纪律：独立 CDP 端口（默认 9601）、独立 profile（`$TMPDIR`，路径 ≤103 字符）、
// 独立工作区 `.cache/ux/editor/ws/`；跑完 `pkill -f user-data-dir=<profile>`。

import { execFileSync, spawn } from "node:child_process";
import {
  cpSync, existsSync, mkdirSync, openSync, readFileSync, rmSync, writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url)); // <repo>/scripts/ux-probes
const REPO = resolve(HERE, "..", "..");               // <repo>
const EXTENSION = join(REPO, "editor/vscode");
// 工作区/截图落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏（同旧版 `.cache/ux/editor/`）
const STATE = join(REPO, ".cache", "ux", "editor");
const WS = join(STATE, "ws");
const RUN = join(tmpdir(), "soko-editor-probe"); // ≤103 字符（见 vim probe §7 坑 1）
const VIM_SRC = join(homedir(), ".vscode", "extensions", "vscodevim.vim-1.32.4");
const BUNDLED_CLI = join(EXTENSION, "bin", "darwin-arm64", "sokonanoda");
const IS_MAC = process.platform === "darwin";
const MOD = IS_MAC ? 4 : 2; // CDP modifiers：Alt=1 Ctrl=2 Meta=4 Shift=8
const ALT = 1;
const log = (...a) => console.error("[editor-probe]", ...a); // stderr ⇒ stdout 只留 --json
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── 参数 ────────────────────────────────────────────────────────────────────
const args = { port: 9601, case: null, all: false, list: false, json: false,
  keep: false, arm: null, timeout: 240_000, file: "Scratch.sokonanoda",
  charDelay: 45, verbose: false, focusEmu: true, bring: false };
for (let i = 2; i < process.argv.length; i++) {
  const k = process.argv[i];
  if (k === "--port") args.port = Number(process.argv[++i]);
  else if (k === "--case") args.case = (args.case ? args.case + "," : "") + process.argv[++i];
  else if (k === "--all") args.all = true;
  else if (k === "--list") args.list = true;
  else if (k === "--json") args.json = true;
  else if (k === "--keep") args.keep = true;
  else if (k === "--arm") args.arm = process.argv[++i];
  else if (k === "--file") args.file = process.argv[++i];
  else if (k === "--timeout") args.timeout = Number(process.argv[++i]);
  else if (k === "--char-delay") args.charDelay = Number(process.argv[++i]);
  else if (k === "--verbose") args.verbose = true;
  else if (k === "--no-focus-emu") args.focusEmu = false;
  else if (k === "--bring") args.bring = true;
  else { console.error(`error: 未知参数 ${k}`); process.exit(2); }
}

function vscodeBin() {
  for (const c of [process.env.SOKO_VSCODE_BIN,
    "/Applications/Visual Studio Code.app/Contents/MacOS/Code", "/usr/share/code/code"])
    if (c && existsSync(c)) return c;
  console.error("error: 找不到 VS Code（设 SOKO_VSCODE_BIN）"); process.exit(3);
}

// ── 构建身份（每条读数都要能归属到一份构建）─────────────────────────────────
function buildIdentity() {
  const git = (a) => { try { return execFileSync("git", ["-C", REPO, ...a], { encoding: "utf8" }).trim(); } catch { return "unknown"; } };
  const ext = join(EXTENSION, "extension.js");
  const lsp = join(EXTENSION, "bin", "darwin-arm64", "sokonanoda-lsp");
  const cli = BUNDLED_CLI;
  const info = (p) => {
    if (!existsSync(p)) return { path: p, missing: true };
    const buf = readFileSync(p);
    return { path: p, bytes: buf.length, sha256_16: createHash("sha256").update(buf).digest("hex").slice(0, 16),
      mtime: execFileSync("stat", ["-f", "%Sm", p], { encoding: "utf8" }).trim() };
  };
  let cliVersion = "unknown";
  try { cliVersion = execFileSync(cli, ["--version"], { encoding: "utf8" }).trim(); } catch {}
  return { head: git(["rev-parse", "--short", "HEAD"]),
    head_dirty: git(["status", "--short", "--", "crates", "editor", "courses", "prelude"]) || "",
    extension_js: info(ext), bundled_lsp: info(lsp), bundled_cli: { ...info(cli), version: cliVersion },
    repo: REPO };
}

// ── 判卷真相（与编辑器同一个内核版本：钉 bundled CLI）───────────────────────
function sokoQuery(op, file, extra = []) {
  const p = resolve(WS, file); // 相对路径按**探针工作区**解，不是仓库根
  try {
    const out = execFileSync(join(REPO, "scripts", "soko"),
      ["query", op, "--file", p, ...extra],
      { encoding: "utf8", env: { ...process.env, SOKONANODA_BIN: BUNDLED_CLI }, timeout: 240_000 });
    return JSON.parse(out);
  } catch (e) {
    const out = String(e.stdout ?? "");
    try { return { ...JSON.parse(out), _exit: e.status }; } catch { return { ok: false, _error: String(e.message).slice(0, 200), _exit: e.status }; }
  }
}
function gradeAll(file) {
  const check = sokoQuery("check", file);
  const holes = sokoQuery("holes", file);
  const goals = sokoQuery("goals", file);
  const c = check?.data?.counts ?? {};
  return { counts: c, failed: check?.data?.failed ?? null, warnings: check?.data?.warnings ?? null,
    holes: (holes?.data?.holes ?? []).map((h) => ({ id: h.id, decl: h.decl, redundant: h.redundant, ty: h.ty, start: h.start })),
    decls: (goals?.data ?? []).map((d) => ({ name: d.name, status: d.status, kind: d.kind,
      ty: d.ty, goals: (d.goals ?? []).map((g) => g.goal ?? g), holes: (d.holes ?? []).length })),
    raw_check_ok: check?.ok, exit: check?._exit ?? null };
}

// ── 用例表 ──────────────────────────────────────────────────────────────────
// step 语言：普通字符串 = 逐字符真 keydown（`\n` = Enter）；
//   `<Esc>` `<Tab>` `<CR>` `<BS>` `<Del>` `<Left/Right/Up/Down>` `<End>` `<Home>`
//   `<M-n>` alt+n · `<M-S-n>` alt+shift+n · `<M-s>` alt+s · `<M-b>` alt+b
//   `<M-S-b>` alt+shift+b · `<M-.>` quick fix · `<C-S-p>` 命令面板 · `<C-S-m>` Problems
//   `<C-S-g>` 跳行 · `<F12>` 定义 · `<C- >`（空格）= 触发补全
//   `@snap:<label>` 记一次快照 · `@wait:<ms>` · `@cmd:<标题>` 命令面板跑命令
//   `@revert` 把缓冲区拉回磁盘 · `@grade` 跑真判卷 · `@problems` 打开 Problems
//   `@infoview` 聚焦 Infoview · `@palette:<标题>` = @cmd
//   `@click:<selector>` 点元素 · `@slow:<文本>` 逐字符 150ms（量诊断延迟用）
// probes（每条 case 末尾多读的几项）：status / problems / infoview / suggest /
//   quickfix / notifications / markers / lncol / hover
const CASES = [
  // ═══ A. 目标 / 信息视图 ═══════════════════════════════════════════════════
  { id: "goal-on-sorry", note: "**洞上的目标视图**：打开 unit01、光标停在练习5 的 `sorry` 上 ⇒ Infoview 该显示什么（记法/隐式实参/⊢ 行）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@infoview", "@snap:infoview@open", "@click:.monaco-editor", "<ctrl+g>", "92", "<CR>", "@wait:2500", "@snap:on-sorry", "@shot:on-sorry", "@grade"],
    probes: ["infoview", "status", "markers", "lncol"] },

  { id: "goal-after-apply", note: "**`apply` 之后的目标与假设**：练习5 里写 `apply` 到一半 ⇒ 目标被换成什么、假设列有没有 h",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@infoview", "@click:.monaco-editor", "<ctrl+g>", "1", "<CR>", "@snap:at-top",
      "<alt+n>", "@wait:900", "@snap:hole-selected", "apply h", "@wait:3000", "@snap:after-apply", "@shot:after-apply", "@grade"],
    probes: ["infoview", "status", "markers"] },

  { id: "goal-after-intro", note: "**`intro` 之后**：练习6 里 `intro x` ⇒ 假设列表里出现 x（隐式实参怎么显示、有没有括号）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@infoview", "@click:.monaco-editor", "<ctrl+g>", "1", "<CR>", "<alt+n>", "<alt+n>", "@wait:900", "@snap:hole2",
      "intro x hx", "@wait:3000", "@snap:after-intro", "@shot:after-intro", "@grade"],
    probes: ["infoview", "status", "markers"] },

  { id: "goal-mid-proof", note: "练习4 已证完的 tactic 序列中段（`unfold` 之后）⇒ 目标/假设怎么渲染（对照 CLI query state）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@infoview", "@click:.monaco-editor", "<ctrl+g>", "82", "<CR>", "@wait:2500", "@snap:mid-proof", "@grade"],
    probes: ["infoview", "status", "markers"] },

  { id: "hints-visibility", note: "**soko:hint 在编辑器里怎么看**：有没有任何 UI 呈现 4 条 hint（命令 `sokonanoda.revealHint` 存在 ⇒ 手动跑一次）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "92", "<CR>", "@snap:before-hint",
      "@cmd:Sokonanoda: Reveal Hint", "@wait:1200", "@snap:after-hint-cmd"],
    probes: ["status", "notifications", "infoview", "markers"] },

  // ═══ B. 洞与导航 ═════════════════════════════════════════════════════════
  { id: "nexthole", note: "**alt+n** 从文件头出发：能不能跳到第一个洞、跳几次到第二个、洞上的呈现（`sorry` 高亮？）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "1", "<CR>", "@snap:at-line1",
      "<alt+n>", "@wait:800", "@snap:alt-n-1", "<alt+n>", "@wait:800", "@snap:alt-n-2", "<alt+n>", "@wait:800", "@snap:alt-n-3", "@shot:alt-n-3"],
    probes: ["status", "lncol", "markers", "notifications"],
    check: (r) => { const got = r.snaps.map((x) => x.cursor_line).join(">");
      const tail = got.split(">").slice(-4).join(">");
      return { ok: tail === "92>103>92>92", why: `alt+n 之后 cursor 轨迹 = ${tail}（期望 92>103>92>92 = 两洞 + 回到第一个洞）` }; } },

  { id: "prevhole", note: "**alt+shift+n**（previousHole）从文件尾往前跳：与 alt+n 对称吗",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "103", "<CR>", "@snap:at-end",
      "<alt+shift+n>", "@wait:800", "@snap:alt-shift-n-1", "<alt+shift+n>", "@wait:800", "@snap:alt-shift-n-2", "@shot:prevhole"],
    probes: ["status", "lncol", "markers", "notifications"] },

  { id: "hole-lightbulb", note: "**灯泡 / quick-fix**：`sorry` 上按 `<M-.>` ⇒ 有没有候选（能不能一键插入并过判卷）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "92", "<CR>", "@snap:on-sorry",
      "<cmd+.>", "@wait:1500", "@snap:quickfix-open", "@shot:quickfix"],
    probes: ["quickfix", "status", "markers", "notifications"] },

  { id: "hole-after-fill", note: "**洞填完的反馈**：把练习5 填成 `constructor` 之后 —— 有没有「这题过了」的可见信号（状态栏/Infoview/通知）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@infoview", "@click:.monaco-editor", "<ctrl+g>", "92", "<CR>", "<End>", "<BS>", "<BS>", "<BS>", "<BS>", "<BS>", "constructor", "@wait:3000", "@snap:after-constructor", "@shot:after-constructor", "@grade"],
    probes: ["infoview", "status", "markers", "notifications"] },

  { id: "all-sorry-filled", note: "**两个洞都填完**（练习5 `constructor` + 练习6 的骨架）后：状态栏 / Infoview 有没有「全过」信号",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@infoview", "@click:.monaco-editor", "<ctrl+g>", "92", "<CR>", "<End>", "<BS>", "<BS>", "<BS>", "<BS>", "<BS>", "constructor",
      "@wait:2500", "<ctrl+g>", "103", "<CR>", "<End>", "<BS>", "<BS>", "<BS>", "<BS>", "<BS>", "constructor", "@wait:3000", "@snap:both-filled", "@shot:both-filled", "@grade"],
    probes: ["infoview", "status", "markers", "notifications"] },

  // ═══ C. 补全 ═════════════════════════════════════════════════════════════
  { id: "completion-in-by", note: "**by 块里的补全候选**：`exact Set.` 后面弹出什么、有没有签名/文档、要写几个实参",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  exact Set."],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "<End>", "@wait:400", "@snap:typed",
      "<ctrl+space>", "@wait:1800", "@snap:suggest-open", "@shot:suggest"],
    probes: ["suggest", "status"] },

  { id: "completion-tactic", note: "**tactic 补全**：空 `by` 块里敲 `ap` ⇒ 候选里有 apply 吗、按 Tab 接受补全 vs Tab 记法键位",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  ap"],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "<End>", "@wait:1500", "@snap:suggest-ap", "@shot:suggest-ap",
      "<Tab>", "@wait:800", "@snap:after-tab", "@grade"],
    probes: ["suggest", "status"] },

  { id: "completion-doc", note: "补全项的**文档/签名**：选中一个候选后 `.suggest-details` 里有什么（用户抱怨「要写几个实参」无从知道）",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  exact Set.sub"],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "<End>", "@wait:400",
      "<ctrl+space>", "@wait:1800", "@snap:suggest-sub", "<Down>", "@wait:600", "@snap:selected"],
    probes: ["suggest", "status"] },

  { id: "notation-abbrev-tab", note: "**`\\` 记法缩写 + Tab**：`\\in` 之后 Tab 是「记法封口」还是「接受补全」（键位归属）",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A : Set α) (a : α) :", "    a ∈ A → a ∈ A := by", "  intro h"],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "7", "<CR>", "<End>", "\n  \\in", "@wait:700", "@snap:typed-in",
      "<Tab>", "@wait:900", "@snap:after-tab", "@grade"],
    probes: ["suggest", "status", "markers"] },

  // ═══ D. 诊断 ═════════════════════════════════════════════════════════════
  { id: "diag-typo-span", note: "**写错时的红波浪线**：`exact hh`（未定义名）⇒ span 指哪一列、文案、几条",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  exact hh"],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "@wait:2500", "@snap:diag", "@problems", "@wait:1200", "@snap:problems-panel", "@shot:problems", "@grade"],
    probes: ["markers", "problems", "status", "notifications"] },

  { id: "diag-timing-typing", note: "**逐字符打字到诊断出现的延迟**（真按键 + 毫秒）：在 `by` 块里逐字符打 `exact hh` 并轮询 DOM",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  "],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "@wait:1500", "@snap:before",
      "@timed:exact hh", "@snap:after-timed", "@grade"],
    probes: ["markers", "status"] },

  { id: "diag-implicit-arg", note: "**隐式实参报错**：只写 `exact Set.subset_def`（少写实参）⇒ 诊断怎么说、位置在哪、可读吗",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) :", "    (A ⊆ B) ↔ ∀ (x : α), A x → B x := by", "  exact Set.subset_def"],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "@wait:2500", "@snap:diag", "@problems", "@wait:1200", "@snap:panel", "@grade"],
    probes: ["markers", "problems", "status"] },

  { id: "diag-notation-fail", note: "**记法补不出类型参数**（课标手册第三条惯用法那条诊断）：写 `Set.powerset A ⊆ A` 之类 ⇒ 文案是否给出路",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A : Set α) :", "    Set.powerset A ⊆ A := by", "  intro x hx"],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "@wait:2500", "@snap:diag", "@problems", "@wait:1200", "@snap:panel", "@grade"],
    probes: ["markers", "problems", "status"] },

  { id: "diag-pp-explicit", note: "**`pp.explicit` 开/关的差别**：unit01 第 35 行有 `set_option pp.explicit true` ⇒ 目标视图里的 `@` 从哪一行开始变",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@infoview", "@click:.monaco-editor", "<ctrl+g>", "30", "<CR>", "@wait:2000", "@snap:before-pp",
      "<ctrl+g>", "38", "<CR>", "@wait:2000", "@snap:after-pp"],
    probes: ["infoview", "status"] },

  { id: "diag-duplicates", note: "**同一条错重复几条**：一个未定义名在 Problems 里出现几次（编辑器 vs 面板计数）",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  exact hh"],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "@wait:3000", "@problems", "@wait:1500", "@snap:panel"],
    probes: ["problems", "markers", "status"] },

  // ═══ E. 命令与流程 ═══════════════════════════════════════════════════════
  { id: "cmd-statusbar", note: "**alt+s**（Show Exercise Status）在洞上给什么：状态栏/通知里有没有「还差什么」",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "92", "<CR>", "@snap:before", "<alt+s>", "@wait:1500", "@snap:after-alt-s", "@shot:alt-s"],
    probes: ["status", "notifications", "infoview"] },

  { id: "cmd-build", note: "**alt+b（Build）**：从零开始第一次按下去 —— 多久、给什么反馈（进度/通知）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "@snap:before-build", "@timedkey:<alt+b>", "@wait:1000", "@snap:build-1s", "@wait:5000", "@snap:build-6s", "@wait:10000", "@snap:build-16s", "@shot:build"],
    probes: ["status", "notifications", "infoview"] },

  { id: "cmd-clean", note: "**Clean Cache** 的可见反馈：通知里三个数（全局/项目）在不在、清完之后编辑器还判卷吗",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "@cmd:Sokonanoda: Clean Cache", "@wait:1500", "@snap:after-clean",
      "@wait:3000", "@snap:after-clean-4s"],
    probes: ["notifications", "status", "markers"] },

  { id: "first-open-latency", note: "**第一次打开项目要等多久**：从窗口起来到 Infoview 有第一份目标（轮询时间戳）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@infoview", "@poll-infoview:60000", "@snap:first-goal"],
    probes: ["infoview", "status"] },

  { id: "course-map", note: "**课程地图 / 项目视图**：Explorer 里的「练习 / 课程 / 项目」三个视图有没有内容（引导「下一步做什么」）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:3000", "@snap:start", "@expand:练习", "@snap:goals-view", "@shot:goals-view",
      "@expand:课程", "@snap:course-view", "@expand:项目", "@snap:project-view", "@shot:project-view"],
    probes: ["status", "views", "notifications", "problems"] },

  // ═══ F. 性能与手感 ═══════════════════════════════════════════════════════
  { id: "perf-undo-redo", note: "**撤销/重做**：打 12 个字符后连撤 12 次再重做 12 次 —— 文本是否逐字符可逆、有无跳变",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A : Set α) (a : α) :", "    a ∈ A → a ∈ A := by", "  intro h"],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "7", "<CR>", "<End>", "\n  exact h", "@wait:1200", "@snap:typed",
      "<cmd+z>", "@wait:400", "@snap:undo1", "<cmd+z>", "@wait:400", "@snap:undo2", "<cmd+shift+z>", "@wait:400", "@snap:redo1", "@grade"],
    probes: ["status", "markers"] },

  { id: "perf-save-reload", note: "**保存**：`<C-s>` 之后磁盘字节与 DOM 一致（有没有「保存了但判卷看不到」）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "92", "<CR>", "<End>", "<BS>", "<BS>", "<BS>", "<BS>", "<BS>", "exact h", "@wait:1500", "<cmd+s>", "@wait:1500", "@snap:after-save", "@grade"],
    probes: ["status", "markers", "notifications"] },

  // ═══ G. vim 模式（只登记**新的**、与已知三条不同的现象）═══════════════════
  { id: "vim-infoview-mode", vim: true, mode: "fresh", note: "**vim 新现象候选**：vim 的 normal 模式里目标视图会不会跟着光标乱跳（移动时 Infoview 刷新风暴/错位）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@infoview", "@click:.monaco-editor", "gg", "G", "@wait:1200", "@snap:after-G", "gg", "@wait:1200", "@snap:after-gg"],
    probes: ["infoview", "status"] },

  { id: "vim-suggest-tab", vim: true, mode: "insert", note: "**vim 新现象候选**：vim 的 insert 模式里补全弹窗与 Tab 的归属（Tab 接受补全 vs 记法键位 vs vim 自己的缩进）",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  exact Set.sub"],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "<End>", "@wait:400", "<ctrl+space>", "@wait:1500", "@snap:suggest",
      "<Tab>", "@wait:900", "@snap:after-tab", "<Esc>", "u", "@wait:800", "@snap:after-undo", "@grade"],
    probes: ["suggest", "status", "markers"] },

  { id: "vim-problems-jump", vim: true, mode: "normal", note: "**vim 新现象候选**：从 Problems 面板点一条错跳到编辑器后，vim 光标/模式是否错乱（跳转后按 j 会不会改文本）",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  exact hh"],
    steps: ["@wait:2500", "@problems", "@wait:1500", "@snap:panel", "@click:.markers-panel .monaco-list-row", "@wait:1500", "@snap:after-jump", "j", "@wait:600", "@snap:after-j"],
    probes: ["problems", "status", "markers"] },

  { id: "nexthole-command", note: "**对照**：同一条路走命令面板（Sokonanoda: Next Hole）⇒ 分清「键位没触发」与「nextHole 本身不跳」",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "1", "<CR>", "@snap:at-line1",
      "@cmd:Sokonanoda: Next Hole", "@wait:1200", "@snap:after-cmd-1", "@shot:after-cmd-1",
      "@cmd:Sokonanoda: Next Hole", "@wait:1200", "@snap:after-cmd-2"],
    probes: ["status", "notifications", "problems"] },

  { id: "nexthole-altn-focus", note: "**alt+n 的焦点前提**：先点编辑器正文确认 `editorTextFocus` 为真，再按 alt+n（与 alt+s 同一窗口）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "1", "<CR>", "@snap:before",
      "<alt+s>", "@wait:1200", "@snap:alt-s", "<alt+n>", "@wait:1200", "@snap:alt-n", "@shot:alt-n"],
    probes: ["status", "notifications"] },

  { id: "caret-line-check", note: "**探针自检**：`<ctrl+g>` 92 之后光标行/可见行/状态栏三个读数是否一致（读数不可信就别下结论）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "@snap:before", "<ctrl+g>", "92", "<CR>", "@wait:1500",
      "@snap:at-92", "@shot:at-92", "<ctrl+g>", "5", "<CR>", "@wait:1000", "@snap:at-5"],
    probes: ["status", "lncol"] },

  { id: "completion-palette", note: "**决定性对照**：绕开 macOS 的 ⌃Space 抢键，走命令面板 `Trigger Suggest` —— 分清「触发不了」与「弹窗坏了」",
    file: "Scratch.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  exact "],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "<End>", "@wait:600",
      "Set.", "@wait:1500", "@snap:after-dot", "@cmd:Trigger Suggest", "@wait:2500", "@snap:suggest-palette",
      "@shot:suggest-palette", "@grade"],
    probes: ["suggest", "status", "markers", "quickinput"],
    check: (r) => { const s = r.snaps.at(-1).suggest;
      return { ok: !!(s && s.visible && s.count > 0), why: `suggest=${JSON.stringify(s).slice(0, 220)}` }; } },

  { id: "problems-jump-sorry", note: "**Problems 里 `sorry` 警告点下去跳到哪**：警告行写 Ln 90、洞在 Ln 92 —— 跳错地方就是导航坑",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@problems", "@wait:1800", "@snap:panel",
      "@clicktext:.markers-panel .monaco-list-row|uses 'sorry'", "@wait:1800", "@snap:after-jump", "@shot:after-jump"],
    probes: ["problems", "status", "markers"] },

  { id: "diag-parse-cascade", note: "**判据**：末尾一个 parse 错**不该**让前面正确行的记法被报成「未声明记法」（期望 = 没有 notation-unknown-symbol）",
    file: "Scratch_diag.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  exact Set."],
    steps: ["@wait:2500", "@click:.monaco-editor", "@wait:3000", "@snap:diag", "@problems", "@wait:1500", "@snap:panel", "@grade"],
    probes: ["markers", "problems", "status"],
    check: (r) => { const bad = (r.grade?.failed ?? []).filter((f) => f.code === "notation-unknown-symbol");
      return { ok: bad.length === 0,
        why: `notation-unknown-symbol ×${bad.length}` + bad.map((f) => ` @Ln${f.start_line}Col${f.start_col}`).join("") }; } },

  { id: "notation-clean-control", note: "**对照**：同一份文件只把最后一行改成 `exact h` ⇒ 判绿、无记法误报（证明①的因果）",
    file: "Scratch_diag_ok.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  exact h"],
    steps: ["@wait:2500", "@click:.monaco-editor", "@wait:3000", "@snap:diag", "@grade"],
    probes: ["markers", "status"],
    check: (r) => ({ ok: (r.grade?.counts?.decl_checked ?? 0) === 1 && (r.grade?.failed?.length ?? 1) === 0,
      why: `checked=${r.grade?.counts?.decl_checked} failed=${r.grade?.failed?.length}` }) },

  { id: "completion-autotype", note: "**打字自动触发**（不靠 ⌃Space）：逐字符打 `exa` 并每 120ms 轮询 `.suggest-widget` 3 秒",
    file: "Scratch_ok.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  "],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "<End>", "@wait:800",
      "e", "@wait:400", "x", "@wait:400", "a", "@wait:400", "@poll-suggest:3000", "@snap:after-exa", "@shot:after-exa"],
    probes: ["suggest", "status"],
    check: (r) => { const ev = r.events.find((x) => x.step.startsWith("@poll-suggest"));
      const s2 = r.snaps.at(-1).suggest;
      return { ok: !!(ev?.best?.visible || s2?.visible), why: `poll=${JSON.stringify(ev?.best)} final=${JSON.stringify(s2).slice(0,120)}` }; } },

  { id: "solve-ex5-full", note: "**把练习5 整题做完**（alt+n 选中 sorry → 打 3 行证明）⇒ 有没有「这题过了」的可见信号（状态栏/Problems/Infoview/练习树）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "1", "<CR>", "<alt+n>", "@wait:800", "@snap:hole",
      "constructor\n  exact mem_of_subset_singleton α A a\n  exact subset_of_mem_singleton α A a", "@wait:4000",
      "@snap:solved", "@shot:solved", "@grade", "@problems", "@wait:1500", "@snap:problems-after"],
    probes: ["infoview", "status", "markers", "notifications", "problems", "views"] },

  { id: "infoview-project-block", note: "**Infoview 的「项目」区块**：编译产物/建议 Rebuild 那些行到底写了什么（DOM 文本，不靠截图猜）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@infoview", "@wait:2000", "@snap:iv", "@cmd:Sokonanoda: Refresh Exercise Panel", "@wait:2000", "@snap:iv-refreshed"],
    probes: ["infoview", "status"] },

  { id: "completion-behavior", note: "**行为判据（不依赖 DOM 选择器）**：`exa` + Enter ⇒ 文本若被改成补全项就是「接受补全」，若多出一行就是「没有补全」",
    file: "Scratch_ok.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  "],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "<End>", "@wait:800",
      "exa", "@wait:1500", "@dump-suggest", "@snap:before-enter", "<CR>", "@wait:1200", "@snap:after-enter", "@shot:after-enter"],
    probes: ["suggest", "status"],
    check: (r) => { const before = r.snaps.find((x) => x.label === "before-enter")?.lines ?? [];
      const after = r.snaps.find((x) => x.label === "after-enter")?.lines ?? [];
      const grew = after.length > before.length;
      return { ok: grew, why: `Enter 前 ${JSON.stringify(before.slice(4))} ⇒ 后 ${JSON.stringify(after.slice(4))}（期望多出一行 = 没被补全吃掉）` }; } },

  { id: "completion-set-prefix", note: "**补全里有没有库里那些引理**：打 `Set.` 之后弹窗的行（课程要用的 Set.subset_def / Set.mem_singleton_iff 在不在）",
    file: "Scratch_ok.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  exact Set."],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "1", "<CR>", "@wait:600",
      "<ctrl+g>", "6", "<CR>", "<End>", "@wait:600", "\n  exact Set.", "@wait:2000", "@snap:after-Set-dot", "@dump-suggest", "@shot:set-prefix"],
    probes: ["suggest", "status"],
    check: (r) => { const s2 = r.snaps.at(-1).suggest;
      const hasSet = (s2?.rows ?? []).some((x) => /Set\./.test(x.text));
      return { ok: hasSet, why: `弹窗 ${s2?.count ?? 0} 行：${JSON.stringify((s2?.rows ?? []).map((x) => x.text).slice(0, 8))}` }; } },

  { id: "hover-signature", note: "**想知道要写几个实参 ⇒ hover 签名**：光标放到 `Set.subset_def` 上按 `Show or Focus Hover`，看弹出来的签名",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "29:12", "<CR>", "@wait:800", "@snap:on-name",
      "@cmd:Show or Focus Hover", "@wait:1800", "@snap:hover", "@shot:hover"],
    probes: ["status", "notifications", "hover"],
    check: (r) => { const h = r.snaps.at(-1).hover;
      return { ok: !!(h && h.text && h.text.length > 0), why: `hover=${String(JSON.stringify(h)).slice(0, 300)}` }; } },

  { id: "completion-word-vs-dot", note: "**打 `Set` vs 打 `Set.`**：确认「点不开候选」是因为 LSP 没注册 triggerCharacters（不是没数据）",
    file: "Scratch_ok.sokonanoda",
    initial: ["import lib.Logic", "import lib.Set", "", "theorem t (α : Type) (A B : Set α) (h : A ⊆ B) :", "    A ⊆ B := by", "  "],
    steps: ["@wait:2500", "@click:.monaco-editor", "<ctrl+g>", "6", "<CR>", "<End>", "@wait:600",
      "Set", "@wait:1800", "@snap:after-Set", "@dump-suggest",
      ".", "@wait:1200", "@snap:after-dot", "@dump-suggest", "@shot:word-vs-dot"],
    probes: ["suggest", "status"],
    check: (r) => { const a = r.snaps.find((x) => x.label === "after-Set")?.suggest;
      const b = r.snaps.find((x) => x.label === "after-dot")?.suggest;
      return { ok: !!(b && b.visible && b.count > 0),
        why: `打 Set ⇒ ${a?.count ?? 0} 行 ${JSON.stringify((a?.rows ?? []).map((x) => x.text).slice(0, 6))}；再打 . ⇒ ${b?.count ?? 0} 行（期望「.」也继续给候选）` }; } },

  { id: "infoview-dom-debug", note: "**探针自检**：把 webview 的 DOM 结构摊开（为什么读不到目标面板文本）",
    file: "units/I.1/unit01-sets-membership.sokonanoda",
    initialFrom: "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
    steps: ["@wait:2500", "@infoview", "@wait:2500", "@dump-webviews", "@snap:iv"],
    probes: ["infoview", "status"] },
];

// ── CDP 客户端 ──────────────────────────────────────────────────────────────
let debugPort = args.port;

async function cdpTargets() {
  return (await fetch(`http://127.0.0.1:${debugPort}/json/list`)).json();
}

const CODE = { "\\": "Backslash", " ": "Space", "\n": "Enter", "{": "BracketLeft",
  "}": "BracketRight", "(": "Digit9", ")": "Digit0", ":": "Semicolon", ".": "Period",
  "_": "Minus", "'": "Quote", ">": "Period", "<": "Comma", "=": "Equal", "|": "Backslash",
  ",": "Comma", "-": "Minus", "/": "Slash", "\t": "Tab" };
const VK = { "\\": 220, " ": 32, "\n": 13, "{": 219, "}": 221, "(": 57, ")": 48,
  ":": 186, ".": 190, "_": 189, "'": 222, "<": 188, ">": 190, "=": 187, "|": 220,
  ",": 188, "-": 189, "/": 191, "\t": 9 };
const SHIFTED = new Set(["{", "}", "(", ")", ":", "_", "<", ">", "|", "?"]);
function codeOf(ch) {
  if (Object.hasOwn(CODE, ch)) return CODE[ch];
  if (/[a-zA-Z]/.test(ch)) return `Key${ch.toUpperCase()}`;
  if (/[0-9]/.test(ch)) return `Digit${ch}`;
  return "";
}
function vkOf(ch) {
  if (Object.hasOwn(VK, ch)) return VK[ch];
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
  const call = (method, params = {}, timeoutMs = 30_000) =>
    new Promise((res, rej) => {
      const n = ++id;
      const timer = setTimeout(() => { pending.delete(n); rej(new Error(`CDP ${method} 超时`)); }, timeoutMs);
      pending.set(n, (m) => { clearTimeout(timer); res(m); });
      ws.send(JSON.stringify({ id: n, method, params }));
    });
  const key = async (key_, code, vk, modifiers = 0, text) => {
    const base = { key: key_, code, modifiers, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk };
    await call("Input.dispatchKeyEvent", { type: "rawKeyDown", ...base, ...(text ? { text } : {}) });
    await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
  };
  const client = {
    close: () => ws.close(),
    call,
    async eval(expression) {
      const m = await call("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
      if (m.result?.exceptionDetails) return { _error: JSON.stringify(m.result.exceptionDetails).slice(0, 300) };
      return m.result?.result?.value;
    },
    /// 跨源 iframe（Infoview webview）里求值：CDP 的 isolated world 是唯一的路。
    async frameEval(urlSubstr, expression) {
      try { await call("Page.enable"); } catch { /* 已开 */ }
      const tree = await call("Page.getFrameTree");
      const found = [];
      const walk = (f) => { found.push(f.frame); (f.childFrames ?? []).forEach(walk); };
      walk(tree.result.frameTree);
      const frame = found.find((f) => String(f.url).includes(urlSubstr));
      if (!frame) return { _error: "no-frame", frames: found.map((f) => String(f.url).slice(0, 90)) };
      const w = await call("Page.createIsolatedWorld", { frameId: frame.id, worldName: "sokoProbe", grantUniveralAccess: true });
      const ctx = w.result?.executionContextId;
      if (!ctx) return { _error: "no-context", raw: JSON.stringify(w).slice(0, 200) };
      const m = await call("Runtime.evaluate", { expression, contextId: ctx, returnByValue: true });
      if (m.result?.exceptionDetails) return { _error: JSON.stringify(m.result.exceptionDetails).slice(0, 300) };
      return m.result?.result?.value;
    },
    /// Infoview 是 **OOPIF**（out-of-process iframe）：`Page.getFrameTree` 里
    /// **看不到**它（第一次实测 frames 只有一个 `vscode-file://…` ⇒ 误判"没有面板"）。
    /// 它在 `/json/list` 里是一个独立 target ⇒ 直接连它的 ws 读 DOM。
    async infoviewEval(expression) {
      let targets = [];
      try { targets = await cdpTargets(); } catch { /* ignore */ }
      // ⚠ 第三版才找对：Infoview 是**两层** webview ——
      //   外层 `vscode-webview://<uuid>/index.html?id=…` 里全是引导脚本（第一版把
      //   它的 innerText 当成了目标面板 ⇒ 读数全是 `// @ts-check`），真正的
      //   `media/infoview.html` 是**另一个** target。判据 = 正文里有「目标 / ⊢ / 练习」。
      const cands = targets.filter((x) => ["iframe", "page", "webview"].includes(x.type)
        && String(x.url).startsWith("vscode-webview://") && x.webSocketDebuggerUrl);
      const tried = [];
      for (const t of cands) {
        let c = null;
        try {
          c = await connect(t.webSocketDebuggerUrl);
          const head = await c.eval(`(() => {
            const out = [];
            const walk = (doc, depth) => { try {
              out.push(((doc.body && doc.body.innerText) || '').slice(0, 300));
              if (depth < 3) Array.from(doc.querySelectorAll('iframe')).forEach((f) => {
                try { if (f.contentDocument) walk(f.contentDocument, depth + 1); } catch (e) {} });
            } catch (e) {} };
            walk(document, 0);
            return out.join('\n'); })()`);
          const looksRight = typeof head === "string" && /目标|⊢|练习|课程|已无目标|等待编译/.test(head);
          tried.push({ url: String(t.url).slice(0, 70), head: String(head).slice(0, 80), picked: looksRight });
          if (looksRight) return await c.eval(expression);
        } catch (e) { tried.push({ url: String(t.url).slice(0, 70), error: String(e?.message ?? e).slice(0, 90) }); }
        finally { try { c?.close(); } catch { /* ignore */ } }
      }
      return { _error: "no-infoview-content", tried };
    },
    key,
    async char(ch, delayMs = args.charDelay) {
      const shift = SHIFTED.has(ch) ? 8 : 0;
      const base = { key: ch, code: codeOf(ch), windowsVirtualKeyCode: vkOf(ch),
        nativeVirtualKeyCode: vkOf(ch), modifiers: shift };
      await call("Input.dispatchKeyEvent", { type: "keyDown", text: ch, unmodifiedText: ch, ...base });
      await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
      await sleep(delayMs);
    },
    async enter() {
      const base = { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 };
      await call("Input.dispatchKeyEvent", { type: "keyDown", text: "\r", unmodifiedText: "\r", ...base });
      await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
    },
    async chord(letter, { ctrl = false, shift = false, alt = false, meta = false } = {}) {
      const mods = (ctrl ? 2 : 0) | (alt ? ALT : 0) | (meta ? MOD : 0) | (shift ? 8 : 0);
      const vk = letter.toUpperCase().charCodeAt(0);
      const base = { key: letter, code: `Key${letter.toUpperCase()}`, modifiers: mods,
        windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk };
      await call("Input.dispatchKeyEvent", { type: "rawKeyDown", ...base });
      await call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
    },
    async clickSelector(sel) {
      const raw = await this.eval(`(() => { const e = document.querySelector(${JSON.stringify(sel)});
        if (!e) return null; const r = e.getBoundingClientRect();
        return JSON.stringify({x:r.x + r.width/2, y:r.y + r.height/2}); })()`);
      if (!raw) return { clicked: false, selector: sel };
      const { x, y } = JSON.parse(raw);
      for (const type of ["mousePressed", "mouseReleased"])
        await call("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1, pointerType: "mouse" });
      return { clicked: true, selector: sel, x: Math.round(x), y: Math.round(y) };
    },
    /// 点**正文里的一行**（不是编辑器 widget 的几何中心）：第一版点中心时
    /// `monaco_focused` 仍是 false（实测 `active: BODY`）⇒ `editorTextFocus` 为假
    /// ⇒ 带 when 的键位不触发。点一行文字是最可靠的"把焦点交给编辑器"。
    async clickEditor() {
      const raw = await this.eval(`(() => {
        const l = document.querySelector('.view-lines .view-line');
        const r = l?.getBoundingClientRect();
        if (r) return JSON.stringify({ x: r.x + 24, y: r.y + r.height / 2, src: 'view-line' });
        const e = document.querySelector('.monaco-editor')?.getBoundingClientRect();
        return e ? JSON.stringify({ x: e.x + e.width * 0.4, y: e.y + 40, src: 'editor' }) : null; })()`);
      if (!raw) throw new Error("找不到编辑器区域");
      const { x, y, src } = JSON.parse(raw);
      for (const type of ["mousePressed", "mouseReleased"])
        await call("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1,
          buttons: type === "mousePressed" ? 1 : 0, pointerType: "mouse" });
      await sleep(250);
      const f = await this.eval(READ_FOCUS);
      return { clicked: true, x: Math.round(x), y: Math.round(y), src, focus: f };
    },
  };
  return client;
}

// ── 具名键 ──────────────────────────────────────────────────────────────────
const NAMED = {
  "<Esc>": ["Escape", "Escape", 27, 0], "<Tab>": ["Tab", "Tab", 9, 0],
  "<CR>": ["\r", "Enter", 13, 0], "<BS>": ["Backspace", "Backspace", 8, 0],
  "<Del>": ["Delete", "Delete", 46, 0], "<Space>": [" ", "Space", 32, 0],
  "<Up>": ["ArrowUp", "ArrowUp", 38, 0], "<Down>": ["ArrowDown", "ArrowDown", 40, 0],
  "<Left>": ["ArrowLeft", "ArrowLeft", 37, 0], "<Right>": ["ArrowRight", "ArrowRight", 39, 0],
  "<End>": ["End", "End", 35, 0], "<Home>": ["Home", "Home", 36, 0],
  "<F12>": ["F12", "F12", 123, 0],
};
// 修饰键组合（**显式拼写**，别用单字母缩写——踩过 `<C-S-g>` 被当成 ctrl+shift+g 的坑）：
//   `<cmd+x>` `<cmd+shift+x>` `<ctrl+x>` `<alt+x>` `<alt+shift+x>` `<ctrl+space>`
const CHORD_RE = /^<(cmd|ctrl|alt)(\+shift)?\+([a-z0-9.]+)>$/;
function chordToken(token) {
  const m = CHORD_RE.exec(token);
  if (!m) return null;
  const [, mod, shift, letter] = m;
  return { letter: letter === "space" ? " " : letter,
    ctrl: mod === "ctrl", alt: mod === "alt", meta: mod === "cmd", shift: !!shift,
    isSpace: letter === "space" };
}

/// 一个 token → 真按键。返回人类可读的"这一步按了什么"。
async function press(client, token) {
  if (token === "<CR>") { await client.enter(); return token; }
  // ⚠ `NAMED[token]` 会命中原型链：打字内容里有 `constructor` 这个词时
  // `NAMED["constructor"]` = Object 构造函数（truthy 且不可迭代）⇒ 探针自己炸
  // （实测踩到，还烧了一轮）。一律 `Object.hasOwn`。
  if (Object.hasOwn(NAMED, token)) { await client.key(...NAMED[token]); return token; }
  const c = chordToken(token);
  if (c) {
    const mods = (c.ctrl ? 2 : 0) | (c.alt ? ALT : 0) | (c.meta ? MOD : 0) | (c.shift ? 8 : 0);
    const vk = c.isSpace ? 32 : c.letter.toUpperCase().charCodeAt(0);
    const code = c.isSpace ? "Space" : /[0-9]/.test(c.letter) ? `Digit${c.letter}`
      : /[.]/.test(c.letter) ? "Period" : `Key${c.letter.toUpperCase()}`;
    const base = { key: c.isSpace ? " " : c.letter, code, modifiers: mods,
      windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk };
    await client.call("Input.dispatchKeyEvent", { type: "rawKeyDown", ...base });
    await client.call("Input.dispatchKeyEvent", { type: "keyUp", ...base });
    return token;
  }
  for (const ch of token) {
    if (ch === "\n") await client.enter();
    else await client.char(ch);
  }
  return token;
}

// ── DOM 读法 ────────────────────────────────────────────────────────────────
const PICK_EDITOR = `(() => {
  // ⚠ 实测踩到：同一个窗口里**上一个 tab 的编辑器仍留在 DOM 里**，
  // querySelectorAll('.view-lines .view-line') 会读到那份旧内容
  // （批量跑时 6 条 case 全在读 Scratch 的行，而 active tab 已经是 unit01 ⇒
  // 差点把"编辑器没切换"写成产品 bug）。一律只读**可见且聚焦**的那个编辑器。
  const eds = Array.from(document.querySelectorAll('.monaco-editor'))
    .filter((e) => e.offsetParent !== null && e.getBoundingClientRect().width > 0);
  return eds.find((e) => e.classList.contains('focused')) || eds[0] || null;
})()`;
const READ_LINES = `(() => {
  const ed = ${PICK_EDITOR};
  if (!ed) return JSON.stringify([]);
  const lines = Array.from(ed.querySelectorAll('.view-lines .view-line'));
  return JSON.stringify(lines.map((l) => l.textContent));
})()`;
const READ_STATUS = `(() => {
  const items = Array.from(document.querySelectorAll('.statusbar-item')).map((i) => (i.textContent || '').trim()).filter(Boolean);
  return JSON.stringify(items);
})()`;
const READ_LNCOL = `(() => {
  const it = Array.from(document.querySelectorAll('.statusbar-item')).map((i) => (i.textContent || '').trim())
    .find((s) => /^Ln /.test(s) || /行 /.test(s));
  return it ?? null;
})()`;
// 左边栏可见行号（比状态栏更硬：alt+n 到底把视口搬到哪一行，看这个）
const READ_VISIBLE = `(() => {
  const ed = ${PICK_EDITOR};
  if (!ed) return JSON.stringify({ visible: [] });
  const nums = Array.from(ed.querySelectorAll('.margin-view-overlays .line-numbers'))
    .map((e) => (e.textContent || '').trim()).filter(Boolean);
  const cursor = ed.querySelector('.cursors-layer .cursor');
  const r = cursor?.getBoundingClientRect();
  return JSON.stringify({ visible: nums, cursor_y: r ? Math.round(r.top) : null });
})()`;
// 光标所在行（用光标 y 与可见行 y 求交）——状态栏不更新时的兜底读数
const READ_CURSOR_LINE = `(() => {
  const ed = ${PICK_EDITOR};
  if (!ed) return null;
  const c = ed.querySelector('.cursors-layer .cursor');
  if (!c) return null;
  const cr = c.getBoundingClientRect();
  const lines = Array.from(ed.querySelectorAll('.view-lines .view-line'));
  let best = null, bd = 1e9;
  lines.forEach((l) => { const r = l.getBoundingClientRect(); const d = Math.abs(r.top - cr.top);
    if (d < bd) { bd = d; best = l; } });
  if (!best) return null;
  const nums = Array.from(ed.querySelectorAll('.margin-view-overlays .line-numbers'));
  let bn = null, bnd = 1e9;
  nums.forEach((n) => { const r = n.getBoundingClientRect(); const d = Math.abs(r.top - cr.top);
    if (d < bnd) { bnd = d; bn = (n.textContent || '').trim(); } });
  return bn;
})()`;
const READ_MODES = `(() => {
  const items = Array.from(document.querySelectorAll('.statusbar-item')).map((i) => (i.textContent || '').trim());
  const t = items.filter((s) => /NORMAL|INSERT|VISUAL|REPLACE/.test(s));
  return t.length ? t[t.length - 1] : null;
})()`;
// ⚠ 第一版读错了地方：Monaco 的波浪线**不在** `.view-line` 的 span 里，而在
// `.view-overlays` 那层绝对定位的 div 上（实测 `.view-line span[class*=squiggly]`
// 永远 0 个，而 Problems 面板里明明有错 ⇒ 差点得出"没有波浪线"的假结论）。
// 这里同时算出**被划线的文本**（用同一行里 token span 与波浪线矩形求交）。
const READ_MARKERS = `(() => {
  const ed = ${PICK_EDITOR};
  if (!ed) return JSON.stringify({ error: 0, warning: 0, info: 0, byLine: {}, spans: [], no_editor: true });
  const lines = Array.from(ed.querySelectorAll('.view-lines .view-line'));
  const tops = lines.map((l) => l.getBoundingClientRect().top);
  const out = { error: 0, warning: 0, info: 0, byLine: {}, spans: [] };
  const els = Array.from(ed.querySelectorAll('[class*="squiggly"]'))
    .filter((e) => !e.closest('.decorationsOverviewRuler') && !e.closest('.minimap'));
  els.forEach((e) => {
    const cls = Array.from(e.classList).filter((c) => c.includes('squiggly')).join(',');
    const r = e.getBoundingClientRect();
    let li = -1, best = 1e9;
    tops.forEach((t, i) => { const d = Math.abs(t - r.top); if (d < best) { best = d; li = i; } });
    const line = li >= 0 ? lines[li] : null;
    let text = '';
    if (line) {
      // 只取**最深**的 span（Monaco 嵌套 span ⇒ 第一版把「  exact hh」记成
      // 「  exact hhexact hh」，读数翻倍）。
      text = Array.from(line.querySelectorAll('span'))
        .filter((s) => !s.querySelector('span'))
        .filter((s) => { const sr = s.getBoundingClientRect();
          return sr.width > 0 && sr.right > r.left + 0.5 && sr.left < r.right - 0.5; })
        .map((s) => s.textContent).join('');
    }
    const rec = { cls, line: li + 1, left: Math.round(r.left), width: Math.round(r.width), text };
    out.spans.push(rec);
    if (li >= 0) (out.byLine[li + 1] = out.byLine[li + 1] || []).push(rec);
    if (cls.includes('error')) out.error += 1;
    else if (cls.includes('warning')) out.warning += 1;
    else out.info += 1;
  });
  return JSON.stringify(out);
})()`;
const READ_NOTIFICATIONS = `(() => {
  const t = Array.from(document.querySelectorAll('.notifications-toasts .notification-list-item, .notifications-toasts .monaco-list-row'))
    .map((n) => (n.textContent || '').replace(/\\s+/g, ' ').trim()).filter(Boolean);
  const uniq = [...new Set(t)]; // 同一个 toast 会同时命中两个选择器 ⇒ 别记两遍
  // 探针 profile 自带的环境噪声（GitHub 登录、argv.json）单独归类，别混进产品读数
  const noise = /github\\.com|argv\\.json|Signing in|安装|Install/i;
  return JSON.stringify({ all: uniq, product: uniq.filter((s) => !noise.test(s)) });
})()`;
const READ_SUGGEST = `(() => {
  // ⚠ 第一版用 offsetParent !== null 判可见 —— **错**：Monaco 的补全弹窗是
  // 固定定位（position: fixed），fixed 元素的 offsetParent **恒为 null**
  // ⇒ 明明弹着（实测 DOM: editor-widget suggest-widget visible, display:block,
  // 432×76，行里有 exact/example/exfalso）却被读成"没弹" ⇒ 差点写出假结论。
  // 改成看 computed display + 尺寸。
  const w = document.querySelector('.suggest-widget');
  const vis = !!w && getComputedStyle(w).display !== 'none' && w.getBoundingClientRect().width > 0;
  const rows = vis ? Array.from(w.querySelectorAll('.monaco-list-row')).map((r) => ({
    text: (r.querySelector('.label-name')?.textContent || r.textContent || '').trim(),
    detail: (r.querySelector('.label-description')?.textContent || '').trim(),
    full: (r.textContent || '').replace(/\\s+/g, ' ').trim().slice(0, 160),
    kind: (r.querySelector('.codicon')?.className || '').replace('codicon codicon-', ''),
    focused: r.classList.contains('focused'),
  })) : [];
  const d = document.querySelector('.suggest-details');
  const details = d && getComputedStyle(d).display !== 'none'
    ? (d.textContent || '').replace(/\\s+/g, ' ').trim().slice(0, 600) : null;
  return JSON.stringify({ visible: vis, count: rows.length, rows: rows.slice(0, 25), details });
})()`;
const READ_QUICKFIX = `(() => {
  const views = Array.from(document.querySelectorAll('.context-view')).filter((v) => v.offsetParent !== null);
  const rows = views.flatMap((v) => Array.from(v.querySelectorAll('.monaco-list-row')).map((r) => ({
    text: (r.textContent || '').replace(/\\s+/g, ' ').trim(),
    focused: r.classList.contains('focused'),
  })));
  const lightbulb = !!document.querySelector('.lightbulb-glyph, .codelens-decoration .lightbulb');
  return JSON.stringify({ visible: rows.length > 0, count: rows.length, rows: rows.slice(0, 20), lightbulb });
})()`;
const READ_PROBLEMS = `(() => {
  const panel = document.querySelector('.markers-panel');
  if (!panel) return JSON.stringify({ open: false });
  const rows = Array.from(panel.querySelectorAll('.monaco-list-row')).map((r) => ({
    aria: r.getAttribute('aria-label') || '',
    text: (r.textContent || '').replace(/\\s+/g, ' ').trim(),
    severity: r.className,
  }));
  const title = (document.querySelector('.panel .title-label, .panel .panel-title')?.textContent || '').replace(/\\s+/g, ' ').trim();
  const filters = Array.from(panel.querySelectorAll('.monaco-toolbar .monaco-action-bar .action-label'))
    .map((a) => (a.getAttribute('aria-label') || a.textContent || '').trim()).filter(Boolean);
  return JSON.stringify({ open: true, title, filters, count: rows.length, rows: rows.slice(0, 30) });
})()`;
const READ_QUICKINPUT = `(() => {
  const w = document.querySelector('.quick-input-widget');
  if (!w || w.offsetParent === null) return JSON.stringify({ open: false });
  const rows = Array.from(w.querySelectorAll('.quick-input-list .monaco-list-row'))
    .map((r) => (r.textContent || '').replace(/\\s+/g, ' ').trim()).slice(0, 15);
  return JSON.stringify({ open: true,
    placeholder: w.querySelector('input')?.placeholder ?? null,
    rows,
    message: (w.querySelector('.quick-input-message')?.textContent || '').replace(/\\s+/g, ' ').trim() || null });
})()`;
const READ_VIEWS = `(() => {
  const sections = Array.from(document.querySelectorAll('.pane-header')).map((h) => ({
    title: (h.querySelector('.title')?.textContent || h.textContent || '').trim(),
    expanded: h.getAttribute('aria-expanded'),
    rows: (() => { const p = h.parentElement; return p ? Array.from(p.querySelectorAll('.monaco-list-row')).map((r) => (r.textContent || '').replace(/\\s+/g, ' ').trim()).slice(0, 20) : []; })(),
  }));
  return JSON.stringify(sections.filter((s) => s.title));
})()`;
const READ_INFOVIEW = `(() => {
  // Infoview 是两层 webview；且外层文档的 body.innerText 实测返回**非字符串**
  // （读成 [object Object] ⇒ 第一/二版都判成「没有内容」）。这里：
  //   ① 用 clone + 去掉 script/style 的 textContent（永远是字符串）
  //   ② 挑"含目标面板关键词"的那个文档，而不是最长的那个
  const docs = [];
  const walk = (doc, depth) => {
    docs.push(doc);
    if (depth < 3) { try { Array.from(doc.querySelectorAll('iframe')).forEach((f) => {
      try { if (f.contentDocument) walk(f.contentDocument, depth + 1); } catch (e) {} }); } catch (e) {} }
  };
  try { walk(document, 0); } catch (e) { return JSON.stringify({ text: '', error: String(e.message) }); }
  const textOf = (doc) => {
    try {
      const body = doc.body;
      if (!body) return '';
      const clone = body.cloneNode(true);
      clone.querySelectorAll('script, style, template').forEach((n) => n.remove());
      return (clone.textContent || '').replace(/[ \t]+/g, ' ').replace(/\n{3,}/g, '\n\n').trim();
    } catch (e) { return ''; }
  };
  const KEY = /目标|⊢|练习|声明|已无目标|等待编译|编译器/;
  let best = null, bestText = '';
  for (const d of docs) { const t = textOf(d);
    if (KEY.test(t)) { best = d; bestText = t; break; }
    if (t.length > bestText.length) { best = d; bestText = t; } }
  const root = (best && (best.querySelector('#root') || best.body)) || null;
  const cards = root ? Array.from(root.querySelectorAll('.card, .goal, .decl, li, p, div'))
    .map((e) => (e.textContent || '').replace(/\s+/g, ' ').trim())
    .filter((x) => x && x.length < 200).slice(0, 60) : [];
  return JSON.stringify({ text: bestText.slice(0, 4000), cards, docs: docs.length });
})()`;
// 焦点状态：`editorTextFocus` 为假 ⇒ 带 when 的键位（alt+n/alt+s/alt+b）**不会触发**，
// 这必须与产品行为区分开（第一版就是在这里差点误判）。
const READ_FOCUS = `(() => {
  const ed = ${PICK_EDITOR};
  const ae = document.activeElement;
  return JSON.stringify({
    window_focused: document.hasFocus(),
    active: ae ? (ae.className || ae.tagName) : null,
    monaco_focused: !!ed && ed.classList.contains('focused'),
    textarea: ae && ae.classList ? ae.classList.contains('inputarea') : false,
  });
})()`;
const READ_HOVER = `(() => {
  const hs = Array.from(document.querySelectorAll('.monaco-hover')).filter((e) => getComputedStyle(e).display !== 'none');
  const t = hs.map((e) => (e.textContent || '').replace(/\\s+/g, ' ').trim()).filter(Boolean);
  return JSON.stringify({ count: t.length, text: t.join(' | ').slice(0, 800) });
})()`;
const SHOT_DIR = join(STATE, "shots");

function cleanDom(raw) {
  const lines = JSON.parse(raw).map((s) => s.replace(/\u00a0/g, " ").replace(/\u200b/g, ""));
  while (lines.length > 1 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}
const norm = (l) => l.replace(/\u00a0/g, " ").replace(/\r\n/g, "\n");
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const jparse = (s) => { try { return typeof s === "string" ? JSON.parse(s) : s; } catch { return { _raw: String(s).slice(0, 400) }; } };

async function snapshot(client, label, probes, extra = {}) {
  const snap = { label, t: Date.now(), ...extra };
  try { snap.lines = cleanDom(await client.eval(READ_LINES)); } catch (e) { snap.lines = `_err ${e.message}`; }
  const want = new Set(probes);
  const maybe = async (key, expr, transform) => {
    if (!want.has(key)) return;
    try { snap[key] = transform ? transform(await client.eval(expr)) : jparse(await client.eval(expr)); }
    catch (e) { snap[key] = `_err ${e.message}`; }
  };
  await maybe("status", READ_STATUS, (v) => jparse(v));
  await maybe("lncol", READ_LNCOL);
  await maybe("modes", READ_MODES);
  await maybe("markers", READ_MARKERS, (v) => jparse(v));
  await maybe("notifications", READ_NOTIFICATIONS, (v) => jparse(v));
  await maybe("suggest", READ_SUGGEST, (v) => jparse(v));
  await maybe("quickfix", READ_QUICKFIX, (v) => jparse(v));
  await maybe("problems", READ_PROBLEMS, (v) => jparse(v));
  await maybe("views", READ_VIEWS, (v) => jparse(v));
  await maybe("quickinput", READ_QUICKINPUT, (v) => jparse(v));
  await maybe("hover", READ_HOVER, (v) => jparse(v));
  await maybe("focus", READ_FOCUS, (v) => jparse(v));
  await maybe("visible", READ_VISIBLE, (v) => jparse(v));
  await maybe("cursor_line", READ_CURSOR_LINE);
  if (want.has("infoview")) {
    const r = await client.infoviewEval(READ_INFOVIEW);
    snap.infoview = typeof r === "string" ? jparse(r) : r;
  }
  return snap;
}

/// 真截图（PNG 落 `.cache/ux/editor/shots/`）——"用户屏幕上是什么"的最终证据。
async function shot(client, caseId, label) {
  try {
    mkdirSync(SHOT_DIR, { recursive: true });
    const m = await client.call("Page.captureScreenshot", { format: "png" });
    const data = m.result?.data;
    if (!data) return { saved: false };
    const file = join(SHOT_DIR, `${caseId}-${String(label).replace(/[^\w.-]+/g, "_")}.png`);
    writeFileSync(file, Buffer.from(data, "base64"));
    return { saved: true, file: file.replace(`${REPO}/`, ""), bytes: Buffer.from(data, "base64").length };
  } catch (e) { return { saved: false, error: String(e?.message ?? e).slice(0, 120) }; }
}

// ── 工作区 / profile / 启动 ─────────────────────────────────────────────────
function settingsFor() {
  return {
    "workbench.startupEditor": "none", "workbench.tips.enabled": false,
    "workbench.colorTheme": "Default Dark Modern", "window.commandCenter": false,
    "breadcrumbs.enabled": false, "editor.minimap.enabled": false,
    "editor.stickyScroll.enabled": false, "editor.wordWrap": "off",
    "editor.renderWhitespace": "none", "editor.detectIndentation": false,
    "editor.insertSpaces": true, "editor.tabSize": 2,
    "editor.autoIndent": "full", "editor.autoClosingBrackets": "languageDefined",
    "editor.quickSuggestions": { other: true, comments: false, strings: false },
    "editor.suggestOnTriggerCharacters": true, "editor.tabCompletion": "off",
    "editor.acceptSuggestionOnEnter": "on",
    "git.openRepositoryInParentFolders": "never", "update.showReleaseNotes": false,
    "extensions.autoCheckUpdates": false, "extensions.autoUpdate": false,
    "telemetry.telemetryLevel": "off", "security.workspace.trust.enabled": false,
    "workbench.panel.defaultLocation": "bottom",
  };
}

function prepare(tag, arm) {
  const profile = join(RUN, `p-${tag}`);
  rmSync(profile, { recursive: true, force: true });
  mkdirSync(join(profile, "User"), { recursive: true });
  mkdirSync(join(profile, "extensions"), { recursive: true });
  writeFileSync(join(profile, "User", "settings.json"), JSON.stringify(settingsFor(), null, 2));
  if (arm === "on") {
    if (!existsSync(join(VIM_SRC, "package.json"))) {
      console.error(`error: 找不到 vim 扩展：${VIM_SRC}`); process.exit(3);
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
  const errLog = join(profile, "vscode-stderr.log");
  const fd = openSync(errLog, "a");
  const child = spawn(vscodeBin(), [
    `--user-data-dir=${profile}`,
    `--extensions-dir=${join(profile, "extensions")}`,
    `--shared-data-dir=${join(profile, "shared")}`,
    `--remote-debugging-port=${debugPort}`,
    `--extensionDevelopmentPath=${EXTENSION}`,
    "--disable-gpu", "--disable-chromium-sandbox", "--no-first-run",
    "--disable-workspace-trust",
    "--disable-extension", "GitHub.copilot-chat", "--disable-extension", "GitHub.copilot",
    "--goto", `${join(WS, file)}:1:1`, WS,
  ], { detached: true, stdio: ["ignore", "ignore", fd] });
  child.unref();
  return errLog;
}

function freePort(port) {
  // `soko-editor-probe` 是本探针**独有**的 tmp 目录名 ⇒ 这条兜底不会误伤别的窗口，
  // 但能收掉"profile 路径对不上"的残留（实测：上一轮窗口退出慢，端口 20s 没释放
  // ⇒ 新一轮直接拒绝跑 = 白等一轮）。
  for (const pat of ["soko-editor-probe", `remote-debugging-port=${port}`,
    `user-data-dir=${join(RUN, `p-${port}`)}`]) {
    try { execFileSync("pkill", ["-f", pat]); } catch { /* 本来就没有 */ }
  }
}
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
      if (page) {
        const client = await connect(page.webSocketDebuggerUrl);
        // ⚠ **必须做焦点仿真**（第一次踩的坑，读数全废）：窗口在后台时
        // `document.hasFocus()` = false ⇒ ① VS Code 的 `editorTextFocus` 为假
        // ⇒ `alt+n` / `alt+s` / `alt+b` 这些带 `when` 的键位**根本不触发**
        // （会被误判成"产品坏了"）② Chromium 节流渲染 ⇒ `.view-line` 读成空
        // （会被误判成"编辑器空白"）。`Emulation.setFocusEmulationEnabled` 让页面
        // 认为自己是前台焦点窗口，这是真用户在前台敲键的等价条件。
        if (args.focusEmu) await client.call("Emulation.setFocusEmulationEnabled", { enabled: true });
        if (args.bring) { try { await client.call("Page.bringToFront"); } catch { /* 可能不支持 */ } }
        return client;
      }
    } catch { /* 还没起来 */ }
  }
  throw new Error("VS Code 没有暴露 CDP");
}

async function waitForEditor(client, deadline) {
  while (Date.now() < deadline) {
    try {
      const raw = await client.eval(READ_LINES);
      if (typeof raw === "string" && raw !== "[]" && raw !== "null") return cleanDom(raw);
    } catch { /* 页面还在换 */ }
    await sleep(400);
  }
  throw new Error("编辑器没有渲染出任何行");
}

const ACTIVE_TAB = `(() => { const t = document.querySelector('.tabs-container .tab.active'); return t ? t.textContent : null; })()`;

async function openFile(client, name) {
  const base = name.replace(/\.sokonanoda$/, "");
  for (const query of [base, name]) {
    await client.chord("p", { meta: true });
    await sleep(800);
    const st = jparse(await client.eval(`(() => { const w = document.querySelector('.quick-input-widget');
      const rows = Array.from(document.querySelectorAll('.quick-input-list .monaco-list-row'))
        .map((r) => (r.textContent || '').replace(/\\s+/g, ' ').trim()).slice(0, 5);
      return JSON.stringify({ open: !!w && w.offsetParent !== null, rows }); })()`));
    if (!st.open) { await sleep(600); }
    for (const ch of query) { await client.char(ch, 30); }
    await sleep(1100);
    const after = jparse(await client.eval(`(() => {
      const rows = Array.from(document.querySelectorAll('.quick-input-list .monaco-list-row'))
        .map((r) => (r.textContent || '').replace(/\\s+/g, ' ').trim()).slice(0, 5);
      return JSON.stringify({ rows, focused: document.querySelector('.quick-input-list .monaco-list-row.focused')?.textContent ?? null }); })()`));
    if (!(after.rows ?? []).some((r) => r.includes(name))) {
      await client.key("Escape", "Escape", 27); await sleep(400); continue;
    }
    await client.enter();
    await sleep(1200);
    const tab = await client.eval(ACTIVE_TAB);
    if (String(tab ?? "").includes(name)) return { opened: name, query, tab };
    await sleep(600);
  }
  const tab = await client.eval(ACTIVE_TAB);
  throw new Error(`快速打开没能切到 ${name}（当前：${tab}）`);
}

async function revertFile(client) {
  await client.chord("p", { meta: true, shift: true });
  await sleep(700);
  for (const ch of "revert") { await client.char(ch, 35); }
  await sleep(700);
  await client.enter();
  await sleep(1000);
}

async function runCommand(client, title) {
  await client.chord("p", { meta: true, shift: true });
  await sleep(700);
  for (const ch of title) { await client.char(ch, 30); }
  await sleep(900);
  await client.enter();
  await sleep(900);
}

/// `Ctrl+G`（macOS 上 Go to Line/Column 是 ⌃G，不是 ⌘G）⇒ 打行号 ⇒ Enter。
/// 打完用状态栏 `Ln x, Col y` 复核（读不到就退回命令面板，别静默错位）。
async function gotoLine(client, line) {
  await client.chord("g", { ctrl: true });
  await sleep(700);
  const w = jparse(await client.eval(`(() => { const q = document.querySelector('.quick-input-widget');
    return JSON.stringify({ open: !!q && q.offsetParent !== null,
      placeholder: q?.querySelector('input')?.placeholder ?? null }); })()`));
  if (!w.open || !/line|行/i.test(String(w.placeholder ?? ""))) {
    await client.key("Escape", "Escape", 27);
    await sleep(300);
    await runCommand(client, "Go to Line/Column");
  }
  for (const ch of String(line)) { await client.char(ch, 35); }
  await sleep(400);
  await client.enter();
  await sleep(700);
  return client.eval(READ_LNCOL);
}

/// 打开/还原之后等 DOM **真的渲染出内容**（实测：`(ready)` 快照会读到 `[""]`，
/// 那是 Monaco 换文档的空档 ⇒ 会把"编辑器空白"误记成产品现象）。
async function waitForContent(client, expectFirst, budgetMs = 10_000) {
  const t0 = Date.now();
  while (Date.now() - t0 < budgetMs) {
    const raw = await client.eval(READ_LINES);
    const lines = typeof raw === "string" ? cleanDom(raw) : [];
    if (lines.length > 1) return { ms: Date.now() - t0, lines: lines.length };
    if (lines.length === 1 && lines[0] && expectFirst && lines[0].startsWith(expectFirst.slice(0, 12)))
      return { ms: Date.now() - t0, lines: 1 };
    await sleep(250);
  }
  return { ms: null, lines: 0 };
}

/// 轮询 DOM 直到 `READ_MARKERS` 里的 error 数 > 0（量"打字到诊断出现"的墙钟）。
async function waitForError(client, deadlineMs) {
  const t0 = Date.now();
  while (Date.now() - t0 < deadlineMs) {
    const m = jparse(await client.eval(READ_MARKERS));
    if ((m?.error ?? 0) > 0) return { ms: Date.now() - t0, markers: m };
    await sleep(50);
  }
  return { ms: null, markers: jparse(await client.eval(READ_MARKERS)) };
}

// ── 一条用例 ────────────────────────────────────────────────────────────────
async function runCase(client, testCase, arm) {
  const startedAt = Date.now();
  const relFile = testCase.file ?? args.file;
  const path = join(WS, relFile);
  const initial = testCase.initialFrom
    ? readFileSync(join(REPO, testCase.initialFrom), "utf8").replace(/\n$/, "").split("\n")
    : testCase.initial;
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, initial.join("\n"), "utf8");
  await openFile(client, relFile.split("/").pop());
  await client.clickEditor();
  await sleep(400);
  await revertFile(client);

  const snaps = [];
  const events = [];
  // 每条 case 开跑前重新声明前台焦点（窗口可能被别的 case/系统抢走）
  if (args.focusEmu) { try { await client.call("Emulation.setFocusEmulationEnabled", { enabled: true }); } catch { /* ignore */ } }
  if (args.bring) { try { await client.call("Page.bringToFront"); } catch { /* ignore */ } }
  await client.clickEditor();
  await sleep(300);
  const settle = await waitForContent(client, initial[0]);
  events.push({ step: "@settle", ...settle });
  // **自检（这条是被一次真实误读逼出来的）**：读到的 DOM 必须属于目标文件，
  // 否则读数全部作废 —— 上一版读到的是**上一个 tab 残留的编辑器**，
  // 6 条 case 全在分析 Scratch 的内容而以为是 unit01。
  {
    // 判据：可见行**逐行都属于目标文件**（不比首行 —— 编辑器会恢复上次的滚动位置，
    // 实测首行常常是文件中间那一行）。
    const probeLines = cleanDom(await client.eval(READ_LINES));
    const wantSet = new Set(initial.map((l) => l.trim()).filter(Boolean));
    const domTrim = probeLines.map((l) => l.trim()).filter(Boolean);
    const hit = domTrim.filter((l) => wantSet.has(l)).length;
    const ok = domTrim.length === 0 ? false : hit >= Math.min(3, domTrim.length);
    events.push({ step: "@identity-guard", ok, matched: hit, visible: domTrim.length,
      dom_first: String(probeLines[0] ?? "").slice(0, 40) });
    if (!ok) throw new Error(`编辑器里显示的不是目标文件（可见 ${domTrim.length} 行，只有 ${hit} 行属于它；首行「${String(probeLines[0] ?? "").slice(0, 40)}」）`);
  }
  const probes = [...new Set(["status", "lncol", "markers", "visible", "cursor_line", "focus",
    ...(testCase.probes ?? [])])];
  const rec = async (label, extra) => {
    const s = await snapshot(client, label, probes, extra);
    snaps.push(s);
    if (args.verbose) log(`  · ${label} → ${JSON.stringify(s.lines)}`);
    return s;
  };

  if (arm === "on" && testCase.mode && testCase.mode !== "fresh") {
    for (const tok of (testCase.mode === "normal" ? ["<Esc>"] : ["<Esc>", "i"])) await press(client, tok);
    await sleep(250);
  }
  await rec("(ready)");

  let grade = null;
  for (const step of testCase.steps ?? []) {
    if (step.startsWith("@snap:")) { await rec(step.slice(6)); continue; }
    if (step.startsWith("@expand:")) {
      const title = step.slice(8);
      const r = await client.eval(`(() => {
        const hs = Array.from(document.querySelectorAll('.pane-header'));
        const h = hs.find((x) => ((x.querySelector('.title')?.textContent || x.textContent || '').trim()) === ${JSON.stringify(title)});
        if (!h) return JSON.stringify({ found: false, titles: hs.map((x) => (x.querySelector('.title')?.textContent || '').trim()) });
        const b = h.getBoundingClientRect();
        return JSON.stringify({ found: true, x: b.x + b.width / 2, y: b.y + b.height / 2, expanded: h.getAttribute('aria-expanded') });
      })()`);
      const j = jparse(r);
      if (j.found) {
        for (const type of ["mousePressed", "mouseReleased"])
          await client.call("Input.dispatchMouseEvent", { type, x: j.x, y: j.y, button: "left",
            clickCount: 1, buttons: type === "mousePressed" ? 1 : 0, pointerType: "mouse" });
      }
      events.push({ step, expand: j });
      await sleep(1200);
      continue;
    }
    if (step === "@dump-webviews") {
      const targets = await cdpTargets();
      const cands = targets.filter((x) => ["iframe", "page", "webview"].includes(x.type) && x.webSocketDebuggerUrl);
      const out = [];
      for (const t of cands) {
        let c = null;
        try {
          c = await connect(t.webSocketDebuggerUrl);
          const probe = await c.eval(`(() => {
            const d = document;
            const body = d.body;
            const root = d.querySelector('#root');
            return JSON.stringify({ bodyType: typeof body, innerTextType: typeof (body && body.innerText),
              textContentLen: body ? (body.textContent || '').length : -1,
              childCount: body ? body.children.length : -1,
              rootText: root ? (root.textContent || '').replace(/\\s+/g, ' ').slice(0, 200) : null,
              title: d.title });
          })()`);
          out.push({ url: String(t.url).slice(0, 60), probe: jparse(probe) });
        } catch (e) { out.push({ url: String(t.url).slice(0, 60), error: String(e?.message ?? e).slice(0, 80) }); }
        finally { try { c?.close(); } catch { /* ignore */ } }
      }
      events.push({ step, targets: out });
      continue;
    }
    if (step === "@dump-suggest") {
      const r = await client.eval(`(() => {
        const out = [];
        document.querySelectorAll('[class*="suggest"], .editor-widget').forEach((e) => {
          const rc = e.getBoundingClientRect();
          const cs = getComputedStyle(e);
          out.push({ cls: e.className, display: cs.display, visibility: cs.visibility,
            w: Math.round(rc.width), h: Math.round(rc.height), top: Math.round(rc.top),
            text: (e.textContent || '').replace(/\\s+/g, ' ').slice(0, 120) });
        });
        return JSON.stringify(out.slice(0, 12));
      })()`);
      events.push({ step, dom: jparse(r) });
      continue;
    }
    if (step.startsWith("@poll-suggest:")) {
      const budget = Number(step.slice(14));
      const t0 = Date.now(); let best = { visible: false, count: 0 }; const seen = [];
      while (Date.now() - t0 < budget) {
        const cur = jparse(await client.eval(READ_SUGGEST));
        if (cur?.visible) { best = cur; break; }
        seen.push(cur?.count ?? 0);
        await sleep(120);
      }
      events.push({ step, ms: Date.now() - t0, best, samples: seen.slice(0, 12) });
      continue;
    }
    if (step.startsWith("@clicktext:")) {
      // `@clicktext:<selector>|<文本片段>`：按**文本**点（Problems 行有多个文件分组，
      // nth-child 会点错行 —— 实测点到别的文件的诊断上）。
      const [sel, frag] = step.slice(11).split("|");
      const r = await client.eval(`(() => {
        const rows = Array.from(document.querySelectorAll(${JSON.stringify(sel)}));
        const i = rows.findIndex((x) => (x.textContent || '').includes(${JSON.stringify(frag)}));
        if (i < 0) return JSON.stringify({ found: false, n: rows.length,
          texts: rows.map((x) => (x.textContent || '').slice(0, 60)) });
        const b = rows[i].getBoundingClientRect();
        return JSON.stringify({ found: true, i, x: b.x + 40, y: b.y + b.height / 2 });
      })()`);
      const j = jparse(r);
      if (j.found) for (const type of ["mousePressed", "mouseReleased"])
        await client.call("Input.dispatchMouseEvent", { type, x: j.x, y: j.y, button: "left",
          clickCount: 1, buttons: type === "mousePressed" ? 1 : 0, pointerType: "mouse" });
      events.push({ step, click: j });
      await sleep(600);
      continue;
    }
    if (step.startsWith("@shot:")) {
      const r = await shot(client, testCase.id, step.slice(6));
      events.push({ step, shot: r }); continue;
    }
    if (step.startsWith("@focus-check")) { await rec("focus-check"); continue; }
    if (step.startsWith("@wait:")) { await sleep(Number(step.slice(6))); continue; }
    if (step === "@revert") { await revertFile(client); continue; }
    if (step === "@problems") { await client.chord("m", { meta: true, shift: true }); await sleep(1200); continue; }
    if (step === "@infoview") { await runCommand(client, "Sokonanoda: Infoview"); await sleep(900); continue; }
    if (step === "@grade") {
      // 判卷读的是**磁盘**：先真存盘（⌘S），再跑 CLI —— 否则 DOM 与判卷读数对不上。
      await client.chord("s", { meta: true });
      await sleep(1000);
      grade = gradeAll(relFile); events.push({ step, grade: grade.counts });
      continue;
    }
    if (step.startsWith("@cmd:")) { await runCommand(client, step.slice(5)); continue; }
    if (step.startsWith("@click:")) { events.push({ step, result: await client.clickSelector(step.slice(7)) }); await sleep(400); continue; }
    if (step.startsWith("@timedkey:")) {
      const tok = step.slice(10); const t0 = Date.now();
      await press(client, tok); await sleep(300);
      const first = await waitForError(client, 30_000);
      events.push({ step, key: tok, ms_to_first_error: first.ms, markers: first.markers });
      continue;
    }
    if (step.startsWith("@timed:")) {
      const text = step.slice(7);
      const per = [];
      let lastKeyAt = null;
      for (const ch of text) {
        if (ch === "\n") await client.enter(); else await client.char(ch, 0);
        lastKeyAt = Date.now();
        const t0 = Date.now();
        const m = jparse(await client.eval(READ_MARKERS));
        per.push({ ch, ms_to_any_error: (m?.error ?? 0) > 0 ? Date.now() - t0 : null, errors: m?.error ?? null });
      }
      // 真正的读数：**敲完最后一个字符**到"本行出现包含所打词的诊断"的墙钟
      const word = text.trim().split(/\s+/).pop();
      const t0 = Date.now(); let hit = null;
      while (Date.now() - t0 < 25_000) {
        const m = jparse(await client.eval(READ_MARKERS));
        const sp = (m?.spans ?? []).find((x) => String(x.text || '').includes(word));
        if (sp) { hit = { ms_from_last_key: Date.now() - t0, span: sp }; break; }
        await sleep(40);
      }
      events.push({ step, typed: text, word, per_char: per, last_key_at: lastKeyAt, real_diag: hit });
      continue;
    }
    if (step.startsWith("@slow:")) {
      const text = step.slice(6);
      const t0 = Date.now();
      for (const ch of text) { if (ch === "\n") await client.enter(); else await client.char(ch, 150); }
      const first = await waitForError(client, 30_000);
      events.push({ step, typed: text, type_ms: Date.now() - t0, ms_to_first_error: first.ms });
      continue;
    }
    if (step.startsWith("@poll-infoview:")) {
      const budget = Number(step.slice(15));
      const t0 = Date.now(); let first = null;
      while (Date.now() - t0 < budget) {
        const r = await client.infoviewEval(READ_INFOVIEW);
        const txt = typeof r === "string" ? jparse(r).text : null;
        if (txt && !/等待编译/.test(txt)) { first = { ms: Date.now() - t0, text: txt.slice(0, 600) }; break; }
        await sleep(250);
      }
      events.push({ step, first_goal_ms: first?.ms ?? null, text: first?.text ?? null });
      continue;
    }
    await press(client, step);
    await sleep(step.startsWith("<") ? 260 : 140);
  }

  await sleep(500);
  await rec("(final)");
  await client.chord("s", { meta: true });
  await sleep(1200);
  const disk = norm(readFileSync(path, "utf8"));
  const diskLines = disk.split("\n");
  while (diskLines.length > 1 && diskLines[diskLines.length - 1] === "") diskLines.pop();
  if (!grade) grade = gradeAll(relFile);

  const dom = snaps[snaps.length - 1]?.lines ?? [];
  let pass = null, verdict = "recorded", check_why = null;
  if (testCase.expect) {
    const domOk = same(dom, testCase.expect), diskOk = same(diskLines, testCase.expect);
    pass = domOk && diskOk;
    verdict = pass ? "match" : `dom=${domOk ? "ok" : "MISMATCH"} disk=${diskOk ? "ok" : "MISMATCH"}`;
  }
  // 判据（可执行）：`check(result) -> {ok, why}` —— 退出码 0 = 与期望一致。
  if (typeof testCase.check === "function") {
    const res = { id: testCase.id, snaps, events, grade, dom_lines: dom, disk_lines: diskLines,
      cursor_lines: snaps.map((x) => x.cursor_line), lncols: snaps.map((x) => x.lncol) };
    try {
      const c = testCase.check(res);
      pass = !!c.ok; check_why = c.why ?? null;
      verdict = (pass ? "match" : "MISMATCH") + (check_why ? ` · ${check_why}` : "");
    } catch (e) { pass = false; verdict = `check 抛错：${e?.message ?? e}`; }
  }
  return { id: testCase.id, arm, ms: Date.now() - startedAt, note: testCase.note,
    file: relFile, steps: testCase.steps, snaps, events, grade,
    dom_lines: dom, disk_lines: diskLines, expect: testCase.expect, verdict, pass };
}

async function runArm(cases, arm, tag, port) {
  debugPort = port;
  const profile = prepare(tag, arm);
  freePort(port);
  if (!(await portFree(port, Date.now() + 60_000)))
    throw new Error(`端口 ${port} 被占着（残留窗口没退干净）——**不跑**`);
  launch(profile, cases[0].file ?? args.file);
  log(`真 VS Code 起来了（arm=${arm} · port=${port} · profile=${profile}）`);
  const out = [];
  let client = null;
  try {
    client = await workbenchPage(Date.now() + args.timeout);
    await waitForEditor(client, Date.now() + args.timeout);
    for (const c of cases) {
      let r;
      try {
        r = await runCase(client, c, arm);
      } catch (e) {
        // 一条 case 失败**不许**带走整组读数（第一次踩：openFile 抛错 ⇒ 前面那条的
        // 全部快照一起丢了）。记成 error 结果继续跑下一条。
        r = { id: c.id, arm, ms: null, note: c.note, file: c.file ?? args.file,
          steps: c.steps, snaps: [], events: [], grade: null,
          error: String(e?.message ?? e) + " @@ " + String(e?.stack ?? "no-stack").split("\n").slice(1, 4).join(" | "),
          dom_lines: [], disk_lines: [], verdict: `error: ${String(e?.message ?? e).slice(0, 120)}`, pass: null };
      }
      out.push(r);
      const g = r.grade?.counts ?? {};
      log(`${r.error ? "ERR " : r.pass === false ? "FAIL" : r.pass ? "PASS" : "REC "} ${r.id} [${arm}] ` +
        `(${r.ms ? (r.ms / 1000).toFixed(1) + "s" : "-"}) ` +
        `grade: checked=${g.decl_checked ?? "?"} open=${g.exercise_open ?? "?"} failed=${r.grade?.failed?.length ?? "?"}` +
        (r.error ? ` · ${r.error}` : ""));
    }
  } finally {
    try { client?.close(); } catch { /* ignore */ }
    if (!args.keep) await quit(profile);
  }
  return out;
}

// ── 主流程 ──────────────────────────────────────────────────────────────────
if (!existsSync(join(EXTENSION, "package.json"))) { console.error("error: 扩展目录不完整"); process.exit(3); }
ensureWorkspace();
const identity = buildIdentity();
if (args.list) {
  for (const c of CASES) console.log(`${(c.vim ? "vim " : "    ")} ${c.id.padEnd(22)} ${c.note}`);
  console.log(`\n共 ${CASES.length} 条`);
  process.exit(0);
}
const wanted = args.case ? args.case.split(",") : null;
const selected = wanted ? CASES.filter((c) => wanted.includes(c.id)) : args.all ? CASES : [];
if (selected.length === 0) { console.error("error: 用 --case <id> / --all（--list 看清单）"); process.exit(2); }
for (const id of wanted ?? []) if (!CASES.some((c) => c.id === id)) { console.error(`error: --case ${id} 不在表里`); process.exit(2); }

const results = [];
let failure = null;
try {
  const groups = new Map();
  for (const c of selected) {
    const arm = args.arm ?? (c.vim ? "on" : "off");
    if (!groups.has(arm)) groups.set(arm, []);
    groups.get(arm).push(c);
  }
  let i = 0;
  for (const [arm, cs] of groups) {
    results.push(...(await runArm(cs, arm, `${args.port}-${i}`, args.port + i)));
    i += 1;
  }
} catch (e) {
  failure = String(e?.message ?? e);
  log(`错误：${failure}`);
}

const graded = results.filter((r) => r.pass !== null);
const failed = graded.filter((r) => !r.pass);
if (args.json) {
  console.log(JSON.stringify({ schema: "soko.editor-probe/1", identity, case: args.case ?? null,
    results, failed: failed.length, error: failure }, null, 2));
} else {
  log(`构建身份：HEAD ${identity.head} · extension.js ${identity.extension_js.bytes}B sha ${identity.extension_js.sha256_16} @ ${identity.extension_js.mtime}`);
  log(`            bundled LSP sha ${identity.bundled_lsp.sha256_16} @ ${identity.bundled_lsp.mtime} · bundled CLI ${identity.bundled_cli.version} sha ${identity.bundled_cli.sha256_16}`);
  for (const r of results) {
    const last = r.snaps[r.snaps.length - 1];
    log(`  ${r.id}: ${r.verdict} · grade checked=${r.grade?.counts?.decl_checked ?? "?"} open=${r.grade?.counts?.exercise_open ?? "?"} failed=${r.grade?.failed?.length ?? "?"}`);
    if (args.verbose) log(`    final dom: ${JSON.stringify(last?.lines)}`);
  }
}
if (failure) process.exit(1);
process.exit(failed.length > 0 ? 1 : 0);
