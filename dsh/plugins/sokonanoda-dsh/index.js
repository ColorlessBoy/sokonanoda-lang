/**
 * `sokonanoda-dsh` — one language-service plugin for sokonanoda inside DSH.
 *
 * Host half:
 *  - starts the quiz fast-path service **in this process** (the course engine's
 *    state machine lives in `lib/quiz-proxy.mjs`; no separate process and no
 *    `quiz-auto` model provider);
 *  - intercepts agent-loop model calls in the `llm/stream` waterfall **only in
 *    sessions the learner explicitly opted in** via `/sokonanoda-fast-think on` (default
 *    off; control lives in `~/.dsh/state/sokonanoda-dsh/sessions.json`). Quiz
 *    turns (letters / start / continue / explain …) are answered by the local
 *    engine and streamed back as ordinary assistant text; everything else
 *    calls `next()` (the user's current model, unchanged);
 *  - turns the proxy's display-only card payloads into `learn-card` Markdown
 *    fences, so the client renders cards without adding a tool call — the
 *    underlying assistant character stream is what the model wrote/read;
 *  - registers `sokonanoda_fast_think` (status/explain/progress/log) so the model can perceive and
 *    diagnose the fast path.
 *
 * Runtime state never lives in the repository: config, quiz progress and the
 * decision history stay under `~/.dsh/state/sokonanoda-dsh/`
 * (`SOKO_DSH_DATA` overrides the whole directory).
 */

import { spawn } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

export const name = 'sokonanoda-dsh'

/** Only the tool registry is a hard dependency; the llm event is root-scoped. */
export const inject = ['tools', 'commands']

const HERE = path.dirname(fileURLToPath(import.meta.url))
const QUIZ_CORE_FILE = path.join(HERE, 'lib', 'quiz-proxy.mjs')

/** Fence language the client half renders as a real card. */
const LEARN_CARD_FENCE = 'learn-card'
/** Minimal `learn_card` declaration: the quiz service only checks that it is declared. */
export const LEARN_CARD_TOOL_DECLARATION = {
  type: 'function',
  function: {
    name: 'learn_card',
    description: 'Display one study card (question / knowledge / lesson) in the chat.',
    parameters: {
      type: 'object',
      properties: {
        kind: { type: 'string', enum: ['question', 'knowledge', 'lesson', 'progress', 'code'] },
        payload: { type: 'object', additionalProperties: true },
      },
      required: ['kind', 'payload'],
      additionalProperties: false,
    },
  },
}

const START_ONLY_RE = /^\s*(?:开始做题|练几题|出题|下一题|下一道|换一题|换一道|再来一题|再来一道|next)\s*[!！。.~～?？]*\s*$/i
const DEFAULT_TIMEOUT_MS = 8000
/** 代码题要 compose + 真内核判卷，快路径超时要比问答宽。 */
const DEFAULT_CODE_TIMEOUT_MS = 45000
const MAX_LOG_LINES = 2000
/** Read-only goal state probe: hot `query state` answers in ~0.12 s; cold compile may be slower. */
const DEFAULT_STATE_TIMEOUT_MS = 3000
/** Cap on child output so a runaway compile can never grow the Host heap. */
const MAX_QUERY_BYTES = 8 * 1024 * 1024
/** Only the first few diagnostics travel to the preview panel. */
const MAX_STATE_DIAGNOSTICS = 3
/** Resource address prefix shared with the Sidebar document preview. */
const FILE_RESOURCE_PREFIX = 'dsh-resource://file/'
/** Same-origin read-only route the preview body polls while the cursor moves. */
const STATE_ROUTE = '/sokonanoda-state'

/**
 * Repository root: the nearest ancestor that still contains the course engine.
 * @param start - directory to start walking up from.
 * @returns absolute repository root (best effort).
 */
export function findRepoRoot(start = HERE) {
  let dir = path.resolve(start)
  for (;;) {
    if (fs.existsSync(path.join(dir, 'courses', 'set-theory', 'knowledge', 'engine.py'))) return dir
    const parent = path.dirname(dir)
    if (parent === dir) break
    dir = parent
  }
  return path.resolve(HERE, '..', '..')
}

const REPO = findRepoRoot()

/**
 * Default user-level data root. Never inside the repository: this directory is
 * machine-local and survives a fresh clone / a shared GitHub checkout.
 * @returns absolute data directory.
 */
export function defaultDataDir() {
  const dshHome = typeof process.env.DSH_HOME === 'string' && process.env.DSH_HOME !== ''
    ? process.env.DSH_HOME
    : path.join(os.homedir(), '.dsh')
  return typeof process.env.SOKO_DSH_DATA === 'string' && process.env.SOKO_DSH_DATA !== ''
    ? process.env.SOKO_DSH_DATA
    : path.join(dshHome, 'state', 'sokonanoda-dsh')
}

/** Read `config.json` from the data dir; a malformed file is ignored. */
export function readFileConfig(dataDir = defaultDataDir()) {
  try {
    const parsed = JSON.parse(fs.readFileSync(path.join(dataDir, 'config.json'), 'utf8'))
    return parsed !== null && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {}
  } catch {
    return {}
  }
}

/**
 * Normalize the plugin configuration. The bundle row wins over `config.json`.
 * @param config - bundle row config.
 * @param fileConfig - parsed `config.json` (optional).
 * @returns resolved settings.
 */
export function normalizeConfig(config = {}, fileConfig = {}) {
  const merged = {
    ...(fileConfig !== null && typeof fileConfig === 'object' ? fileConfig : {}),
    ...(config !== null && typeof config === 'object' ? config : {}),
  }
  const dataDir = typeof merged.dataDir === 'string' && merged.dataDir.trim() !== ''
    ? path.resolve(merged.dataDir.trim())
    : defaultDataDir()
  const timeout = Number(merged.timeoutMs)
  const codeTimeout = Number(merged.codeTimeoutMs)
  const stateTimeout = Number(merged.stateTimeoutMs)
  const repoRoot = typeof merged.repoRoot === 'string' && merged.repoRoot.trim() !== ''
    ? path.resolve(merged.repoRoot.trim())
    : REPO
  const sokoPath = typeof merged.sokoPath === 'string' && merged.sokoPath.trim() !== ''
    ? path.resolve(merged.sokoPath.trim())
    : path.join(repoRoot, 'scripts', 'soko')
  return {
    enabled: merged.enabled !== false,
    dataDir,
    repoRoot,
    sokoPath,
    historyPath: path.join(dataDir, 'history', 'decisions.jsonl'),
    sessionsPath: path.join(dataDir, 'sessions.json'),
    quizStateDir: path.join(dataDir, 'quiz'),
    logDir: path.join(dataDir, 'logs'),
    timeoutMs: Number.isFinite(timeout) && timeout >= 500 ? Math.floor(timeout) : DEFAULT_TIMEOUT_MS,
    codeTimeoutMs: Number.isFinite(codeTimeout) && codeTimeout >= 500
      ? Math.floor(codeTimeout)
      : DEFAULT_CODE_TIMEOUT_MS,
    stateTimeoutMs: Number.isFinite(stateTimeout) && stateTimeout >= 300
      ? Math.floor(stateTimeout)
      : DEFAULT_STATE_TIMEOUT_MS,
    decisionsLimit: Number.isFinite(Number(merged.decisionsLimit))
      ? Math.max(1, Math.min(MAX_LOG_LINES, Math.floor(Number(merged.decisionsLimit))))
      : 20,
    cards: merged.cards !== false,
  }
}

