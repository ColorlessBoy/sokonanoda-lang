#!/usr/bin/env node
// ============================================================================
// sokonanoda-lang · 编辑器服务面（LSP）体验找茬探针
//
// 用法：
//   node scripts/ux-probes/lsp-probe.mjs --list                 # 列出全部用例
//   node scripts/ux-probes/lsp-probe.mjs --case <id>            # 只跑一条
//   node scripts/ux-probes/lsp-probe.mjs --all                  # 跑全部
//   node scripts/ux-probes/lsp-probe.mjs --case <id> --json     # 机器可读
//   node scripts/ux-probes/lsp-probe.mjs --all --exe target/debug/sokonanoda-lsp
//
// 退出码（与 docs/gaps/README.md 的复现件同一套约定，但**语义按本轮任务书**）：
//   0 = 观测与用例的 `expect`（期望的好行为）**一致**（这一面没问题）
//   1 = **不一致**（缺口仍在；`observed` 就是现场）
//   2 = 环境缺失（起不了 LSP / 判卷 CLI 不在）
//
// 形状照抄先例 docs/gaps/repro/G20-lsp-drops-rescued-report.js（.sh 外壳 + node
// JSON-RPC 客户端），但**不写任何盘上正式代码**：只读 `courses/`，一切夹具落在
// `.cache/ux/lsp/` 内。
//
// 构建身份：每条读数都带 `soko/version`（服务器自述 {version,pid}）+ `git rev-parse
// --short HEAD`。⚠ `scripts/soko version --json` 的 `lsp.marker` 是**缓存目录**的
// marker（本机实测 stale `0.73.0`），**不是**仓库构建的版本 ⇒ 不许拿它当构建身份
// （见 REPORT.md「环境与构建身份」）。
'use strict';

import { spawn, execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url)); // <repo>/scripts/ux-probes
const ROOT = path.resolve(HERE, '..', '..');               // <repo>
// 夹具落 `.cache/`（gitignore）⇒ 跑探针不会把仓库弄脏（路径与旧版 `.cache/ux/lsp/` 一致）
const STATE = path.join(ROOT, '.cache', 'ux', 'lsp');
const FIXTURES = path.join(STATE, 'fixtures');
const COURSE = path.join(ROOT, 'courses', 'set-theory');
const CANVAS_REL = path.join('units', 'I.1', 'unit01-sets-membership.sokonanoda');
const CANVAS = path.join(FIXTURES, CANVAS_REL);

// ── 环境：夹具（只读 courses/ ⇒ 写 .cache/ux/lsp/fixtures/） ──────────────────

function copyIfChanged(from, to) {
  const src = fs.readFileSync(from);
  try {
    if (fs.readFileSync(to).equals(src)) return;
  } catch (_) {
    /* 目标不存在 */
  }
  fs.mkdirSync(path.dirname(to), { recursive: true });
  fs.writeFileSync(to, src);
}

/** 把真课程树镜像进 `.cache/ux/lsp/fixtures/`（模块根 = fixtures/，带清单）。 */
function ensureFixtures() {
  copyIfChanged(path.join(COURSE, 'sokonanoda.toml'), path.join(FIXTURES, 'sokonanoda.toml'));
  for (const name of fs.readdirSync(path.join(COURSE, 'lib'))) {
    if (!name.endsWith('.sokonanoda')) continue;
    copyIfChanged(path.join(COURSE, 'lib', name), path.join(FIXTURES, 'lib', name));
  }
  copyIfChanged(path.join(COURSE, CANVAS_REL), CANVAS);
}

/** 写一份夹具（**必须**落在 `fixtures/` 里：模块根 = 最近的 sokonanoda.toml）。 */
function fixture(name, text) {
  const file = path.join(FIXTURES, 'probe', name);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, text);
  return file;
}

// ── 最小 JSON-RPC 客户端（stdio） ───────────────────────────────────────────

const uriOf = (file) => 'file://' + file;

class Lsp {
  constructor({ exe, args, cwd }) {
    this.exe = exe || process.execPath;
    this.args = args || [path.join(ROOT, 'scripts', 'soko'), 'lsp'];
    this.cwd = cwd || ROOT;
    this.nextId = 1;
    this.pending = new Map();
    this.waiters = [];
    this.stderr = '';
    this.exit = null;
  }

  start() {
    this.child = spawn(this.exe, this.args, { stdio: ['pipe', 'pipe', 'pipe'], cwd: this.cwd });
    let buffer = Buffer.alloc(0);
    this.child.stdout.on('data', (chunk) => {
      buffer = Buffer.concat([buffer, chunk]);
      for (;;) {
        const sep = buffer.indexOf('\r\n\r\n');
        if (sep < 0) return;
        const match = /Content-Length: (\d+)/i.exec(buffer.slice(0, sep).toString('utf8'));
        if (!match) return;
        const length = Number(match[1]);
        if (buffer.length < sep + 4 + length) return;
        const body = buffer.slice(sep + 4, sep + 4 + length).toString('utf8');
        buffer = buffer.slice(sep + 4 + length);
        let message;
        try {
          message = JSON.parse(body);
        } catch (_) {
          continue;
        }
        if (message.method && message.id !== undefined && message.id !== null) {
          const result = message.method === 'workspace/configuration' ? [null] : null;
          this.write({ jsonrpc: '2.0', id: message.id, result });
          continue;
        }
        if (message.method) {
          for (let i = 0; i < this.waiters.length; i += 1) {
            if (this.waiters[i].match(message)) {
              const waiter = this.waiters.splice(i, 1)[0];
              clearTimeout(waiter.timer);
              waiter.resolve(message);
              break;
            }
          }
          continue;
        }
        const entry = this.pending.get(message.id);
        if (entry) {
          this.pending.delete(message.id);
          clearTimeout(entry.timer);
          entry.resolve(message);
        }
      }
    });
    this.child.stderr.on('data', (chunk) => {
      this.stderr += chunk.toString('utf8');
    });
    this.child.on('exit', (code) => {
      this.exit = code;
    });
    return this;
  }

  write(message) {
    const body = Buffer.from(JSON.stringify(message), 'utf8');
    this.child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
    this.child.stdin.write(body);
  }

  notify(method, params) {
    this.write({ jsonrpc: '2.0', method, params });
  }

  raw(method, params, timeoutMs = 30000) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`LSP 超时未应答：${method}`));
      }, timeoutMs);
      this.pending.set(id, { resolve, timer });
      this.write({ jsonrpc: '2.0', id, method, params });
    });
  }

  async request(method, params, timeoutMs = 30000) {
    const message = await this.raw(method, params, timeoutMs);
    if (message.error) throw new Error(`${method}: ${JSON.stringify(message.error)}`);
    return message.result;
  }

  waitFor(match, timeoutMs = 30000) {
    return new Promise((resolve, reject) => {
      const waiter = {
        match,
        resolve,
        timer: setTimeout(() => {
          const i = this.waiters.indexOf(waiter);
          if (i >= 0) this.waiters.splice(i, 1);
          reject(new Error('LSP 通知超时（等 publishDiagnostics）'));
        }, timeoutMs),
      };
      this.waiters.push(waiter);
    });
  }

  diagnostics(file, { version, timeoutMs = 30000 } = {}) {
    const uri = uriOf(file);
    return this
      .waitFor(
        (m) =>
          m.method === 'textDocument/publishDiagnostics' &&
          m.params &&
          m.params.uri === uri &&
          (version === undefined || m.params.version === version),
        timeoutMs,
      )
      .then((m) => m.params);
  }

  async initialize(root = FIXTURES) {
    const result = await this.request('initialize', {
      processId: process.pid,
      rootUri: uriOf(root),
      capabilities: {
        textDocument: {
          hover: { contentFormat: ['markdown', 'plaintext'] },
          completion: { completionItem: { documentationFormat: ['markdown', 'plaintext'] } },
          codeAction: { codeActionLiteralSupport: { codeActionKind: { valueSet: [] } } },
          publishDiagnostics: { versionSupport: true },
        },
        workspace: { workspaceFolders: true },
      },
      workspaceFolders: [{ uri: uriOf(root), name: path.basename(root) }],
    });
    this.notify('initialized', {});
    return result;
  }

  async open(file, text, version = 1) {
    const pending = this.diagnostics(file, { version });
    this.notify('textDocument/didOpen', {
      textDocument: { uri: uriOf(file), languageId: 'sokonanoda', version, text },
    });
    return pending;
  }

  async change(file, text, version) {
    const pending = this.diagnostics(file, { version });
    this.notify('textDocument/didChange', {
      textDocument: { uri: uriOf(file), version },
      contentChanges: [{ text }],
    });
    return pending;
  }

  stop() {
    try {
      this.child.kill();
    } catch (_) {
      /* 已退出 */
    }
  }
}

