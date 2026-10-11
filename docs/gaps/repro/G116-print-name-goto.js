#!/usr/bin/env node
// G116 —— `#print <名字>` 里的名字**跳不了定义**（F12 / `textDocument/definition`）。
//
// 用户现场（2026-10-11）：「`#print Set.singleton` 里 `Set.singleton` 不支持 F12 跳转」。
//
// 病根（`crates/front/src/compile/check/walk.rs::print`）：这条命令只 push
// `PendingOp::Print`、**从不 push `CmdHover`** ⇒ `#print` 那一行**一个 hover 行都没有**
// ⇒ `definition_at`（它只认 hover 行的 `resolution`）答 `null` ✗。hover 那半看不出来
// 是因为 `command_line_hover` 在**行**一级顶上（显示命令自己的输出，C3 契约 ✓）。
//
// 最小复现（真 LSP 子进程 + 真 `textDocument/definition`，项目模式 = 用户现场的
// 「库模块里的名字」那一半）：
//   对照组 `#check Set.singleton` → **必须**跳得到 `SetLib.sokonanoda` 的 `def singleton`
//     （它走 `elab_expr` 那条已有的 hover 行通道 ✓；跳不了 ⇒ 夹具/环境异常 ⇒ exit 2）
//   本体   `#print Set.singleton` → 跳得到 = 已修（exit 1）· 跳不了 = 缺口仍在（exit 0）
//
// 退出码（docs/gaps/README.md）：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常
'use strict';
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SOKO = path.join(ROOT, 'scripts', 'soko');
const uri = (f) => 'file://' + f;
const posOf = (text, offset) => ({
  line: text.slice(0, offset).split('\n').length - 1,
  character: (text.slice(0, offset).split('\n').pop() || '').length,
});

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
  let id = 0;
  const write = (payload) => {
    const text = JSON.stringify(payload);
    child.stdin.write(`Content-Length: ${Buffer.byteLength(text)}\r\n\r\n${text}`);
    return payload.id;
  };
  const send = (method, params) => write({ jsonrpc: '2.0', id: ++id, method, params });
  const notify = (method, params) =>
    child.stdin.write(
      (() => {
        const text = JSON.stringify({ jsonrpc: '2.0', method, params });
        return `Content-Length: ${Buffer.byteLength(text)}\r\n\r\n${text}`;
      })(),
    );
  const responseFor = (want) =>
    new Promise((res) => {
      const timer = setInterval(() => {
        const i = queue.findIndex((m) => m.id === want);
        if (i >= 0) {
          clearInterval(timer);
          res(queue.splice(i, 1)[0]);
        }
      }, 5);
      setTimeout(() => {
        clearInterval(timer);
        res(null);
      }, 30000);
    });
  return { send, notify, responseFor, stop: () => child.kill() };
}

const LIB = 'namespace Set\n\ndef singleton (a : Prop) : Prop := a\n\nend Set\n';
const ENTRY = 'import SetLib\n\n#check Set.singleton\n#print Set.singleton\n';

(async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g116-'));
  const lib = path.join(dir, 'SetLib.sokonanoda');
  const entry = path.join(dir, 'Canvas.sokonanoda');
  fs.writeFileSync(lib, LIB);
  fs.writeFileSync(entry, ENTRY);

  const lsp = startLsp();
  lsp.send('initialize', { processId: process.pid, rootUri: uri(dir), capabilities: {} });
  await lsp.responseFor(1);
  lsp.notify('initialized', {});
  lsp.notify('textDocument/didOpen', {
    textDocument: { uri: uri(entry), languageId: 'sokonanoda', version: 1, text: ENTRY },
  });
  await new Promise((r) => setTimeout(r, 3000));

  const jump = async (needle) => {
    const at = ENTRY.indexOf(needle) + '#check '.length + 2;
    const reply = await lsp.responseFor(
      lsp.send('textDocument/definition', {
        textDocument: { uri: uri(entry) },
        position: posOf(ENTRY, at),
      }),
    );
    const result = reply && reply.result;
    if (!result || !result.uri) return null;
    // **断言落点行**（只断言"非 null"会放过原地跳 ✗）：必须是库里那条 `def singleton`。
    const landed = fs
      .readFileSync(result.uri.replace('file://', ''), 'utf8')
      .split('\n')[result.range.start.line];
    return { file: result.uri, line: (landed || '').trim() };
  };

  const check = await jump('#check Set.singleton');
  const print = await jump('#print Set.singleton');
  lsp.stop();

  console.log(`[对照 #check Set.singleton] ⇒ ${check ? `${check.file} :: ${check.line}` : 'null'}`);
  console.log(`[本体 #print Set.singleton] ⇒ ${print ? `${print.file} :: ${print.line}` : 'null'}`);
  const landedOnDef = (r) => r && r.line.startsWith('def singleton');
  if (!landedOnDef(check)) {
    console.error('对照组（`#check`）都跳不到 `def singleton` ⇒ 夹具/环境异常');
    process.exit(2);
  }
  if (landedOnDef(print)) {
    console.log('已修：`#print <名字>` 的名字能 F12 到定义');
    process.exit(1);
  }
  console.log('缺口仍在：`#print <名字>` 的名字跳不了定义（`#check` 那条路可以 ⇒ 差别在命令本身）');
  process.exit(0);
})();