/**
 * Read the Sidebar file resource address into the Session and path it names.
 * Mirrors `dsh-util-workspace-path`'s session scope: every segment is
 * component-encoded, query/fragment suffixes are ignored, and a malformed
 * escape is not a usable address.
 * @param address - `dsh-resource://file/session/<id>/<path>` (or absolute scope).
 * @returns `{ sessionId, path }`, or null when it is not a session file address.
 */
export function parseFileResource(address) {
  if (typeof address !== 'string' || !address.startsWith(FILE_RESOURCE_PREFIX)) return null
  const end = address.search(/[?#]/)
  const tail = address.slice(FILE_RESOURCE_PREFIX.length, end === -1 ? undefined : end)
  const parts = tail.split('/')
  if (parts[0] !== 'session' || parts.length < 3) return null
  try {
    const sessionId = decodeURIComponent(parts[1])
    const filePath = parts.slice(2).map(decodeURIComponent).join('/')
    if (sessionId === '' || filePath === '') return null
    return { sessionId, path: filePath }
  } catch {
    return null
  }
}

/**
 * Whether `target` is `root` itself or sits below it (separator-aware, so
 * `/repo-other` never counts as inside `/repo`).
 * @param root - absolute container path.
 * @param target - absolute candidate path.
 * @returns true when the candidate stays inside the container.
 */
export function isPathWithin(root, target) {
  const relative = path.relative(path.resolve(root), path.resolve(target))
  return relative === '' || (relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative))
}

/** Resolve symlinks when possible; a missing path keeps its lexical resolution. */
function realPathOf(target) {
  try {
    return typeof fs.realpathSync.native === 'function' ? fs.realpathSync.native(target) : fs.realpathSync(target)
  } catch {
    return path.resolve(target)
  }
}

/**
 * Resolve a preview resource to the one file the read-only state route may ask
 * the kernel about. A relative address resolves against the Session workspace;
 * an absolute one stays itself. The canonical file must sit inside either the
 * Session workspace or the repository, and must exist as a regular file.
 * @param options - resource address, Session workspace root and repository root.
 * @returns `{ ok: true, path, sessionId }` or `{ ok: false, code, message }`.
 */
export function resolveAllowedStatePath({ resource, workspaceRoot, repoRoot }) {
  const parsed = parseFileResource(resource)
  if (parsed === null) return { ok: false, code: 'bad-resource', message: '无法识别的文件资源地址' }
  if (typeof workspaceRoot !== 'string' || workspaceRoot === '') {
    return { ok: false, code: 'workspace-unavailable', message: '当前会话没有可用的工作区' }
  }
  const lexical = path.isAbsolute(parsed.path)
    ? path.resolve(parsed.path)
    : path.resolve(workspaceRoot, parsed.path)
  let info = null
  try {
    info = fs.statSync(lexical)
  } catch {
    info = null
  }
  /* A missing file is judged lexically (it cannot be read yet); an existing
   * one must also survive symlink resolution inside the real root. */
  const realTarget = info === null ? lexical : realPathOf(lexical)
  const allowedRoots = [workspaceRoot, repoRoot]
    .filter((root) => typeof root === 'string' && root !== '')
  const within = allowedRoots.some((root) => {
    if (!isPathWithin(root, lexical)) return false
    return info === null || isPathWithin(realPathOf(root), realTarget)
  })
  if (!within) return { ok: false, code: 'forbidden', message: '路径不在当前工作区或仓库内' }
  if (info === null) return { ok: false, code: 'not-found', message: '目标文件不存在' }
  if (!info.isFile()) return { ok: false, code: 'not-found', message: '目标不是普通文件' }
  return { ok: true, path: realTarget, sessionId: parsed.sessionId }
}

/**
 * Run one read-only `scripts/soko` query with an argument vector (never a
 * shell string) and a hard timeout that kills the whole child process group.
 * @param options - launcher path, working directory, arguments and timeout.
 * @returns `{ ok: true, envelope }` with the parsed JSON, or a structured error.
 */
export function runSokoCommand({ sokoPath, cwd, args, timeoutMs = DEFAULT_STATE_TIMEOUT_MS }) {
  return new Promise((resolve) => {
    if (typeof sokoPath !== 'string' || !fs.existsSync(sokoPath)) {
      resolve({ ok: false, error: { code: 'backend', message: '找不到 scripts/soko 启动器' } })
      return
    }
    const child = spawn(process.execPath, [sokoPath, ...args], {
      cwd: cwd === undefined ? path.dirname(sokoPath) : cwd,
      detached: process.platform !== 'win32',
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
      env: process.env,
    })
    let stdout = ''
    let stderr = ''
    let settled = false
    const finish = (result) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      resolve(result)
    }
    const killTree = (signal) => {
      try {
        if (process.platform !== 'win32' && child.pid !== undefined) process.kill(-child.pid, signal)
        else child.kill(signal)
      } catch {
        try { child.kill(signal) } catch { /* already gone */ }
      }
    }
    const timer = setTimeout(() => {
      killTree('SIGKILL')
      finish({ ok: false, error: { code: 'timeout', message: `状态查询超过 ${timeoutMs} ms` } })
    }, timeoutMs)
    child.stdout.on('data', (chunk) => {
      stdout += chunk
      if (stdout.length > MAX_QUERY_BYTES) {
        killTree('SIGKILL')
        finish({ ok: false, error: { code: 'backend', message: '状态查询输出过大' } })
      }
    })
    child.stderr.on('data', (chunk) => {
      if (stderr.length < 64 * 1024) stderr += chunk
    })
    child.on('error', (error) => {
      finish({ ok: false, error: { code: 'backend', message: String(error?.message || error) } })
    })
    child.on('close', (code) => {
      const text = stdout.trim()
      if (text === '') {
        finish({
          ok: false,
          error: {
            code: code === 0 ? 'invalid-response' : 'backend',
            message: (stderr.trim() || `query 退出码 ${code}`).slice(0, 300),
          },
        })
        return
      }
      let envelope
      try {
        envelope = JSON.parse(text)
      } catch {
        finish({ ok: false, error: { code: 'invalid-response', message: 'query 输出不是 JSON' } })
        return
      }
      finish({ ok: true, envelope, code: code === null ? -1 : code })
    })
  })
}