// ── 小工具 ─────────────────────────────────────────────────────────────────

/** 第 nth 个**落在非注释行**里的 needle 的位置（0 基 LSP 位置）。 */
function codePos(hay, needle, nth = 1) {
  let from = 0;
  let seen = 0;
  for (;;) {
    const idx = hay.indexOf(needle, from);
    if (idx < 0) {
      // ⚠ **判据不可用 ≠ 缺口仍在**（2026-10-12 独立复核 + LSP-B 双双实测）：
      // 画布在 `5c6eeff8` 被重写后，一批用例的锚点（行号/needle）**在画布上不存在**了
      // ⇒ `codePos` 抛错 ⇒ 旧代码把它记成「不一致（缺口仍在）」✗ —— 于是
      // **判据坏掉被读成产品有缺口**（H2/H3/N1/C3 实测全是这个形状）。
      // ⇒ 打上 `fixture` 标签，让上层判成 **exit 2 = 无法判定**（不是缺口）✓。
      const e = new Error(`夹具里找不到 ${JSON.stringify(needle)}#${nth}`);
      e.fixture = true;
      throw e;
    }
    const lineStart = hay.lastIndexOf('\n', idx) + 1;
    const line = hay.slice(lineStart, hay.indexOf('\n', idx));
    if (!line.trimStart().startsWith('--')) {
      seen += 1;
      if (seen === nth) {
        const before = hay.slice(0, idx);
        return { line: before.split('\n').length - 1, character: (before.split('\n').pop() || '').length };
      }
    }
    from = idx + needle.length;
  }
}

function hoverText(result) {
  if (!result || !result.contents) return null;
  const c = result.contents;
  if (typeof c === 'string') return c;
  if (c.value !== undefined) return c.value;
  return null;
}

/** 把 edits 应用到文本（从后往前；支持单/多行）。 */
function applyEdits(text, edits) {
  const lines = text.split('\n');
  const sorted = [...edits].sort(
    (a, b) => b.range.start.line - a.range.start.line || b.range.start.character - a.range.start.character,
  );
  for (const e of sorted) {
    const { start, end } = e.range;
    if (start.line === end.line) {
      lines[start.line] = lines[start.line].slice(0, start.character) + e.newText + lines[start.line].slice(end.character);
    } else {
      lines.splice(
        start.line,
        end.line - start.line + 1,
        lines[start.line].slice(0, start.character) + e.newText + lines[end.line].slice(end.character),
      );
    }
  }
  return lines.join('\n');
}

const editsOf = (action) => Object.values((action.edit && action.edit.changes) || {}).flat();

/** 真判卷（CLI `grade --json`）；返回 {exit, codes, checked, open}。 */
function grade(name, text) {
  const file = fixture(name, text);
  let out = '';
  let exit = 0;
  try {
    out = execFileSync(path.join(ROOT, 'scripts', 'soko'), ['grade', '--json', file], {
      encoding: 'utf8',
      timeout: 240000,
    });
  } catch (error) {
    out = String(error.stdout || '');
    exit = error.status === undefined ? 1 : error.status;
  }
  const events = out
    .split('\n')
    .filter(Boolean)
    .map((l) => {
      try {
        return JSON.parse(l);
      } catch (_) {
        return null;
      }
    })
    .filter(Boolean);
  return {
    file,
    exit,
    codes: events.filter((e) => e.type === 'diagnostic').map((e) => e.code),
    checked: events.filter((e) => e.type === 'decl.checked').map((e) => e.name),
    open: events.filter((e) => e.type === 'exercise.open').map((e) => e.name),
  };
}

// ── 用例 ───────────────────────────────────────────────────────────────────
//
// 每条：id / title / severity / blocker / expect（期望的好行为）/ run(ctx)。
// run 返回 { ok, observed, evidence }；ok=false ⇒ 缺口仍在。

const CASES = [];
const CASE = (spec) => CASES.push(spec);

// —— hover ——

CASE({
  id: 'H1-notation-hover-type',
  title: '记法符号 hover 的末块「表达式 : 类型」给的是**常量**的类型，不是表达式的',
  severity: 'high',
  blocker: false,
  expect: '`a ∈ A` 的末块类型必须是 `Prop`（表达式自己的类型）；`a = b` 同理',
  async run(ctx) {
    const file = fixture('h1.sokonanoda', 'import lib.Set\n\ntheorem h1 (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n');
    const text = fs.readFileSync(file, 'utf8');
    await ctx.lsp.open(file, text, 1);
    const res = await ctx.lsp.raw('textDocument/hover', { textDocument: { uri: uriOf(file) }, position: codePos(text, '∈', 1) });
    const md = hoverText(res.result) || '';
    const last = md.split('\n\n').filter((b) => b.startsWith('```sokonanoda') && b.includes(' : ')).pop() || '';
    const line = last.replace(/```sokonanoda|\n|```/g, '').trim();
    const file2 = fixture('h1b.sokonanoda', 'theorem h1b (α : Type) (a b : α) (h : a = b) : a = b := h\n');
    const text2 = fs.readFileSync(file2, 'utf8');
    await ctx.lsp.open(file2, text2, 1);
    const res2 = await ctx.lsp.raw('textDocument/hover', { textDocument: { uri: uriOf(file2) }, position: codePos(text2, '=', 1) });
    const md2 = hoverText(res2.result) || '';
    const last2 = md2.split('\n\n').filter((b) => b.startsWith('```sokonanoda') && b.includes(' : ')).pop() || '';
    const line2 = last2.replace(/```sokonanoda|\n|```/g, '').trim();
    return {
      ok: line.endsWith(' : Prop') && line2.endsWith(' : Prop'),
      observed: { '∈ 末块': line, '= 末块': line2 },
      evidence: `期望两条都以 \` : Prop\` 收尾；实际分别以 \`${line.split(' : ').pop()}\` / \`${line2.split(' : ').pop()}\` 收尾`,
    };
  },
});

CASE({
  id: 'H2-hover-def-value-drift',
  title: '同一个 `def` 的 hover 在四处给的信息不一致（tactic 里的名字**没有 `:=` 值块**）',
  severity: 'high',
  blocker: false,
  expect: '按声明卡片契约（用户原话「只要是 def，任何位置 hover 到它，都应显示带 `:=` 的完整信息」）四处都该有 `:=` 值块',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    const a = hoverText((await ctx.lsp.raw('textDocument/hover', { textDocument: { uri: uriOf(CANVAS) }, position: codePos(text, 'Set.singleton', 1) })).result) || '';
    const b = hoverText((await ctx.lsp.raw('textDocument/hover', { textDocument: { uri: uriOf(CANVAS) }, position: codePos(text, '{a}', 1) })).result) || '';
    const c = hoverText((await ctx.lsp.raw('textDocument/hover', { textDocument: { uri: uriOf(CANVAS) }, position: codePos(text, '#check Set.singleton', 1) })).result) || '';
    const has = (md) => md.includes(':=');
    return {
      ok: has(a) && has(b) && has(c),
      observed: {
        'unfold Set.singleton（tactic 里的名字）': a.replace(/\n/g, ' ⏎ '),
        '记法 {a} 上': b.replace(/\n/g, ' ⏎ '),
        '#check Set.singleton 行': c.replace(/\n/g, ' ⏎ '),
      },
      evidence: `tactic 里的名字 hover 无 \`:=\`（只有签名）；记法/{a} 与 #check 行有 \`:=\` ⇒ 同一常量四种渲染`,
    };
  },
});

