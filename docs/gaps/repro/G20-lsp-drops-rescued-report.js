#!/usr/bin/env node
// G-20 自断言复现：**单文件 parse 失败不得吃掉项目报告**（X15）。
//
// 形状（课程单元的常态）：记法随 `import` 传播（G-04 第二刀）⇒ 用库记法的入口
// 文件**单独 parse 必然失败**（`∈` 不在本文件里），而**闭包是好的**。
//
// 台账 `today`（0.61.0 实测，修前）：
//   LSP `publishDiagnostics` 发**一条假诊断** `notation-unknown-symbol`，
//   `textDocument/documentSymbol` 回答 `null`、`textDocument/hover` 回答 `null`；
//   而**同一份文本**走 CLI `grade` 是 exit 0。根因在 `crates/lsp/src/lib.rs`：
//   `set_text_with_overlay` 已经装好了闭包报告（`query/mod.rs`），LSP 紧接着因
//   `parse_error.is_some()` 把 `report = None` 丢掉。
//
// 修后契约（设计 `docs/design/notation-input.md` §1.1 的 R-5）：
//   ① 闭包编译成功 ⇒ 诊断以**闭包报告**为准（0 条），hover / documentSymbol 都活；
//   ② 闭包**也**失败（入口真有语法错误）⇒ 仍按老契约发 parse 错误。
//
// 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
//   0 = 缺口仍在（假诊断 / hover 为 null） · 1 = 已修（修后形状成立）
//   2 = 环境/形状异常（需要人看）

'use strict';

const { spawn } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const LSP = path.join(ROOT, 'scripts', 'soko');

const LIB = 'def Set (α : Type) : Type := α -> Prop\n' +
  'def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n' +
  'infix:50 " ∈ " => Set.mem\n';

const CANVAS = 'import SetLib\n\n' +
  'theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n';

const BROKEN = 'import SetLib\n\ntheorem t (α : Type) (a : α) (A : Set α) : a ∈ A :=\n';

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g20-lsp-'));
fs.writeFileSync(path.join(dir, 'SetLib.sokonanoda'), LIB);

function uri(file) {
  return 'file://' + file;
}

/** 起一个 LSP 进程，返回 { send, next, stop }。 */
function startLsp() {
  const child = spawn(process.execPath, [LSP, 'lsp'], { stdio: ['pipe', 'pipe', 'pipe'] });
  let buffer = Buffer.alloc(0);
  const queue = [];
  const waiters = [];
  child.stdout.on('data', (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    for (;;) {
      const sep = buffer.indexOf('\r\n\r\n');
      if (sep < 0) return;
      const header = buffer.slice(0, sep).toString('utf8');
      const match = /Content-Length: (\d+)/i.exec(header);
      if (!match) return;
      const length = Number(match[1]);
      if (buffer.length < sep + 4 + length) return;
      const body = buffer.slice(sep + 4, sep + 4 + length).toString('utf8');
      buffer = buffer.slice(sep + 4 + length);
      const message = JSON.parse(body);
      if (waiters.length) waiters.shift()(message);
      else queue.push(message);
    }
  });
  child.stderr.on('data', () => {});
  return {
    send(message) {
      const body = Buffer.from(JSON.stringify(message), 'utf8');
      child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
      child.stdin.write(body);
    },
    next(timeoutMs = 20000) {
      if (queue.length) return Promise.resolve(queue.shift());
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('LSP 超时未应答')), timeoutMs);
        waiters.push((message) => {
          clearTimeout(timer);
          resolve(message);
        });
      });
    },
    stop() {
      try { child.kill(); } catch (_) { /* 已退出 */ }
    },
  };
}

/** 等到某个 id 的应答（中间的消息丢掉）。 */
async function responseFor(lsp, id) {
  for (;;) {
    const message = await lsp.next();
    if (message.id === id) return message;
  }
}

/** 等第一条针对 `file` 的 publishDiagnostics。 */
async function diagnosticsFor(lsp, file) {
  for (;;) {
    const message = await lsp.next();
    if (message.method === 'textDocument/publishDiagnostics' &&
        message.params && message.params.uri === uri(file)) {
      return message.params.diagnostics || [];
    }
  }
}

