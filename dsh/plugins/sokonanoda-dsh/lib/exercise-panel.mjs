// exercise-panel —— C9.7 侧栏做题面板的 Host 桥。
//
// 面板本身只做「展示题目 + 编辑 + 点运行」；判卷**只有一条路**：
// 这里解析可信题目 → C9.4 的 `gradeCodeSubmission`（compose.py + `scripts/soko
// grade --json`）→ 把 `decl.checked` / `exercise.open` / `diagnostic` 原样交回。
//
// 三条硬约束（与 code-grade.mjs 同源）：
//   1. **题目来自仓库里的数据集/课程源**：客户端只发 `exerciseId`，签名/模板
//      永远由 Host 从可信记录取；客户端不能换命题、不能提交自己的签名。
//   2. **绝不伪造判定**：只有 Host 返回 `passed:true` 才是通过；基础设施失败
//      是 `{ok:false}`，面板必须显示明确的不可用，而不是绿色。
//   3. **不落盘用户代码**：与聊天判卷一样只写系统临时目录，判完即删；日志与
//      错误里不出现用户代码（错误消息只含内核自己的诊断文本）。
//
// 只在 DSH Web 的 loopback 路由上用（index.js 注册），`auth: 'anonymous'`
// 与既有的 `/sokonanoda-client-report` 一致；路由只读仓库、只读题目、不写
// 任何仓库文件。

import fs from 'node:fs'
import path from 'node:path'

import {
  CODE_MAX_BYTES,
  DEFAULT_COMPOSE_TIMEOUT_MS,
  DEFAULT_GRADE_TIMEOUT_MS,
  expectedDeclarationName,
  gradeCodeSubmission,
} from './code-grade.mjs'

export const PANEL_EXERCISE_PATH = '/sokonanoda-exercise'
export const PANEL_GRADE_PATH = '/sokonanoda-grade'

/** 请求体上限：选择题之外的代码题最多 64 KiB（与 code-grade 同值）。 */
export const PANEL_BODY_MAX_BYTES = CODE_MAX_BYTES + 16 * 1024

const ID_RE = /^[A-Za-z0-9][A-Za-z0-9._-]*(?:\/[A-Za-z0-9._-]+)*$/
const COURSE_RE = /^[a-z0-9][a-z0-9-]*$/
const ENGINE_CODE_BANK = path.join('courses', 'set-theory', 'knowledge', 'data', 'code-exercises.json')

/** dataset 文件按 (mtime, size) 缓存；题目集是生成物，可能被重新 build。 */
const datasetCache = new Map()

function ignored() {
  /* 缓存/清理失败不影响判卷 */
}

/** 读一份 JSON，按 mtime+size 缓存（数据集最大 ~2.6 MB，避免每点一次运行都重读）。 */
function readJsonCached(file) {
  let stats
  try {
    stats = fs.statSync(file)
  } catch (error) {
    return null
  }
  const cached = datasetCache.get(file)
  if (cached !== undefined && cached.mtimeMs === stats.mtimeMs && cached.size === stats.size) {
    return cached.value
  }
  let value = null
  try {
    const parsed = JSON.parse(fs.readFileSync(file, 'utf8'))
    value = parsed !== null && typeof parsed === 'object' ? parsed : null
  } catch (error) {
    value = null
  }
  datasetCache.set(file, { mtimeMs: stats.mtimeMs, size: stats.size, value })
  return value
}

/**
 * 校验题目 ID（只允许 dataset 的稳定 ID 形状，杜绝路径穿越）。
 * @param value - 客户端传来的 exerciseId。
 * @returns 合法 ID 或 null。
 */
export function normalizeExerciseId(value) {
  if (typeof value !== 'string') return null
  const id = value.trim()
  if (id === '' || id.length > 300 || !ID_RE.test(id)) return null
  /* 数据集 ID 从不用 `.`/`..` 段；拒绝它们，宁可 400 也不让路径形状混进来。 */
  for (const segment of id.split('/')) {
    if (segment === '.' || segment === '..') return null
  }
  return id
}

function tags(value) {
  return Array.isArray(value) ? value : []
}

/** dataset 记录 → 面板题目（不含 solution，不把答案发给客户端）。 */
function fromDatasetRecord(record, course) {
  const source = record.source !== null && typeof record.source === 'object' ? record.source : null
  return {
    id: record.id,
    course: record.course || course,
    chapter: String(record.chapter || ''),
    chapterTitle: String(record.chapter_title || ''),
    unit: String(record.unit || ''),
    title: String(record.title || record.name || record.id),
    prompt: String(record.prompt || ''),
    kind: String(record.kind || 'theorem'),
    name: String(record.name || ''),
    anonymous: record.anonymous === true,
    hints: tags(record.hints).map((hint) => (
      typeof hint === 'string'
        ? { label: '', body: hint }
        : { label: String((hint && hint.label) || ''), body: String((hint && hint.body) || '') }
    )).filter((hint) => hint.body !== ''),
    template: String(record.template || ''),
    dataset: true,
    source: source !== null && typeof source.file === 'string'
      ? source.file + (Number.isFinite(Number(source.line)) ? ':' + Number(source.line) : '')
      : '',
  }
}