CASE({
  id: 'H3-hover-panel-drift',
  title: '同一行 `#check Iff.refl`：hover 与 Infoview 命令输出**渲染不一致**（`@Iff A A` vs `A ↔ A`）',
  severity: 'high',
  blocker: false,
  expect: 'protocol.md 明写 command-line hover 与面板「cannot drift」⇒ 两处必须同形（且应当折叠记法）',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    const hover = hoverText((await ctx.lsp.raw('textDocument/hover', { textDocument: { uri: uriOf(CANVAS) }, position: codePos(text, '#check Iff.refl', 1) })).result) || '';
    const at = codePos(text, '#check Iff.refl', 1);
    const state = await ctx.lsp.request('soko/stateAt', { textDocument: { uri: uriOf(CANVAS) }, position: at });
    const msg = (state.messages || []).map((m) => m.text).join(' ｜ ');
    const unfolded = hover.includes('@Iff');
    return {
      ok: !unfolded && hover.includes('↔'),
      observed: { hover: hover.replace(/\n/g, ' ⏎ '), 'soko/stateAt.messages': msg },
      evidence: `hover 给 \`@Iff A A\`（未折叠、带显式 @），面板给 \`A ↔ A\`（已折叠）⇒ 同一行两个面自相矛盾`,
    };
  },
});

CASE({
  id: 'H4-notation-hover-slice',
  title: '记法符号 hover 末块的**源切片**会从表达式中间开始（丢掉开括号 ⇒ 括号不配对）',
  severity: 'medium',
  blocker: false,
  expect: '末块 `expr : type` 的 expr 必须是包含光标的那个**完整**表达式（括号配对）',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    // 第二个 ↔：单行 `(a ∈ A) ↔ A a`，切片实测从 `a ∈ A)` 开始（丢掉开括号）。
    const pos = codePos(text, '↔', 2);
    const md = hoverText((await ctx.lsp.raw('textDocument/hover', { textDocument: { uri: uriOf(CANVAS) }, position: pos })).result) || '';
    const last = md.split('\n\n').filter((b) => b.startsWith('```sokonanoda') && b.includes(' : ')).pop() || '';
    const line = last.replace(/```sokonanoda|\n|```/g, '').trim();
    const expr = line.split(' : ')[0];
    const open = (expr.match(/\(/g) || []).length;
    const close = (expr.match(/\)/g) || []).length;
    return {
      ok: open === close,
      observed: { 位置: `${pos.line}:${pos.character}`, 末块: line, 左括号: open, 右括号: close },
      evidence: `expr = ${JSON.stringify(expr)}：右括号 ${close} 个、左括号 ${open} 个 ⇒ 切片起点丢掉了开括号（源是 \`(a ∈ A) ↔ A a\`）`,
    };
  },
});

// —— completion ——

CASE({
  id: 'C1-completion-binders-at-caret',
  title: '补全只在**光标落在已存在的 token 里面**时给本域假设；行末（真正打字处）一个都不给',
  severity: 'high',
  blocker: false,
  expect: '在 tactic 行末（准备写下一个名字的位置）应当能补全本域 binder（h/a/A/α）',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    const lines = text.split('\n');
    const line = lines.findIndex((l) => l === '  exact h a');
    const width = lines[line].length;
    const sweep = [];
    for (let c = 0; c <= width + 1; c += 1) {
      const res = await ctx.lsp.raw('textDocument/completion', { textDocument: { uri: uriOf(CANVAS) }, position: { line, character: c } });
      const items = res.result || [];
      sweep.push({ col: c, binders: items.filter((i) => i.detail === '本域 binder').map((i) => i.label) });
    }
    const atEnd = sweep.find((s) => s.col === width + 1);
    const withBinders = sweep.filter((s) => s.binders.length);
    return {
      ok: atEnd.binders.length > 0,
      observed: { 行: lines[line], 列扫描: sweep },
      evidence: `binder 只在 col=${withBinders.map((s) => s.col).join(',')}（正好是 h/a 两个 token 的字符位）出现；col=${width + 1}（行末）⇒ ${JSON.stringify(atEnd.binders)}`,
    };
  },
});

CASE({
  id: 'C2-completion-imported-names',
  title: '补全**从不**给 import 进来的库名（课程 stdlib 对补全不可见）',
  severity: 'high',
  blocker: false,
  expect: '`import lib.Set` 之后，`Set.mem` / `Set.singleton` / `Set.subset_def` 等库名应在候选里',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    const res = await ctx.lsp.raw('textDocument/completion', { textDocument: { uri: uriOf(CANVAS) }, position: { line: 3, character: 2 } });
    const items = res.result || [];
    const labels = items.map((i) => i.label);
    const wanted = ['Set.mem', 'Set.singleton', 'Set.subset_def', 'Set.ext', 'Set.mem_singleton_iff'];
    const missing = wanted.filter((w) => !labels.includes(w));
    return {
      ok: missing.length === 0,
      observed: { 候选数: items.length, '以 Set. 开头': labels.filter((l) => l.startsWith('Set.')), 缺失: missing },
      evidence: `候选 ${items.length} 条里以 \`Set.\` 开头的 **0 条**；缺 ${JSON.stringify(missing)}（只有 prelude 名 + 本文件自己的声明）`,
    };
  },
});

CASE({
  id: 'C3-completion-position-blind',
  title: '补全**与位置无关**：by 块/项位/`#check ` 后/注释里都给同一份 110 条，且含 `ctor`/`rec`/`iota`',
  severity: 'medium',
  blocker: false,
  expect: '候选应当与位置相关（项位不该给 `ctor`/`rec`/`iota` 这类声明内部关键字），至少要有 `isIncomplete`/`sortText` 让客户端能排序',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    const spots = [
      ['by 块空行', { line: 3, character: 2 }],
      ['项位（:= 之后）', codePos(text, 'exact h a', 1)],
      ['#check 后', codePos(text, '#check Iff.refl', 1)],
      ['注释行内', { line: 1, character: 0 }],
    ];
    const seen = [];
    for (const [label, pos] of spots) {
      const res = await ctx.lsp.raw('textDocument/completion', { textDocument: { uri: uriOf(CANVAS) }, position: pos });
      const items = res.result || [];
      seen.push({ label, count: items.length, labels: items.map((i) => i.label), shape: Array.isArray(res.result) ? 'Array' : 'List' });
    }
    const first = JSON.stringify(seen[0].labels);
    const same = seen.every((s) => JSON.stringify(s.labels) === first);
    const internals = seen[0].labels.filter((l) => ['ctor', 'rec', 'iota', 'binder_notation', 'scoped'].includes(l));
    return {
      ok: !same || internals.length === 0,
      observed: { 各处: seen.map((s) => [s.label, s.count, s.shape]), 声明内部关键字: internals },
      evidence: `四处候选**逐字节相同**（都是 ${seen[0].count} 条、Array 形状、无 sortText/isIncomplete），且包含 ${JSON.stringify(internals)}`,
    };
  },
});

