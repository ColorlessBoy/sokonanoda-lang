#!/usr/bin/env node
// **G-99 自断言复现**：tactic 里的**名字**上没有「它本身的类型」（只有 goal state）。
//
// 用户原话（2026-10-09/10）：「`apply Set.ext` 这种 tactic 下，我鼠标在 `Set.ext` 上时，
// 我希望 hover 的内容除了当前的 goal state，**分割线后再加上 `Set.ext` 本身的类型内容**」。
//
// 缺口（修前实测，真 LSP）：`tactic_goal_hover()` 对「光标落在任何 `by` step span 内」
// **无条件早退**（在 hover 链最前面），只读 `by_steps` 的 goal 快照、**不解析 tactic 里的
// 名字** ⇒ 光标在 `Set.ext` 上与在 `apply` 关键字上**逐字节相同**（都只有 tactic + `tactic i/n`
// + goal state）✗。
//
// 修后契约（2026-10-10，`docs/protocol.md` §Tactic goal-state hover）：
//   * 光标在名字上 ⇒ 现有内容 + **Markdown 水平线 `---`**（独占一行、前后各一空行）
//     + 一行该名字的类型（折记法、与其它 hover 同一观感）；
//   * 光标在 **tactic 关键字**上 ⇒ **一个字节都不加**（不含分割线）；
//   * 拿不到干净类型（含 `$N` 松散变量 / unknown identifier）⇒ **不编那一行**。
//
// 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
//   0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（需要人看）

'use strict';

const { spawn } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SOKO = path.join(ROOT, 'scripts', 'soko');

const uri = (f) => 'file://' + f;

function startLsp() {
  const child = spawn(process.execPath, [SOKO, 'lsp'], { stdio: ['pipe', 'pipe', 'pipe'] });
  let buffer = Buffer.alloc(0);
  const queue = [];
  const waiters = [];
  child.stdout.on('data', (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    for (;;) {
      const sep = buffer.indexOf('\r\n\r\n');
      if (sep < 0) return;
      const m = /Content-Length: (\d+)/i.exec(buffer.slice(0, sep).toString('utf8'));
      if (!m) return;
      const len = Number(m[1]);
      if (buffer.length < sep + 4 + len) return;
      const body = buffer.slice(sep + 4, sep + 4 + len).toString('utf8');
      buffer = buffer.slice(sep + 4 + len);
      const msg = JSON.parse(body);
      if (waiters.length) waiters.shift()(msg);
      else queue.push(msg);
    }
  });
  child.stderr.on('data', () => {});
  return {
    send(msg) {
      const body = Buffer.from(JSON.stringify(msg), 'utf8');
      child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
      child.stdin.write(body);
    },
    next(timeoutMs = 120000) {
      if (queue.length) return Promise.resolve(queue.shift());
      return new Promise((resolve, reject) => {
        const t = setTimeout(() => reject(new Error('LSP timeout')), timeoutMs);
        waiters.push((m) => {
          clearTimeout(t);
          resolve(m);
        });
      });
    },
    stop() {
      try {
        child.kill();
      } catch {
        /* ignore */
      }
    },
  };
}

async function responseFor(lsp, id) {
  for (;;) {
    const m = await lsp.next();
    if (m.id === id) return m;
  }
}

function posOf(text, offset) {
  const before = text.slice(0, offset);
  return {
    line: before.split('\n').length - 1,
    character: (before.split('\n').pop() || '').length,
  };
}

async function main() {
const problems = [];
const ok = (cond, msg) => {
  if (!cond) problems.push(msg);
};

// ---------- 夹具：`apply Set.ext`（与用户现场同形） ----------
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g99-appl-'));
fs.writeFileSync(path.join(dir, 'sokonanoda.toml'), 'entry = "Canvas.sokonanoda"\n');
fs.writeFileSync(
  path.join(dir, 'SetLib.sokonanoda'),
  [
    'def Set (α : Type) : Type := α -> Prop',
    'def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a',
    'axiom Set.ext {α : Type} {A B : Set α} : (forall (x : α), A x ↔ B x) -> Eq.{1} (Set α) A B',
    'infix:50 " ∈ " => Set.mem',
    '',
  ].join('\n'),
);
const src = [
  'import SetLib',
  '',
  'theorem ext_test (α : Type) (A B : Set α) (h : forall (x : α), A x ↔ B x) :',
  '    Eq.{1} (Set α) A B := by',
  '  apply Set.ext',
  '  exact h',
  '',
].join('\n');
const file = path.join(dir, 'Canvas.sokonanoda');
fs.writeFileSync(file, src);

const lsp = startLsp();
lsp.send({
  jsonrpc: '2.0',
  id: 1,
  method: 'initialize',
  params: {
    processId: process.pid,
    rootUri: uri(dir),
    capabilities: {},
    workspaceFolders: [{ uri: uri(dir), name: 'repro' }],
  },
});
await responseFor(lsp, 1);
lsp.send({ jsonrpc: '2.0', method: 'initialized', params: {} });
lsp.send({
  jsonrpc: '2.0',
  method: 'textDocument/didOpen',
  params: { textDocument: { uri: uri(file), languageId: 'sokonanoda', version: 1, text: src } },
});
for (;;) {
  const m = await lsp.next();
  if (m.method === 'textDocument/publishDiagnostics' && m.params?.uri === uri(file)) break;
}
const ask = async (id, pos) => {
  lsp.send({
    jsonrpc: '2.0',
    id,
    method: 'textDocument/hover',
    params: { textDocument: { uri: uri(file) }, position: pos },
  });
  return (await responseFor(lsp, id)).result ?? null;
};
const nameOff = src.indexOf('Set.ext');
const onName = await ask(10, posOf(src, nameOff + 1)); // 用户实际指的位置：`Set.ext` 的字符上
const onKeyword = await ask(11, posOf(src, src.indexOf('apply') + 1)); // 反向：`apply` 关键字上
lsp.stop();

const text = (h) => (h && h.contents && h.contents.value) || '';
const nameHover = text(onName);
const kwHover = text(onKeyword);

ok(nameHover.length > 0, '`Set.ext` 上 hover 是 null（连 goal state 都没了？）');
ok(/⊢/.test(nameHover), '`Set.ext` 上 hover 丢了 goal state 块');
ok(nameHover.includes('\n\n---\n\n'), '`Set.ext` 上 hover 没有分割线（`---` 独占一行、前后各一空行）');
ok(/`Set\.ext : /.test(nameHover), '`Set.ext` 上 hover 没有 `Set.ext` 的类型行');

ok(kwHover.length > 0, '`apply` 关键字上 hover 是 null（goal state 不该丢）');
ok(/⊢/.test(kwHover), '`apply` 关键字上 hover 丢了 goal state 块');
ok(!kwHover.includes('---'), '`apply` 关键字上**也**加了分割线/类型行（防「整条 tactic 一律加」✗）');

if (problems.length === 0) {
  console.log('G-99 已修 ✓ —— 名字上有「分割线 + 类型行」，关键字上一个字节都没加');
  process.exit(1);
}
console.log('G-99 缺口仍在（或出现回归）✗：');
for (const p of problems) console.log('  - ' + p);
console.log('--- hover on `Set.ext` ---\n' + nameHover);
console.log('--- hover on `apply` ---\n' + kwHover);
process.exit(0);
}

main().catch((e) => {
  console.error('G-99' + ' 探针异常（按环境问题处理，不是「已修」）：', e && e.message ? e.message : e);
  process.exit(2);
});
