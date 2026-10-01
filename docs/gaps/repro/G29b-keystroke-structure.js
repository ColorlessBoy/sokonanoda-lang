// G-29 **诊断探针**（2026-10-01）：一次 `didChange` 到底让 LSP 干了多少活。
//
// 为什么要有它（用户 2026-10-01 反馈：「每次修改代码，整个文件就会被高亮。theorem
// 应该能自然分块，不需要编译其他 theorem 才对」）：
//   `G29-edit-recompiles-whole-closure.js` 量的是**墙钟比值**（它自己的注释记了两次
//   翻面）。本脚本给两个**可复现的视图**，并明确各自能判什么、不能判什么：
//
//   * **LSP 视图**（默认）：`SOKO_LSP_TRACE=1` ⇒ 每次编译一行
//     `LSP_TRACE compile <uri> v<版本> <ms>ms publish=<n>`。
//     判「一次按键 = 几次编译、多久、几条诊断」✓；
//     **判不了**「编译内部按不按声明分块」✗（那要结构计数，见下）。
//   * **结构计数视图**（`--cli`）：`SOKO_STAGE_STATS=1` 跑 CLI 的**同一条编译路**
//     （闭包入口），给 `passes` / `JUDGE_PREFIX runs` / `by_calls` —— **机器无关** ✓。
//
// 用法：
//   node docs/gaps/repro/G29b-keystroke-structure.js            # LSP 视图
//   node docs/gaps/repro/G29b-keystroke-structure.js --cli      # 结构计数视图
//
// 退出码：0 = 跑完（**它只报告，不做判据** —— 判据在台账/设计文档里）· 2 = 环境/形状异常。
"use strict";
const { spawn, spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const ROOT = path.join(__dirname, "..", "..", "..");
const SOKO = path.join(ROOT, "scripts", "soko");
const ENTRY =
  process.env.SOKO_G29_ENTRY ||
  path.join(ROOT, "courses/set-theory/units/unit08-images-preimages.sokonanoda");
const CLI = path.join(ROOT, "target", "release", "sokonanoda");

function startLsp(env) {
  const child = spawn(process.execPath, [SOKO, "lsp"], {
    stdio: ["pipe", "pipe", "pipe"],
    env,
  });
  let buf = Buffer.alloc(0);
  let stderr = "";
  const queue = [];
  child.stderr.on("data", (chunk) => {
    stderr += chunk.toString();
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
  return { child, send, wait, stderr: () => stderr };
}

/// 位置：`first` = 文件里第一条 theorem，`last` = 最后一条。
function editPairs(which) {
  const base = fs.readFileSync(ENTRY, "utf8");
  const names = [...base.matchAll(/^theorem\s+([A-Za-z0-9_]+)/gm)].map((m) => m[1]);
  if (names.length < 2) return undefined;
  const name = which === "last" ? names[names.length - 1] : names[0];
  return [
    [name, name + "_a"],
    [name + "_a", name],
  ];
}

async function lspView(which, pairs, cacheDir) {
  const env = {
    ...process.env,
    SOKONANODA_CACHE_DIR: cacheDir,
    SOKO_LSP_TRACE: "1",
  };
  delete env.SOKONANODA_BIN;
  delete env.SOKONANODA_LSP_BIN;
  const lsp = startLsp(env);
  const uri = "file://" + ENTRY;
  let text = fs.readFileSync(ENTRY, "utf8");
  lsp.send({
    jsonrpc: "2.0",
    id: 1,
    method: "initialize",
    params: { processId: null, rootUri: "file://" + ROOT, capabilities: {} },
  });
  await lsp.wait((m) => m.id === 1);
  lsp.send({ jsonrpc: "2.0", method: "initialized", params: {} });
  const t0 = Date.now();
  lsp.send({
    jsonrpc: "2.0",
    method: "textDocument/didOpen",
    params: { textDocument: { uri, languageId: "sokonanoda", version: 1, text } },
  });
  await lsp.wait(
    (m) => m.method === "textDocument/publishDiagnostics" && m.params.version === 1,
  );
  const openMs = Date.now() - t0;
  const times = [];
  for (let i = 0; i < pairs.length; i++) {
    text = text.replace(pairs[i][0], pairs[i][1]);
    const started = Date.now();
    lsp.send({
      jsonrpc: "2.0",
      method: "textDocument/didChange",
      params: { textDocument: { uri, version: i + 2 }, contentChanges: [{ text }] },
    });
    await lsp.wait(
      (m) => m.method === "textDocument/publishDiagnostics" && m.params.version === i + 2,
    );
    times.push(Date.now() - started);
  }
  lsp.child.kill();
  const trace = lsp
    .stderr()
    .split("\n")
    .filter((l) => l.includes("LSP_TRACE compile"))
    .map((l) => l.trim());
  return { which, openMs, times, trace };
}

function cliView() {
  if (!fs.existsSync(CLI)) return undefined;
  const base = fs.readFileSync(ENTRY, "utf8");
  const out = [];
  const firstPair = editPairs("first");
  const lastPair = editPairs("last");
  for (const [label, text] of [
    ["改**第一条** theorem", base.replace(firstPair[0][0], firstPair[0][1])],
    ["改**最后一条** theorem", base.replace(lastPair[0][0], lastPair[0][1])],
  ]) {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "g29b-cli-"));
    const file = path.join(dir, path.basename(ENTRY));
    fs.writeFileSync(file, text);
    // 模块根 = 入口目录 ⇒ 把依赖也复制过去，闭包才装得起来。
    for (const dep of ["lib", "sokonanoda.toml"]) {
      const src = path.join(path.dirname(ENTRY), "..", dep);
      if (fs.existsSync(src)) {
        fs.cpSync(src, path.join(dir, dep), { recursive: true });
      }
    }
    const r = spawnSync(CLI, ["--json", file], {
      env: { ...process.env, SOKONANODA_CACHE_DIR: dir, SOKO_STAGE_STATS: "1" },
      encoding: "utf8",
      timeout: 600_000,
    });
    const lines = (r.stderr || "")
      .split("\n")
      .filter((l) => /^(STAGE_STATS|JUDGE_PREFIX|JUDGE_STATS|JUDGE_INPLACE)/.test(l))
      .map((l) => l.trim());
    out.push({ label, lines, code: r.status });
  }
  return out;
}

(async () => {
  if (!fs.existsSync(ENTRY)) {
    console.error(`   → 找不到 ${ENTRY}`);
    process.exit(2);
  }
  if (process.argv.includes("--cli")) {
    const rows = cliView();
    if (!rows) {
      console.error(`   → 找不到 ${CLI}（先 scripts/soko update 或 cargo build --release）`);
      process.exit(2);
    }
    console.log(`== G-29 结构计数（CLI 同一条编译路，${path.basename(ENTRY)}）==`);
    for (const row of rows) {
      console.log(`-- ${row.label}（exit ${row.code}）`);
      for (const line of row.lines) console.log(`   ${line}`);
    }
    console.log(
      "   读法：`passes` = 流水线趟数 · `JUDGE_PREFIX runs` = judge_infer 的前缀重跑趟数\n" +
        "        （judge.rs:1941，**不是** by 批次）· `by_calls` = by 引擎调用。",
    );
    process.exit(0);
  }
  const first = editPairs("first");
  const last = editPairs("last");
  if (!first || !last) {
    console.error("   → 夹具前提不成立：入口里找不到两条 theorem");
    process.exit(2);
  }
  // **同一起点**：共用一个缓存目录，先跑一次"只开不改"把缓存烘热 ——
  // 否则第一轮替第二轮付冷编译的钱（实测：冷开 3020ms vs 11ms），两次读数不可比 ✗。
  const cacheDir = fs.mkdtempSync(path.join(os.tmpdir(), "g29b-"));
  await lspView("warmup", [], cacheDir);
  const a = await lspView("first", first, cacheDir);
  const b = await lspView("last", last, cacheDir);
  console.log(`== G-29 LSP 视图（${path.basename(ENTRY)}）==`);
  for (const row of [a, b]) {
    console.log(
      `-- 改${row.which === "first" ? "**第一条**" : "**最后一条**"} theorem：开档 ${row.openMs}ms · 两次按键 ${row.times.join("/")}ms`,
    );
    for (const line of row.trace) console.log(`   ${line}`);
  }
  // **结构计数**（2026-10-01 起 LSP_TRACE 自带）：从 trace 行里抠出来对比。
  // 判据用它（机器无关），墙钟只做数量级兜底 —— `AGENTS.md` 的硬规矩。
  const parse = (trace) =>
    trace.map((line) => {
      const m =
        /v(\d+) (\d+)ms publish=(\d+) modules=(\d+) by=(\d+) infer=(\d+)\/(\d+) prefix=(\d+)/.exec(
          line,
        );
      return m
        ? {
            version: Number(m[1]),
            ms: Number(m[2]),
            publish: Number(m[3]),
            modules: Number(m[4]),
            by: Number(m[5]),
            inferMiss: Number(m[6]),
            inferCalls: Number(m[7]),
            prefixRuns: Number(m[8]),
          }
        : undefined;
    });
  const rows = [
    ["改**第一条**", parse(a.trace)],
    ["改**最后一条**", parse(b.trace)],
  ];
  console.log("   结构计数（每次按键一行；`modules` = 模块编译次数 ⇒ 库层有没有被重编）：");
  for (const [label, list] of rows) {
    for (const row of list) {
      if (!row) continue;
      console.log(
        `     ${label} v${row.version}: ${row.ms}ms · modules=${row.modules} · by=${row.by} · ` +
          `infer_miss=${row.inferMiss}/${row.inferCalls} · prefix_runs=${row.prefixRuns}`,
      );
    }
  }
  console.log(
    "   读法：**一次按键 = 几次编译**看行数 · 结构计数是判据（机器无关）·\n" +
      "        墙钟只作数量级参考（共享机器会翻面，见 G29 复现件的两次翻面记录）；\n" +
      "        更细的读数（`passes` / `JUDGE_PREFIX bytes`）看 `--cli`。",
  );
  process.exit(0);
})().catch((error) => {
  console.error(`   → 探针异常：${error && error.stack ? error.stack : error}`);
  process.exit(2);
});