CASE({
  id: 'C4-completion-prelude-arity',
  title: 'prelude 名在补全里没有签名/实参个数（`Eq.subst` 只有 detail="prelude"）',
  severity: 'medium',
  blocker: false,
  expect: '`Eq.subst` 这类名字在候选里应能看到签名（要写几个实参）——正是用户抱怨的隐参数面',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    const res = await ctx.lsp.raw('textDocument/completion', { textDocument: { uri: uriOf(CANVAS) }, position: { line: 3, character: 2 } });
    const items = res.result || [];
    const hit = items.find((i) => i.label === 'Eq.subst');
    const doc = hit && hit.documentation ? (hit.documentation.value || '') : null;
    return {
      ok: Boolean(hit && doc),
      observed: hit ? { label: hit.label, kind: hit.kind, detail: hit.detail, documentation: doc, sortText: hit.sortText } : null,
      evidence: `Eq.subst 候选：detail=${JSON.stringify(hit && hit.detail)}、documentation=${JSON.stringify(doc)} ⇒ 补全里看不到 {α p a b} 这些实参`,
    };
  },
});

CASE({
  id: 'C5-completion-backslash-input',
  title: '`\\in` 输入法不在 LSP 里：补全无 `\\in`，字面 `\\in` 被当成 `\\`（Set.sdiff）报错',
  severity: 'high',
  blocker: false,
  expect: 'hover 承诺「输入：`\\in`」⇒ 非 VS Code 客户端（vim 等）也应能靠服务面拿到 `\\in` 候选或改写',
  async run(ctx) {
    const file = fixture('c5.sokonanoda', 'import lib.Set\n\ntheorem c5 (α : Type) (a : α) (A : Set α) : a \\in A := by\n  sorry\n');
    const text = fs.readFileSync(file, 'utf8');
    const diags = await ctx.lsp.open(file, text, 1);
    const line = text.split('\n')[2];
    const col = line.indexOf('\\in') + 3;
    const res = await ctx.lsp.raw('textDocument/completion', { textDocument: { uri: uriOf(file) }, position: { line: 2, character: col } });
    const items = res.result || [];
    const slashItems = items.filter((i) => i.label.includes('\\'));
    const codes = diags.diagnostics.map((d) => d.code);
    return {
      ok: slashItems.length > 0,
      observed: {
        '\\in 之后的候选数': items.length,
        含反斜杠的候选: slashItems.map((i) => i.label),
        诊断: diags.diagnostics.map((d) => [d.code, d.range.start.line, d.range.start.character, (d.message || '').split('\n')[0]]),
      },
      evidence: `无任何 \`\\…\` 候选；字面 \`\\in\` 的诊断为 ${JSON.stringify(codes)}（消息把 \`\\\` 当集合差 \`Set.sdiff\`）`,
    };
  },
});

// —— code action ——

CASE({
  id: 'A1-codeaction-by-hole-breaks',
  title: '【blocker】`by` 块里的洞：quick fix 插入的是**项**不是 tactic ⇒ 应用后语法错误、判卷红',
  severity: 'blocker',
  blocker: true,
  expect: '灯泡给的插入文本必须能直接过判卷（`scripts/soko grade` exit 0）',
  async run(ctx) {
    const text = 'theorem by_hole (p : Prop) (hp : p) : p := by\n  sorry\n';
    const file = fixture('a1.sokonanoda', text);
    await ctx.lsp.open(file, text, 1);
    const pos = codePos(text, 'sorry', 1);
    const res = await ctx.lsp.raw('textDocument/codeAction', { textDocument: { uri: uriOf(file) }, range: { start: pos, end: pos }, context: { diagnostics: [] } });
    const actions = res.result || [];
    const first = actions[0];
    const edited = first ? applyEdits(text, editsOf(first)) : text;
    const g = grade('a1-out.sokonanoda', edited);
    return {
      ok: actions.length > 0 && g.exit === 0,
      observed: {
        候选: actions.map((a) => [a.title, editsOf(a).map((e) => e.newText)]),
        结果文本: edited,
        'grade.exit': g.exit,
        'grade.codes': g.codes,
      },
      evidence: actions.length === 0
        ? '洞上一条候选都没有'
        : `标题写 \`exact hp …\` 但 newText=${JSON.stringify(editsOf(first).map((e) => e.newText))} ⇒ 结果 ${JSON.stringify(edited.split('\n')[1])} ⇒ grade exit=${g.exit} codes=${JSON.stringify(g.codes)}`,
    };
  },
});

CASE({
  id: 'A1b-codeaction-position-mode-is-per-hole',
  title: '【反向臂】一份文件里 tactic 位洞给 **tactic**（`exact hp`）、项位洞给**项**（`hp`）—— 模式按洞的**出身**判，不按文本',
  severity: 'control',
  blocker: false,
  expect: '两处插入都必须能直接过判卷；且 `by` 块那处带 `exact `、声明值位那处**不带**（`:= exact hp` 不是项）',
  async run(ctx) {
    // 形状说明（**如实**）：用户点名的字面反向臂 `:= by exact (sorry : p)` **在本语言里
    // 不可解析**（`(e : T)` 不是语法），而"`by` 嵌在项里"的形状（`And.intro (by exact hp) sorry`）
    // **不可 elaborate**（`unknown identifier \`by\``）⇒ 那种洞里**任何**插入都过不了判卷。
    // 可表达的等价臂 = **同一份文件里的两类洞**：位置模式必须逐洞判（一个"整份文件有 by
    // 就加 exact"的实现会在这里翻面 ✗）。两条读数都记在 observed 里。
    const text =
      'theorem rev_by (p : Prop) (hp : p) : p := by\n  sorry\n\n' +
      'theorem rev_term (p : Prop) (hp : p) : p := sorry\n';
    const file = fixture('a1b.sokonanoda', text);
    await ctx.lsp.open(file, text, 1);
    const shown = {};
    const texts = {};
    const exits = {};
    for (const nth of [1, 2]) {
      const pos = codePos(text, 'sorry', nth);
      const res = await ctx.lsp.raw('textDocument/codeAction', { textDocument: { uri: uriOf(file) }, range: { start: pos, end: pos }, context: { diagnostics: [] } });
      const actions = res.result || [];
      const first = actions[0];
      const insert = first ? editsOf(first).map((e) => e.newText).join('') : null;
      const edited = first ? applyEdits(text, editsOf(first)) : text;
      const g = grade(`a1b-out-${nth}.sokonanoda`, edited);
      const key = nth === 1 ? 'by 块里的洞' : '声明值位的洞';
      shown[key] = { 候选: actions.map((a) => a.title), 插入文本: insert };
      exits[key] = g.exit;
      texts[key] = edited;
    }
    const byInsert = shown['by 块里的洞'].插入文本;
    const termInsert = shown['声明值位的洞'].插入文本;
    const ok =
      typeof byInsert === 'string' &&
      typeof termInsert === 'string' &&
      byInsert.startsWith('exact ') &&
      !termInsert.startsWith('exact ') &&
      exits['by 块里的洞'] === 0 &&
      exits['声明值位的洞'] === 0;
    return {
      ok,
      observed: { ...shown, 'grade.exit': exits },
      evidence: `by 块 ⇒ ${JSON.stringify(byInsert)}（grade exit=${exits['by 块里的洞']}）· 值位 ⇒ ${JSON.stringify(termInsert)}（grade exit=${exits['声明值位的洞']}）`,
    };
  },
});

