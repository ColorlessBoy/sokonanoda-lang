#!/usr/bin/env node
// **G-55 自断言复现**：`documentHighlight` 在**记法声明的目标名**上返回 `null`。
//
// 用户动作（判据必须打在这里 ✗ 不许拿别处当替身）：光标停在
// `prefix:70 " 𝒫 " => Set.powerset` 这行的**目标名**（`Set.powerset` 的 `S`）上。
//
// 台账 `today`（E08 实测，真课程库 11 条）：`textDocument/documentHighlight` ⇒ **null**
//   —— 目标名在 AST 里**不是使用点**（没有 hover 行、没有 `resolution`），而反查要先
//   有"至少一个使用处"起头 ⇒ 空手（连目标在闭包内的 `Set.powerset` 也是 null）。
//
// 修后契约（2026-10-07 定案 = ①+②+③ 累积式，理由见
// `docs/design/notation-subset.md` §15 / 台账 G-55）：
//   ① 结果**非空**且**包含用户点的那个位置**（声明行上那个名字）—— 地板；
//   ② 含**本文件里展开到这个目标的记法符号的每一处**（`𝒫 A` ×2）；
//   ③ 含**目标在本文件内的定义名**那一处（`def powerset`）与**点名使用处**
//      （`Set.powerset α A` ×2）。
//
// 退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
//   0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（需要人看）

'use strict';

