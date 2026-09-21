#!/usr/bin/env node
// G-23 自断言复现：**记法符号既不能跳转，hover 也不给原始类型**。
//
// 用户原话：「代码里的 notation 不能跳转，hover 信息也没有对应的原始类型」。
//
// 台账 `today`（0.63.0 实测，修前）——光标停在 unit01 第 32 行的 `∈` 上：
//   hover = 「`∈` —— 记法符号 / 输入：`\in`（别名 `\mem`） / `a ∈ A : Prop`」
//     ⇒ **有**输入法提示、**有**外层表达式类型；
//        **没有**「展开成 `Set.mem`」（`target` 只认本文件声明的记法，
//        `notation_input.rs:261-267`）、**没有** `Set.mem` 的原始类型；
//   textDocument/definition = **null**（记法使用处在 elab 里硬编码
//     `resolution: None`，`elab.rs:2914`；而 `definition_at` 就是
//     `hover_type_at(…).resolution`，`lsp/src/render.rs:306-314`，没有第二条路）；
//   textDocument/documentHighlight = **null**。
//
// 修后契约（两条都要）：
//   ① hover 的 markdown 里出现 `Set.mem` 的**原始类型**（签名行）；
//   ② definition 在 `∈` 上返回非空 Location，且指向声明 `∈` 的那个文件。
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

const LIB = [
  'def Set (α : Type) : Type := α -> Prop',
  'def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a',
  'infix:50 " ∈ " => Set.mem',
  '',
].join('\n');

// 只留一条声明，让 `∈` 的位置唯一、可复现。
const CANVAS = [
  'import SetLib',
  '',
  'theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h',
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
      const match = /Content-Length: (\d+)/i.exec(buffer.slice(0, sep).toString('utf8'));
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
        waiters.push((message) => { clearTimeout(timer); resolve(message); });
      });
    },
    stop() { try { child.kill(); } catch (_) { /* 已退出 */ } },
  };
}

async function responseFor(lsp, id) {
  for (;;) {
    const message = await lsp.next();
    if (message.id === id) return message;
  }
}

/** `needle` 在 `text` 里第一次出现的位置（1 基行列，按 UTF-16 码元计）。 */
function positionOf(text, needle) {
  const offset = text.indexOf(needle);
  const before = text.slice(0, offset);
  const line = before.split('\n').length - 1;
  const character = (before.split('\n').pop() || '').length;
  return { line, character };
}

(async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g23-lsp-'));
  const libFile = path.join(dir, 'SetLib.sokonanoda');
  const entry = path.join(dir, 'Canvas.sokonanoda');
  fs.writeFileSync(libFile, LIB);
  fs.writeFileSync(entry, CANVAS);

  const pos = positionOf(CANVAS, '∈');
  const lsp = startLsp();
  lsp.send({
    jsonrpc: '2.0', id: 1, method: 'initialize',
    params: {
      processId: process.pid,
      rootUri: uri(dir),
      capabilities: {},
      workspaceFolders: [{ uri: uri(dir), name: 'g23' }],
    },
  });
  await responseFor(lsp, 1);
  lsp.send({ jsonrpc: '2.0', method: 'initialized', params: {} });
  lsp.send({
    jsonrpc: '2.0', method: 'textDocument/didOpen',
    params: { textDocument: { uri: uri(entry), languageId: 'sokonanoda', version: 1, text: CANVAS } },
  });
  // 等一份诊断，确保编译完成再问 hover/definition。
  for (;;) {
    const message = await lsp.next();
    if (message.method === 'textDocument/publishDiagnostics' &&
        message.params && message.params.uri === uri(entry)) break;
  }

  for (const [id, method] of [[2, 'textDocument/hover'], [3, 'textDocument/definition']]) {
    lsp.send({
      jsonrpc: '2.0', id, method,
      params: { textDocument: { uri: uri(entry) }, position: pos },
    });
  }
  const hover = await responseFor(lsp, 2);
  const definition = await responseFor(lsp, 3);
  lsp.stop();

  const value = (hover.result && hover.result.contents && hover.result.contents.value) || '';
  const def = definition.result === undefined ? null : definition.result;
  const defList = Array.isArray(def) ? def : def === null ? [] : [def];

  console.log(`== 光标停在 \`∈\` 上（${entry.replace(dir, '<tmp>')}:${pos.line + 1}:${pos.character + 1}）==`);
  console.log('   hover =');
  for (const line of value.split('\n')) console.log(`     | ${line}`);
  console.log(`   definition = ${def === null ? 'null' : JSON.stringify(defList.map((d) => String(d.uri).replace(dir, '<tmp>')))}`);

  const hasSignature = /Set\.mem/.test(value);
  const jumps = defList.length > 0;

  if (!hasSignature && !jumps) {
    console.error('结论：G-23 仍在——hover 不给 `Set.mem` 的原始类型，definition 返回 null。');
    process.exit(0);
  }
  if (hasSignature && !jumps) {
    console.error('   → hover 已给原始类型，但 definition 仍为 null（修了一半）⇒ 需要人看。');
    process.exit(2);
  }
  if (!hasSignature && jumps) {
    console.error('   → definition 已能跳，但 hover 仍无原始类型（修了一半）⇒ 需要人看。');
    process.exit(2);
  }
  const targetsLib = defList.some((d) => String(d.uri).endsWith('SetLib.sokonanoda'));
  if (!targetsLib) {
    console.error(`   → definition 跳到了 ${JSON.stringify(defList.map((d) => d.uri))}，不是声明 \`∈\` 的库 ⇒ 需要人看。`);
    process.exit(2);
  }
  console.log('结论：G-23 已修——hover 给出 `Set.mem` 的原始类型，definition 跳到库里的声明。');
  process.exit(1);
})().catch((error) => {
  console.error(`   → 探针异常：${error && error.stack ? error.stack : error}`);
  process.exit(2);
});