CASE({
  id: 'A1c-codeaction-tactic-ladder-grades',
  title: 'tactic 位上**每一条**候选都必须能直接过判卷（不止 exact —— `rfl`/`intro` 也得是 tactic 形态）',
  severity: 'control',
  blocker: false,
  expect: '`by` 块里每条候选应用后 `scripts/soko grade` exit 0（不许给出语法错误的候选）',
  async run(ctx) {
    const probes = [
      // exact + rfl 各一条（都是 tactic 位）
      ['exact 与 rfl', 'theorem tac_both (α : Type) (a : α) (h : Eq.{1} α a a) : Eq.{1} α a a := by\n  sorry\n'],
      // intro（目标 `p → q`；项位那条会插 `fun (x : p) => sorry` ⇒ 语法错误）
      ['intro', 'theorem tac_intro (p q : Prop) (h : q) : p → q := by\n  sorry\n'],
    ];
    const observed = {};
    let allGreen = true;
    for (const [label, text] of probes) {
      const file = fixture(`a1c-${label.length}.sokonanoda`, text);
      await ctx.lsp.open(file, text, 1);
      const pos = codePos(text, 'sorry', 1);
      const res = await ctx.lsp.raw('textDocument/codeAction', { textDocument: { uri: uriOf(file) }, range: { start: pos, end: pos }, context: { diagnostics: [] } });
      const actions = res.result || [];
      const rows = [];
      for (let i = 0; i < actions.length; i += 1) {
        const edited = applyEdits(text, editsOf(actions[i]));
        const g = grade(`a1c-${label.length}-${i}.sokonanoda`, edited);
        rows.push({ 标题: actions[i].title, 插入文本: editsOf(actions[i]).map((e) => e.newText).join(''), 'grade.exit': g.exit, 'grade.codes': g.codes });
        if (g.exit !== 0) allGreen = false;
      }
      observed[label] = rows;
      if (rows.length === 0) allGreen = false;
    }
    return {
      ok: allGreen,
      observed,
      evidence: allGreen
        ? `两组共 ${Object.values(observed).reduce((n, rows) => n + rows.length, 0)} 条候选全部判卷 exit 0`
        : `有不判绿的候选：${JSON.stringify(observed)}`,
    };
  },
});

CASE({
  id: 'A2-codeaction-term-hole-ok',
  title: '【对照】项位的洞：同一条 quick fix 应用后判卷**绿**（证明缺口只在 `by` 块那一侧）',
  severity: 'control',
  blocker: false,
  expect: '项位洞的插入文本应当过判卷（对照组，用来证明探针不是"一律判红"）',
  async run(ctx) {
    const text = 'theorem term_hole (p : Prop) (hp : p) : p := sorry\n';
    const file = fixture('a2.sokonanoda', text);
    await ctx.lsp.open(file, text, 1);
    const pos = codePos(text, 'sorry', 1);
    const res = await ctx.lsp.raw('textDocument/codeAction', { textDocument: { uri: uriOf(file) }, range: { start: pos, end: pos }, context: { diagnostics: [] } });
    const actions = res.result || [];
    const edited = actions[0] ? applyEdits(text, editsOf(actions[0])) : text;
    const g = grade('a2-out.sokonanoda', edited);
    return {
      ok: actions.length > 0 && g.exit === 0,
      observed: { 候选: actions.map((a) => a.title), 结果文本: edited, 'grade.exit': g.exit, checked: g.checked },
      evidence: `项位：newText=${JSON.stringify(actions[0] ? editsOf(actions[0]).map((e) => e.newText) : null)} ⇒ grade exit=${g.exit}`,
    };
  },
});

CASE({
  id: 'A3-codeaction-ladder-thin',
  title: '灯泡的梯子很窄：`by` 块里 rfl 能解的等式、∧ 目标、真课程画布的两个练习 ⇒ **0 条**',
  severity: 'medium',
  blocker: false,
  expect: '`by\\n sorry`（目标 `a = a`，`by rfl` 判卷绿）与 `∧` 目标应当至少给一条内核验证过的下一步',
  async run(ctx) {
    const probes = [
      ['by + a = a（by rfl 判卷绿）', 'theorem a3a (α : Type) (a : α) : a = a := by\n  sorry\n', 'sorry'],
      ['∧ 目标', 'theorem a3b (p q : Prop) (hp : p) (hq : q) : p ∧ q := by\n  sorry\n', 'sorry'],
    ];
    const observed = {};
    for (const [label, text, anchor] of probes) {
      const file = fixture(`a3-${label.length}.sokonanoda`, text);
      await ctx.lsp.open(file, text, 1);
      const pos = codePos(text, anchor, 1);
      const res = await ctx.lsp.raw('textDocument/codeAction', { textDocument: { uri: uriOf(file) }, range: { start: pos, end: pos }, context: { diagnostics: [] } });
      observed[label] = (res.result || []).map((a) => a.title);
    }
    // 真课程画布的两个练习（练习 5 / 练习 6 的 sorry）
    const canvasText = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, canvasText, 1);
    for (const nth of [1, 2]) {
      const pos = codePos(canvasText, 'sorry', nth);
      const res = await ctx.lsp.raw('textDocument/codeAction', { textDocument: { uri: uriOf(CANVAS) }, range: { start: pos, end: pos }, context: { diagnostics: [] } });
      observed[`画布练习 ${nth}（第 ${pos.line + 1} 行 sorry）`] = (res.result || []).map((a) => a.title);
    }
    const control = grade('a3-rfl-control.sokonanoda', 'theorem a3c (α : Type) (a : α) : a = a := by\n  rfl\n');
    const empty = Object.values(observed).every((v) => v.length === 0);
    return {
      ok: !empty,
      observed: { ...observed, '对照 by rfl 判卷': { exit: control.exit, checked: control.checked } },
      evidence: `四处全部 0 条（含真画布两个练习）；而同形状的 \`by rfl\` 判卷 exit=${control.exit}、checked=${JSON.stringify(control.checked)} ⇒ 建议梯子漏了 rfl/构造子这一类`,
    };
  },
});

CASE({
  id: 'A4-codeaction-eq-refl-missing',
  title: '判红声明的 quick fix 只有 `Iff` 那一半：`a = a := a` **0 条**，而 `Eq.refl α a` 判卷绿',
  severity: 'medium',
  blocker: false,
  expect: 'actions.rs 自己写明「kernel 验证过的 refl 整值替换（`Eq`/`Iff` 形状声明：`Eq.refl …` / `Iff.rfl`）」⇒ Eq 形状必须也给',
  async run(ctx) {
    const eqText = 'theorem fail_eq (α : Type) (a : α) : a = a := a\n';
    const eqFile = fixture('a4-eq.sokonanoda', eqText);
    await ctx.lsp.open(eqFile, eqText, 1);
    const eqRes = await ctx.lsp.raw('textDocument/codeAction', { textDocument: { uri: uriOf(eqFile) }, range: { start: { line: 0, character: 9 }, end: { line: 0, character: 9 } }, context: { diagnostics: [] } });
    const iffText = 'theorem fail_iff (p : Prop) : p ↔ p := p\n';
    const iffFile = fixture('a4-iff.sokonanoda', iffText);
    await ctx.lsp.open(iffFile, iffText, 1);
    const iffRes = await ctx.lsp.raw('textDocument/codeAction', { textDocument: { uri: uriOf(iffFile) }, range: { start: { line: 0, character: 9 }, end: { line: 0, character: 9 } }, context: { diagnostics: [] } });
    const control = grade('a4-control.sokonanoda', 'theorem fail_eq (α : Type) (a : α) : a = a := Eq.refl α a\n');
    const eqActions = (eqRes.result || []).map((a) => a.title);
    const iffActions = (iffRes.result || []).map((a) => a.title);
    return {
      ok: eqActions.length > 0,
      observed: { 'Eq 形状的候选': eqActions, 'Iff 形状的候选': iffActions, '对照 Eq.refl α a 判卷': { exit: control.exit, checked: control.checked } },
      evidence: `Eq 形状 0 条 vs Iff 形状 ${iffActions.length} 条；\`Eq.refl α a\` 判卷 exit=${control.exit}（checked=${JSON.stringify(control.checked)}）⇒ 漏的是 Eq 分支`,
    };
  },
});

