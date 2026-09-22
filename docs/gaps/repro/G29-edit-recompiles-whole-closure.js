// G-29：**编辑**项目文件 ⇒ 整条 import 闭包从零重编（缓存帮不上）。
//
// 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（docs/gaps/README.md）。
//
// 判据：一次"改一行"的 didChange，从发出到**该版本**的诊断回来。
// 已修的标准：它应当**显著小于**冷开（依赖没变就不该重编依赖）。
const { spawn } = require('node:child_process')
const fs = require('node:fs'), os = require('node:os'), path = require('node:path')
const ROOT = path.join(__dirname, '..', '..', '..')
const SOKO = path.join(ROOT, 'scripts', 'soko')
const ENTRY = path.join(ROOT, 'courses/set-theory/units/unit08-images-preimages.sokonanoda')

function startLsp(env) {
  const child = spawn(process.execPath, [SOKO, 'lsp'], { stdio: ['pipe', 'pipe', 'pipe'], env })
  let buf = Buffer.alloc(0)
  const queue = []
  child.stdout.on('data', (chunk) => {
    buf = Buffer.concat([buf, chunk])
    for (;;) {
      const sep = buf.indexOf('\r\n\r\n')
      if (sep < 0) return
      const m = /Content-Length: (\d+)/i.exec(buf.slice(0, sep).toString())
      if (!m) return
      const len = Number(m[1])
      if (buf.length < sep + 4 + len) return
      queue.push(JSON.parse(buf.slice(sep + 4, sep + 4 + len).toString()))
      buf = buf.slice(sep + 4 + len)
    }
  })
  const send = (msg) =>
    child.stdin.write(`Content-Length: ${Buffer.byteLength(JSON.stringify(msg))}\r\n\r\n${JSON.stringify(msg)}`)
  const wait = (pred) =>
    new Promise((resolve) => {
      const tick = () => {
        const i = queue.findIndex(pred)
        if (i >= 0) return resolve(queue.splice(i, 1)[0])
        setTimeout(tick, 1)
      }
      tick()
    })
  return { child, send, wait }
}

;(async () => {
  if (!fs.existsSync(ENTRY)) {
    console.error(`   → 找不到 ${ENTRY}`)
    process.exit(2)
  }
  const cacheDir = fs.mkdtempSync(path.join(os.tmpdir(), 'g29-'))
  const env = { ...process.env, SOKONANODA_CACHE_DIR: cacheDir }
  delete env.SOKONANODA_BIN
  delete env.SOKONANODA_LSP_BIN

  const lsp = startLsp(env)
  let text = fs.readFileSync(ENTRY, 'utf8')
  const needle = 'demo_mem_image'
  if (!text.includes(needle)) {
    console.error(`   → 夹具前提不成立：unit08 里没有 ${needle}`)
    process.exit(2)
  }

  lsp.send({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { processId: null, rootUri: 'file://' + ROOT, capabilities: {} } })
  await lsp.wait((m) => m.id === 1)
  lsp.send({ jsonrpc: '2.0', method: 'initialized', params: {} })

  const t0 = Date.now()
  lsp.send({ jsonrpc: '2.0', method: 'textDocument/didOpen', params: { textDocument: { uri: 'file://' + ENTRY, languageId: 'sokonanoda', version: 1, text } } })
  await lsp.wait((m) => m.method === 'textDocument/publishDiagnostics' && m.params.version === 1)
  const cold = Date.now() - t0

  // 第二次打开（同缓存）应当是毫秒级——缓存对**打开**是有效的。
  lsp.send({ jsonrpc: '2.0', method: 'textDocument/didClose', params: { textDocument: { uri: 'file://' + ENTRY } } })
  const t1 = Date.now()
  lsp.send({ jsonrpc: '2.0', method: 'textDocument/didOpen', params: { textDocument: { uri: 'file://' + ENTRY, languageId: 'sokonanoda', version: 2, text } } })
  await lsp.wait((m) => m.method === 'textDocument/publishDiagnostics' && m.params.version === 2)
  const warmOpen = Date.now() - t1

  // **改一行**（真存在的标识符）。
  const edited = text.replace(needle, needle + '_x')
  const t2 = Date.now()
  lsp.send({ jsonrpc: '2.0', method: 'textDocument/didChange', params: { textDocument: { uri: 'file://' + ENTRY, version: 3 }, contentChanges: [{ text: edited }] } })
  await lsp.wait((m) => m.method === 'textDocument/publishDiagnostics' && m.params.version === 3)
  const edit = Date.now() - t2
  lsp.child.kill()

  console.log('== unit08（4 个 import）：打开 / 热开 / **改一行** ==')
  console.log(`   冷开 = ${cold}ms`)
  console.log(`   热开（缓存命中）= ${warmOpen}ms`)
  console.log(`   改一行 = **${edit}ms**`)

  // 已修的标准：编辑**便宜到和热开同量级**——依赖一个字节没变，就不该有编译。
  //
  // 为什么不用 `edit * 2 < cold`（2026-09-21 改）：那条判据的余量只有 ~13%
  // （实测 cold 4413ms / edit 2489ms），机器一抖就翻面 ⇒ `gap.py check` 随机红。
  // 冷开里含**进程启动**，把启动时间混进分母本来就不该是判据。
  // 热开（同缓存、零重编）才是"什么都不用做"的基线：修好后 edit 应当与它同量级。
  const budget = 10 * warmOpen + 200
  if (edit < budget) {
    console.log(`结论：G-29 已修——编辑不再重编整条闭包（改一行 ${edit}ms < 预算 ${budget}ms）。`)
    process.exit(1)
  }
  console.error(
    '结论：G-29 仍在——**编辑**项目文件会从零重编整条 import 闭包（依赖一个字节没变也照编），' +
      `缓存只对"打开"有效（冷开 ${cold}ms / 热开 ${warmOpen}ms / 改一行 ${edit}ms，` +
      `预算 ${budget}ms = 10×热开 + 200ms）。`,
  )
  process.exit(0)
})().catch((error) => {
  console.error(`   → 探针异常：${error && error.stack ? error.stack : error}`)
  process.exit(2)
})