/** Last-on-disk `query check` answer by file identity; avoids re-checking on every outside line. */
const diagnosticsCache = new Map()

/**
 * Kernel diagnostics for one file, cached by (path, mtime, size). The preview
 * uses them only to tell "cursor outside every declaration" from "the source
 * does not parse"; the answer never gates a declaration.
 * @param settings - resolved plugin settings.
 * @param file - absolute source path.
 * @returns up to {@link MAX_STATE_DIAGNOSTICS} `{ line, message }` entries.
 */
export async function diagnosticsForFile(settings, file) {
  if (diagnosticsCache.size > 40) diagnosticsCache.clear()
  let key = file
  try {
    const stat = fs.statSync(file)
    key = `${file}\u0000${stat.mtimeMs}\u0000${stat.size}`
  } catch {
    return []
  }
  if (diagnosticsCache.has(key)) return diagnosticsCache.get(key)
  const result = await runSokoCommand({
    sokoPath: settings.sokoPath,
    cwd: settings.repoRoot,
    args: ['query', 'check', '--file', file],
    timeoutMs: settings.stateTimeoutMs,
  })
  const failed = result.ok === true && result.envelope !== null && typeof result.envelope === 'object'
    ? result.envelope?.data?.failed
    : undefined
  const diagnostics = Array.isArray(failed)
    ? failed.slice(0, MAX_STATE_DIAGNOSTICS).map((item) => ({
      line: Number(item?.start_line) || 0,
      message: String(item?.message || '').slice(0, 240),
    }))
    : []
  diagnosticsCache.set(key, diagnostics)
  return diagnostics
}

/**
 * Answer one preview cursor position: a successful kernel state, or a
 * structured reason there is none (outside every declaration, file errors,
 * timeout, backend failure). The route only ever reads; nothing is written.
 * @param settings - resolved plugin settings.
 * @param file - absolute source path.
 * @param line - 1-based source line.
 * @param col - 1-based source column.
 * @returns the response body the client renders.
 */
export async function stateAnswer(settings, file, line, col) {
  const result = await runSokoCommand({
    sokoPath: settings.sokoPath,
    cwd: settings.repoRoot,
    args: ['query', 'state', '--file', file, '--line', String(line), '--col', String(col)],
    timeoutMs: settings.stateTimeoutMs,
  })
  if (result.ok !== true) return { ok: false, error: result.error }
  const envelope = result.envelope
  if (envelope !== null && typeof envelope === 'object' && envelope.ok === true
    && envelope.data !== null && typeof envelope.data === 'object') {
    return { ok: true, line, col, path: file, state: envelope.data }
  }
  const code = typeof envelope?.error?.code === 'string' ? envelope.error.code : 'invalid-response'
  const message = typeof envelope?.error?.message === 'string'
    ? envelope.error.message
    : '内核没有返回可读的状态'
  if (code === 'outside-declarations') {
    const diagnostics = await diagnosticsForFile(settings, file)
    if (diagnostics.length > 0) {
      return {
        ok: false,
        error: { code: 'file-errors', message: `源码有 ${diagnostics.length} 处错误` },
        diagnostics,
      }
    }
    return { ok: false, error: { code, message: '该位置不在任何声明内部' } }
  }
  return { ok: false, error: { code, message } }
}

/**
 * Per-session fast-path opt-in. The switch is explicit: no session is enabled
 * until the learner runs `/sokonanoda-fast-think on` there. State lives in the plugin data
 * dir (`sessions.json`), never in the repository.
 * @param file - absolute sessions.json path.
 * @returns set of enabled session ids (empty when missing/malformed).
 */
export function loadSessions(file) {
  try {
    const parsed = JSON.parse(fs.readFileSync(file, 'utf8'))
    const out = new Set()
    for (const [sessionId, value] of Object.entries(parsed !== null && typeof parsed === 'object' ? parsed : {})) {
      if (value === true) out.add(sessionId)
    }
    return out
  } catch {
    return new Set()
  }
}

/**
 * Persist the enabled-session set atomically (tmp + rename).
 * @param file - absolute sessions.json path.
 * @param sessions - session ids currently enabled.
 */
export function saveSessions(file, sessions) {
  fs.mkdirSync(path.dirname(file), { recursive: true })
  const tmp = `${file}.tmp`
  const payload = Object.fromEntries([...sessions].sort().map((sessionId) => [sessionId, true]))
  fs.writeFileSync(tmp, `${JSON.stringify(payload, null, 2)}\n`, 'utf8')
  fs.renameSync(tmp, file)
}

/**
 * One shared switch mutation for both callers: the learner's `/sokonanoda-fast-think`
 * command and the agent's `sokonanoda_fast_think` tool. Persists the session
 * allowlist and rolls back memory when the write fails.
 * @param enabledSessions - live set (mutated on success).
 * @param settings - resolved settings (sessionsPath).
 * @param sessionId - exact session to toggle.
 * @param enabled - target state.
 * @param record - decision logger.
 * @param via - 'command' | 'sokonanoda_fast_think'.
 * @returns `{ ok, enabled? , error? }`.
 */
export function setSessionEnabled(enabledSessions, settings, sessionId, enabled, record, via) {
  if (typeof sessionId !== 'string' || sessionId === '') {
    return { ok: false, error: 'session id unavailable' }
  }
  const wasEnabled = enabledSessions.has(sessionId)
  try {
    if (enabled) enabledSessions.add(sessionId)
    else enabledSessions.delete(sessionId)
    saveSessions(settings.sessionsPath, enabledSessions)
    if (typeof record === 'function') {
      record({ event: 'session', mode: enabled ? 'on' : 'off', sessionId, via: via || 'unknown' })
    }
    return { ok: true, enabled }
  } catch (error) {
    if (wasEnabled) enabledSessions.add(sessionId)
    else enabledSessions.delete(sessionId)
    return { ok: false, error: String(error?.message || error) }
  }
}

let quizCorePromise
/**
 * Load the in-process quiz service module (vendored from `.cache/quiz-proxy`).
 * @param file - absolute module path.
 * @returns the module once.
 */