CASE({
  id: 'A5-codeaction-duplicate',
  title: '同一个判红声明给两条**逐字节相同**的候选（只有标题不同）',
  severity: 'low',
  blocker: false,
  expect: '候选应当互不相同（相同编辑文本的候选是纯噪声）',
  async run(ctx) {
    const text = 'theorem fail_fun (p q : Prop) : p → p := fun (x : p) => q\n';
    const file = fixture('a5.sokonanoda', text);
    await ctx.lsp.open(file, text, 1);
    const res = await ctx.lsp.raw('textDocument/codeAction', { textDocument: { uri: uriOf(file) }, range: { start: { line: 0, character: 9 }, end: { line: 0, character: 9 } }, context: { diagnostics: [] } });
    const actions = res.result || [];
    const texts = actions.map((a) => JSON.stringify(editsOf(a).map((e) => [e.range.start.character, e.range.end.character, e.newText])));
    const dup = new Set(texts).size !== texts.length;
    return {
      ok: !dup,
      observed: { 候选: actions.map((a) => a.title), 编辑文本: texts },
      evidence: `两条候选的 (range, newText) 完全相同：${texts[0]}`,
    };
  },
});

// —— diagnostics ——

CASE({
  id: 'D1-diag-blames-wrong-line',
  title: '`by` 块里 tactic 拼错 ⇒ 报的是**签名里的 `∈`**（第 2 行）与「去声明记法」，真错在第 4 行',
  severity: 'high',
  blocker: false,
  expect: '诊断必须指到真正要改的那一列（`exactt`），且不该建议给一份已经 `import lib.Set` 的文件补记法声明',
  async run(ctx) {
    const text = 'import lib.Set\n\ntheorem d1 (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := by\n  exactt h\n';
    const file = fixture('d1.sokonanoda', text);
    const diags = await ctx.lsp.open(file, text, 1);
    const realLine = text.split('\n').findIndex((l) => l.includes('exactt'));
    const realCol = text.split('\n')[realLine].indexOf('exactt');
    const first = diags.diagnostics[0];
    const onRealToken = first && first.range.start.line === realLine && first.range.start.character >= realCol - 1;
    const cli = grade('d1-cli.sokonanoda', text);
    return {
      ok: Boolean(onRealToken),
      observed: {
        LSP诊断: diags.diagnostics.map((d) => [d.code, d.range.start.line, d.range.start.character, (d.message || '').split('\n')[0]]),
        真错位置: [realLine, realCol],
        'CLI grade codes': cli.codes,
      },
      evidence: `LSP 报 ${JSON.stringify(first && first.code)} @${first && first.range.start.line}:${first && first.range.start.character}（签名里的 ∈），真错在 @${realLine}:${realCol}；CLI 同文本报 ${JSON.stringify(cli.codes)}（带真位置）`,
    };
  },
});

CASE({
  id: 'D2-diag-half-folded',
  title: '类型不匹配消息**一半折叠一半点名**：期望 `a ∈ A`，实际是 `Set.mem b B`',
  severity: 'medium',
  blocker: false,
  expect: '同一句里的两侧应当同一种渲染（都用记法，或都用点名）',
  async run(ctx) {
    const text = 'import lib.Set\n\ntheorem d2 (α : Type) (a b : α) (A B : Set α) (hb : b ∈ B) : a ∈ A := by\n  exact hb\n';
    const file = fixture('d2.sokonanoda', text);
    const diags = await ctx.lsp.open(file, text, 1);
    const msg = (diags.diagnostics[0] || {}).message || '';
    const expected = /期望 `([^`]*)`/.exec(msg);
    const actual = /实际是 `([^`]*)`/.exec(msg);
    const folded = (s) => /[∈⊆∧↔¬]/.test(s || '');
    return {
      ok: folded(expected && expected[1]) === folded(actual && actual[1]),
      observed: { message: msg, 期望: expected && expected[1], 实际: actual && actual[1], range: diags.diagnostics[0] && [diags.diagnostics[0].range.start.line, diags.diagnostics[0].range.start.character, diags.diagnostics[0].range.end.character] },
      evidence: `期望侧折叠（\`${expected && expected[1]}\`）、实际侧点名（\`${actual && actual[1]}\`）⇒ 同一句两种渲染`,
    };
  },
});

CASE({
  id: 'D3-diag-sorry-english',
  title: '`sorry` 警告是**英文**、覆盖整条声明；空 `by` 也谎称「uses \'sorry\'」',
  severity: 'low',
  blocker: false,
  expect: '消息与其余诊断一致用中文；没有 `sorry` 的文本不该说 uses sorry（空 `by` 是"未完成"，不是"sorry"）',
  async run(ctx) {
    const holeText = 'theorem d3a (p : Prop) (hp : p) : p := by\n  sorry\n';
    const holeFile = fixture('d3a.sokonanoda', holeText);
    const hd = await ctx.lsp.open(holeFile, holeText, 1);
    const emptyText = 'theorem d3b (p : Prop) (hp : p) : p := by\n';
    const emptyFile = fixture('d3b.sokonanoda', emptyText);
    const ed = await ctx.lsp.open(emptyFile, emptyText, 1);
    const h = hd.diagnostics[0] || {};
    const e = ed.diagnostics[0] || {};
    const asciiOnly = (m) => !/[\u4e00-\u9fff]/.test(m || '');
    return {
      ok: !asciiOnly(h.message) && !/uses 'sorry'/.test(e.message || ''),
      observed: {
        'sorry 洞': { code: h.code, severity: h.severity, range: [h.range.start.line, h.range.start.character, h.range.end.line, h.range.end.character], message: h.message },
        '空 by（源里没有 sorry）': { code: e.code, severity: e.severity, message: e.message },
      },
      evidence: `洞：severity=${h.severity}、range 覆盖整条声明、消息纯英文；空 by：消息 ${JSON.stringify(e.message)}（源里没有 sorry）`,
    };
  },
});

CASE({
  id: 'D4-diag-clears-on-fix',
  title: '【对照】改对之后诊断**及时消失**（带版本回显）',
  severity: 'control',
  blocker: false,
  expect: '修好后同一份文档的下一个版本必须是 0 条诊断（对照组）',
  async run(ctx) {
    const bad = 'import lib.Set\n\ntheorem d4 (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := by\n  exact hb\n';
    const good = bad.replace('  exact hb\n', '  exact h\n');
    const file = fixture('d4.sokonanoda', bad);
    await ctx.lsp.open(file, bad, 1);
    const after = await ctx.lsp.change(file, good, 2);
    return {
      ok: after.diagnostics.length === 0 && after.version === 2,
      observed: { version: after.version, diagnostics: after.diagnostics.map((d) => [d.code, d.range.start.line]) },
      evidence: `修好后 version=${after.version}、诊断 ${after.diagnostics.length} 条`,
    };
  },
});

CASE({
  id: 'D5-diag-placeholder-leak',
  title: '`apply Eq.subst`（实参给少）的消息里漏出占位符 `???` 与无关的「未完成的证明请写 sorry」',
  severity: 'medium',
  blocker: false,
  expect: '消息应当是"缺实参"的人话（`???` 是 token.rs 里已移除语法的旧占位符文案）',
  async run(ctx) {
    const text = 'import lib.Set\n\ntheorem d5 (α : Type) (a b : α) (A B : Set α) (h : a ∈ A) : a ∈ A := by\n  apply Eq.subst\n';
    const file = fixture('d5.sokonanoda', text);
    const diags = await ctx.lsp.open(file, text, 1);
    const msg = (diags.diagnostics[0] || {}).message || '';
    return {
      ok: !msg.includes('???'),
      observed: { code: (diags.diagnostics[0] || {}).code, message: msg },
      evidence: `消息 = ${JSON.stringify(msg.split('\n')[0])}`,
    };
  },
});

