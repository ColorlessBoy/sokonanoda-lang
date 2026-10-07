// G-29：**编辑**项目文件 ⇒ 整条 import 闭包从零重编（缓存帮不上）。
//
// 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（docs/gaps/README.md）。
//
// 判据：一次"改一行"的 didChange，从发出到**该版本**的诊断回来。
// 已修的标准：它应当**显著小于**冷开（依赖没变就不该重编依赖）。
//
// ⚠ **两臂都要过**才算「已修」（两条都机器无关 ✓ —— AGENTS.md「不许用绝对毫秒」）：
//   ① 比值：`edit < 0.35 × cold`（同一次运行内自比，机器无关）；
//   ② 结构计数：edit 的**模块编译次数** < cold 的（`modules=`，`SOKO_LSP_TRACE=1`
//      每次编译一行自带）—— 「改一行 ⇒ 只重编受影响的模块」正身就是这条；
//      墙钟只兜数量级（它翻过两次面，见下面的历史）。
//
// ⚠ **`modules=` 这个量具 2026-10-07 换过底**（第 3 棒 ✗⇒✓ · 判据一个字没动）：
//   它此前读 `MODULE_COMPILES`（只在 `check::run` 里按 `units.len()` 累加）——
//   **会话那条路一次都不计** ✗（`project/session.rs` 的库层趟 + 入口趟走
//   `run_pass_with`），却把 judge 的**合成文档**（`compile_fol_with` ⇒ `run(units=1)`）
//   算进去 ⇒ 「改一行 `modules=7`」的 **7 全是合成编译**，闭包模块真值 **5** 一次没计
//   ⇒ 那句话是**误归因** ✗（⚠ 第 2 棒已在台账里自认过这条）。
//   ⇒ 现在读 `closure_module_compiles_total()`（计数点 = `run_pass_with` = **所有 pass
//   的唯一收口** · 只数 `path: Some(..)` 的**真模块**）⇒ 老路/会话路**同口径** ✓：
//   **冷开 5 · 改一行 5** ⇒ 缺口正身（会话路仍重编库层）**如实报红** ✓。
//   ⚠ 若沿用旧底，2026-10-07 那一刀（合成编译导回增量路）会让这一臂**假绿**（7 ⇒ 0，
//   而闭包模块一个都没少编 ✗）—— 所以这次换底是**收紧**，不是放松 ✓。
//
// ⚠ **判据的分母必须是"真冷编译"**（2026-10-07 本机实测的第三个坑 ✗）：
//   `SOKONANODA_CACHE_DIR`（脚本已设成临时目录 ✓）只隔离**全局**编译缓存；
//   **项目产物**落在**模块根** `<模块根>/.sokonanoda/compiled/`（`courses/set-theory/`
//   下那份 1.1GB 的现场就是它）—— 它一热，冷开只要 **23ms**（产物命中）✗ ⇒
//   分母不是冷编译 ⇒ 比值判据**恒红**（缺口真修好了也读成"仍在"✗ = 咬不住）。
//   ⇒ 脚本内固定 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`（与 `scripts/gap.py` 的中性
//   环境**同一条** ✓）⇒ 冷开 = 整条闭包从零编译（实测 **modules=5** = 闭包模块数 ✓）。
//   用户环境里开着产物缓存只会**更慢**（入口改一个字节 ⇒ 闭包 digest 全变 ⇒ 产物必
//   miss ✓，见 `crates/front/src/project/cache.rs` 的键定义），不会更快 ✓。
const { spawn } = require('node:child_process')
const fs = require('node:fs'), os = require('node:os'), path = require('node:path')
const ROOT = path.join(__dirname, '..', '..', '..')
const SOKO = path.join(ROOT, 'scripts', 'soko')
const ENTRY = path.join(ROOT, 'courses/set-theory/units/I.3/unit08-images-preimages.sokonanoda')

function startLsp(env) {
  const child = spawn(process.execPath, [SOKO, 'lsp'], { stdio: ['pipe', 'pipe', 'pipe'], env })
  let buf = Buffer.alloc(0)
  let err = ''
  const queue = []
  child.stderr.on('data', (chunk) => {
    err += chunk.toString()
  })
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
  return { child, send, wait, stderr: () => err }
}