export function loadQuizCore(file = QUIZ_CORE_FILE) {
  if (quizCorePromise === undefined) quizCorePromise = import(pathToFileURL(file).href)
  return quizCorePromise
}

let loopMarkerPromise
/**
 * Load `isAgentLoopRequest` from the running DSH installation when possible.
 * The marker is a module-private WeakSet, so it only works when this import
 * resolves to the same module instance the Host loaded; absence is fine.
 */
export function loadLoopMarker() {
  if (loopMarkerPromise !== undefined) return loopMarkerPromise
  const installDir = typeof process.env.DSH_INSTALL_DIR === 'string' && process.env.DSH_INSTALL_DIR !== ''
    ? process.env.DSH_INSTALL_DIR
    : ''
  if (installDir === '') {
    loopMarkerPromise = Promise.resolve(null)
    return loopMarkerPromise
  }
  const entry = path.join(installDir, 'node_modules', '@deepseek-ai', 'dsh-llm', 'lib', 'index.js')
  loopMarkerPromise = import(pathToFileURL(entry).href)
    .then((mod) => (typeof mod.isAgentLoopRequest === 'function' ? mod.isAgentLoopRequest : null))
    .catch(() => null)
  return loopMarkerPromise
}

/** True when a request looks like one agent-loop turn (not an auxiliary call). */
export function isLoopRequestShape(options) {
  return options !== null && typeof options === 'object'
    && options.purpose === undefined
    && Array.isArray(options.messages)
    && options.messages.length > 0
    && typeof options.sessionId === 'string' && options.sessionId !== ''
    && Object.isFrozen(options)
}

/** Compatible loop-request test: the exact marker when available, otherwise the shape test. */
export async function isAgentLoopRequestCompat(options) {
  if (!isLoopRequestShape(options)) return false
  const marker = await loadLoopMarker().catch(() => null)
  if (typeof marker === 'function' && marker(options) === true) return true
  return true
}

/** Collect every substantive human user text, in order. */
export function userTexts(messages, helpers) {
  const texts = []
  for (const message of Array.isArray(messages) ? messages : []) {
    if (message === null || typeof message !== 'object' || message.role !== 'user') continue
    const text = helpers.contentToText(message.content)
    if (typeof text !== 'string' || text === '') continue
    if (helpers.isRuntimeContextText(text)) continue
    texts.push(text)
  }
  return texts
}

/** Whether an earlier user message already opened a quiz session. */
export function hasPriorStart(messages, helpers) {
  const texts = userTexts(messages, helpers)
  texts.pop() // the text being classified
  return texts.some((text) => {
    const fast = helpers.classifyFast('quiz-auto', text)
    return fast !== null && fast.type === 'next'
  })
}

/**
 * Read the most recent `learn-card` question fence from assistant history.
 * @param options - loop request.
 * @param helpers - quiz-core text helpers.
 * @returns `{card, open}` for the last question card, or null.
 */
export function lastAssistantQuestionCard(options, helpers) {
  const messages = options !== null && typeof options === 'object' && Array.isArray(options.messages) ? options.messages : []
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index]
    if (message === null || typeof message !== 'object' || message.role !== 'assistant') continue
    const content = helpers.contentToText(message.content)
    if (typeof content !== 'string' || content.trim() === '') continue
    const match = /```learn-card\s*\n([\s\S]*?)\n```/i.exec(content)
    if (!match) continue
    let card = null
    try { card = JSON.parse(match[1]) } catch { return null }
    if (card === null || typeof card !== 'object' || card.kind !== 'question') return null
    const payload = card.payload !== null && typeof card.payload === 'object' ? card.payload : {}
    const optionsList = Array.isArray(payload.options) ? payload.options : []
    const questionKind = typeof payload.questionKind === 'string' ? payload.questionKind : 'choice'
    return { card, open: questionKind !== 'choice' || optionsList.length === 0 }
  }
  return null
}

/**
 * Read the currently issued code exercise (if any) from the shared quiz state.
 * The fast path only treats a fenced-code message as a submission when the
 * engine has an `issued_code`; otherwise the message falls through to `next()`.
 * @param stateDir - quiz state directory (`settings.quizStateDir`).
 * @returns the issued code id, or ''.
 */
export function readIssuedCode(stateDir) {
  try {
    const state = JSON.parse(fs.readFileSync(path.join(stateDir, 'default.json'), 'utf8'))
    return state !== null && typeof state === 'object' && typeof state.issued_code === 'string'
      ? state.issued_code
      : ''
  } catch {
    return ''
  }
}

/**
 * Decide whether this request is a fast-path candidate.
 * @param options - the loop request.
 * @param helpers - quiz-core helpers.
 * @param extras - `{ codeIssued }`; enables fenced-code submissions when a code exercise is issued.
 * @returns `{fast, text, conversation}` or null.
 */
export function decideFast(options, helpers, extras = {}) {
  const messages = options !== null && typeof options === 'object' ? options.messages : undefined
  if (!Array.isArray(messages) || messages.length === 0) return null
  const last = messages[messages.length - 1]
  /* A tool-result continuation must reach the model; never answer it locally. */
  if (last === null || typeof last !== 'object' || last.role !== 'user') return null
  const conversation = helpers.extractConversation(messages)
  const text = conversation.lastUserText
  if (typeof text !== 'string' || text.trim() === '') return null
  const fast = helpers.classifyFast('quiz-auto', text)
  if (fast === null) {
    /* 屏幕上最后一张是开放性问答题卡：这段自由文本就是它的回答，
     * 交给本地引擎判卷（dry-run 指出缺口 / 记账后出下一卡）。 */
    const pending = lastAssistantQuestionCard(options, helpers)
    if (pending !== null && pending.open === true) {
      return { fast: { type: 'open' }, text, conversation }
    }
    /* C9.4：已发出代码题 + 围栏代码 ⇒ 真内核判卷；否则回落模型。 */
    if (extras.codeIssued === true && typeof helpers.extractFencedCode === 'function'
      && helpers.extractFencedCode(text) !== null) {
      return { fast: { type: 'code' }, text, conversation }
    }
    return null
  }
  const priorStart = hasPriorStart(messages, helpers)
  if (fast.type === 'next') {
    if (!START_ONLY_RE.test(helpers.normalizeUserText(text)) && !priorStart) return null
  } else if (!priorStart && fast.type !== 'map' && fast.type !== 'code-next') {
    /* 「进度/地图」是只读快照，可以在任何会话里直接看；「代码题」也能开局；
     * 其余（字母/继续/跳过…）需要已有 quiz 会话。 */
    return null
  }
  return { fast, text, conversation }
}