// —— goal 视图 ——

CASE({
  id: 'G1-goal-redundant-parens',
  title: '目标/提示的显示层加**多余括号**：源 `({a} ⊆ A) ↔ a ∈ A` 渲染成 `(({a}) ⊆ A) ↔ (a ∈ A)`',
  severity: 'low',
  blocker: false,
  expect: '目标文本应当与源同形（记法 `{a}` / `a ∈ A` 是原子，不需要括号）',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    const pos = codePos(text, 'sorry', 1);
    const state = await ctx.lsp.request('soko/stateAt', { textDocument: { uri: uriOf(CANVAS) }, position: pos });
    const goal = state.goal;
    const hints = await ctx.lsp.request('textDocument/inlayHint', { textDocument: { uri: uriOf(CANVAS) }, range: { start: { line: 0, character: 0 }, end: { line: 200, character: 0 } } });
    const label = (hints || []).map((h) => h.label).find((l) => l.includes('{a}')) || '';
    const source = '({a} ⊆ A) ↔ a ∈ A';
    const noise = goal.includes('({a})') || goal.includes('(a ∈ A)');
    return {
      ok: !noise,
      observed: { 源: source, stateAt_goal: goal, inlayHint_label: label, inlayHint_tooltip: ((hints || [])[0] || {}).tooltip },
      evidence: `goal=${JSON.stringify(goal)}（多出 \`({a})\` 与 \`(a ∈ A)\` 两处括号）；inlayHint label=${JSON.stringify(label)}`,
    };
  },
});

CASE({
  id: 'G2-stateat-entering-vs-after',
  title: '【对照】`soko/stateAt` 的「进/出 tactic」语义正确（起点=进入态、内部=之后态）',
  severity: 'control',
  blocker: false,
  expect: '按 protocol.md：tactic 起点给进入态、严格内部给之后态（对照组）',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    const at = codePos(text, 'apply Set.ext', 1);
    const enter = await ctx.lsp.request('soko/stateAt', { textDocument: { uri: uriOf(CANVAS) }, position: at });
    const inside = await ctx.lsp.request('soko/stateAt', { textDocument: { uri: uriOf(CANVAS) }, position: { line: at.line, character: at.character + 3 } });
    return {
      ok: enter.goal === 'A = B' && inside.goal !== 'A = B' && enter.step === -1 && inside.step === 0,
      observed: { 起点: { step: enter.step, goal: enter.goal }, 内部: { step: inside.step, goal: inside.goal } },
      evidence: `起点 step=${enter.step} goal=${JSON.stringify(enter.goal)}；内部 step=${inside.step} goal=${JSON.stringify(inside.goal)}`,
    };
  },
});

// —— 导航 ——

CASE({
  id: 'N1-nav-uri-unnormalized',
  title: 'F12 落点的 `file://` URI **没有规范化**：`…/crates/front/../../prelude/Prelude.sokonanoda`',
  severity: 'medium',
  blocker: false,
  expect: 'Location.uri 应当是规范路径（同一个文件只有一个 URI 身份）',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    const out = {};
    for (const [label, needle] of [['Eq.subst（prelude）', 'Eq.subst'], ['tactic: intro（reference/）', 'intro'], ['#check（reference/）', '#check'], ['Set.mem（lib，工作区内）', 'Set.mem']]) {
      const pos = codePos(text, needle, 1);
      const res = await ctx.lsp.raw('textDocument/definition', { textDocument: { uri: uriOf(CANVAS) }, position: pos });
      const loc = res.result && (Array.isArray(res.result) ? res.result[0] : res.result);
      out[label] = loc ? decodeURIComponent(loc.uri) : null;
    }
    const dirty = Object.values(out).filter((u) => u && u.includes('/../'));
    return {
      ok: dirty.length === 0,
      observed: out,
      evidence: `${dirty.length} 条落点带 \`/../\`（prelude 与 reference 都经 \`crates/front/../..\` 拼出来）；工作区内的 lib 落点是干净的`,
    };
  },
});

CASE({
  id: 'N2-nav-one-bad-decl-kills-file',
  title: '一条**没写完的声明**让整份文件的 hover/F12/大纲/灯泡**全灭**（只有补全还答，因为它是位置无关的）',
  severity: 'high',
  blocker: false,
  expect: '别的声明（`ok_one`/`ok_two`）本身是好的 ⇒ 至少 hover / documentSymbol 应当照常答（G-20/G-108 的方向）',
  async run(ctx) {
    const text = 'theorem ok_one (p : Prop) (hp : p) : p := hp\n\ntheorem ok_two (p : Prop) (hp : p) : p := ok_one p hp\n\ntheorem broken (p : Prop) : p := \n';
    const file = fixture('n2.sokonanoda', text);
    const diags = await ctx.lsp.open(file, text, 1);
    const lines = text.split('\n');
    const l0 = lines.findIndex((l) => l.includes('theorem ok_one'));
    const c0 = lines[l0].indexOf('ok_one');
    const l1 = lines.findIndex((l) => l.includes('ok_one p hp'));
    const c1 = lines[l1].indexOf('ok_one');
    const hover = await ctx.lsp.raw('textDocument/hover', { textDocument: { uri: uriOf(file) }, position: { line: l0, character: c0 } });
    const def = await ctx.lsp.raw('textDocument/definition', { textDocument: { uri: uriOf(file) }, position: { line: l1, character: c1 } });
    const sym = await ctx.lsp.raw('textDocument/documentSymbol', { textDocument: { uri: uriOf(file) } });
    const ca = await ctx.lsp.raw('textDocument/codeAction', { textDocument: { uri: uriOf(file) }, range: { start: { line: l0, character: c0 }, end: { line: l0, character: c0 } }, context: { diagnostics: [] } });
    const comp = await ctx.lsp.raw('textDocument/completion', { textDocument: { uri: uriOf(file) }, position: { line: l1, character: c1 } });
    const dead = hoverText(hover.result) === null && def.result === null && sym.result === null && (ca.result || []).length === 0;
    return {
      ok: !dead,
      observed: {
        诊断: diags.diagnostics.map((d) => [d.code, d.range.start.line, (d.message || '').split('\n')[0]]),
        'hover(ok_one)': hoverText(hover.result),
        'definition(ok_one 使用处)': def.result,
        documentSymbol: sym.result,
        'codeAction(ok_one)': (ca.result || []).map((a) => a.title),
        'completion 条数（仍答）': (comp.result || []).length,
      },
      evidence: `只有第 5 行一条 \`unexpected-token\`（未写完的声明），但 ok_one/ok_two 的 hover=null、F12=null、documentSymbol=null、灯泡=0 条`,
    };
  },
});

CASE({
  id: 'N3-nav-use-to-binder-ok',
  title: '【对照】使用处 → binder 的 F12 正常（canvas `exact h a`）',
  severity: 'control',
  blocker: false,
  expect: 'tactic 里的 `h` 应当跳到签名里的 binder（对照组）',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    const lines = text.split('\n');
    const line = lines.findIndex((l) => l === '  exact h a');
    const col = lines[line].indexOf('h');
    const res = await ctx.lsp.raw('textDocument/definition', { textDocument: { uri: uriOf(CANVAS) }, position: { line, character: col } });
    const loc = res.result && (Array.isArray(res.result) ? res.result[0] : res.result);
    return {
      ok: Boolean(loc && loc.range.start.line === 42),
      observed: { 落点: loc && [loc.range.start.line, loc.range.start.character, loc.range.end.character] },
      evidence: `h @${line}:${col} ⇒ ${JSON.stringify(loc && [loc.range.start.line, loc.range.start.character])}`,
    };
  },
});

