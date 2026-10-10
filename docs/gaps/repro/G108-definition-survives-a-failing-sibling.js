#!/usr/bin/env node
// G108 —— 一处报错（`sorry` / tactic 失败）**不该**毁掉同一份文件里其它正确代码的
// 符号跳转（F12 / `textDocument/definition`）。
//
// 用户现场（2026-10-10）：
//   theorem mem_of_subset_singleton (α : Type) (A : Set α) (a : α) (h : {a} ⊆ A) : a ∈ A := by
//     apply h
//     exact Eq.refl a
//     sorry
//   「加了 sorry（或其他引起报错的 tactic），就会导致 `Eq.refl` 无法跳转（F12 失效）」。
//
// 最小复现（本文件跑三个夹具，真 LSP 子进程 + 真 `textDocument/definition`）：
//   A 干净            → `Eq.refl` 必须跳得到 `prelude/Prelude.sokonanoda`（对照组）
//   B 证明后再来 sorry → `Eq.refl` **同样要跳得到**
//   C 前面一条 tactic 失败 → `Eq.refl` **同样要跳得到**
// 判据：A 必须能跳（环境自检）；B/C 若能跳 ⇒ 已修（exit 1）；只要 B/C 有一个跳不了
// ⇒ 缺口仍在（exit 0）。
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

(async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g108-'));
  const cases = {
    A_clean: 'theorem t (a : Nat) : a = a := by\n  exact Eq.refl a\n',
    B_redundant_sorry: 'theorem t (a : Nat) : a = a := by\n  exact Eq.refl a\n  sorry\n',
    C_failed_tactic: 'theorem t (a : Nat) : a = a := by\n  exact Nat.zero\n  exact Eq.refl a\n',
  };
  const jumped = {};
  for (const [tag, src] of Object.entries(cases)) {
    const file = path.join(dir, `${tag}.sokonanoda`);
    fs.writeFileSync(file, src);
    const lsp = startLsp();
    lsp.send('initialize', { processId: process.pid, rootUri: uri(dir), capabilities: {} });
    await lsp.responseFor(1);
    lsp.notify('initialized', {});
    lsp.notify('textDocument/didOpen', {
      textDocument: { uri: uri(file), languageId: 'sokonanoda', version: 1, text: src },
    });
    await new Promise((r) => setTimeout(r, 3000));
    const at = src.indexOf('Eq.refl') + 3;
    const reply = await lsp.responseFor(
      lsp.send('textDocument/definition', {
        textDocument: { uri: uri(file) },
        position: posOf(src, at),
      }),
    );
    jumped[tag] = reply && reply.result ? reply.result.uri || JSON.stringify(reply.result) : null;
    console.log(`[${tag}] definition ⇒ ${jumped[tag] === null ? 'null（跳不了 ✗）' : jumped[tag]}`);
    lsp.stop();
    await new Promise((r) => setTimeout(r, 300));
  }
  if (!jumped.A_clean) {
    console.error('对照组 A（干净文件）都跳不了 ⇒ 环境/形状异常');
    process.exit(2);
  }
  if (jumped.B_redundant_sorry && jumped.C_failed_tactic) {
    console.log('已修：同文件里有报错时，其它正确符号仍可 F12');
    process.exit(1);
  }
  console.log('缺口仍在：报错（sorry / tactic 失败）之后，同一份文件里正确的 `Eq.refl` 跳不了');
  process.exit(0);
})();
