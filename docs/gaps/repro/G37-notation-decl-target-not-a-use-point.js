#!/usr/bin/env node
// G-37 自断言复现：**记法声明行里的目标名不是使用点**。
//
// 用户原话（2026-09-23）：「`Set.image` `Set.preimage` 和 `Set.prod` 没有高亮，
// 另外 `Set.xxx`（ctrl+点击）不能跳转到定义。……这个应该是一个共性问题。」
//
// **机制（真 LSP 探针实测）**：`=>` 后面的目标名**从来不是使用点**——
// `textDocument/{definition,hover,documentHighlight}` 打在五条声明行的目标名上
// **全是 `null`**。⇒ ctrl+点击不能跳转是**共性问题**（与符号种类无关）。
//
// 那为什么"高亮"只坏了三条？语义 token 的**类型号**不同：
//   115–125 行的目标名（`Set.mem`/`Set.powerset`/`Set.compl`…）→ **4 = FUNCTION**
//            （名字**在本文件里声明** ⇒ 作用域查得到）
//   126–128 行的目标名（`Set.image`/`Set.preimage`/`Set.prod`）→ **5 = VARIABLE**
//            （名字**不在本文件作用域**，在 `lib/Image.sokonanoda` / 单元⑤ 画布里
//             ⇒ 落成 `SemanticKind::UnknownIdent`）
// ⇒ 三条"没高亮"是**同一个根因的第二种症状**：目标名不是"已知引用"，
//    只能退回作用域查找，查不到就只好说"不知道这是什么"。
//
// 退出码：0 = 缺口仍在 · 1 = 已修（目标名成为使用点）· 2 = 环境/形状异常。
const { spawn } = require('node:child_process');
// ⚠ **看门狗（2026-09-25 round 362 ✓）**：LSP 不应答时 `await lsp.next()` **永不 resolve**
// ⇒ **事件循环排空 ⇒ node 静默 exit 0** ✗ ⇒ `gap.py` 会把"**没跑完**"读成
// "**缺口仍在**" ✗（**实测** ✓：CI 与本地都静默 0 ✓，而 `G-37` 的台账其实是 `fixed` ✓）。
// 加这个看门狗后：够不到 LSP ⇒ **exit 2** ✓（"环境/形状异常" ✓ —— `docs/gaps/README.md` 的三态 ✓）。
setTimeout(() => {
  console.error('结论：复现脚本超时（LSP 未应答）——环境/形状异常，**不是**"缺口仍在"');
  process.exit(2);
}, 120000);
const fs = require('node:fs');
const path = require('node:path');
const ROOT = path.resolve(__dirname, '..', '..', '..');
const SOKO = path.join(ROOT, 'scripts', 'soko');
const FILE = path.join(ROOT, 'courses', 'set-theory', 'lib', 'Set.sokonanoda');
const SRC = fs.readFileSync(FILE, 'utf8');
const uri = 'file://' + FILE;