CASE({
  id: 'N4-nav-selection-range',
  title: '`documentSymbol.selectionRange` 等于整条声明（应当是名字 token）',
  severity: 'low',
  blocker: false,
  expect: 'selectionRange ⊂ range，且正好是声明的名字 token（大纲/面包屑/改名都依赖它）',
  async run(ctx) {
    const text = fs.readFileSync(CANVAS, 'utf8');
    await ctx.lsp.open(CANVAS, text, 1);
    const syms = await ctx.lsp.request('textDocument/documentSymbol', { textDocument: { uri: uriOf(CANVAS) } });
    const same = (syms || []).filter((s) => JSON.stringify(s.range) === JSON.stringify(s.selectionRange));
    return {
      ok: same.length === 0,
      observed: { 条数: (syms || []).length, 'selectionRange == range': same.length, 例: (syms || []).slice(0, 2).map((s) => [s.name, s.range, s.selectionRange]) },
      evidence: `${same.length}/${(syms || []).length} 条的 selectionRange 与 range 逐字节相同（整条声明，而不是名字）`,
    };
  },
});

// —— 环境（构建身份 / 版本口径） ——

CASE({
  id: 'E1-version-marker-misleading',
  title: '`scripts/soko version --json` 把**缓存 marker** 当成仓库构建的 marker 报告（0.90.0 仓库报 0.73.0）',
  severity: 'low',
  blocker: false,
  expect: '`source: "repo-build"` 的条目其 marker 要么是这份构建的真实版本，要么明说是缓存 marker',
  async run(ctx) {
    const out = execFileSync(path.join(ROOT, 'scripts', 'soko'), ['version', '--json'], { encoding: 'utf8' });
    const info = JSON.parse(out);
    const self = await ctx.lsp.request('soko/version', {});
    const mismatch = info.version !== self.version || (info.lsp.marker || '').split(' ')[0] !== self.version;
    return {
      ok: !mismatch,
      observed: { 'version.version': info.version, 'version.lsp': info.lsp, 'soko/version': self },
      evidence: `version --json 报 lsp.marker=${JSON.stringify(info.lsp.marker)}（source=${info.lsp.source}），而服务器自述 ${JSON.stringify(self.version)} ⇒ marker 是缓存目录的，不是这份构建的`,
    };
  },
});

// ── 运行器 ─────────────────────────────────────────────────────────────────

function parseArgs(argv) {
  const opts = { case: null, all: false, list: false, json: false, exe: null, timeout: 60000 };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (a === '--case') opts.case = argv[++i];
    else if (a === '--all') opts.all = true;
    else if (a === '--list') opts.list = true;
    else if (a === '--json') opts.json = true;
    else if (a === '--exe') opts.exe = argv[++i];
    else if (a === '--timeout') opts.timeout = Number(argv[++i]);
    else if (a === '--help' || a === '-h') opts.help = true;
    else {
      process.stderr.write(`未知参数 ${a}\n`);
      opts.help = true;
    }
  }
  return opts;
}

function usage() {
  process.stdout.write(
    [
      'usage: node scripts/ux-probes/lsp-probe.mjs (--list | --case <id> | --all) [--json] [--exe <lsp 二进制>]',
      '',
      'exit 0 = 观测与用例 expect 一致 · 1 = 不一致（缺口仍在）· 2 = 环境缺失',
      '',
      ...CASES.map((c) => `  ${c.id.padEnd(32)} [${c.severity}] ${c.title}`),
      '',
    ].join('\n'),
  );
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  if (opts.help || (!opts.case && !opts.all && !opts.list)) {
    usage();
    process.exit(opts.help ? 0 : 2);
  }
  if (opts.list) {
    usage();
    process.exit(0);
  }
  const selected = opts.all ? CASES : CASES.filter((c) => c.id === opts.case);
  if (selected.length === 0) {
    process.stderr.write(`没有这条用例：${opts.case}\n`);
    process.exit(2);
  }

  ensureFixtures();
  let head = 'unknown';
  try {
    head = execFileSync('git', ['rev-parse', '--short', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).trim();
  } catch (_) {
    /* 不在 git 里 */
  }

  const lsp = new Lsp({ exe: opts.exe }).start();
  let build = null;
  try {
    await lsp.initialize(FIXTURES);
    build = await lsp.request('soko/version', {});
  } catch (error) {
    process.stderr.write(`环境缺失：起不了 LSP（${error.message}）\n${lsp.stderr.slice(0, 400)}\n`);
    process.exit(2);
  }

  const ctx = { lsp, head, build, fixtures: FIXTURES, canvas: CANVAS, root: ROOT };
  const results = [];
  for (const spec of selected) {
    const started = Date.now();
    let result;
    try {
      const r = await Promise.race([
        spec.run(ctx),
        new Promise((_, reject) => setTimeout(() => reject(new Error('用例超时')), opts.timeout)),
      ]);
      result = { ...r, error: null };
    } catch (error) {
      result = {
        ok: false,
        observed: null,
        evidence: `用例抛错：${error.message}`,
        error: error.message,
        // 夹具/锚点问题 ⇒ **判据不可用**（不是"缺口仍在"）⇒ 上层走 exit 2 ✓
        unusable: Boolean(error && error.fixture),
      };
    }
    results.push({
      id: spec.id,
      title: spec.title,
      severity: spec.severity,
      blocker: Boolean(spec.blocker),
      expect: spec.expect,
      ok: result.ok,
      unusable: Boolean(result.unusable),
      observed: result.observed,
      evidence: result.evidence,
      ms: Date.now() - started,
    });
  }
  lsp.stop();

  const unusable = results.filter((r) => r.unusable);
  const gaps = results.filter((r) => !r.ok && !r.unusable);
  if (opts.json) {
    process.stdout.write(
      JSON.stringify(
        {
          tool: 'soko-lsp-ux-probe',
          build: { head, 'soko/version': build, exe: opts.exe || `node scripts/soko lsp` },
          summary: {
            total: results.length,
            consistent: results.filter((r) => r.ok).length,
            inconsistent: gaps.length,
            unusable: unusable.length,
          },
          cases: results,
        },
        null,
        2,
      ) + '\n',
    );
  } else {
    process.stdout.write(`构建身份：HEAD=${head} ｜ soko/version=${JSON.stringify(build)}\n\n`);
    for (const r of results) {
      const tag = r.ok ? '一致 ✓' : '不一致 ✗';
      process.stdout.write(`${tag}  ${r.id}  [${r.severity}] ${r.title}\n`);
      process.stdout.write(`      期望：${r.expect}\n`);
      process.stdout.write(`      现场：${r.evidence}\n`);
      process.stdout.write(`      观测：${JSON.stringify(r.observed)}\n\n`);
    }
    process.stdout.write(
      `合计 ${results.length} 条：一致 ${results.filter((r) => r.ok).length} · 不一致 ${gaps.length}` +
        (unusable.length ? ` · **判据不可用 ${unusable.length}**` : '') +
        '\n',
    );
    if (gaps.length) process.stdout.write(`缺口：${gaps.map((g) => g.id).join(' ')}\n`);
    if (unusable.length) {
      process.stdout.write(
        `判据不可用（**不是缺口**）：${unusable.map((g) => g.id).join(' ')}` +
          ' ⇒ 锚点/夹具在**当前画布**上不存在（画布重写后未跟上）⇒ 修判据，别当成产品缺口 ✗\n',
      );
    }
  }
  // **判据不可用优先于缺口**：读不出来就不许判 ✓（`judge_rc` 把 2 判成 `error`）
  process.exit(unusable.length ? 2 : gaps.length ? 1 : 0);
}

main().catch((error) => {
  process.stderr.write(`探针异常：${error && error.stack ? error.stack : error}\n`);
  process.exit(2);
});
