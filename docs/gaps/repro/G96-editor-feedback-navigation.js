#!/usr/bin/env node
// **G-96 自断言复现**：内建记法 `=` 上没有 F12 落点 + 指令注释行里的目标名不可跳。
//
// 用户动作（判据必须打在这里 ✗ 不许拿别处当替身）：光标停在
//   * `a = b` 里那个 **`=` 字符**上（有 `import` 的项目 + 单文件两种都要）；
//   * prelude 里 `-- sokonanoda:builtin-notation "∧" => And` 这行的 **`And` 字符**上。
//
// 台账 `today`（2026-10-10 实测，真 LSP）：
//   * `=` ⇒ `textDocument/definition` = **null**（三处特例叠加：词法剔除 `=` ·
//     prelude 不登记它 ⇒ `Span::default()` · LSP 的 `span.start.offset != 0` 闸门
//     把 offset 0 的内建直接放过）—— 既不报错也不跳 ✗；
//   * 注释里的 `And` ⇒ **null**（`notation_target_at` 是**词法**扫描，注释不产生
//     token ⇒ F12 / hover / documentHighlight 三条消费者全空 ✗）—— 与仓库白纸黑字的
//     **两跳模型**（`G23-notation-navigation.js`）矛盾；
//   * 同行的 `∧`（对照）**跳得动** ⇒ 用户看到的「有的能跳、有的不能」。
//
// 修后契约（2026-10-10，`docs/design/notation-subset.md` 的 T-D20 段）：
//   ① prelude 登记区补 `-- sokonanoda:builtin-notation "=" => Eq` ⇒ `=` 的 F12 落
//      **那一行**（与另外 5 条内建记法对称）；
//   ② `notation_target_at` 同时认**注释形态** ⇒ 注释里的目标名走已有真相通道
//      （`project_definition` → `prelude_def_span`）落到**定义那一行**
//      （`And` ⇒ `inductive And`，`Eq` ⇒ `axiom Eq`）——**必须断言落点行号**，
//      只断言「非 null」会放过**原地跳**（E05/G-37 的教训 ✗）。
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
const PRELUDE = path.join(ROOT, 'prelude', 'Prelude.sokonanoda');
const PRELUDE_R = path.resolve(PRELUDE);

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

async function session(dir, file, text, requests) {
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
    params: { textDocument: { uri: uri(file), languageId: 'sokonanoda', version: 1, text } },
  });
  for (;;) {
    const m = await lsp.next();
    if (m.method === 'textDocument/publishDiagnostics' && m.params?.uri === uri(file)) break;
  }
  const out = {};
  let id = 10;
  for (const r of requests) {
    const reqId = id++;
    lsp.send({
      jsonrpc: '2.0',
      id: reqId,
      method: 'textDocument/definition',
      params: { textDocument: { uri: uri(file) }, position: r.pos },
    });
    out[r.label] = (await responseFor(lsp, reqId)).result ?? null;
  }
  lsp.stop();
  return out;
}

