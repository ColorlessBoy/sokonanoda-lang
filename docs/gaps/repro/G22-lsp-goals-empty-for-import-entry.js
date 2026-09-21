#!/usr/bin/env node
// G-22 自断言复现：**项目入口（含 `import` 且用库记法）的 `soko/goals` 恒为空**。
//
// 形状（课程单元的常态）：`import` 来的记法让入口**单独 parse 必然失败**
// （`∈` 不在本文件里），而**闭包是好的**。G-20 修好了 `report`（诊断/hover/
// documentSymbol 都活），但 `QueryDoc::goals` 的第一行 `self.parsable()?`
// （`crates/front/src/query/mod.rs:481` → `:403-408`）**没有**那条例外
// ⇒ `Err(NotParsable)` ⇒ LSP 侧 `.unwrap_or_default()`（`crates/lsp/src/lib.rs:495`）
// ⇒ `decls: []`。Infoview 的「声明」栏因此恒为「暂无声明。」，
// 而同一份文档的「目标」栏（走 `soko/stateAt`，读 `report`）是好的。
//
// 台账 `today`（0.63.0 实测，修前）：
//   units/unit01 = 0 条声明 / documentSymbol 8 项；
//   units/unit08 = 0 条 / 27 项；notation-cheatsheet = 0 条 / 23 项；
//   而无 `import` 的 course/unit1 = 13 条 / 13 项。
//
// 修后契约：项目入口的 `soko/goals` 非空，且**条数与 `documentSymbol` 一致**。
//
// 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
//   0 = 缺口仍在（decls 为空） · 1 = 已修 · 2 = 环境/形状异常（需要人看）

'use strict';

const { spawn } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SOKO = path.join(ROOT, 'scripts', 'soko');

// 库：声明 `∈` 并把记法导出（第二刀起随 import 传播）。
const LIB = [
  'def Set (α : Type) : Type := α -> Prop',
  'def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a',
  'infix:50 " ∈ " => Set.mem',
  '',
].join('\n');

// 入口：**用库记法** ⇒ 单独 parse 失败，但闭包好。
const CANVAS = [
  'import SetLib',
  '',
  'theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h',
  '',
  'theorem open_one (α : Type) (a : α) (A : Set α) : a ∈ A := sorry',
  '',
].join('\n');

const uri = (file) => 'file://' + file;

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
    next(timeoutMs = 60000) {
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

async function responseFor(lsp, id) {
  for (;;) {
    const message = await lsp.next();
    if (message.id === id) return message;
  }
}

async function diagnosticsFor(lsp, file) {
  for (;;) {
    const message = await lsp.next();
    if (message.method === 'textDocument/publishDiagnostics' &&
        message.params && message.params.uri === uri(file)) {
      return message.params.diagnostics || [];
    }
  }
}

(async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g22-lsp-'));
  const libFile = path.join(dir, 'SetLib.sokonanoda');
  const entry = path.join(dir, 'Canvas.sokonanoda');
  fs.writeFileSync(libFile, LIB);
  fs.writeFileSync(entry, CANVAS);

  const lsp = startLsp();
  lsp.send({
    jsonrpc: '2.0', id: 1, method: 'initialize',
    params: {
      processId: process.pid,
      rootUri: uri(dir),
      capabilities: {},
      workspaceFolders: [{ uri: uri(dir), name: 'g22' }],
    },
  });
  await responseFor(lsp, 1);
  lsp.send({ jsonrpc: '2.0', method: 'initialized', params: {} });
  lsp.send({
    jsonrpc: '2.0', method: 'textDocument/didOpen',
    params: { textDocument: { uri: uri(entry), languageId: 'sokonanoda', version: 1, text: CANVAS } },
  });

  let diagnostics;
  try {
    diagnostics = await diagnosticsFor(lsp, entry);
  } catch (error) {
    console.error(`   → 等诊断超时：${error.message}`);
    lsp.stop();
    process.exit(2);
  }
  const codes = diagnostics.map((d) => (d && d.code) || '?');

  lsp.send({
    jsonrpc: '2.0', id: 2, method: 'soko/goals',
    params: { textDocument: { uri: uri(entry) } },
  });
  const goals = await responseFor(lsp, 2);
  lsp.send({
    jsonrpc: '2.0', id: 3, method: 'textDocument/documentSymbol',
    params: { textDocument: { uri: uri(entry) } },
  });
  const symbols = await responseFor(lsp, 3);
  lsp.stop();

  const decls = (goals.result && goals.result.decls) || [];
  const syms = symbols.result === null || symbols.result === undefined ? null : symbols.result;

  console.log('== 项目入口（import + 库记法）的 soko/goals ==');
  console.log(`   诊断 = ${JSON.stringify(codes)}（只应有 sorry 警告，不该有 notation-unknown-symbol）`);
  console.log(`   soko/goals 的 decls = ${decls.length} 条 ${JSON.stringify(decls.map((d) => d.name))}`);
  console.log(`   documentSymbol      = ${syms === null ? 'null' : syms.length + ' 项'}`);

  if (codes.includes('notation-unknown-symbol')) {
    console.error('   → 出现假的 notation-unknown-symbol（G-20 回归）⇒ 需要人看。');
    process.exit(2);
  }
  if (syms === null) {
    console.error('   → documentSymbol 为 null（G-20 回归：报告被丢掉了）⇒ 需要人看。');
    process.exit(2);
  }
  if (decls.length === 0) {
    console.error('结论：G-22 仍在——项目入口的声明栏恒为空（goals 被 parsable() 判死）。');
    process.exit(0);
  }
  if (decls.length !== syms.length) {
    console.error(
      `   → decls(${decls.length}) 与 documentSymbol(${syms.length}) 条数不一致 ⇒ 需要人看。`,
    );
    process.exit(2);
  }
  console.log('结论：G-22 已修——项目入口的声明栏非空，且与 documentSymbol 条数一致。');
  process.exit(1);
})().catch((error) => {
  console.error(`   → 探针异常：${error && error.stack ? error.stack : error}`);
  process.exit(2);
});
