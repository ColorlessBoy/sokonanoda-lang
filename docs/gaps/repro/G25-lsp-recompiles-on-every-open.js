#!/usr/bin/env node
// G-25 自断言复现：**含 `import` 的文档在 LSP 上没有任何持久缓存，每次打开都从零重编**。
//
// 用户原话：「编译很慢」「新打开一个文件就有临时编译」。
//
// 台账 `today`（0.63.0 实测，修前；release LSP，`didOpen → 诊断`）：
//   units/unit01（2 个 import）= **1781ms**；unit08（4）= **4599ms**；
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

/** 起一个**全新**的 LSP 进程，`didOpen` 入口，返回 `didOpen → 诊断` 的毫秒数。 */
async function openOnce(dir, entry, cacheDir) {
  const env = { ...process.env, SOKONANODA_CACHE_DIR: cacheDir };
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
  lsp.stop();
  return elapsed;
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
  console.log(`   冷开（全新缓存）        = ${cold}ms`);
  console.log(`   热开（CLI 预热过缓存）  = ${hot}ms   （${(cold / hot).toFixed(1)}×）`);

  if (hot * 3 < cold) {
    console.log('结论：G-25 已修——第二次打开命中了编译缓存，明显快于冷开。');
    process.exit(1);
  }
  console.error(
    '结论：G-25 仍在——含 `import` 的文档在 LSP 上不读任何缓存，' +
      `每次打开都从零重编（冷 ${cold}ms vs 热 ${hot}ms，没有提速）。`,
  );
  process.exit(0);
})().catch((error) => {
  console.error(`   → 探针异常：${error && error.stack ? error.stack : error}`);
  process.exit(2);
});
