#!/usr/bin/env node
// G-39 自断言复现：**import 进来的用户自定义记法符号，在使用它的文件里认不出来**
// ⇒ `definition`/`hover` 全 `null`（ctrl+点击没反应、悬停静默）。
//
// 现场（T-D23 挖出来的，2026-09-24）：`QueryDoc::notation_at` 原来用
// `notation_input::symbol_at`，而它只看**输入表 + 本文件声明**——`⊗` 这种用户
// 自己定的符号**不在输入表里**，于是"声明它的文件"里认得出、"import 它的文件"里
// 认不出。**既有跨文件用例没抓到**，是因为它用的 `∈` 恰好在输入表里（`\in`）
// ——夹具选得太顺手，把这条路整个遮住了。
//
// 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（docs/gaps/README.md）。
const { spawn } = require('node:child_process');
// ⚠ **看门狗（2026-09-25 round 363 ✓）**：LSP 不应答时 `await …` **永不 resolve**
// ⇒ **事件循环排空 ⇒ node 静默 exit 0** ✗ ⇒ `gap.py` 会把"**没跑完**"读成
// "**缺口仍在**" ✗（**实测** ✓：`G-37` 与本地/CI 都静默 0 ✓）。
// 加看门狗后：够不到 LSP ⇒ **exit 2** ✓（"环境/形状异常" ✓ —— `docs/gaps/README.md` 的三态 ✓）。
setTimeout(() => {
  console.error('结论：复现脚本超时（LSP 未应答）——环境/形状异常，**不是**"缺口仍在"');
  process.exit(2);
}, 120000);
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const ROOT = path.resolve(__dirname, '..', '..', '..');
const SOKO = path.join(ROOT, 'scripts', 'soko');

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'soko-g39-'));
const lib = path.join(dir, 'Lib.sokonanoda');
const entry = path.join(dir, 'Canvas.sokonanoda');
fs.writeFileSync(lib, 'def Lib.op (a b : Prop) : Prop := a\ninfix:60 " ⊗ " => Lib.op\n');
const SRC = 'import Lib\n\ntheorem t (a b : Prop) (h : a ⊗ b) : a ⊗ b := h\n';
fs.writeFileSync(entry, SRC);
const uri = 'file://' + entry;

function startLsp() {
  const child = spawn(process.execPath, [SOKO, 'lsp'], { stdio: ['pipe', 'pipe', 'pipe'] });
  let buffer = Buffer.alloc(0); const queue = []; const waiters = [];
  child.stdout.on('data', (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    for (;;) {
      const sep = buffer.indexOf('\r\n\r\n'); if (sep < 0) return;
      const m = /Content-Length: (\d+)/i.exec(buffer.slice(0, sep).toString()); if (!m) return;
      const len = Number(m[1]); if (buffer.length < sep + 4 + len) return;
      const body = buffer.slice(sep + 4, sep + 4 + len).toString(); buffer = buffer.slice(sep + 4 + len);
      const msg = JSON.parse(body); const w = waiters.shift(); if (w) w(msg); else queue.push(msg);
    }
  });
  return {
    send(m) { const b = JSON.stringify(m); child.stdin.write(`Content-Length: ${Buffer.byteLength(b)}\r\n\r\n${b}`); },
    next() { return queue.length ? Promise.resolve(queue.shift()) : new Promise((r) => waiters.push(r)); },
    stop() { child.kill(); },
  };
}
async function responseFor(lsp, id) { for (;;) { const m = await lsp.next(); if (m.id === id) return m; } }

(async () => {
  const lsp = startLsp();
  lsp.send({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { processId: process.pid, rootUri: 'file://' + dir, capabilities: {}, workspaceFolders: [{ uri: 'file://' + dir, name: 'p' }] } });
  await responseFor(lsp, 1);
  lsp.send({ jsonrpc: '2.0', method: 'initialized', params: {} });
  lsp.send({ jsonrpc: '2.0', method: 'textDocument/didOpen', params: { textDocument: { uri, languageId: 'sokonanoda', version: 1, text: SRC } } });
  for (;;) { const m = await lsp.next(); if (m.method === 'textDocument/publishDiagnostics' && m.params && m.params.uri === uri) break; }

  // 光标落在**使用处**的 `⊗` 上（最后那个），不是 import 行。
  const offset = SRC.lastIndexOf('⊗');
  const before = SRC.slice(0, offset);
  const pos = { line: before.split('\n').length - 1, character: (before.split('\n').pop() || '').length };
  const asks = [['definition', 'textDocument/definition'], ['hover', 'textDocument/hover']];
  const answers = new Map();
  let id = 10;
  for (const [label, method] of asks) {
    lsp.send({ jsonrpc: '2.0', id, method, params: { textDocument: { uri }, position: pos } });
    answers.set(label, id); id += 1;
  }
  let answered = 0;
  for (const [label, rid] of answers) {
    const r = await responseFor(lsp, rid);
    const v = r.result;
    const isNull = v === null || v === undefined;
    console.log(`${label.padEnd(11)} @${pos.line + 1}:${pos.character + 1}  ${isNull ? 'null' : JSON.stringify(v).slice(0, 90)}`);
    if (!isNull) answered += 1;
  }
  lsp.stop();
  fs.rmSync(dir, { recursive: true, force: true });
  if (answered === asks.length) {
    console.log('结论：G-39 已修——import 的用户记法符号在使用处也能跳转、也能悬停。');
    process.exit(1);
  }
  console.error('结论：G-39 仍在——import 进来的用户自定义记法符号在使用它的文件里认不出来');
  console.error('      （definition/hover 全 null；根因见脚本头部注释）。');
  process.exit(0);
})().catch((error) => {
  console.error('复现脚本自身出错（环境/形状异常）：', error);
  process.exit(2);
});
