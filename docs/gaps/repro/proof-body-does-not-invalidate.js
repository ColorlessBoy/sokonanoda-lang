// **诊断探针（不带 G-号 ✓）**：改一条**靠前定理的证明体**（陈述一字不动）⇒ 后面还重跑几趟前缀？
//
// ⚠ **为什么不带号**（2026-10-04 值守记账复核 ✓）：它服务的是 `ONBOARDING.md` §0.2 #6
// 「只改证明，后面不需要重编」✓，**不在这份缺口清单里** ✗ —— 先前借了 **G-85** 的号 ✗
// （G-85 是另一件事：省掉前导隐式实参 + 结果再收一个实参时实参被按位置装错 ✓）⇒ 已改名 ✓。
//
// 用户 2026-10-04 点名的特性：「只改证明，后面不需要重编」。
//
// **为什么它是判据而不是感觉**：证明体对**定理**是 proof-irrelevant 的
// （`theorem` 的值是证明，证明不参与 `def_eq`）⇒ 改它**不该**让后面任何一条判定失效。
// 而今天的键把**前缀原文**折进去了（`judge_infer_key` 的 `prefix_src`/`extra_prefix`）
// ⇒ 原文一变，后面**每一条**判定缓存全部失效 ✗。
//
// **两刀都要测**（值守 2026-10-04 指令 ✓）：
//   * `same`   —— **等长**改（总字节不变 ⇒ 后面命令不平移 ✓）
//   * `shift`  —— **不等长**改（后面命令**整体平移** ⇒ 同时考"脏集按偏移判"那条 ✗）
//
// 用法：node docs/gaps/repro/proof-body-does-not-invalidate.js
// 退出码：0 = 跑完（它只报告；判据在 LSP 测试 `lsp_keystroke_structure.rs` 里）
"use strict";
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const ROOT = path.join(__dirname, "..", "..", "..");
const SOKO = path.join(ROOT, "scripts", "soko");
const ENTRY =
  process.env.SOKO_G85_ENTRY ||
  path.join(ROOT, "courses/set-theory/units/I.3/unit08-images-preimages.sokonanoda");

function startLsp(env) {
  const child = spawn(process.execPath, [SOKO, "lsp"], { stdio: ["pipe", "pipe", "pipe"], env });
  let buf = Buffer.alloc(0);
  let trace = "";
  const queue = [];
  child.stderr.on("data", (chunk) => {
    trace += chunk.toString();
  });
  child.stdout.on("data", (chunk) => {
    buf = Buffer.concat([buf, chunk]);
    for (;;) {
      const sep = buf.indexOf("\r\n\r\n");
      if (sep < 0) return;
      const m = /Content-Length: (\d+)/i.exec(buf.slice(0, sep).toString());
      if (!m) return;
      const len = Number(m[1]);
      if (buf.length < sep + 4 + len) return;
      queue.push(JSON.parse(buf.slice(sep + 4, sep + 4 + len).toString()));
      buf = buf.slice(sep + 4 + len);
    }
  });
  const send = (msg) =>
    child.stdin.write(
      `Content-Length: ${Buffer.byteLength(JSON.stringify(msg))}\r\n\r\n${JSON.stringify(msg)}`,
    );
  const wait = (pred) =>
    new Promise((resolve) => {
      const tick = () => {
        const i = queue.findIndex(pred);
        if (i >= 0) return resolve(queue.splice(i, 1)[0]);
        setTimeout(tick, 1);
      };
      tick();
    });
  return { child, send, wait, trace: () => trace };
}

/// 每一刀都是**真改动**（不许改回去 —— 改回去会命中**磁盘缓存** ⇒ 读数把"没生效"看成"很快" ✗）。
///
/// **两刀**：第 1 刀**预热**（v2 是冷启动，缓存还空着 ⇒ `prefix` 必然大 ✓），
/// 第 2 刀才是**被量的那一刀**（v3 ✓ —— 热态，后面该复用 ✓）。
function edits(kind, text) {
  const m = /^(  )(Set\.mem_image .*)$/m.exec(text);
  if (!m) return undefined;
  const [, indent, body] = m;
  if (kind === "same") {
    // **等长**：缩进与行尾空白对调（总字节不变 ⇒ 后面命令不平移 ✓）。
    return [
      [`${indent}${body}\n`, `${body}${indent}\n`],
      // 第二刀：把一个空格从**行尾**挪到**行首**（总字节仍不变 ✓，且与第一刀不同 ✓）。
      [`${body}${indent}\n`, ` ${body}${indent.slice(1)}\n`],
    ];
  }
  // **不等长**：给证明体加括号（语义相同 ✓，+2 字节 ⇒ 后面命令**整体平移** ✗）。
  return [
    [`${indent}${body}`, `${indent}(${body})`],
    [`${indent}(${body})`, `${indent}((${body}))`],
  ];
}

async function run(kind, cacheDir) {
  const env = { ...process.env, SOKONANODA_CACHE_DIR: cacheDir, SOKO_LSP_TRACE: "1" };
  delete env.SOKONANODA_BIN;
  delete env.SOKONANODA_LSP_BIN;
  const lsp = startLsp(env);
  const uri = "file://" + ENTRY;
  let text = fs.readFileSync(ENTRY, "utf8");
  const pairs = edits(kind, text);
  if (!pairs) throw new Error("找不到第一条 theorem 的证明体那一行 ✗");
  lsp.send({
    jsonrpc: "2.0",
    id: 1,
    method: "initialize",
    params: { processId: null, rootUri: "file://" + ROOT, capabilities: {} },
  });
  await lsp.wait((m) => m.id === 1);
  lsp.send({ jsonrpc: "2.0", method: "initialized", params: {} });
  lsp.send({
    jsonrpc: "2.0",
    method: "textDocument/didOpen",
    params: { textDocument: { uri, languageId: "sokonanoda", version: 1, text } },
  });
  await lsp.wait((m) => m.method === "textDocument/publishDiagnostics" && m.params.version === 1);
  const out = [];
  for (let i = 0; i < pairs.length; i++) {
    const before = text;
    text = text.replace(pairs[i][0], pairs[i][1]);
    if (text === before) throw new Error(`第 ${i + 1} 刀没改到东西 ✗`);
    lsp.send({
      jsonrpc: "2.0",
      method: "textDocument/didChange",
      params: {
        textDocument: { uri, version: i + 2 },
        contentChanges: [{ text }],
      },
    });
    await lsp.wait(
      (m) => m.method === "textDocument/publishDiagnostics" && m.params.version === i + 2,
    );
    out.push({ len: text.length, delta: text.length - before.length });
  }
  lsp.child.kill();
  return { out, trace: lsp.trace() };
}

(async () => {
  console.log("== G-85 改**证明体**（陈述不动）⇒ 后面还重跑几趟前缀？==");
  for (const kind of ["same", "shift"]) {
    const cache = fs.mkdtempSync(path.join(require("node:os").tmpdir(), `g85-${kind}-`));
    const { out, trace } = await run(kind, cache);
    const label = kind === "same" ? "等长（总字节不变）" : "不等长（后面整体平移）";
    console.log(`-- ${label}：` + out.map((o) => `len=${o.len} Δ=${o.delta}`).join(" · "));
    for (const line of trace.split("\n")) {
      if (line.includes("LSP_TRACE compile")) console.log("   " + line.trim());
    }
  }
  console.log("读法：`prefix=` 是这次按键里 judge_infer 重跑前缀的趟数 ⇒ **目标 0** ✓");
  console.log("（`SOKO_LSP_TRACE=1` 的 `LSP_TRACE compile` 行；本脚本只报告 ✓）");
})();