/**
 * Build the minimal message list the quiz service needs. The system text is a
 * per-session stable stub, so a fingerprint state key stays stable per session.
 */
export function toProxyMessages(options, helpers) {
  const sessionId = options !== null && typeof options === 'object' && typeof options.sessionId === 'string'
    ? options.sessionId
    : 'unknown'
  const out = [{ role: 'system', content: `sokonanoda-dsh/${sessionId}` }]
  for (const text of userTexts(options.messages, helpers)) out.push({ role: 'user', content: text })
  return out
}

/** Split a reply into stream chunks. */
export async function* synthesizeChunks(text, size = 160) {
  const content = typeof text === 'string' ? text : String(text ?? '')
  yield { type: 'block-start', index: 0, blockType: 'text' }
  for (let offset = 0; offset < content.length; offset += size) {
    yield { type: 'text-delta', index: 0, text: content.slice(offset, offset + size) }
  }
  yield { type: 'block-end', index: 0, block: { type: 'text', text: content } }
  yield { type: 'finish', reason: { kind: 'stop' } }
}

/** Call the in-process quiz service; aborts with the turn signal. */
export async function callProxy(settings, body, signal) {
  const signals = [AbortSignal.timeout(settings.timeoutMs)]
  if (signal !== null && typeof signal === 'object' && typeof signal.aborted === 'boolean') signals.push(signal)
  const composed = signals.length === 1 ? signals[0] : AbortSignal.any(signals)
  const response = await fetch(settings.proxyUrl, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
    signal: composed,
  })
  if (response.ok !== true) throw new Error(`quiz service HTTP ${response.status}`)
  return await response.json()
}

/** Extract the assistant text from an OpenAI chat-completions body. */
export function replyTextOf(payload) {
  const message = payload !== null && typeof payload === 'object'
    ? payload.choices?.[0]?.message
    : undefined
  const content = message !== null && typeof message === 'object' ? message.content : undefined
  return typeof content === 'string' ? content : ''
}

/**
 * Turn one quiz-service reply into the assistant text this plugin streams. It
 * keeps the proxy's feedback text and converts a `learn_card` tool call (card
 * mode) into a `learn-card` fenced block — the client renders it, and the text
 * stays an ordinary assistant character stream (no tool call, no tool result).
 */
/**
 * Resolve a card's `source` (e.g. "units/I.2/unit07-functions.sokonanoda:194")
 * into absolute + repo-relative coordinates the client can turn into a
 * `dsh-resource://file/...` address for the right-sidebar file viewer.
 * @param payload - learn-card payload; returned unchanged when no source line.
 * @returns payload with `sourceLink` added, or the original payload.
 */
export function withCardSourceLink(payload) {
  if (payload === null || typeof payload !== 'object' || Array.isArray(payload)) return payload
  if (payload.sourceLink !== undefined) return payload
  const source = typeof payload.source === 'string' ? payload.source : ''
  const match = /([A-Za-z0-9_.\/-]+\.sokonanoda):(\d+)/.exec(source)
  if (!match) return payload
  const relative = match[1]
  const line = Number(match[2])
  if (!Number.isInteger(line) || line <= 0) return payload
  const candidates = [
    path.join(REPO, 'courses', 'set-theory', relative),
    path.join(REPO, relative),
    path.resolve(REPO, relative),
  ]
  const absolute = candidates.find((candidate) => fs.existsSync(candidate))
  if (absolute === undefined) return payload
  return Object.assign({}, payload, {
    sourceLink: {
      path: absolute,
      relativePath: path.relative(REPO, absolute).split(path.sep).join('/'),
      line,
    },
  })
}

export function composeReplyText(payload) {
  const message = payload !== null && typeof payload === 'object'
    ? payload.choices?.[0]?.message
    : undefined
  if (message === null || typeof message !== 'object') return ''
  const blocks = []
  if (typeof message.content === 'string' && message.content.trim() !== '') blocks.push(message.content)
  const toolCalls = Array.isArray(message.tool_calls) ? message.tool_calls : []
  for (const call of toolCalls) {
    if (call === null || typeof call !== 'object' || call.type !== 'function') continue
    const fn = call.function
    if (fn === null || typeof fn !== 'object' || fn.name !== 'learn_card' || typeof fn.arguments !== 'string') continue
    let args = null
    try { args = JSON.parse(fn.arguments) } catch { args = null }
    if (args === null || typeof args !== 'object' || typeof args.kind !== 'string' || args.kind === '') continue
    const cardPayload = withCardSourceLink(args.payload ?? {})
    blocks.push(`\`\`\`${LEARN_CARD_FENCE}\n${JSON.stringify({ kind: args.kind, payload: cardPayload })}\n\`\`\``)
  }
  return blocks.join('\n\n')
}

/** One JSONL decision line. Never let logging break a turn. */
export function makeRecorder(logPath) {
  let disabled = false
  return (entry) => {
    if (disabled) return
    try {
      fs.mkdirSync(path.dirname(logPath), { recursive: true })
      fs.appendFileSync(logPath, `${JSON.stringify({ time: new Date().toISOString(), ...entry })}\n`, 'utf8')
    } catch {
      disabled = true
    }
  }
}

function readLogTail(logPath, limit) {
  try {
    const lines = fs.readFileSync(logPath, 'utf8').split('\n').filter((line) => line.trim() !== '')
    return lines.slice(-limit).map((line) => {
      try { return JSON.parse(line) } catch { return { raw: line } }
    })
  } catch {
    return []
  }
}