/** 引擎自带小题库记录 → 同一份面板题目形状（template 自包含）。 */
function fromEngineRecord(record) {
  const template = String(record.template || '')
  return {
    id: String(record.id || ''),
    course: 'set-theory',
    chapter: String(record.chapter || ''),
    chapterTitle: '',
    unit: String(record.unit || ''),
    title: String(record.title || record.id || ''),
    prompt: String(record.prompt || ''),
    kind: 'theorem',
    name: expectedDeclarationName({ template }),
    anonymous: false,
    hints: tags(record.hints).map((hint) => (typeof hint === 'string' ? { label: '', body: hint } : hint))
      .filter((hint) => hint && typeof hint === 'object' && String(hint.body || '') !== '')
      .map((hint) => ({ label: String(hint.label || ''), body: String(hint.body) })),
    template,
    dataset: false,
    source: '',
  }
}

/**
 * 由题目 ID 解析出可信题目（数据集优先，其次引擎小题库）。
 * @param repoRoot - 仓库根。
 * @param exerciseId - 客户端传来的 ID（会先校验）。
 * @returns `{ok:true, exercise}` 或 `{ok:false, code, status}`。
 */
export function resolvePanelExercise(repoRoot, exerciseId) {
  const id = normalizeExerciseId(exerciseId)
  if (id === null) return { ok: false, code: 'bad-id', status: 400 }
  const course = id.split('/')[0]
  const datasetRoot = path.resolve(path.join(repoRoot, 'courses', 'exercise-dataset', 'data'))
  if (COURSE_RE.test(course)) {
    const file = path.resolve(path.join(datasetRoot, course + '.json'))
    /* 只读 `data/<course>.json` 这一个文件：course 已受限，且 file 必须留在 data 下。 */
    if (path.dirname(file) === datasetRoot) {
      const payload = readJsonCached(file)
      const records = payload !== null && Array.isArray(payload.exercises) ? payload.exercises : []
      const record = records.find((item) => item !== null && typeof item === 'object' && item.id === id)
      if (record !== undefined) {
        const exercise = fromDatasetRecord(record, course)
        if (exercise.template.trim() === '') return { ok: false, code: 'no-template', status: 422 }
        return { ok: true, exercise }
      }
    }
  }
  /* 引擎小题库（6 道，template 自包含；与聊天快路径同一来源）。 */
  const bank = readJsonCached(path.join(repoRoot, ENGINE_CODE_BANK))
  const records = bank !== null && Array.isArray(bank.exercises) ? bank.exercises : []
  const record = records.find((item) => item !== null && typeof item === 'object' && item.id === id)
  if (record !== undefined) {
    const exercise = fromEngineRecord(record)
    if (exercise.template.trim() === '') return { ok: false, code: 'no-template', status: 422 }
    return { ok: true, exercise }
  }
  return { ok: false, code: 'not-found', status: 404 }
}

/**
 * 面板可见的题目字段（显式白名单；绝不包含 solution / 答案）。
 * @param exercise - resolvePanelExercise 的结果。
 * @returns 可安全发给客户端的对象。
 */
export function panelExerciseView(exercise) {
  const value = exercise || {}
  return {
    id: String(value.id || ''),
    course: String(value.course || ''),
    chapter: String(value.chapter || ''),
    chapterTitle: String(value.chapterTitle || ''),
    unit: String(value.unit || ''),
    title: String(value.title || ''),
    prompt: String(value.prompt || ''),
    kind: String(value.kind || 'theorem'),
    name: String(value.name || ''),
    anonymous: value.anonymous === true,
    hints: Array.isArray(value.hints) ? value.hints : [],
    template: String(value.template || ''),
    dataset: value.dataset === true,
    source: String(value.source || ''),
  }
}

function failure(code, message) {
  return { ok: false, code, message }
}

/**
 * 判一次面板提交：题目可信解析 → C9.4 判卷桥。
 * 用户代码错误（空/声明名不符）原样返回 `{ok:true, result:{ok:false,...}}`；
 * 基础设施失败返回 `{ok:false, code, message}`，面板据此显示“不可用”。
 * @param repoRoot - 仓库根。
 * @param input - `{exerciseId, code}`。
 * @param options - 超时等可选项。
 * @returns 面板 JSON 响应体。
 */