function positionOf(text, needle, from = 0) {
  const offset = text.indexOf(needle, from);
  const before = text.slice(0, offset);
  return { line: before.split('\n').length - 1, character: (before.split('\n').pop() || '').length, offset };
}
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
  lsp.send({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { processId: process.pid, rootUri: 'file://' + path.dirname(FILE), capabilities: {}, workspaceFolders: [{ uri: 'file://' + path.dirname(FILE), name: 'p' }] } });
  await responseFor(lsp, 1);
  lsp.send({ jsonrpc: '2.0', method: 'initialized', params: {} });
  lsp.send({ jsonrpc: '2.0', method: 'textDocument/didOpen', params: { textDocument: { uri, languageId: 'sokonanoda', version: 1, text: SRC } } });
  for (;;) { const m = await lsp.next(); if (m.method === 'textDocument/publishDiagnostics' && m.params && m.params.uri === uri) break; }

  const cases = [
    ['Set.powerset (prefix)', 'prefix:100 " 𝒫 " => Set.powerset'],
    ['Set.compl (postfix)', 'postfix:100 " ᶜ " => Set.compl'],
    ['Set.image (infixr)', "infixr:80 \" '' \" => Set.image"],
    ['Set.preimage (infixr)', "infixr:80 \" ⁻¹' \" => Set.preimage"],
    ['Set.prod (infixr)', 'infixr:80 " ×ˢ " => Set.prod'],
  ];
  let id = 10;
  const pending = [];
  for (const [label, line] of cases) {
    // ⚠ **必须取"那一行里的那个名字"**：`positionOf(SRC, 'Set.powerset')` 会命中
    // **第一次出现**——它在文件更早的**注释**里（实测：光标落在注释上 ⇒ 三种请求
    // 全 null，量到的根本不是"记法声明的目标名"）。用行首偏移 + 行内偏移定位。
    const at = positionOf(SRC, line);
    const inLine = line.indexOf(line.split('=> ')[1]);
    const target = { line: at.line, character: at.character + inLine, offset: at.offset + inLine };
    for (const [method, suffix] of [['textDocument/definition', 'def'], ['textDocument/hover', 'hover'], ['textDocument/documentHighlight', 'hl']]) {
      lsp.send({ jsonrpc: '2.0', id, method, params: { textDocument: { uri }, position: { line: target.line, character: target.character + 2 } } });
      pending.push([id, label, suffix, target]);
      id += 1;
    }
    void at;
  }
  const out = new Map();
  const missing = [];
  for (const [rid, label, suffix, target] of pending) {
    const r = await responseFor(lsp, rid);
    const v = r.result;
    let summary;
    if (v === null || v === undefined) summary = 'null';
    else if (Array.isArray(v)) summary = `array(${v.length})`;
    else if (v.contents) summary = JSON.stringify((v.contents.value || '').split('\n')[0]).slice(0, 60);
    else summary = JSON.stringify(v).slice(0, 80);
    out.set(`${label}|${suffix}`, { r, target, summary });
    console.log(`${`${label}|${suffix}`.padEnd(34)} @${target.line + 1}:${target.character + 3}  ${summary}`);
    if (summary === 'null') missing.push(`${label}|${suffix}`);
  }
  lsp.stop();
  // **修后契约**（T-D50）：
  //   ① **hover** 五条目标名**全部**答得上（"`X` —— 记法的目标"）；
  //   ② **definition** 对**在本文件闭包里**的目标答得上（`Set.powerset`/
  //      `Set.compl` 就在 `lib/Set.sokonanoda` 里）——**不在**闭包里的
  //      （`Set.image`/`Set.preimage`/`Set.prod`，声明在别的模块）**诚实为
  //      null**：这份文件没 import 那个模块，没有可跳的目标，编不出来。
  //   ③ `documentHighlight` 本轮**不做**（那要 use→def 映射，属于另一条线）。
  const hoverOk = cases.every(([label]) => {
    const entry = out.get(`${label}|hover`);
    return entry && entry.summary && entry.summary !== 'null';
  });
  const inScope = ['Set.powerset (prefix)', 'Set.compl (postfix)'];
  const defOk = inScope.every((label) => {
    const entry = out.get(`${label}|def`);
    return entry && entry.summary && entry.summary !== 'null';
  });
  if (hoverOk && defOk) {
    console.log('结论：G-37 已修——目标名 hover 五条全答得上；在闭包里的两条 definition 也答得上');
    console.log('      （不在闭包里的三条诚实为 null——那份文件没 import 声明它们的模块）。');
    process.exit(1);
  }
  console.error('结论：G-37 仍在——记法声明的目标名不是使用点（definition/hover/');
  console.error('      documentHighlight 全为 null）⇒ ctrl+点击不能跳转，且名字');
  console.error('      不在本文件作用域时会被当成未知标识符（着色成 VARIABLE）。');
  process.exit(0);
})().catch((error) => {
  console.error('复现脚本自身出错（环境/形状异常）：', error);
  process.exit(2);
});