/** `sokonanoda_fast_think` tool: let the model perceive and diagnose the fast path. */
export function createFastThinkTool(settings, record, service, enabledSessions = new Set(), clientModules = undefined) {
  return {
    name: 'sokonanoda_fast_think',
    description: 'Inspect the local fast-thinking path of the sokonanoda language-service plugin. Quiz turns '
      + '(letters / start / continue / explain inside a quiz session) are answered by the local course engine '
      + 'through the llm/stream waterfall, not by a model call. Use action="status" for the in-process quiz '
      + 'service state and the latest decisions, action="explain" with `text` to classify one message without '
      + 'side effects, action="log" for recent decision lines. Nothing here changes state.',
    parameters: {
      type: 'object',
      properties: {
        action: { type: 'string', enum: ['status', 'explain', 'progress', 'log', 'enable', 'disable', 'client', 'reload-client'], description: 'status / explain / progress / log / enable / disable / client.' },
        text: { type: 'string', description: 'For action="explain": the user text to classify.' },
        limit: { type: 'number', description: 'For action="log": number of recent lines (1-100, default 10).' },
      },
      required: ['action'],
      additionalProperties: false,
    },
    output: {
      schema: {
        type: 'object',
        properties: {
          ok: { type: 'boolean' },
          action: { type: 'string' },
          enabled: { type: 'boolean' },
          proxy: { type: 'object', additionalProperties: true },
          rule: { type: 'string' },
          candidate: { type: 'boolean' },
          entries: { type: 'array' },
          sessions: { type: 'array' },
          entry: { type: 'object', additionalProperties: true },
          clientPath: { type: 'string' },
          payload: { type: 'object', additionalProperties: true },
          sessionId: { type: 'string' },
          enabledSession: { type: 'boolean' },
          error: { type: 'string' },
        },
        required: ['ok'],
        additionalProperties: true,
      },
      render(args, value) {
        const data = value !== null && typeof value === 'object' ? value : {}
        if (data.ok !== true) return [{ type: 'text', text: `sokonanoda_fast_think 失败：${String(data.error || 'unknown')}` }]
        const lines = []
        if (data.action === 'status') {
          const proxy = data.proxy !== null && typeof data.proxy === 'object' ? data.proxy : {}
          lines.push(`快思考：${data.enabled === true ? '全局已启用' : '全局已关闭'}；本机已开启会话 ${Array.isArray(data.sessions) ? data.sessions.length : 0} 个；本地 quiz 服务 ${proxy.ok === true ? '在线' : '不可达'}`)
          const entries = Array.isArray(data.entries) ? data.entries : []
          lines.push(`最近 ${entries.length} 条决策：`)
          for (const entry of entries.slice(-5)) {
            const rule = entry && typeof entry === 'object' && typeof entry.rule === 'string' ? entry.rule : (entry?.mode || '?')
            const ms = entry && typeof entry === 'object' && Number.isFinite(entry.ms) ? `${entry.ms}ms` : ''
            lines.push(`- ${rule}${ms !== '' ? ` ${ms}` : ''}${entry?.mode === 'fallback' ? '（回落慢路径）' : ''}`)
          }
        } else if (data.action === 'explain') {
          lines.push(`分类：${data.rule || '（不命中快路径）'}；${data.candidate === true ? '是快路径候选（字母/继续等还需已有 quiz 会话）' : '不是快路径候选'}`)
        } else if (data.action === 'progress') {
          const card = data.payload !== null && typeof data.payload === 'object' ? data.payload : {}
          const summary = card.summary !== null && typeof card.summary === 'object' ? card.summary : {}
          const hotspots = Array.isArray(card.hotspots) ? card.hotspots : []
          lines.push(`进展：连对 ${summary.streak ?? 0} · 正确率 ${Math.round((Number(summary.accuracy) || 0) * 100)}% · 覆盖 ${Math.round((Number(summary.coverage) || 0) * 100)}% · 已答 ${summary.asked ?? 0}`)
          lines.push(`热点 ${hotspots.length} 个${hotspots.length > 0 ? '：' + hotspots.map((hot) => hot.label || hot.reason || '?').join(' / ') : ''}`)
        } else if (data.action === 'reload-client') {
          lines.push(data.rev ? `已请求 client 热重载 · rev=${data.rev}` : 'client 热重载未发生（rev 未变化或 client-modules 不可用）')
        } else if (data.action === 'client') {
          lines.push(data.entry === null || data.entry === undefined
            ? 'client 半不在 boot graph 里（未注册或 manifest 未被扫描）'
            : `client 半已在 boot graph：${data.entry.id} · ${data.entry.immediately === true ? 'immediately' : 'lazy'} · inject=${Array.isArray(data.entry.inject) ? data.entry.inject.join(', ') : '—'}`)
          if (typeof data.entry?.url === 'string') lines.push(`url: ${data.entry.url}`)
          if (typeof data.clientPath === 'string') lines.push(`clientPath: ${data.clientPath}`)
        } else if (data.action === 'enable' || data.action === 'disable') {
          lines.push(data.enabledSession === true
            ? '本会话快思考已开启：做题消息由本地课程引擎直接回答，其它消息仍走当前模型。'
            : '本会话快思考已关闭：所有消息都走当前模型。')
        } else {
          for (const entry of Array.isArray(data.entries) ? data.entries : []) lines.push(JSON.stringify(entry))
        }
        return [{ type: 'text', text: lines.join('\n') }]
      },
    },
    async execute(args, exec) {
      const action = args !== null && typeof args === 'object' && typeof args.action === 'string' ? args.action : 'status'
      if (action === 'explain') {
        const text = args !== null && typeof args === 'object' && typeof args.text === 'string' ? args.text : ''
        const core = await loadQuizCore()
        const fast = core.classifyFast('quiz-auto', text)
        return { ok: true, action, rule: fast === null ? '' : fast.type, candidate: fast !== null, enabled: settings.enabled }
      }
      if (action === 'log') {
        const limit = Math.max(1, Math.min(100, Math.floor(Number(args?.limit) || 10)))
        return { ok: true, action, entries: readLogTail(settings.historyPath, limit), enabled: settings.enabled }
      }
      if (action === 'enable' || action === 'disable') {
        const sessionId = exec !== null && exec !== undefined && exec.agent !== null && exec.agent !== undefined
          && exec.agent.session !== null && exec.agent.session !== undefined
          ? exec.agent.session.id
          : undefined
        const enable = action === 'enable'
        const result = setSessionEnabled(enabledSessions, settings, sessionId, enable, record, 'sokonanoda_fast_think')
        if (result.ok !== true) return { ok: false, action, error: result.error, enabled: settings.enabled }
        return {
          ok: true,
          action,
          sessionId,
          enabledSession: enable,
          enabled: settings.enabled,
          sessions: [...enabledSessions],
        }
      }
      if (action === 'reload-client') {
        if (clientModules === undefined || clientModules === null) {
          return { ok: false, action, error: 'clientModules service unavailable', enabled: settings.enabled }
        }
        try {
          const clientPath = clientModules.clientPath('sokonanoda-dsh')
          if (typeof clientPath === 'string' && clientPath !== '') {
            const now = new Date()
            fs.utimesSync(clientPath, now, now)
          }
          const rev = clientModules.rebuilt('sokonanoda-dsh')
          let entry = null
          try {
            const graph = clientModules.graph()
            entry = (graph.entries || []).find((item) => item && item.id === 'sokonanoda-dsh') || null
          } catch { entry = null }
          return {
            ok: true,
            action,
            rev: typeof rev === 'string' ? rev : null,
            entry: entry === null ? null : { id: entry.id, url: entry.url, rev: entry.rev },
            enabled: settings.enabled,
          }
        } catch (error) {
          return { ok: false, action, error: String(error?.message || error), enabled: settings.enabled }
        }
      }
      if (action === 'client') {
        if (clientModules === undefined || clientModules === null) {
          return { ok: false, action, error: 'clientModules service unavailable', enabled: settings.enabled }
        }
        try {
          const graph = clientModules.graph()
          const entries = Array.isArray(graph.entries) ? graph.entries : []
          const entry = entries.find((item) => item && item.id === 'sokonanoda-dsh') || null
          let clientPath
          try { clientPath = clientModules.clientPath('sokonanoda-dsh') } catch { clientPath = undefined }
          return {
            ok: true,
            action,
            entry: entry === null ? null : {
              id: entry.id,
              url: entry.url,
              rev: entry.rev,
              inject: entry.inject,
              immediately: entry.immediately,
            },
            clientPath,
            enabled: settings.enabled,
          }
        } catch (error) {
          return { ok: false, action, error: String(error?.message || error), enabled: settings.enabled }
        }
      }
      if (action === 'progress') {
        if (typeof service.url !== 'string' || service.url === '' || service.app === undefined) {
          return { ok: false, action, error: 'quiz service not started', enabled: settings.enabled }
        }
        const core = await loadQuizCore()
        const card = await core.buildProgressCardPayload(service.app, path.join(settings.quizStateDir, 'default.json'))
        return { ok: true, action, payload: card, enabled: settings.enabled }
      }
      let proxy = { ok: false }
      if (typeof service.url === 'string' && service.url !== '') {
        try {
          const response = await fetch(`${service.url}/health`, { signal: AbortSignal.timeout(1500) })
          proxy = response.ok === true ? await response.json() : { ok: false, status: response.status }
        } catch (error) {
          proxy = { ok: false, error: String(error?.message || error) }
        }
      } else {
        proxy = { ok: false, error: 'service not started' }
      }
      const entries = readLogTail(settings.historyPath, settings.decisionsLimit)
      record({ event: 'tool', mode: 'status', via: 'sokonanoda_fast_think' })
      return { ok: true, action: 'status', enabled: settings.enabled, proxy, entries, sessions: [...enabledSessions] }
    },
  }
}