export async function gradePanelSubmission(repoRoot, input, options = {}) {
  const value = input !== null && typeof input === 'object' ? input : {}
  const resolved = resolvePanelExercise(repoRoot, value.exerciseId)
  if (resolved.ok !== true) return resolved
  const code = value.code
  if (typeof code !== 'string') return failure('bad-code', '没有收到可判的代码。')
  const maxBytes = options.maxBytes || CODE_MAX_BYTES
  /* 超长是**用户输入问题**，交给判卷桥按 `{ok:false, code:'too-large'}` 返回，
   * 面板显示“代码太长”而不是“服务不可用”。 */
  try {
    const result = await gradeCodeSubmission({
      repoRoot,
      exercise: resolved.exercise,
      userCode: code,
      maxBytes,
      timeoutMs: Number(options.timeoutMs) > 0 ? Number(options.timeoutMs) : DEFAULT_GRADE_TIMEOUT_MS,
      composeTimeoutMs: Number(options.composeTimeoutMs) > 0
        ? Number(options.composeTimeoutMs)
        : DEFAULT_COMPOSE_TIMEOUT_MS,
    })
    return { ok: true, exercise: panelExerciseView(resolved.exercise), result }
  } catch (error) {
    const code = error !== null && typeof error === 'object' && typeof error.code === 'string'
      ? error.code
      : 'run-failed'
    const message = error !== null && typeof error === 'object' && typeof error.message === 'string'
      ? error.message.slice(0, 200)
      : '判卷进程启动失败'
    return failure(code, message)
  }
}

/**
 * 读 JSON 请求体（有上限；坏 JSON / 超限都是可读的失败）。
 * @param request - Node IncomingMessage。
 * @param maxBytes - 上限。
 * @returns `{ok:true, value}` 或 `{ok:false, code, status}`。
 */
export function readJsonBody(request, maxBytes = PANEL_BODY_MAX_BYTES) {
  return new Promise((resolve) => {
    const chunks = []
    let size = 0
    let over = false
    let settled = false
    const finish = (result) => {
      if (settled) return
      settled = true
      resolve(result)
    }
    request.on('data', (chunk) => {
      if (over) return
      size += chunk.length
      if (size > maxBytes) {
        over = true
        chunks.length = 0
        return
      }
      chunks.push(chunk)
    })
    request.on('end', () => {
      if (over) return finish({ ok: false, code: 'too-large', status: 413 })
      try {
        const value = JSON.parse(Buffer.concat(chunks).toString('utf8'))
        if (value === null || typeof value !== 'object' || Array.isArray(value)) {
          return finish({ ok: false, code: 'bad-json', status: 400 })
        }
        return finish({ ok: true, value })
      } catch (error) {
        return finish({ ok: false, code: 'bad-json', status: 400 })
      }
    })
    request.on('error', () => finish({ ok: false, code: 'read-failed', status: 400 }))
  })
}

/**
 * 写一个 JSON 响应（no-store；面板轮询/重试都不该被缓存）。
 * @param response - Node ServerResponse。
 * @param status - HTTP 状态码。
 * @param payload - JSON 可序列化对象。
 */
export function sendJson(response, status, payload) {
  try {
    const body = JSON.stringify(payload)
    response.writeHead(status, {
      'content-type': 'application/json; charset=utf-8',
      'cache-control': 'no-store',
      'content-length': Buffer.byteLength(body, 'utf8'),
    })
    response.end(body)
  } catch (error) {
    try {
      response.writeHead(500, { 'content-type': 'application/json; charset=utf-8' })
      response.end('{"ok":false,"code":"write-failed"}')
    } catch (ignoredError) {
      /* response already gone */
    }
  }
}

/**
 * `/sokonanoda-exercise` 的 handler：GET ?id=… 返回面板题目。
 * @param repoRoot - 仓库根。
 * @param request - Node IncomingMessage。
 * @param response - Node ServerResponse。
 */
export function handleExerciseRequest(repoRoot, request, response) {
  if (request.method !== 'GET') {
    sendJson(response, 405, { ok: false, code: 'method-not-allowed' })
    return
  }
  let url
  try {
    url = new URL(request.url || '/', 'http://dsh.invalid')
  } catch (error) {
    sendJson(response, 400, { ok: false, code: 'bad-url' })
    return
  }
  const resolved = resolvePanelExercise(repoRoot, url.searchParams.get('id'))
  if (resolved.ok !== true) {
    sendJson(response, resolved.status || 404, { ok: false, code: resolved.code })
    return
  }
  sendJson(response, 200, { ok: true, exercise: panelExerciseView(resolved.exercise) })
}

/**
 * `/sokonanoda-grade` 的 handler：POST {exerciseId, code} → 真内核判卷。
 * @param repoRoot - 仓库根。
 * @param request - Node IncomingMessage。
 * @param response - Node ServerResponse。
 * @param options - 判卷超时等可选项。
 */
export async function handleGradeRequest(repoRoot, request, response, options = {}) {
  if (request.method !== 'POST') {
    sendJson(response, 405, { ok: false, code: 'method-not-allowed' })
    return
  }
  const type = String(request.headers?.['content-type'] || '').toLowerCase()
  if (!type.startsWith('application/json')) {
    /* 只收 JSON：跨站简单请求（text/plain）无法在不触发 preflight 的情况下命中。 */
    sendJson(response, 415, { ok: false, code: 'unsupported-media-type' })
    return
  }
  const body = await readJsonBody(request)
  if (body.ok !== true) {
    sendJson(response, body.status || 400, { ok: false, code: body.code })
    return
  }
  const result = await gradePanelSubmission(repoRoot, body.value, options)
  if (result.ok !== true && (result.code === 'bad-id' || result.code === 'not-found')) {
    sendJson(response, result.status || 404, { ok: false, code: result.code })
    return
  }
  sendJson(response, 200, result)
}