/// 落点行（1-based）与那一行的原文 —— **必须断言行号**（非 null 会放过原地跳 ✗）。
function landing(res) {
  if (!res || !res.uri || !res.range) return null;
  // ⚠ 服务端回的 URI 可能是**未规范化**的路径（实测 `crates/front/../../prelude/…`）
  // ⇒ 比路径前先 `resolve`（否则「落点对、路径字符串不等」会**假红** ✗）。
  const p = path.resolve(decodeURIComponent(res.uri.replace(/^file:\/\//, '')));
  let text;
  try {
    text = fs.readFileSync(p, 'utf8');
  } catch {
    return null;
  }
  const line = res.range.start.line;
  return { file: p, line: line + 1, text: (text.split('\n')[line] || '').trim() };
}

async function main() {
const problems = [];
const ok = (cond, msg) => {
  if (!cond) problems.push(msg);
};

// ---------- ① 项目模式：`a = b` 的 `=` ----------
{
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g96-eq-'));
  fs.writeFileSync(path.join(dir, 'sokonanoda.toml'), 'entry = "Canvas.sokonanoda"\n');
  fs.writeFileSync(path.join(dir, 'SetLib.sokonanoda'), 'def Set (α : Type) : Type := α -> Prop\n');
  const src = 'import SetLib\n\ntheorem eq_test (α : Type) (a b : α) (h : a = b) : a = b := h\n';
  const file = path.join(dir, 'Canvas.sokonanoda');
  fs.writeFileSync(file, src);
  const eqOff = src.indexOf(' = ');
  const res = await session(dir, file, src, [
    { label: 'def_on_eq', pos: posOf(src, eqOff + 1) },
  ]);
  const at = landing(res.def_on_eq);
  ok(at !== null, '`=`（项目模式）上 F12 仍是 null ✗');
  if (at) {
    ok(
      at.file === PRELUDE_R && at.text.startsWith('-- sokonanoda:builtin-notation "=" =>'),
      `\`=\` 的 F12 落点不对：${at.file}:${at.line} = ${JSON.stringify(at.text)}`,
    );
  }
}

// ---------- ② 单文件（无 import）：`=` ----------
{
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g96-eq-solo-'));
  const src = 'theorem eq_test (a b : Prop) (h : a = b) : a = b := h\n';
  const file = path.join(dir, 'Solo.sokonanoda');
  fs.writeFileSync(file, src);
  const eqOff = src.indexOf(' = ');
  const res = await session(dir, file, src, [
    { label: 'def_on_eq', pos: posOf(src, eqOff + 1) },
  ]);
  const at = landing(res.def_on_eq);
  ok(at !== null, '`=`（单文件）上 F12 仍是 null ✗');
  if (at) {
    ok(
      at.file === PRELUDE_R && at.text.startsWith('-- sokonanoda:builtin-notation "=" =>'),
      `\`=\`（单文件）的 F12 落点不对：${at.file}:${at.line} = ${JSON.stringify(at.text)}`,
    );
  }
}

// ---------- ③ 对照：`∧` 仍然跳得动（防"修一个坏一个"） ----------
{
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g96-and-'));
  const src = 'theorem and_test (a b : Prop) (h : a ∧ b) : a ∧ b := h\n';
  const file = path.join(dir, 'Solo.sokonanoda');
  fs.writeFileSync(file, src);
  const off = src.indexOf('∧');
  const res = await session(dir, file, src, [{ label: 'def_on_and', pos: posOf(src, off) }]);
  const at = landing(res.def_on_and);
  ok(at !== null, '`∧` 上 F12 变成 null 了（回归 ✗）');
  if (at) {
    ok(
      at.file === PRELUDE_R && at.text.startsWith('-- sokonanoda:builtin-notation "∧" =>'),
      `\`∧\` 的 F12 落点不对：${at.file}:${at.line} = ${JSON.stringify(at.text)}`,
    );
  }
}

// ---------- ④ prelude 指令注释行里的目标名（第二跳） ----------
{
  const text = fs.readFileSync(PRELUDE, 'utf8');
  const lineStart = text.indexOf('-- sokonanoda:builtin-notation "∧" => And');
  ok(lineStart >= 0, 'prelude 里找不到 `"∧" => And` 那条登记行（形状变了？）');
  const andOff = lineStart + '-- sokonanoda:builtin-notation "∧" => '.length + 1; // `And` 中间
  const res = await session(path.dirname(PRELUDE), PRELUDE, text, [
    { label: 'def_on_comment_And', pos: posOf(text, andOff) },
  ]);
  const at = landing(res.def_on_comment_And);
  ok(at !== null, '注释登记行里的 `And` 上 F12 仍是 null ✗');
  if (at) {
    ok(
      at.file === PRELUDE_R && at.text.startsWith('inductive And'),
      `注释里 \`And\` 的落点不是定义那一行：${at.file}:${at.line} = ${JSON.stringify(at.text)}`,
    );
  }
}

if (problems.length === 0) {
  console.log('G-96 已修 ✓ —— `=`（项目/单文件）落 prelude 登记行 · 注释里的 `And` 落 `inductive And` · `∧` 未回归');
  process.exit(1);
}
console.log('G-96 缺口仍在（或出现回归）✗：');
for (const p of problems) console.log('  - ' + p);
process.exit(0);
}

main().catch((e) => {
  console.error('G-96' + ' 探针异常（按环境问题处理，不是「已修」）：', e && e.message ? e.message : e);
  process.exit(2);
});