/**
 * Plugin entry: start the in-process quiz service, register the fast path and
 * the inspection tool for this Host. Service failure never breaks Host startup:
 * the plugin stays loaded and every model call falls through to `next()`.
 * @param ctx - plugin context.
 * @param config - bundle row config (all fields optional).
 */
export async function apply(ctx, config = {}) {
  const fileConfig = readFileConfig(defaultDataDir())
  const settings = normalizeConfig(config, fileConfig)
  for (const dir of [settings.quizStateDir, settings.logDir, path.dirname(settings.historyPath)]) {
    try { fs.mkdirSync(dir, { recursive: true }) } catch { /* the service/tool report the real error later */ }
  }
  const record = makeRecorder(settings.historyPath)
  const enabledSessions = loadSessions(settings.sessionsPath)
  let clientModules
  try { clientModules = ctx.get('clientModules') } catch { clientModules = undefined }
  let webServer
  try { webServer = ctx.get('webServer') } catch { webServer = undefined }

  const writeJson = (response, status, body) => {
    try {
      const text = JSON.stringify(body)
      response.writeHead(status, {
        'content-type': 'application/json; charset=utf-8',
        'cache-control': 'no-store',
      })
      response.end(text)
    } catch {
      try { response.writeHead(500); response.end() } catch { /* the socket is already gone */ }
    }
  }

  /** The Session workspace the resource address speaks for, when the Host knows it. */
  const workspaceRootOf = async (sessionId) => {
    let files
    try { files = ctx.get('workspaceFiles') } catch { files = undefined }
    if (files !== undefined && files !== null && typeof files.scopeOf === 'function') {
      try {
        const scope = await files.scopeOf(sessionId)
        if (scope !== null && scope !== undefined && typeof scope.workspaceRoot === 'string') {
          return scope.workspaceRoot
        }
      } catch { /* the sandbox fallback below still has the deployment root */ }
    }
    try {
      const policy = ctx.get('sandboxPolicy')
      if (policy !== undefined && policy !== null && typeof policy.workspaceRoot === 'string') {
        return policy.workspaceRoot
      }
    } catch { /* no policy in this composition */ }
    return null
  }

  /** GET /sokonanoda-state — owner-only, same-origin read of one cursor position. */
  const handleStateRequest = async (request, response, peer) => {
    if (peer === null || peer === undefined || peer?.principal?.kind !== 'owner') {
      writeJson(response, 403, { ok: false, error: { code: 'forbidden', message: '只接受本机所有者' } })
      return
    }
    let url
    try {
      url = new URL(String(request?.url || '/'), 'http://dsh.invalid')
    } catch {
      writeJson(response, 400, { ok: false, error: { code: 'bad-request', message: '无法解析请求地址' } })
      return
    }
    const resource = String(url.searchParams.get('resource') || '')
    const line = Number(url.searchParams.get('line'))
    const colRaw = url.searchParams.get('col')
    const col = colRaw === null ? 1 : Number(colRaw)
    if (resource === '' || !Number.isInteger(line) || line < 1 || line > 10_000_000
      || !Number.isInteger(col) || col < 1 || col > 1_000_000) {
      writeJson(response, 400, { ok: false, error: { code: 'bad-request', message: '需要 resource 与正整数 line/col' } })
      return
    }
    const parsed = parseFileResource(resource)
    if (parsed === null) {
      writeJson(response, 400, { ok: false, error: { code: 'bad-resource', message: '无法识别的文件资源地址' } })
      return
    }
    const workspaceRoot = await workspaceRootOf(parsed.sessionId)
    const target = resolveAllowedStatePath({ resource, workspaceRoot, repoRoot: settings.repoRoot })
    if (!target.ok) {
      const status = target.code === 'not-found' ? 404 : target.code === 'bad-resource' ? 400 : 403
      writeJson(response, status, { ok: false, error: { code: target.code, message: target.message } })
      return
    }
    const answer = await stateAnswer(settings, target.path, line, col)
    writeJson(response, 200, answer)
  }

  const registerWebRoutes = (server) => {
    if (server === null || server === undefined || typeof server.register !== 'function') return
    ctx.effect(() => server.register({
      kind: 'exact',
      path: '/sokonanoda-client-report',
      auth: 'anonymous',
      handler(request, response) {
        try {
          const url = new URL(request.url || '/', 'http://dsh.invalid')
          const entry = {
            time: new Date().toISOString(),
            stage: String(url.searchParams.get('stage') || '').slice(0, 40),
            key: String(url.searchParams.get('key') || '').slice(0, 40),
            kind: String(url.searchParams.get('kind') || '').slice(0, 40),
            ua: String(request.headers?.['user-agent'] || '').slice(0, 120),
          }
          fs.mkdirSync(settings.logDir, { recursive: true })
          fs.appendFileSync(path.join(settings.logDir, 'client-report.log'), `${JSON.stringify(entry)}\n`, 'utf8')
          response.writeHead(204, { 'cache-control': 'no-store' })
          response.end()
        } catch {
          try { response.writeHead(500); response.end() } catch { /* report failure stays local */ }
        }
      },
    }), 'sokonanoda-dsh: client report route')
    ctx.effect(() => server.register({
      kind: 'exact',
      path: STATE_ROUTE,
      auth: 'required',
      handler: (request, response, peer) => handleStateRequest(request, response, peer),
    }), 'sokonanoda-dsh: read-only goal state route')
  }

  if (webServer !== undefined && webServer !== null && typeof webServer.register === 'function') {
    registerWebRoutes(webServer)
  } else if (typeof ctx.inject === 'function') {
    /* The Web composition registers `webServer` after some plugins apply; a
     * one-shot ctx.get would silently lose both routes. Wait for the service. */
    try {
      ctx.inject(['webServer'], (scope) => {
        let server
        try { server = scope?.webServer ?? scope?.get?.('webServer') } catch { server = undefined }
        registerWebRoutes(server)
      })
    } catch { /* non-Web compositions get no routes */ }
  }
  const service = { url: '', close: null }
  try {
    const core = await loadQuizCore()
    const started = await core.startQuizService({
      repoRoot: settings.repoRoot,
      stateDir: settings.quizStateDir,
      runDir: path.join(settings.logDir, 'quiz-proxy'),
      cardsMode: settings.cards === false ? 'off' : 'learn',
    })
    service.url = started.url
    service.close = started.close
    service.app = started.app
    record({ event: 'service-start', url: started.url, dataDir: settings.dataDir })
  } catch (error) {
    record({ event: 'service-error', message: String(error?.message || error), dataDir: settings.dataDir })
  }
  ctx.effect(function () {
    return function () { if (typeof service.close === 'function') void service.close() }
  }, 'sokonanoda-dsh: quiz service lifetime')

  ctx.effect(() => ctx.on('llm/stream', (options, next) => {
    if (!settings.enabled || service.url === '' || typeof options?.sessionId !== 'string'
      || !enabledSessions.has(options.sessionId)) return next()
    return (async function* fastPath() {
      let decision = null
      try {
        if (!(await isAgentLoopRequestCompat(options))) {
          yield* next()
          return
        }
        const core = await loadQuizCore()
        /* 只有「已发出代码题 + 围栏代码」才读状态；普通消息不付这次 I/O。 */
        const lastUser = core.extractConversation(options.messages).lastUserText
        const codeIssued = typeof lastUser === 'string' && lastUser.includes('```')
          ? readIssuedCode(settings.quizStateDir) !== ''
          : false
        decision = decideFast(options, core, { codeIssued })
        if (decision === null) {
          yield* next()
          return
        }
        const startedAt = Date.now()
        /* 代码题要 compose + 真内核判卷，超时比普通问答宽（仍由 codeTimeoutMs 兜住）。 */
        const codeTurn = decision.fast.type === 'code' || decision.fast.type === 'code-next'
        const payload = await callProxy({
          ...settings,
          timeoutMs: codeTurn ? settings.codeTimeoutMs : settings.timeoutMs,
          proxyUrl: `${service.url}/v1/chat/completions`,
        }, {
          model: 'quiz-auto',
          stream: false,
          messages: toProxyMessages(options, core),
          tools: [LEARN_CARD_TOOL_DECLARATION],
        }, options.signal)
        const text = composeReplyText(payload)
        if (text.trim() === '') {
          record({ event: 'miss', mode: 'fallback', reason: 'empty-reply', rule: decision.fast.type, sessionId: options.sessionId })
          yield* next()
          return
        }
        record({
          event: 'fast',
          mode: 'fast',
          rule: decision.fast.type,
          sessionId: options.sessionId,
          ms: Date.now() - startedAt,
          chars: text.length,
          text: decision.text.slice(0, 80),
        })
        yield* synthesizeChunks(text)
      } catch (error) {
        record({
          event: 'fallback',
          mode: 'fallback',
          reason: 'error',
          rule: decision !== null ? decision.fast.type : null,
          sessionId: options !== null && typeof options === 'object' ? options.sessionId : null,
          message: String(error?.message || error),
        })
        yield* next()
      }
    })()
  }), 'sokonanoda-dsh: llm/stream fast path')

  ctx.effect(() => ctx.commands.register({
    name: 'sokonanoda-fast-think',
    description: '按会话开关 sokonanoda 快思考；只在上「形式化证明」学习课时开启',
    input: { hint: 'on | off | status' },
    handler(invocation) {
      const agent = invocation !== null && typeof invocation === 'object' ? invocation.agent : undefined
      const session = agent !== null && typeof agent === 'object' ? agent.session : undefined
      const sessionId = session !== null && typeof session === 'object' && typeof session.id === 'string' ? session.id : ''
      if (sessionId === '') return { kind: 'error', text: '当前会话不可用，无法切换快思考。' }
      const action = String(invocation.rawInput || '').trim().toLowerCase()
      if (action === '' || action === 'status') {
        return {
          kind: 'success',
          text: `本会话快思考：${enabledSessions.has(sessionId) ? '已开启' : '已关闭'}。用法：/sokonanoda-fast-think on | off | status`,
        }
      }
      if (action !== 'on' && action !== 'off') {
        return { kind: 'error', text: '用法：/sokonanoda-fast-think on | off | status' }
      }
      const enable = action === 'on'
      const result = setSessionEnabled(enabledSessions, settings, sessionId, enable, record, 'command')
      if (result.ok !== true) {
        return { kind: 'error', text: `保存快思考开关失败：${String(result.error || 'unknown')}` }
      }
      return {
        kind: 'success',
        text: enable
          ? '本会话快思考已开启：做题消息会由本地课程引擎直接回答，其它消息仍走当前模型。'
          : '本会话快思考已关闭：所有消息都走当前模型。',
      }
    },
  }), 'sokonanoda-dsh: /sokonanoda-fast-think command')

  ctx.effect(() => ctx.tools.register(createFastThinkTool(settings, record, service, enabledSessions, clientModules)), 'sokonanoda-dsh: sokonanoda_fast_think tool')
}
