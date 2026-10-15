#!/usr/bin/env node
// G-25 自断言复现：**含 `import` 的文档在 LSP 上没有任何持久缓存，每次打开都从零重编**。
//
// 用户原话：「编译很慢」「新打开一个文件就有临时编译」。
//
// 台账 `today`（0.63.0 实测，修前；release LSP，`didOpen → 诊断`）：
//   units/I.1/unit01（2 个 import）= **1781ms**；units/I.3/unit08（4）= **4599ms**；
//   unit12（7）= **8465ms**；无 `import` 的 course/unit1 = ~100ms（走单文件缓存）。
//   同一份文档在同一会话里再改（内容相同）只要 121–476ms（`Session` 的
//   "内容未变零重编译"是好的）⇒ **贵的是"第一次打开一份新文档"**。
//
// 根因（`crates/lsp/src/lib.rs:148-156`）：
//
//     let has_imports = sokonanoda_front::project::is_project_source(text);
//     let cached = if cfg!(test) || has_imports { None } else { cache::load(text, &options) };
//
//   ⇒ 有 `import` 就**既不读也不写**任何缓存；`:183-194` 的 `cache::store` 同样被门住。
//   项目缓存只活在 CLI crate（`crates/cli/src/project_cache.rs`），LSP 依赖不到
//   （`grep digest crates/lsp` = 0）。
//
// **量具必须用两个独立进程**：同一个 LSP 进程里 `Doc`/`Session` 会保留，
// 第二次 `didOpen` 本来就快（测不出问题）；"重启编辑器后重开文件"才是用户场景。
//
// 修后契约：第二个独立进程的 `didOpen → 诊断` 必须**明显快于**第一个
// （缓存命中；目标 <1/3）。
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

const LIB = [
  'def Set (α : Type) : Type := α -> Prop',
  'def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a',
  'infix:50 " ∈ " => Set.mem',
  'def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x',
  'infix:50 " ⊆ " => Set.subset',
  // 夹具要**足够大**才有判别力（T-A10 实测的教训）：缓存省掉的是**内核检查**，
  // 而算摘要本身仍要读 + 解析整个闭包。夹具太小时解析占满全部时间，冷热都 ~10ms，
  // 量不出差别（第一版就是这样，`hit=true` 却看不出提速）。
  //
  // 24 + 12 条是**调过的**：再小就量不出比值，再大门禁跑不起
  // （160 + 80 时冷开 42s、整个复现 85s——判别力够了但太慢）。
  ...Array.from({ length: 24 }, (_, i) =>
    `theorem lib_lemma_${i + 1} (α : Type) (A B : Set α) (h : A ⊆ B) : A ⊆ B := h`),
  '',
].join('\n');

// 第二个模块，让闭包不止一层。
const MID = [
  'import SetLib',
  '',
  ...Array.from({ length: 12 }, (_, i) =>
    `theorem mid_lemma_${i + 1} (α : Type) (A B : Set α) (h : A ⊆ B) : A ⊆ B := h`),
  '',
].join('\n');

const CANVAS = [
  'import SetLib',
  'import MidLib',
  '',
  'theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h',
  '',
  'theorem open_one (α : Type) (A B : Set α) : A ⊆ B := by',
  '  sorry',
  '',
].join('\n');