async function openAndProbe(name, text) {
  const file = path.join(dir, name);
  fs.writeFileSync(file, text);
  const lsp = startLsp();
  lsp.send({
    jsonrpc: '2.0', id: 1, method: 'initialize',
    params: {
      processId: process.pid,
      rootUri: uri(dir),
      capabilities: {},
      workspaceFolders: [{ uri: uri(dir), name: 'g20' }],
    },
  });
  await responseFor(lsp, 1);
  lsp.send({ jsonrpc: '2.0', method: 'initialized', params: {} });
  lsp.send({
    jsonrpc: '2.0', method: 'textDocument/didOpen',
    params: { textDocument: { uri: uri(file), languageId: 'sokonanoda', version: 1, text } },
  });
  const diagnostics = await diagnosticsFor(lsp, file);

  // hover 落在 `∈` 上（第一个使用处）。
  const offset = text.indexOf('∈', text.indexOf('theorem'));
  const before = text.slice(0, offset);
  const line = before.split('\n').length - 1;
  const character = [...(before.split('\n').pop() || '')].length;
  lsp.send({
    jsonrpc: '2.0', id: 2, method: 'textDocument/hover',
    params: { textDocument: { uri: uri(file) }, position: { line, character } },
  });
  const hover = await responseFor(lsp, 2);
  lsp.send({
    jsonrpc: '2.0', id: 3, method: 'textDocument/documentSymbol',
    params: { textDocument: { uri: uri(file) } },
  });
  const symbols = await responseFor(lsp, 3);
  lsp.stop();
  return { diagnostics, hover: hover.result, symbols: symbols.result };
}

function codes(diagnostics) {
  return diagnostics.map((d) => (d && d.code) || '?');
}

(async () => {
  let failed = false;

  console.log('== ① 闭包救得回来的入口：必须 0 诊断 + hover 活 + documentSymbol 活 ==');
  const rescued = await openAndProbe('Canvas.sokonanoda', CANVAS);
  console.log('   诊断 =', JSON.stringify(codes(rescued.diagnostics)));
  console.log('   hover =', rescued.hover === null ? 'null' : '有内容');
  console.log('   documentSymbol =', rescued.symbols === null ? 'null' : `${rescued.symbols.length} 项`);
  const fake = codes(rescued.diagnostics).includes('notation-unknown-symbol');
  if (fake) {
    console.log('   → 假的 `notation-unknown-symbol`（G-20 仍在）');
    failed = true;
  }
  if (rescued.hover === null || rescued.hover === undefined) {
    console.log('   → hover 为 null（G-20 仍在：报告被丢掉了）');
    failed = true;
  }
  if (rescued.symbols === null || rescued.symbols === undefined) {
    console.log('   → documentSymbol 为 null（G-20 仍在）');
    failed = true;
  }

  console.log();
  console.log('== ② 对照组：入口**自己**有语法错误 ⇒ 老契约不许被放宽（必须有诊断）==');
  const broken = await openAndProbe('Broken.sokonanoda', BROKEN);
  console.log('   诊断 =', JSON.stringify(codes(broken.diagnostics)));
  if (broken.diagnostics.length === 0) {
    console.log('   → 真有语法错误却不报（契约被放宽了）⇒ 需要人看');
    console.log('结论：形状既不是旧缺口也不是修后契约。');
    process.exit(2);
  }

  console.log();
  if (failed) {
    console.log('结论：G-20 仍在——单文件 parse 失败吃掉了闭包报告。');
    process.exit(0);
  }
  console.log('结论：G-20 已修——闭包编译成功时报告存活（0 假诊断 + hover/documentSymbol 可用），');
  console.log('      闭包也失败时仍发 parse 错误。');
  process.exit(1);
})().catch((error) => {
  console.error('环境/形状异常：', error && error.message);
  process.exit(2);
}).finally(() => {
  fs.rmSync(dir, { recursive: true, force: true });
});