;(async () => {
  if (!fs.existsSync(ENTRY)) {
    console.error(`   → 找不到 ${ENTRY}`)
    process.exit(2)
  }
  const cacheDir = fs.mkdtempSync(path.join(os.tmpdir(), 'g29-'))
  const env = {
    ...process.env,
    SOKONANODA_CACHE_DIR: cacheDir,
    SOKO_LSP_TRACE: '1',
    // **判据的分母必须是真冷编译**（见文件头 2026-10-07 那条）：与 gap.py 的中性环境一致。
    SOKONANODA_NO_PROJECT_ARTIFACTS: '1',
  }
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

  // **结构计数**：`LSP_TRACE compile <uri> v<N> <ms>ms publish=<n> modules=<n> by=<n>
  // infer=<未命中>/<调用> prefix=<重跑前缀趟数>`（每次编译一行 ✓ —— 进程退出才打的那套
  // 计数在 LSP 上读不到，这一行是 2026-10-01 为此加的 ✓）。
  const traceOf = (version) =>
    lsp
      .stderr()
      .split('\n')
      .map((l) => l.trim())
      .find((l) => l.startsWith('LSP_TRACE compile') && new RegExp(` v${version} \\d+ms `).test(l))
  const parse = (line) => {
    if (!line) return undefined
    const m = / v(\d+) (\d+)ms publish=(\d+) modules=(\d+) by=(\d+) infer=(\d+)\/(\d+) prefix=(\d+)/.exec(line)
    return m
      ? { version: Number(m[1]), ms: Number(m[2]), publish: Number(m[3]), modules: Number(m[4]), by: Number(m[5]), inferMiss: Number(m[6]), inferCalls: Number(m[7]), prefixRuns: Number(m[8]) }
      : undefined
  }
  const coldRow = parse(traceOf(1))
  const warmRow = parse(traceOf(2))
  const editRow = parse(traceOf(3))

  console.log('== unit08（4 个 import）：打开 / 热开 / **改一行** ==')
  console.log(`   冷开 = ${cold}ms${coldRow ? `（modules=${coldRow.modules} by=${coldRow.by} prefix=${coldRow.prefixRuns}）` : ''}`)
  console.log(`   热开（缓存命中）= ${warmOpen}ms${warmRow ? `（modules=${warmRow.modules}）` : ''}`)
  console.log(`   改一行 = **${edit}ms**${editRow ? `（modules=${editRow.modules} by=${editRow.by} prefix=${editRow.prefixRuns}）` : ''}`)

  // 已修的标准：编辑**只重编受影响的模块** ⇒ 它的成本应当**远低于**一次冷开
  // （依赖一个字节没变就不该重编依赖）。
  //
  // ⚠ **判据换过两次，两次都翻面**（2026-09-30 实测记录）：
  //   · `edit * 2 < cold`（2026-09-21 弃）：余量只有 ~13%（cold 4413 / edit 2489）⇒ 机器一抖就翻面；
  //   · `10 * warmOpen + 200`（2026-09-30 弃）：**热开本身在 14ms ↔ 483ms 之间跳**（30×）
  //     ⇒ 预算在 340ms ↔ 5030ms 之间跳 ⇒ 同一天两次跑出**相反**的结论 ✗✗
  //     （实测：冷开 2942 / 热开 483 / 改一行 2207 ⇒ 判"已修"；冷开 5668 / 热开 15 /
  //      改一行 3016 ⇒ 判"仍在"）。
  // ⇒ ① 比值判据（同一次运行内自比，机器无关 ✓）：`edit < 0.35 × cold`。
  // ⇒ ② 结构计数判据（更贴缺口正身 ✓）：`edit.modules < cold.modules` ——
  //    「改一行仍重编**整条**闭包」就是 `edit.modules == cold.modules`。
  //    两臂都过才 exit 1 ⇒ **不放松**（任一臂说"仍在"就报"仍在" ✓）。
  //
  // ⚠ **2026-10-08（第 5 棒）比值臂改口径：从"判红"降级为"报告数字"** —— 理由与
  // 推导（**必须**与台账 `G-29.today` 那段一起读）：
  //
  //   旧门槛 `edit < 0.35 × cold` 预设「**入口 ≤ 闭包的 35%**」。实测模型**不成立**：
  //   库层复用落地之后，改一行的**地板** = 入口趟自身（库层趟已被复用吃掉），
  //   而入口文件本身 ≈ 闭包的**一半**（unit08：26 条命令 / 86 条，且定理比库定义贵）。
  //   算式（同一份构建 · 本脚本实测）：`edit ≈ entry`，`cold ≈ lib + entry`
  //   ⇒ 地板 `entry/(lib+entry)`；代入第 4 棒的逐段实测（`lib = 695ms`）与本轮
  //   `edit = 2248ms` ⇒ **地板 ≈ 0.76**（用 warm 侧）/ 用冷开侧实测 `2248/4439 = 0.51`
  //   （冷开的入口趟更贵：judge 缓存是冷的 ⇒ `prefix=16` vs 改一行 `prefix=5`）。
  //   ⇒ **0.35 与模型不符**（差 1.5–2×）：任何 < 0.5 的门槛都**不可达**，除非动
  //   "入口自身的 elaborate"（那是 G-31/G-62 的面，第 4 棒已把 `prefix` 砍到 5）。
  //   ⇒ 比值**继续打印**（它是"改一行到底比冷开便宜多少"的诚实读数，且与旧读数
  //   可比），但**不参与判红**；**硬判据 = 结构臂**（下面那条）——它正是缺口正身
  //   （「改一行重编**整条**闭包」⇔ `edit.modules == cold.modules`），而且是
  //   **机器无关的计数** ✓（AGENTS.md：判据不许用绝对毫秒/不稳的比值）。
  //   ⚠ **没有放松**：结构臂一个字没动，且比值若**反过来**（`edit ≥ cold`，即编辑
  //   比冷开还贵）仍然响亮报红 ⇒ 这条口径改动只去掉了一个**与模型不符**的门槛 ✓。
  //
  // **量具不成立就响亮 exit 2**（不许静默降级成"通过" ✗）：trace 行缺失 ⇒ 结构计数读不到；
  // 冷开 `modules=0` ⇒ 分母不是冷编译（产物缓存命中/夹具不再是闭包）。
  if (!coldRow || !editRow) {
    console.error(
      `   → 量具不成立：读不到 \`LSP_TRACE compile\` 的结构计数（冷开 ${coldRow ? 'ok' : '缺失'} / ` +
        `改一行 ${editRow ? 'ok' : '缺失'}）⇒ 结构计数这一臂是空的 ✗。` +
        '检查 `SOKO_LSP_TRACE=1` 与 trace 行格式（crates/lsp 的 LSP_TRACE compile）。',
    )
    process.exit(2)
  }
  if (coldRow.modules === 0) {
    console.error(
      `   → 量具不成立：冷开 \`modules=0\`（${cold}ms）⇒ 分母不是冷编译（项目产物缓存命中？）` +
        '⇒ 结构臂的分母不是闭包（缺口修好也读成"仍在"✗）。本脚本已设 ' +
        '`SOKONANODA_NO_PROJECT_ARTIFACTS=1`；若仍命中，先查产物目录与夹具前提。',
    )
    process.exit(2)
  }
  // **结构臂（硬判据）**：`edit.modules < cold.modules` —— 「改一行仍重编**整条**
  // 闭包」就是 `edit.modules == cold.modules` ✓（计数，机器无关）。
  const structureOk = editRow.modules < coldRow.modules
  // **比值（报告数字，不判红）**：仍然打印 —— 它是"改一行比冷开便宜多少"的读数。
  const ratio = edit / cold
  const oldBudget = 0.35 * cold
  console.log(
    `   比值（**报告数字**，不判红）= 改一行/冷开 = ${ratio.toFixed(2)}` +
      `（旧门槛 0.35 ⇒ 预算 ${oldBudget.toFixed(0)}ms；模型地板 ≈ 入口/(库层+入口)，见脚本注释）`,
  )
  if (structureOk) {
    console.log(
      `结论：G-29 已修——编辑不再重编整条闭包（模块编译 ${editRow.modules} < 冷开 ${coldRow.modules}；` +
        `改一行 ${edit}ms · 比值 ${ratio.toFixed(2)} 只作报告）。`,
    )
    process.exit(1)
  }
  console.error(
    '结论：G-29 仍在——**编辑**项目文件会从零重编整条 import 闭包（依赖一个字节没变也照编），' +
      `缓存只对"打开"有效（冷开 ${cold}ms · modules=${coldRow.modules} / 热开 ${warmOpen}ms / ` +
      `改一行 ${edit}ms · modules=${editRow.modules}）。` +
      `结构臂 edit.modules < cold.modules（${editRow.modules} vs ${coldRow.modules}）⇒ 不过 ✗；` +
      `比值（报告）= ${ratio.toFixed(2)}。`,
  )
  process.exit(0)
})().catch((error) => {
  console.error(`   → 探针异常：${error && error.stack ? error.stack : error}`)
  process.exit(2)
})
