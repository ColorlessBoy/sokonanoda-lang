#!/usr/bin/env node
// G-36 判定实验：**`position_to_offset` 按 `char` 计数，不是 LSP 要求的 UTF-16 码元**。
//
// 计划 T-D31（§2.7 第 3 条；`docs/protocol.md` 已把这条记为独立缺口）。
// **本文件只做判定实验，不做修复**——真修（改成 UTF-16 计数）会动所有位置映射与
// 测试夹具，单独立项。
//
// 课程里唯一星平面符号是 `𝒫`（U+1D4AB）：UTF-16 里它是**两个码元**（代理对），
// 而 Rust 的 `char` 只算**一个** ⇒ 服务端按 `character` 直接当字符下标用，
// 于是 `𝒫` 之后的同一行**整体偏移一格**。
//
// 退出码：0 = 缺口仍在（位置映射偏离）· 1 = 已修（映射正确）· 2 = 环境/形状异常。
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

// `𝒫` 之后紧跟一个名字；光标落在那个名字上。
const SRC = [
  'def Set (α : Type) : Type := α -> Prop',
  'def Set.powerset (α : Type) (A : Set α) : Set α := A',
  'prefix:100 " 𝒫 " => Set.powerset',
  // ⚠ 夹具必须**编译干净**（`soko grade` 退出 0、无诊断）：否则 hover 会退化成
  // "未通过，见诊断"的错误卡片，红的原因就不是位置映射了——第一版踩过。
  'theorem demo (α : Type) (A : Set α) (h : 𝒫 A = 𝒫 A) : 𝒫 A = 𝒫 A := h',
  '',
].join('\n');

function uri(p) { return 'file://' + p; }
function positionOf(text, needle, nth = 0) {
  let offset = -1;
  for (let i = 0; i <= nth; i += 1) offset = text.indexOf(needle, offset + 1);
  const before = text.slice(0, offset);
  return {
    line: before.split('\n').length - 1,
    // **JS 的 `.length` 就是 UTF-16 码元数**——正是 LSP 的口径。
    character: (before.split('\n').pop() || '').length,
    offset,
  };
}
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
      const header = buffer.slice(0, sep).toString();
      const m = /Content-Length: (\d+)/i.exec(header);
      if (!m) return;
      const len = Number(m[1]);
      if (buffer.length < sep + 4 + len) return;
      const body = buffer.slice(sep + 4, sep + 4 + len).toString();
      buffer = buffer.slice(sep + 4 + len);
      const message = JSON.parse(body);
      const waiter = waiters.shift();
      if (waiter) waiter(message); else queue.push(message);
    }
  });
  let stderr = '';
  child.stderr.on('data', (c) => { stderr += c.toString(); });
  return {
    send(message) {
      const body = JSON.stringify(message);
      child.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
    },
    next() {
      if (queue.length) return Promise.resolve(queue.shift());
      return new Promise((resolve) => waiters.push(resolve));
    },
    stop() { child.kill(); },
    get stderr() { return stderr; },
  };
}
async function responseFor(lsp, id) {
  for (;;) {
    const message = await lsp.next();
    if (message.id === id) return message;
  }
}

(async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g36-lsp-'));
  const entry = path.join(dir, 'Canvas.sokonanoda');
  fs.writeFileSync(entry, SRC);

  // 第 4 行里 `𝒫 A` 的 `A`：`𝒫` 是代理对 ⇒ 它的 UTF-16 长度是 2。
  const pos = positionOf(SRC, '𝒫 A');
  pos.character += '𝒫 '.length; // 落到 `A` 上（`𝒫` 与空格都是 1 个码元 + 空格 1）
  const expectedChar = SRC.split('\n')[pos.line][pos.character];

  const lsp = startLsp();
  lsp.send({
    jsonrpc: '2.0', id: 1, method: 'initialize',
    params: {
      processId: process.pid,
      rootUri: uri(dir),
      capabilities: {},
      workspaceFolders: [{ uri: uri(dir), name: 'g36' }],
    },
  });
  await responseFor(lsp, 1);
  lsp.send({ jsonrpc: '2.0', method: 'initialized', params: {} });
  lsp.send({
    jsonrpc: '2.0', method: 'textDocument/didOpen',
    params: { textDocument: { uri: uri(entry), languageId: 'sokonanoda', version: 1, text: SRC } },
  });
  for (;;) {
    const message = await lsp.next();
    if (message.method === 'textDocument/publishDiagnostics' &&
        message.params && message.params.uri === uri(entry)) break;
  }
  lsp.send({
    jsonrpc: '2.0', id: 2, method: 'textDocument/hover',
    params: { textDocument: { uri: uri(entry) }, position: pos },
  });
  // **对照**：列号减一（= 服务端按字符数算出来的那个位置）应当命中 `A`。
  // 有这条对照才能证明差别就是那 ±1 的映射，而不是"A 本来就没有 hover"。
  lsp.send({
    jsonrpc: '2.0', id: 3, method: 'textDocument/hover',
    params: {
      textDocument: { uri: uri(entry) },
      position: { line: pos.line, character: pos.character - 1 },
    },
  });
  const hover = await responseFor(lsp, 2);
  const control = await responseFor(lsp, 3);
  lsp.stop();
  fs.rmSync(dir, { recursive: true, force: true });

  const range = hover.result && hover.result.range;
  const value = (hover.result && hover.result.contents && hover.result.contents.value) || '';
  console.log(`== 光标在 \`𝒫 A\` 的 \`${expectedChar}\` 上（UTF-16 列 ${pos.character}）==`);
  console.log(`   hover.range = ${range ? JSON.stringify(range) : 'null'}`);
  console.log('   hover 全文  =');
  for (const line of value.split('\n')) console.log(`     | ${line}`);

  // **判据**：hover 必须**说到我们指的那个名字**（`A : Set α`）。
  // 只看 range 太弱——落偏时服务端会回退成"整行表达式"的 hover，range 照样覆盖光标
  // （第一版就这么写，恒真、量不出缺口）。
  const controlValue =
    (control.result && control.result.contents && control.result.contents.value) || '';
  console.log('   —— 对照（列号减一，即服务端实际落到的位置）——');
  for (const line of controlValue.split('\n')) console.log(`     | ${line}`);
  const controlHits = /A : Set/.test(controlValue);
  const covers = /A : Set/.test(value);
  if (!controlHits) {
    console.error('复现脚本自身出错：对照位置也说不出 `A : Set α` ⇒ 夹具/形状不对，');
    console.error('                量到的不是位置映射。');
    process.exit(2);
  }
  if (!covers) {
    console.error('结论：G-36 仍在——光标在 `𝒫` 之后的 `A` 上时，服务端按**字符数**算');
    console.error('      offset（`𝒫` 在 UTF-16 里是 2 个码元、在 Rust 里是 1 个 char）');
    console.error('      ⇒ 位置偏一格，hover 退化成"整行表达式"，说不出 `A : Set α`。');
    process.exit(0);
  }
  console.log('结论：G-36 已修——`𝒫` 之后的 UTF-16 位置映射正确。');
  process.exit(1);
})().catch((error) => {
  console.error('复现脚本自身出错（环境/形状异常）：', error);
  process.exit(2);
});