const { spawn, spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SOKO = path.join(ROOT, 'scripts', 'soko');

const LIB = ['def Set (α : Type) : Type := α -> Prop', ''].join('\n');
const SRC = [
  'import SetLib',
  '',
  'namespace Set',
  'def powerset (α : Type) (A : Set α) : Set α := fun (a : α) => A a',
  'prefix:70 " 𝒫 " => Set.powerset',
  'theorem t (α : Type) (A : Set α) (h : 𝒫 A = Set.powerset α A) : 𝒫 A = Set.powerset α A := h',
  'end Set',
  '',
].join('\n');

const uri = (file) => 'file://' + file;

/** 探针读数必须带**构建身份**（AGENTS.md：跨轮比较前先确认同一份构建）。 */
function buildIdentity() {
  const doctor = spawnSync(process.execPath, [SOKO, 'doctor', '--json'], { encoding: 'utf8' });
  if (doctor.status !== 0 || !doctor.stdout) return 'lsp=<未解析>';
  let info;
  try {
    info = JSON.parse(doctor.stdout);
  } catch (_) {
    return 'lsp=<doctor --json 不是 JSON>';
  }
  const lsp = (info.lsp && info.lsp.path) || '<none>';
  let sha = '<no-file>';
  try {
    sha = crypto.createHash('sha256').update(fs.readFileSync(lsp)).digest('hex').slice(0, 16);
  } catch (_) { /* 读不到就留标记 */ }
  return `lsp=${lsp} sha256=${sha} version=${info.version}`;
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
    // CI 慢 runner 上 60 秒不够（G-23 的教训）⇒ 180 秒 + 可覆盖。
    next(timeoutMs = Number(process.env.SOKO_LSP_REPLY_TIMEOUT_MS || 180000)) {
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

/** 0 基偏移 → 0 基 LSP 位置（`character` 按 **UTF-16 码元** —— LSP 的口径）。 */
function posOf(text, offset) {
  const before = text.slice(0, offset);
  const line = before.split('\n').length - 1;
  const character = (before.split('\n').pop() || '').length;
  return { line, character };
}

/** `needle` 在 `text` 里的**每一处**的 0 基偏移。 */
function offsetsOf(text, needle) {
  const out = [];
  for (let i = text.indexOf(needle); i >= 0; i = text.indexOf(needle, i + 1)) out.push(i);
  return out;
}

const samePos = (a, b) => a.line === b.line && a.character === b.character;
const showPos = (p) => `${p.line + 1}:${p.character + 1}`;

(async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g55-lsp-'));
  const entry = path.join(dir, 'Canvas.sokonanoda');
  fs.writeFileSync(path.join(dir, 'sokonanoda.toml'), 'entry = "Canvas.sokonanoda"\n');
  fs.writeFileSync(path.join(dir, 'SetLib.sokonanoda'), LIB);
  fs.writeFileSync(entry, SRC);

  // 用户实际点的那个位置：声明行上目标名的第一个字符。
  const clicked = posOf(SRC, SRC.indexOf('=> Set.powerset') + 3);
  const expected = [
    ['① 目标名自己（用户点的位置）', clicked],
    ['③ 定义名 `def powerset`', posOf(SRC, SRC.indexOf('def powerset') + 4)],
  ];
  // ⚠ 声明行**字符串里**那个 `𝒫`（`" 𝒫 "`）**不是使用处** ⇒ 不进期望
  //   （它是符号的**引入处**：`Str` token，不是 `Sym` —— 清单里那条"另一条" ✗）。
  const declaredSymbol = SRC.indexOf('" 𝒫 "') + 2;
  for (const offset of offsetsOf(SRC, '𝒫')) {
    if (offset === declaredSymbol) continue;
    expected.push(['② 记法符号 `𝒫`', posOf(SRC, offset)]);
  }
  for (const offset of offsetsOf(SRC, 'Set.powerset')) {
    if (offset === SRC.indexOf('=> Set.powerset') + 3) continue; // 声明行那一处 = ①
    expected.push(['③ 点名使用处 `Set.powerset`', posOf(SRC, offset)]);
  }

  const lsp = startLsp();
  lsp.send({
    jsonrpc: '2.0', id: 1, method: 'initialize',
    params: {
      processId: process.pid,
      rootUri: uri(dir),
      capabilities: {},
      workspaceFolders: [{ uri: uri(dir), name: 'g55' }],
    },
  });
  await responseFor(lsp, 1);
  lsp.send({ jsonrpc: '2.0', method: 'initialized', params: {} });
  lsp.send({
    jsonrpc: '2.0', method: 'textDocument/didOpen',
    params: { textDocument: { uri: uri(entry), languageId: 'sokonanoda', version: 1, text: SRC } },
  });
  let diagnostics = null;
  for (;;) {
    const message = await lsp.next();
    if (message.method === 'textDocument/publishDiagnostics' &&
        message.params && message.params.uri === uri(entry)) {
      diagnostics = message.params.diagnostics || [];
      break;
    }
  }
  lsp.send({
    jsonrpc: '2.0', id: 2, method: 'textDocument/documentHighlight',
    params: { textDocument: { uri: uri(entry) }, position: clicked },
  });
  const answer = await responseFor(lsp, 2);
  lsp.stop();

  console.log(`== 构建身份：${buildIdentity()} ==`);
  console.log(`== 夹具：${entry.replace(dir, '<tmp>')} · 光标停在**记法声明的目标名**上`
    + `（${showPos(clicked)}）==`);
  console.log(`   诊断 ${diagnostics.length} 条`);
  if (diagnostics.length > 0) {
    for (const d of diagnostics) console.log(`     [sev ${d.severity}] ${String(d.message).slice(0, 100)}`);
    console.error('结论：夹具自身没编译干净 ⇒ 读数不可归属，先修夹具（exit 2）。');
    process.exit(2);
  }

  const result = answer.result === undefined ? null : answer.result;
  const highlights = Array.isArray(result) ? result : result === null ? [] : [result];
  console.log(`   documentHighlight = ${result === null ? 'null' : `${highlights.length} 项`}`);
  for (const h of highlights) {
    const line = SRC.split('\n')[h.range.start.line] || '';
    console.log(`     ${showPos(h.range.start)}~${showPos(h.range.end)} `
      + JSON.stringify(line.slice(h.range.start.character, h.range.end.character)));
  }

  if (result === null || highlights.length === 0) {
    console.error('结论：G-55 仍在——记法声明的目标名上 `documentHighlight` 返回 `null` ✗');
    console.error('      （目标名不是使用点 ⇒ 按"定义 → 引用"反查空手；用户点它看不到任何东西 ✗）');
    process.exit(0);
  }

  const starts = highlights.map((h) => h.range.start);
  const missing = expected.filter(([, pos]) => !starts.some((s) => samePos(s, pos)));
  const duplicated = starts.filter((s, i) => starts.findIndex((o) => samePos(o, s)) !== i);
  console.log(`   期望 ${expected.length} 处（①名字 + ②符号每一处 + ③定义名与点名使用处）`
    + ` · 缺 ${missing.length} · 重复 ${duplicated.length}`);
  for (const [label, pos] of missing) console.log(`     ✗ 缺：${label} @${showPos(pos)}`);

  if (missing.length > 0 || duplicated.length > 0) {
    console.error('结论：G-55 仍在——高亮答上了，但**位置集合不对** ✗');
    console.error('      （判据不只看"非 null"：① 必须含**用户实际点的那个位置**、'
      + '②③ 必须含同一个定义的每一处 ⇒ 否则就是"能跑通的位置"顶替了"用户点的位置" ✗）');
    process.exit(0);
  }
  console.log('结论：G-55 已修——目标名上返回**非 null**，且含用户点的位置与同一个定义的每一处 ✓');
  process.exit(1);
})().catch((error) => {
  console.error(`   → 探针异常：${error && error.stack ? error.stack : error}`);
  process.exit(2);
});