const uri = (file) => 'file://' + file;

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
    next(timeoutMs = 120000) {
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

/// `LSP_TRACE compile …` 行**等多久**（毫秒）。轨迹行由服务端在 `publishDiagnostics`
/// **之前**写，所以正常情况下它早就到了 ⇒ 这个等待只在"它被拆成片段"时才真的用上。
const TRACE_WAIT_MS = Number(process.env.SOKO_G25_TRACE_WAIT_MS || 2000);

/// **stderr 的行缓冲**（⚠ 这是本探针 2026-10-15 修掉的洞，见 `docs/gaps/README.md`）。
///
/// 为什么**必须**缓冲：`eprintln!` 走的是 Rust 的 `Stderr`，它是**无缓冲**的 ⇒
/// 一次 `write_fmt` 会按**格式片段**发多个 `write()`（`"LSP_TRACE compile "` ·
/// uri · `" v"` · version · … · `" modules="` · modules · …）⇒ 每个 `'data'` 块
/// **可能只是一行的一段**。旧写法 `String(chunk).split('\n')` 把**每个块当整行**：
/// 片段里照样含 `LSP_TRACE compile`（于是被收进 `traceLines`），却**不含
/// `modules=`** ⇒ `/modules=(\d+)/` 不匹配 ⇒ **那一相记 0**。
///
/// 实测（2026-10-15，本机 release 仓库构建，`/tmp/g25K-chunks.js` 逐块读数）：
/// `chunks=1 len=140 endsWithNewline=false tail="1 31ms publish=1 modules=3 by="`
/// —— 一行 197 字节被**截成 140 字节**送达。CI（2 核 ubuntu，ledger job 第一次真跑）
/// 读到的正是这个洞的最坏落点：冷热两相都 0 ⇒ 判据 `hot < cold` 恒假 ⇒
/// 报「缺口仍在」✗（原文见 `docs/ux-queue-extras.jsonl` 的
/// `G25-PROBE-DEGENERATE-IN-CI`）。
function stderrLineReader() {
  const traceLines = [];
  const raw = [];
  let pending = '';
  return {
    feed(chunk) {
      pending += String(chunk);
      for (;;) {
        const nl = pending.indexOf('\n');
        if (nl < 0) break;
        const line = pending.slice(0, nl);
        pending = pending.slice(nl + 1);
        if (!line.trim()) continue;
        raw.push(line);
        if (line.includes('LSP_TRACE compile')) traceLines.push(line);
      }
    },
    traceLines,
    raw,
    /// 已经收到、但**还没收尾**的那一段（判"观测不到"时要说清看到了什么）。
    get pending() { return pending; },
  };
}

/** 起一个**全新**的 LSP 进程，`didOpen` 入口，返回 `didOpen → 诊断` 的毫秒数。 */
async function openOnce(dir, entry, cacheDir) {
  // **结构性判据的来源**（2026-10-12）：`SOKO_LSP_TRACE=1` 每次编译打一行
  // `LSP_TRACE compile … modules=<真实闭包模块重编数> …`。墙钟比值在本仓是**禁止的判据**
  // （"同一处连红 3 次"里 3 次都是假红：同代码 44ms↔2431ms）；而 `modules=` 是**计数** ⇒ 噪声免疫。
  const env = { ...process.env, SOKONANODA_CACHE_DIR: cacheDir, SOKO_LSP_TRACE: '1' };
  const reader = stderrLineReader();
  delete env.SOKONANODA_NO_CACHE;
  delete env.SOKONANODA_BIN;
  delete env.SOKONANODA_LSP_BIN;
  const child = spawn(process.execPath, [SOKO, 'lsp'], { stdio: ['pipe', 'pipe', 'pipe'], env });
  const lsp = (() => {
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
    child.stderr.on('data', (chunk) => reader.feed(chunk));
    return {
      send(message) {
        const body = Buffer.from(JSON.stringify(message), 'utf8');
        child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
        child.stdin.write(body);
      },
      next(timeoutMs = 120000) {
        if (queue.length) return Promise.resolve(queue.shift());
        return new Promise((resolve, reject) => {
          const timer = setTimeout(() => reject(new Error('LSP 超时未应答')), timeoutMs);
          waiters.push((message) => { clearTimeout(timer); resolve(message); });
        });
      },
      stop() { try { child.kill(); } catch (_) { /* 已退出 */ } },
    };
  })();

  lsp.send({
    jsonrpc: '2.0', id: 1, method: 'initialize',
    params: {
      processId: process.pid,
      rootUri: uri(dir),
      capabilities: {},
      workspaceFolders: [{ uri: uri(dir), name: 'g25' }],
    },
  });
  await responseFor(lsp, 1);
  lsp.send({ jsonrpc: '2.0', method: 'initialized', params: {} });

  const started = Date.now();
  lsp.send({
    jsonrpc: '2.0', method: 'textDocument/didOpen',
    params: { textDocument: { uri: uri(entry), languageId: 'sokonanoda', version: 1, text: CANVAS } },
  });
  for (;;) {
    const message = await lsp.next();
    if (message.method === 'textDocument/publishDiagnostics' &&
        message.params && message.params.uri === uri(entry)) break;
  }
  const elapsed = Date.now() - started;
  // **等那一行收尾**（有界）：轨迹行写在诊断**之前** ⇒ 正常情况下这里立刻满足；
  // 只有它被拆成片段、尾巴还在管道里时才会真等（见 `stderrLineReader`）。
  // ⚠ **不是**在等"计数变好"：等到的是**完整的一行**，`modules=` 是多少就记多少
  // ⇒ 判据一条都没松 ✓。
  const traceDeadline = Date.now() + TRACE_WAIT_MS;
  while (reader.traceLines.length === 0 && Date.now() < traceDeadline) {
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  // `modules=` 是**计数**（真闭包模块重编数）⇒ 结构性判据用这个，不用墙钟 ✓
  let modules = 0;
  for (const line of reader.traceLines) {
    const m = /modules=(\d+)/.exec(line);
    if (m) modules += Number(m[1]);
  }
  lsp.stop();
  return {
    elapsed,
    modules,
    /// 收到几条**完整**的 `LSP_TRACE compile` 行 —— 判"到底观测到没有"的唯一依据 ✓
    observed: reader.traceLines.length,
    raw: reader.raw,
    pending: reader.pending,
  };
}

(async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g25-lsp-'));
  const cacheDir = path.join(dir, 'cache');
  const entry = path.join(dir, 'Canvas.sokonanoda');
  fs.writeFileSync(path.join(dir, 'SetLib.sokonanoda'), LIB);
  fs.writeFileSync(path.join(dir, 'MidLib.sokonanoda'), MID);
  fs.writeFileSync(entry, CANVAS);

  // **判据是"冷 vs 热"**，不是"两次都热"。
  //
  // 第一版探针先用 CLI 预热、再开两次——两次都是热的，比值恒为 ~1×，
  // 无论 LSP 读不读缓存（实测：夹具放大到 240 条声明之后仍然是 80ms vs 65ms，
  // 而两次都 `hit=true`）。那是探针自己的前提错了，不是缺口。
  //
  // 正确的三步：① 全新缓存 ⇒ **冷**；② CLI `build` 预热；③ 同一份缓存 ⇒ **热**。
  const cold = await openOnce(dir, entry, cacheDir);

  const warm = spawn(process.execPath, [SOKO, 'build', entry], {
    stdio: 'ignore',
    env: { ...process.env, SOKONANODA_CACHE_DIR: cacheDir },
  });
  await new Promise((resolve) => warm.on('exit', resolve));

  const hot = await openOnce(dir, entry, cacheDir);

  console.log('== 两个**独立** LSP 进程打开同一份项目入口 ==');
  console.log(`   冷开（全新缓存）        = ${cold.elapsed}ms · 重编模块 ${cold.modules}`);
  console.log(`   热开（CLI 预热过缓存）  = ${hot.elapsed}ms · 重编模块 ${hot.modules}` +
    `   （墙钟 ${(cold.elapsed / hot.elapsed).toFixed(1)}× · 模块 ${cold.modules}→${hot.modules}）`);

  // ⭐ **观测不到主体 ⇒ exit 2（环境/形状异常），不是 exit 0（缺口仍在）** ✗
  // （2026-10-15，洞 `G25-PROBE-DEGENERATE-IN-CI`；同族 `G112-REPRO-STALE-BLIND-SPOT`）。
  //
  // 判据 `hot.modules < cold.modules` 的**唯一输入**是 `modules=`，而它只出现在
  // 服务端 stderr 的 `LSP_TRACE compile` 行里。**一行都没收到**时：
  //   冷 = 0、热 = 0 ⇒ `0 < 0` 恒假 ⇒ 旧写法**无条件**报「缺口仍在」✗
  //   —— 把"我没看见"说成了"缺口还在"，而台账是契约（`fixed` ⇒ 应转绿）⇒ 判红 ✗。
  // 这正是 2026-10-15 CI `ledger (1)` 第一次真跑的读数（冷 0 / 热 0，墙钟 68ms/11ms）。
  // ⇒ 现在：**说不清观测到什么，就不许下结论**；把"看到了什么"原样带出来 ✓。
  // ⚠ **判据一条都没松**：观测得到时仍然走下面那条 `hot < cold`，真缺口照样 exit 0 ✓。
  if (cold.observed === 0 || hot.observed === 0) {
    const show = (phase, name) => {
      const tail = phase.raw.length ? phase.raw.slice(-3).join(' ｜ ') : '（一行 stderr 都没有）';
      const cut = phase.pending ? ` ｜未收尾的尾段：${JSON.stringify(phase.pending.slice(-60))}` : '';
      return `   · ${name}：完整 \`LSP_TRACE compile\` 行 ${phase.observed} 条` +
        `（收到 stderr ${phase.raw.length} 行${cut}）⇒ ${tail}`;
    };
    console.error(
      '结论：**观测不到主体** ⇒ 环境/形状异常（exit 2），**不是**「缺口仍在」✗\n' +
        show(cold, '冷相') + '\n' +
        show(hot, '热相') + '\n' +
        '   · 结构性判据的输入是 `modules=`，它只由服务端 stderr 的 `LSP_TRACE compile` 行提供\n' +
        '     ⇒ 一行都没有时 `hot < cold` 恒假 ⇒ 那是**量具瞎了**，不是缺口 ✓\n' +
        '   · 常见原因（按可能性）：`scripts/soko lsp` 解析到的**不是本仓库构建**\n' +
        '     （旧缓存 / 下载 / VS Code 扩展自带 ⇒ 版本可能不带这条轨迹）·\n' +
        '     `SOKO_LSP_TRACE` 没传进服务端 · 服务端 stderr 被吞掉 / 被拆成片段后没等到收尾\n' +
        '   · 先查：`node scripts/soko version --json`（看 `lsp.source`）与\n' +
        '     `SOKO_LSP_TRACE=1 node scripts/soko lsp`（应打 `LSP_TRACE compile …`）\n',
    );
    process.exit(2);
  }

  // **判据 = 结构性计数**（2026-10-12 改）：热开必须**真的少重编模块**
  // —— 墙钟比值在本仓是禁止的判据（实测同代码 44ms↔2431ms；本轮 pre-push 上
  // `hot*3 < cold` 就因为 42ms vs 15ms 的抖动判了一次假红 ✗）。
  // 计数是机器无关的：热开命中缓存 ⇒ 真闭包模块重编数应当明显下降。
  if (hot.modules < cold.modules) {
    console.log(`结论：G-25 已修——第二次打开命中了编译缓存（重编模块 ${cold.modules} ⇒ ${hot.modules}）。`);
    process.exit(1);
  }
  console.error(
    '结论：G-25 仍在——含 `import` 的文档在 LSP 上不读任何缓存，' +
      `每次打开都从零重编（重编模块 冷 ${cold.modules} vs 热 ${hot.modules}，没有下降；` +
      `墙钟 ${cold.elapsed}ms vs ${hot.elapsed}ms 只作参考 ✗）。`,
  );
  process.exit(0);
})().catch((error) => {
  console.error(`   → 探针异常：${error && error.stack ? error.stack : error}`);
  process.exit(2);
});
