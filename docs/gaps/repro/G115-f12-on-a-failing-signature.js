#!/usr/bin/env node
// **G-115 自断言复现**：**失败**声明里，签名中的跳转行不许被丢掉。
//
// 用户动作（判据必须打在这里 ✗ 不许拿别处当替身）：光标停在**失败**声明签名里
// `{a}` 的 **`{` 字符**上，按 F12（= `textDocument/definition`）。
//
// 台账 `today`（2026-10-11 实测，真 LSP + 课程模块根）：
//   * 失败声明 `theorem bad_one (α : Type) (a : α) : a ∈ {a} := by exact a`
//     ⇒ 签名里 `{a}` 的 F12 = **null** ✗（同文件其它**能编过**的 `{a}` 都落
//     `lib/Set.sokonanoda` 的 `def singleton` ✓ —— 所以用户看到的正是
//     「有的能跳、有的不能」）；
//   * 根因：`lower_value` 一失败，签名里那些**真读数**（`{a}` → `Set.singleton`）
//     随它的**局部** `hovers` 一起被丢掉；`walk.rs` 的 Err 臂只补了**词法行**
//     （G-108 第一刀补的是体里的名字，**签名里已 elaborate 过的行**仍丢 ✗）。
//
// 修后契约（2026-10-11，G-108 第二刀）：Err 臂把签名**再 elaborate 一遍只为收行**
// （`elab::elab_type_for_hovers`，不 `add_declar` ⇒ 环境不动 ✓）⇒ 签名里的
// `{a}` 照样落到 `def singleton` ✓。
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
const LIB = path.join(ROOT, 'courses', 'set-theory', 'lib');

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
    next(timeoutMs = 180000) {
      if (queue.length) return Promise.resolve(queue.shift());
      return new Promise((resolve, reject) => {
        const t = setTimeout(() => reject(new Error('LSP 超时未应答')), timeoutMs);
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
  if (!fs.existsSync(LIB)) {
    console.error('环境异常：找不到课程库 ' + LIB);
    process.exit(2);
  }
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g115-'));
  fs.cpSync(LIB, path.join(dir, 'lib'), { recursive: true });
  fs.writeFileSync(path.join(dir, 'sokonanoda.toml'), 'entry = "Probe.sokonanoda"\n');
  const src = [
    'import lib.Logic',
    'import lib.Set',
    '',
    'theorem bad_one (α : Type) (a : α) : a ∈ {a} := by',
    '  exact a',
    '',
  ].join('\n');
  const file = path.join(dir, 'Probe.sokonanoda');
  fs.writeFileSync(file, src);
  const brace = src.indexOf('{a}');
  if (brace < 0) {
    console.error('形状异常：夹具里没有 `{a}`');
    process.exit(2);
  }

  const lsp = startLsp();
  lsp.send({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      processId: process.pid,
      rootUri: uri(dir),
      capabilities: {},
      workspaceFolders: [{ uri: uri(dir), name: 'g115' }],
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
  lsp.send({
    jsonrpc: '2.0',
    id: 10,
    method: 'textDocument/definition',
    params: { textDocument: { uri: uri(file) }, position: posOf(src, brace) },
  });
  const def = (await responseFor(lsp, 10)).result ?? null;
  lsp.stop();

  if (!def || !def.uri) {
    console.log('缺口仍在：失败声明签名里 `{a}`（offset ' + brace + '）的 F12 = null ✗');
    process.exit(0);
  }
  const landed = path.resolve(decodeURIComponent(def.uri.replace(/^file:\/\//, '')));
  const text = fs.readFileSync(landed, 'utf8');
  const line = (text.split('\n')[def.range.start.line] || '').trim();
  const lineNo = def.range.start.line + 1;
  // **必须断言落点**（只断言"非 null"会放过**原地跳** ✗ —— E05/G-37 的教训）：
  // 落点必须是课程库里的 `def singleton`，不是夹具自己那一行。
  if (!landed.includes(`${path.sep}lib${path.sep}Set.sokonanoda`) || !line.startsWith('def singleton')) {
    console.log(`落点不对：${landed}:${lineNo} = ${JSON.stringify(line)}`);
    process.exit(2);
  }
  console.log(`已修：失败声明签名里 \`{a}\` 的 F12 落到 ${path.basename(landed)}:${lineNo} = ${line}`);
  process.exit(1);
}

main().catch((e) => {
  console.error('环境异常：' + (e && e.message ? e.message : e));
  process.exit(2);
});
