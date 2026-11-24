#!/usr/bin/env node
// quiz-proxy —— 本地 OpenAI 兼容「中转模型站」（零依赖，Node >= 20，实测 Node 24）。
//
// 分流（按请求里【最后一条 user 消息】的文本，model 为 quiz-auto 时）：
//   ① 字母答案 A–D  → 本地题库引擎判卷（毫秒级、不调任何模型），反馈里自动附下一题；
//   ② 开始做题/下一题/继续/… → 本地引擎出题（不调任何模型）；
//   ③ 其它消息       → 慢路径：原样转发到 opencode-go（用户现有 key 的慢思考）。
//   ④ 卡片模式（QUIZ_PROXY_CARDS=learn，且请求 tools 里声明了 learn_card 时）：
//      快路径直接以 assistant 身份发 learn_card 的 tool_call → DSH 聊天里渲染真卡片；
//      卡片是「只读展示」的，用户直接在输入框回字母；工具结果回来（role=tool）时按
//      tool_call_id 认领并回一条空 content + finish_reason=stop（绝不走慢路径）。
//      QUIZ_PROXY_CARDS=1 保留旧的 ask_user_question 点选卡片逻辑（legacy，默认关闭）。
//
// 只监听 127.0.0.1。日志写 stderr，**永不打印 API key**。
//
// CLI：node server.mjs            （环境变量见 README.md / DEFAULTS）
// 测试：import { createQuizProxy } from './server.mjs'
import http from 'node:http';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';

const execFileAsync = promisify(execFile);

// ───────────────────────── 常量 ─────────────────────────
export const ENGINE_REL = path.join('courses', 'set-theory', 'knowledge', 'engine.py');
export const QUIZ_MODEL = 'quiz-auto';
export const SLOW_MODEL = 'deepseek-v4.1-flash';
/**
 * 慢路径用的上游 API key 名。默认跟当前会话的 provider 一致（opencode-go-7），
 * 换 provider 时只改这一个默认值或设 QUIZ_PROXY_API_KEY_ENV。
 */
export const API_KEY_NAME = String(process.env.QUIZ_PROXY_API_KEY_ENV || 'OPENCODE_GO_API_KEY_7').trim();

const DEFAULT_UPSTREAM = 'https://opencode.ai/zen/go/v1/chat/completions';
const DEFAULT_HOST = '127.0.0.1';
const DEFAULT_PORT = 3090;
const DEFAULT_MAX_BODY_BYTES = 8 * 1024 * 1024;
const DEFAULT_UPSTREAM_TIMEOUT_MS = 120_000;
const DEFAULT_ENGINE_TIMEOUT_MS = 30_000;

const LETTER_RE = /^\s*([A-Da-d])\s*$/;
const START_RE = /^\s*(?:开始做题|练几题|出题|下一题|下一道|继续|换一题|换一道|再来一题|再来一道|next)\s*[!！。.~～?？]*\s*$/i;
const LETTER_LOOSE_RE = /(?:^|[^A-Za-z])(?:选|答案|我选|choose|answer)?\s*[:：]?\s*[（(\[【]?\s*([A-Da-d])\s*[）)\]】]?\s*(?:[.。、)）]|$)/;
const CJK_LETTERS = { 甲: 'A', 乙: 'B', 丙: 'C', 丁: 'D' };
const SKIP_RE = /^(?:我会了|懂了不用做|太简单|跳过(?:这个|本)?知识点|跳过)$/;
const MAP_RE = /^\s*(?:(?:学习)?地图|我在哪(?:里|儿)?|到哪(?:了|儿)?|进度|学到哪|还有多少|学习概况|学习进度|学习进展|看看?进度|看下进度)(?:了|啦|呀|呢)?\s*[!！。.?？~～]*\s*$/i;
const EXPLAIN_RE = /^(?:我)?(?:不懂|不会|不明白|看不懂|没懂|不理解|讲讲|讲一下|讲解|解释(?:一下)?|知识卡|卡片|帮我讲(?:讲|解)?|这题(?:是)?(?:什么|怎么)|为什么这题)(?:这题|一下|下|吧|啊|呢|\?|？|。|!|！)*$/;

// 卡片模式常量
export const ASK_TOOL_NAME = 'ask_user_question'; // legacy：点选卡片（QUIZ_PROXY_CARDS=1）
export const LEARN_CARD_TOOL_NAME = 'learn_card';

/**
 * learn_card 是纯展示工具：DSH 执行完工具会**再请求一次模型**，这一轮必须回一条
 * assistant 消息把回合关掉。回 `content:''` 时 DSH 不会把它记成一条 assistant 消息，
 * 于是它带着**同一条 tool 结果**反复重试（实测一次点题重试 6 次），最终界面报
 * 「卡片渲染失败」。所以这里回一个**非空但不可见**的零宽空格：DSH 记为普通
 * assistant 文本（渲染为空白），回合正常结束，卡片仍是唯一可见输出。
 * 需要完全静默可设 QUIZ_PROXY_ECHO_CONTENT=''。
 */
export const ECHO_SILENT_CONTENT =
  process.env.QUIZ_PROXY_ECHO_CONTENT === undefined
    ? '\u200b'
    : String(process.env.QUIZ_PROXY_ECHO_CONTENT); // 真卡片插件 @local/dsh-learn-card
const CARD_ID_PREFIX = 'quiz-';
const CARD_HEADER_MAX_CHARS = 60;
const CARD_REPLY_CACHE_LIMIT = 200;
const CARD_PENDING_LIMIT = 100;
/** I2：题目已更新（句柄对不上）时的固定提示。 */
const STALE_QUESTION_TEXT = '这道题已更新，请按屏幕上的题重答。';
/** 句柄/判分幂等缓存的容量上限。 */
const HANDLE_LIMIT = 500;
const GRADE_CACHE_LIMIT = 200;

/** 用户输入归一化：去 markdown 代码标记/空白，全角字母转半角，甲乙丙丁映射。 */
export function normalizeUserText(raw) {
  if (typeof raw !== 'string') return '';
  return raw
    .replace(/[`*_~]/g, '')
    .replace(/[ＡＢＣＤａｂｃｄ]/g, (ch) => String.fromCharCode(ch.charCodeAt(0) - 0xFEE0))
    .replace(/\s+/g, ' ')
    .trim();
}

/** 从（可能很啰嗦的）用户输入里抽 1-based 选项序号；抽不到返回 0。 */
export function parseChoiceLetter(raw) {
  const text = normalizeUserText(raw);
  if (text === '') return 0;
  const cjk = text.match(/[甲乙丙丁]/);
  if (cjk && text.replace(/[甲乙丙丁]/g, '').replace(/[\s选答案:：]/g, '') === '') {
    return 'ABCD'.indexOf(CJK_LETTERS[cjk[0]]) + 1;
  }
  const strict = text.match(LETTER_RE);
  if (strict) return 'ABCD'.indexOf(strict[1].toUpperCase()) + 1;
  if (text.length <= 12) {
    const loose = text.match(LETTER_LOOSE_RE);
    if (loose) return 'ABCD'.indexOf(loose[1].toUpperCase()) + 1;
  }
  return 0;
}
const ENCOURAGEMENTS = [
  '这一步很稳。',
  '保持这个节奏。',
  '思路已经对了。',
  '手感不错。',
  '继续这样推下去就好。',
  '很好，就这样一点点来。',
];

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// ───────────────────────── 小工具 ─────────────────────────
function defaultLogger(line) {
  process.stderr.write(`[${new Date().toISOString()}] ${line}\n`);
}

function defaultCredentialsPath() {
  return path.join(os.homedir(), '.dsh', '.credentials.yaml');
}

/** 只从 credentials.yaml 抽当前 key 名（`API_KEY_NAME`）这一行；绝不打日志。 */
export function readApiKeyFromCredentials(file, keyName = API_KEY_NAME) {
  try {
    const text = fs.readFileSync(file, 'utf8');
    const escaped = keyName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const match = text.match(new RegExp('^\\s*' + escaped + '\\s*:\\s*(.+?)\\s*$', 'm'));
    if (!match) return '';
    let value = match[1].trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    } else {
      value = value.replace(/\s+#.*$/, '').trim();
    }
    return value;
  } catch {
    return '';
  }
}

/** 环境变量优先，其次 ~/.dsh/.credentials.yaml。 */
export function loadApiKey({ credentialsPath } = {}) {
  const fromEnv = String(process.env[API_KEY_NAME] || '').trim();
  if (fromEnv) return fromEnv;
  return readApiKeyFromCredentials(credentialsPath || defaultCredentialsPath());
}

/** 从 startDir 逐级向上找含 engine.py 的仓库根。 */
/* ── 升级提醒（用户 2026-11-23：后台 quiz-auto 升级要给个提醒）────────────────
 * 启动时算一次 server.mjs 指纹，和 run/last-build.json 里记的比：
 *   变了 ⇒ 写 run/announce.json（一次性），下一条快路径回复开头带一行「服务已更新」；
 *   没变 ⇒ 不动。
 * 这样重启/换代码后，用户第一眼就知道「后台动过什么」，不用猜为什么行为变了。
 */
const BUILD_FILE = 'last-build.json';
const ANNOUNCE_FILE = 'announce.json';

export function buildRev(file) {
  try {
    const bytes = fs.readFileSync(file);
    return crypto.createHash('sha1').update(bytes).digest('hex').slice(0, 12);
  } catch {
    return '';
  }
}

export function recordBuild(runDir, { rev, mode, pid }) {
  try {
    fs.mkdirSync(runDir, { recursive: true });
    const file = path.join(runDir, BUILD_FILE);
    let previous = '';
    try {
      previous = JSON.parse(fs.readFileSync(file, 'utf8')).rev || '';
    } catch {
      previous = '';
    }
    const record = { rev, mode, pid, at: new Date().toISOString() };
    fs.writeFileSync(file, JSON.stringify(record, null, 2));
    if (previous !== '' && previous !== rev) {
      fs.writeFileSync(path.join(runDir, ANNOUNCE_FILE), JSON.stringify({ ...record, previous }, null, 2));
      return true;
    }
    return false;
  } catch {
    return false;
  }
}

/** 取出并清掉一次性提醒（返回一行文字，或空串）。 */
export function takeAnnouncement(runDir, { rev, mode }) {
  const file = path.join(runDir, ANNOUNCE_FILE);
  let record = null;
  try {
    record = JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch {
    return '';
  }
  try {
    fs.unlinkSync(file);
  } catch {
    /* 删不掉也不影响：下次会再试 */
  }
  const modeText = mode === 'learn' ? '卡片模式' : '纯文本模式';
  return '🔧 题库服务已更新（rev ' + String(rev || '') + ' · ' + modeText + '）——行为可能和刚才不同。';
}

export function findRepoRoot(startDir) {
  let dir = path.resolve(startDir);
  for (;;) {
    if (fs.existsSync(path.join(dir, ENGINE_REL))) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

/** 仓库根解析：显式参数 > QUIZ_PROXY_REPO > server.mjs 自身位置 > 当前目录。 */
export function resolveRepoRoot(explicit) {
  const candidates = [];
  if (explicit) candidates.push(explicit);
  if (process.env.QUIZ_PROXY_REPO) candidates.push(process.env.QUIZ_PROXY_REPO);
  candidates.push(path.resolve(__dirname, '..', '..')); // .cache/quiz-proxy -> 仓库根
  candidates.push(process.cwd());
  for (const candidate of candidates) {
    const root = findRepoRoot(candidate);
    if (root) return root;
  }
  throw new Error(
    '找不到仓库根（要求存在 ' + ENGINE_REL + '）；可用环境变量 QUIZ_PROXY_REPO 指定',
  );
}

/** OpenAI content（string 或 parts 数组）→ 纯文本。 */
export function contentToText(content) {
  if (typeof content === 'string') return content;
  if (Array.isArray(content)) {
    return content
      .map((part) => {
        if (typeof part === 'string') return part;
        if (part && typeof part === 'object') {
          if (typeof part.text === 'string') return part.text;
          if (part.type === 'text' && typeof part.content === 'string') return part.content;
        }
        return '';
      })
      .join('');
  }
  if (content && typeof content === 'object' && typeof content.text === 'string') return content.text;
  return '';
}

/**
 * DSH 会把运行时上下文（时间、时区、elapsed）作为**额外的 user 消息**追加在真实输入之后，
 * 形如 "Time sampled while preparing turn …" / "Browser time zone for this request …" /
 * "Current runtime context: …". 快路径必须跳过这些，取最后一条**真实**用户消息。
 */
export function isRuntimeContextText(text) {
  if (typeof text !== 'string') return false;
  const head = text.trimStart().slice(0, 64);
  return /^(?:Time sampled|Browser time zone|Elapsed since|Current runtime context|Current working directory|Current DSH file policy|Approval policy|You are the Team Lead|You are an AI agent|\[model changed|Background subagent|Team message from|\[system\]|\[reminder\]|<system-reminder|<user_instructions|<runtime|<environment|A skill is a reusable set|Agent Teams is available|Current runtime context)/i.test(
    head,
  );
}

export function extractConversation(messages) {
  const systemParts = [];
  const userParts = [];
  const substantive = [];
  for (const message of messages) {
    if (!message || typeof message !== 'object') continue;
    if (message.role === 'system') systemParts.push(contentToText(message.content));
    else if (message.role === 'user') {
      const text = contentToText(message.content);
      userParts.push(text);
      if (!isRuntimeContextText(text)) substantive.push(text);
    }
  }
  return {
    systemText: systemParts.join('\n'),
    firstUserText: substantive.length ? substantive[0] : (userParts.length ? userParts[0] : ''),
    lastUserText: substantive.length ? substantive[substantive.length - 1] : '',
    userCount: userParts.length,
    charCount: systemParts.join('\n').length + userParts.reduce((sum, text) => sum + text.length, 0),
  };
}

/** fingerprint = sha1(system 内容 + "\n" + 第一条 user 内容)。 */
export function fingerprintOf(messages) {
  const { systemText, firstUserText } = extractConversation(messages);
  return crypto.createHash('sha1').update(systemText + '\n' + firstUserText).digest('hex');
}


/** 默认单学习流（跨轮共享）：system prompt 每轮都会变，fingerprint 会漂，不能当会话 ID 用。 */
export function stateKeyOf(app, fingerprint) {
  return app && app.perConversation === true ? fingerprint : 'default';
}

export function stateFileOf(app, fingerprint) {
  return path.join(app.stateDir, stateKeyOf(app, fingerprint) + '.json');
}

// ───────────────────────── 题目句柄（I2：qid + nonce） ─────────────────────────
/**
 * V5 正确性不变量 I2：题目句柄 nonce = **题面内容 hash**（qid + plain/prompt + 选项）。
 * 稳定可复算：只要题库没变，任何时候从 engine 的 question 对象算出的 nonce 都一样。
 */
export function questionNonce(question) {
  const q = question || {};
  const options = Array.isArray(q.options) ? q.options.map((item) => String(item)) : [];
  const material = [
    String(q.id || ''),
    String(q.plain || ''),
    String(q.prompt || ''),
    ...options,
  ].join('\u0001');
  return crypto.createHash('sha1').update(material).digest('hex').slice(0, 16);
}

/**
 * 记录「这个 state key 最近一次发给屏幕上的题」；返回句柄。
 * issueId 单调递增：响应层用它判断「这一轮到底有没有发新题」，决定要不要带句柄响应头。
 */
export function rememberQuestionHandle(app, stateKey, question) {
  const qid = String((question && question.id) || '');
  if (qid === '') return null;
  const previous = app.questionHandles.get(stateKey);
  const handle = {
    qid,
    nonce: questionNonce(question),
    issueId: (previous && Number(previous.issueId) ? Number(previous.issueId) : 0) + 1,
    at: Date.now(),
  };
  app.questionHandles.set(stateKey, handle);
  trimOldest(app.questionHandles, HANDLE_LIMIT);
  return handle;
}

/** 句柄 → 响应头（对用户不可见，DSH 忽略；测试/排障可断言）。 */
export function questionHandleHeaders(handle) {
  if (!handle || !handle.qid || !handle.nonce) return {};
  return { 'x-quiz-qid': String(handle.qid), 'x-quiz-nonce': String(handle.nonce) };
}

/** 判分请求可显式携带句柄（响应头回显即可；headers 优先，其次 body）。 */
export function extractRequestHandle(req, body) {
  const headers = (req && req.headers) || {};
  const bodyHandle =
    body && body.quiz_handle && typeof body.quiz_handle === 'object' ? body.quiz_handle : {};
  const first = (...values) => {
    for (const value of values) {
      if (typeof value === 'string' && value.trim() !== '') return value.trim();
    }
    return '';
  };
  const qid = first(headers['x-quiz-qid'], body && body.quiz_qid, bodyHandle.qid);
  const nonce = first(headers['x-quiz-nonce'], body && body.quiz_nonce, bodyHandle.nonce);
  return qid !== '' && nonce !== '' ? { qid, nonce, explicit: true } : null;
}

/** 引擎当前 issued 题（不推进）：{ question, position } | null。 */
async function currentIssuedQuestion(app, stateFile) {
  let step = null;
  try {
    step = await engineNext(app, stateFile, false);
  } catch {
    return null;
  }
  if (!step || step.kind !== 'question' || !step.question) return null;
  /* resume = 引擎恢复「已发出但未答」的题；其它 reason 说明引擎顺手换了题，
   * 不算「当前 issued」（调用方会按 stale/无题处理，绝不误判）。 */
  if (!(step.decision && step.decision.reason === 'resume')) return null;
  return { question: step.question, position: step.position || null };
}

/**
 * I2：判分前把「屏幕上的题」和「engine 当前 issued 题」对上。
 * - 显式句柄的 qid / nonce 对不上 ⇒ stale；
 * - 无句柄（代理重启 / 从未经代理出题）⇒ 用引擎当前题补记句柄（宽松，保留旧行为）；
 * - 返回 { ok, reason?, question, position, qid, nonce, handle }。
 */
async function resolveGradeTarget(app, stateKey, stateFile, requestHandle) {
  const state = readStateJson(stateFile);
  if (!state || !state.issued_qid) {
    return { ok: false, reason: 'no-question', question: null, position: null };
  }
  const issued = String(state.issued_qid);
  const current = await currentIssuedQuestion(app, stateFile);
  if (!current || String((current.question || {}).id || '') !== issued) {
    return {
      ok: false,
      reason: 'stale',
      question: current ? current.question : null,
      position: current ? current.position : null,
      issued,
    };
  }
  const nonce = questionNonce(current.question);
  if (requestHandle && requestHandle.qid !== issued) {
    return { ok: false, reason: 'stale', question: current.question, position: current.position, issued };
  }
  if (requestHandle && requestHandle.nonce !== nonce) {
    return { ok: false, reason: 'stale', question: current.question, position: current.position, issued };
  }
  const known = app.questionHandles.get(stateKey);
  if (known && known.qid === issued && known.nonce !== nonce) {
    /* 代理记录里「屏幕上那道题」的 nonce 与 engine 当前题不一致 ⇒ 状态漂移，
     * 判分一律不记（这正是「用户看到 A 题、判分记到 B 题」的事故防线）。 */
    return { ok: false, reason: 'stale', question: current.question, position: current.position, issued };
  }
  const handle = rememberQuestionHandle(app, stateKey, current.question);
  return { ok: true, question: current.question, position: current.position, qid: issued, nonce, handle };
}

/** 是否是选择题（verbatim 引擎题面有选项）。 */
function isChoiceQuestion(question) {
  return Boolean(question) && (question.kind || 'choice') === 'choice' && Array.isArray(question.options) && question.options.length >= 2;
}

/** I3：同一 (qid, nonce) 的显式句柄判分幂等缓存。 */
function gradeCacheKey(stateKey, qid, nonce) {
  return stateKey + '\u0001' + String(qid) + '\u0001' + String(nonce);
}

function readGradeCache(app, stateKey, qid, nonce) {
  const cached = app.gradeResults.get(gradeCacheKey(stateKey, qid, nonce));
  return cached || null;
}

function writeGradeCache(app, stateKey, qid, nonce, reply) {
  app.gradeResults.set(gradeCacheKey(stateKey, qid, nonce), reply);
  trimOldest(app.gradeResults, GRADE_CACHE_LIMIT);
}

/**
 * I2：句柄校验失败时的回退回复 —— 不判分、不 advance、不写历史，
 * 只把 engine **当前**题重新发一遍（text 模式给文本，卡片模式给卡片）。
 */
async function buildStaleQuestionReply(app, stateKey, stateFile, resolved, cardMode) {
  const question = resolved.question || null;
  let handle = null;
  let toolCall = null;
  const lines = [STALE_QUESTION_TEXT];
  if (question) {
    handle = rememberQuestionHandle(app, stateKey, question);
    if (cardMode && isChoiceQuestion(question)) {
      toolCall =
        app.cardsMode === 'learn'
          ? buildLearnQuestionCard(question, resolved.position)
          : buildQuestionCard(app, stateKey, stateFile, question);
    } else {
      lines.push(formatQuestion(question, '', resolved.position));
    }
  } else {
    lines.push('回「开始做题」重新发一题吧。');
  }
  return { kind: 'stale', text: lines.join('\n\n'), toolCall, handle, stale: true };
}

/**
 * I4：「模型不得产出题目」的保守判定。
 * 只有「至少两个 A./B./C./D. 选项行」+「有≥8 字的非选项文字（题干）」才判为题目。
 * 宁可少拦：普通解释里出现 A. / B. 两行列表也不会拦（prose 不够长/行首不像选项则不触发）。
 */
export function looksLikeModelQuestion(text) {
  const raw = typeof text === 'string' ? text : '';
  if (raw.trim() === '') return false;
  const optionLetters = new Set();
  const prose = [];
  for (const line of raw.split(/\r?\n/)) {
    const trimmed = line.trim().replace(/^\*\*/, '').replace(/\*\*$/, '').trim();
    const match = trimmed.match(/^([A-D])\s*[.、．)）:：]\s*(\S.*)$/);
    if (match) optionLetters.add(match[1]);
    else if (trimmed !== '') prose.push(trimmed);
  }
  if (optionLetters.size < 2) return false;
  return prose.join(' ').trim().length >= 8;
}

/** 从上游非流式 JSON / SSE 原文里抽 assistant 文本（guard 判定 + 日志用）。 */
function upstreamTextOfJson(raw) {
  try {
    const parsed = JSON.parse(raw);
    return contentToText(parsed && parsed.choices && parsed.choices[0] && parsed.choices[0].message && parsed.choices[0].message.content);
  } catch {
    return '';
  }
}

function upstreamTextOfSse(raw) {
  let text = '';
  for (const line of String(raw || '').split('\n')) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('data:')) continue;
    const payload = trimmed.slice(5).trim();
    if (payload === '' || payload === '[DONE]') continue;
    let chunk = null;
    try {
      chunk = JSON.parse(payload);
    } catch {
      continue;
    }
    const choice = chunk && chunk.choices && chunk.choices[0];
    if (choice && choice.delta && typeof choice.delta.content === 'string') text += choice.delta.content;
    if (choice && choice.message && typeof choice.message.content === 'string') text += choice.message.content;
  }
  return text;
}

/** I4：把上游「自作题」丢掉的回复：改为 engine 当前题（text / 卡片两种形态）。 */
function buildGuardedQuestionReply(app, guard) {
  const handle = rememberQuestionHandle(app, guard.stateKey, guard.question);
  let toolCall = null;
  let text = '这道题以屏幕上的引擎题为准。';
  if (guard.cardMode && isChoiceQuestion(guard.question)) {
    toolCall =
      app.cardsMode === 'learn'
        ? buildLearnQuestionCard(guard.question, guard.position)
        : buildQuestionCard(app, guard.stateKey, guard.stateFile, guard.question);
  } else {
    text = '（刚才那段不是引擎题目，已拦下，以这道题为准）\n\n' + formatQuestion(guard.question, '', guard.position);
  }
  return { kind: 'guard', text, toolCall, handle };
}

// ───────────────────────── 卡片模式工具 ─────────────────────────
/** 请求 tools 里是否声明了 ask_user_question（DSH 会带上全部工具）。 */
export function hasAskUserQuestionTool(tools) {
  if (!Array.isArray(tools)) return false;
  return tools.some((tool) => {
    if (!tool || typeof tool !== 'object') return false;
    const name = tool.function && typeof tool.function === 'object' ? tool.function.name : tool.name;
    return name === ASK_TOOL_NAME;
  });
}

/** 请求 tools 里是否声明了 learn_card（DSH 装了 @local/dsh-learn-card 时会有）。 */
export function hasLearnCardTool(tools) {
  if (!Array.isArray(tools)) return false;
  return tools.some((tool) => {
    if (!tool || typeof tool !== 'object') return false;
    const name = tool.function && typeof tool.function === 'object' ? tool.function.name : tool.name;
    return name === LEARN_CARD_TOOL_NAME;
  });
}

/** tool_choice 明确禁止/指定了别的工具时，不主动发卡片。 */
function cardCallsAllowed(body, toolName) {
  const choice = body && body.tool_choice;
  if (choice === 'none') return false;
  if (choice && typeof choice === 'object') {
    const name = choice.function && choice.function.name;
    if (typeof name === 'string' && name !== '' && name !== toolName) return false;
  }
  return true;
}

/**
 * 卡片模式开关：QUIZ_PROXY_CARDS=learn → learn_card 真卡片；=1/ask → 旧的
 * ask_user_question 点选卡片（legacy）；其它/默认 → off（纯文本快路径）。
 * options.cardsMode 可在测试里显式指定；options.cards === false 强制 off。
 */
export function resolveCardsMode(options = {}) {
  if (options && options.cards === false) return 'off';
  const explicit = typeof (options && options.cardsMode) === 'string' ? options.cardsMode.trim().toLowerCase() : '';
  const raw = explicit !== '' ? explicit : String(process.env.QUIZ_PROXY_CARDS || '').trim().toLowerCase();
  if (raw === 'learn' || raw === 'learn_card' || raw === 'learn-card') return 'learn';
  if (raw === '1' || raw === 'ask' || raw === 'ask_user_question' || raw === 'on' || raw === 'true') return 'ask';
  return 'off';
}

/**
 * 最后一条消息是不是 learn_card 的工具执行回显。
 * **按 tool_call_id 匹配前面 assistant 的 tool_calls**（不解析 content）：
 * Host 的 tool result 文本只是 render() 的一行说明，不是卡片 JSON。
 */
/** 跳过尾部的 runtime-context 用户消息（DSH 会在工具结果后面再插 <system-reminder>）。 */
export function lastNonRuntimeMessage(messages) {
  if (!Array.isArray(messages)) return null;
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (!message || typeof message !== 'object') continue;
    if (message.role === 'user' && isRuntimeContextText(contentToText(message.content))) continue;
    return message;
  }
  return null;
}

/** 尾部（跳过 runtime-context）是否是一条 tool 结果。 */
export function endsWithToolResult(messages) {
  const message = lastNonRuntimeMessage(messages);
  return Boolean(message && message.role === 'tool');
}

export function isLearnCardToolResult(messages) {
  if (!Array.isArray(messages) || messages.length === 0) return false;
  const last = lastNonRuntimeMessage(messages);
  if (!last || last.role !== 'tool') return false;
  const id = last.tool_call_id || last.toolCallId;
  if (typeof id !== 'string' || id === '') return false;
  return messages.some(
    (message) =>
      message &&
      message.role === 'assistant' &&
      Array.isArray(message.tool_calls) &&
      message.tool_calls.some((call) => {
        if (!call || call.id !== id) return false;
        const name =
          call.function && typeof call.function === 'object' ? call.function.name : call.name;
        return name === LEARN_CARD_TOOL_NAME;
      }),
  );
}

export function isQuizModel(model) {
  return !model || model === QUIZ_MODEL;
}

/** 最后一条消息若是「我们发的卡片」的回答，返回解析后的答案；否则 null。 */
export function lastQuizCardAnswer(messages) {
  if (!Array.isArray(messages) || messages.length === 0) return null;
  const message = messages[messages.length - 1];
  if (!message || message.role !== 'tool') return null;
  let parsed = null;
  try {
    parsed = JSON.parse(contentToText(message.content));
  } catch {
    return null;
  }
  const answers = parsed && Array.isArray(parsed.answers) ? parsed.answers : [];
  const answer = answers.find(
    (item) => item && typeof item.id === 'string' && item.id.startsWith(CARD_ID_PREFIX),
  );
  if (!answer) return null;
  return {
    toolCallId:
      typeof message.tool_call_id === 'string' && message.tool_call_id !== ''
        ? message.tool_call_id
        : answer.id,
    cardId: answer.id,
    selected: Array.isArray(answer.selected)
      ? answer.selected.filter((label) => typeof label === 'string' && label !== '')
      : [],
    custom: typeof answer.custom === 'string' ? answer.custom : '',
  };
}

function readStateJson(stateFile) {
  try {
    return JSON.parse(fs.readFileSync(stateFile, 'utf8'));
  } catch {
    return null;
  }
}

function trimOldest(map, limit) {
  if (!(map instanceof Map) || map.size <= limit) return;
  let drop = map.size - limit;
  for (const key of map.keys()) {
    if (drop <= 0) break;
    map.delete(key);
    drop -= 1;
  }
}

function makeCardHeader(question) {
  const text = [question && question.chapter, question && question.unit].filter(Boolean).join(' · ');
  return text.length > CARD_HEADER_MAX_CHARS ? text.slice(0, CARD_HEADER_MAX_CHARS) : text;
}

/** 卡片序号：state.flow.served（引擎每发一步 +1）派生的单调序号，保证同流内不重复。 */
function nextCardSeq(app, stateKey, stateFile) {
  const state = readStateJson(stateFile) || {};
  const served = Number(state.flow && state.flow.served) || 0;
  let used = app.cardSeqs.get(stateKey);
  if (!used) {
    used = new Set();
    app.cardSeqs.set(stateKey, used);
  }
  let seq = Math.max(1, served);
  while (used.has(seq)) seq += 1;
  used.add(seq);
  if (used.size > 500) {
    const keep = [...used].sort((a, b) => a - b).slice(-200);
    used.clear();
    for (const value of keep) used.add(value);
  }
  return seq;
}

/** 把一道选择题做成 ask_user_question 的 tool_call（DSH 原生卡片）。 */
export function buildQuestionCard(app, stateKey, stateFile, question) {
  const seq = nextCardSeq(app, stateKey, stateFile);
  const cardId = CARD_ID_PREFIX + seq;
  const options = (Array.isArray(question.options) ? question.options : []).filter(
    (item) => typeof item === 'string',
  );
  const args = {
    questions: [
      {
        header: makeCardHeader(question),
        id: cardId,
        question: String(question.prompt || ''),
        options: options.slice(0, 4).map((label) => ({ label })),
      },
    ],
  };
  const toolCall = {
    id: 'call_' + crypto.randomUUID().replace(/-/g, '').slice(0, 20),
    type: 'function',
    function: { name: ASK_TOOL_NAME, arguments: JSON.stringify(args) },
  };
  app.pendingCards.set(toolCall.id, {
    stateKey,
    cardId,
    qid: String(question.id || ''),
    options: options.slice(0, 4),
  });
  trimOldest(app.pendingCards, CARD_PENDING_LIMIT);
  return toolCall;
}

/**
 * learn_card 的通用调用：{ id, type, function: { name: 'learn_card',
 * arguments: JSON.stringify({ kind, payload }) } }（payload 原样透传给客户端渲染）。
 */
export function buildLearnCardToolCall(kind, payload) {
  return {
    id: 'call_' + crypto.randomUUID().replace(/-/g, '').slice(0, 20),
    type: 'function',
    function: {
      name: LEARN_CARD_TOOL_NAME,
      arguments: JSON.stringify({ kind, payload: payload && typeof payload === 'object' ? payload : {} }),
    },
  };
}

/**
 * 选择题 → learn_card(kind='question')。
 * payload 字段按 card-dev 的 README §Exact proxy change spec：
 * {chapter, unit, unitNo, unitTotal, questionNo, questionTotal, stem, plain, options:[{key,text}]}。
 */
export function buildLearnQuestionCard(question, position = null) {
  const q = question || {};
  const pos = position && typeof position === 'object' ? position : {};
  const options = (Array.isArray(q.options) ? q.options : []).filter(
    (item) => typeof item === 'string',
  );
  const payload = {
    chapter: q.chapter || pos.chapter || '',
    unit: q.unit || pos.unit || '',
    unitNo: Number(pos.unitIndex) || undefined,
    unitTotal: Number(pos.unitTotal) || undefined,
    questionNo: Number(pos.questionIndex) || undefined,
    questionTotal: Number(pos.questionTotal) || undefined,
    stem: String(q.prompt || ''),
    plain: typeof q.plain === 'string' && q.plain !== '' ? q.plain : undefined,
    options: options.slice(0, 4).map((text, index) => ({ key: 'ABCD'[index], text })),
  };
  const questionKind = String(q.kind || 'choice');
  if (questionKind !== 'choice') payload.questionKind = questionKind;
  for (const key of Object.keys(payload)) {
    if (payload[key] === undefined) delete payload[key];
  }
  return buildLearnCardToolCall('question', payload);
}

function normalizeOptionLabel(value) {
  return String(value || '')
    .replace(/[`*_~]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

/** 把卡片回来的 selected label / custom 文本解析成 1-based 选项序号；0 = 认不出。 */
export function resolveCardChoice(options, selected, custom) {
  const labels = Array.isArray(options) ? options.map((item) => String(item)) : [];
  for (const candidate of Array.isArray(selected) ? selected : []) {
    if (typeof candidate !== 'string' || candidate === '') continue;
    const exact = labels.indexOf(candidate);
    if (exact >= 0) return exact + 1;
    const normalized = normalizeOptionLabel(candidate);
    if (normalized !== '') {
      const loose = labels.findIndex((label) => normalizeOptionLabel(label) === normalized);
      if (loose >= 0) return loose + 1;
    }
    const byLetter = parseChoiceLetter(candidate);
    if (byLetter >= 1) return byLetter;
  }
  if (typeof custom === 'string' && custom.trim() !== '') {
    const letter = parseChoiceLetter(custom);
    if (letter >= 1) return letter;
  }
  return 0;
}

function isCardQuestion(question) {
  return (
    Boolean(question) &&
    (question.kind || 'choice') === 'choice' &&
    Array.isArray(question.options) &&
    question.options.length >= 2
  );
}

function rememberCardReply(app, toolCallId, reply) {
  app.cardReplies.set(toolCallId, reply);
  trimOldest(app.cardReplies, CARD_REPLY_CACHE_LIMIT);
}

function epochSeconds() {
  return Math.floor(Date.now() / 1000);
}

function completionId() {
  return 'chatcmpl-' + crypto.randomUUID().replace(/-/g, '').slice(0, 24);
}

function estimateTokens(chars) {
  if (!chars) return 0;
  return Math.max(1, Math.ceil(chars / 3));
}

function errorPayload(status, message, type = 'invalid_request_error', code) {
  return {
    error: {
      message: String(message),
      type,
      code: code || `${type}_${status}`,
      status,
    },
  };
}

function sendJson(res, status, payload, extraHeaders = {}) {
  const body = Buffer.from(JSON.stringify(payload), 'utf8');
  res.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'content-length': String(body.length),
    ...extraHeaders,
  });
  res.end(body);
}

function readBody(req, maxBytes) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    let failed = false;
    req.on('data', (chunk) => {
      if (failed) return;
      size += chunk.length;
      if (size > maxBytes) {
        failed = true;
        reject(Object.assign(new Error('request body too large'), { code: 'too_large' }));
        return;
      }
      chunks.push(chunk);
    });
    req.on('end', () => {
      if (!failed) resolve(Buffer.concat(chunks));
    });
    req.on('error', (err) => {
      if (!failed) reject(err);
    });
  });
}

// ───────────────────────── 本地引擎（快路径） ─────────────────────────
const engineEnv = (() => {
  const env = { ...process.env };
  delete env[API_KEY_NAME]; // 引擎不需要 key，不给它
  env.PYTHONDONTWRITEBYTECODE = '1';
  env.PYTHONIOENCODING = 'utf-8';
  return env;
})();

async function runEngine(app, args) {
  const { stdout } = await execFileAsync('python3', [app.enginePath, ...args], {
    cwd: app.repoRoot,
    timeout: app.engineTimeoutMs,
    maxBuffer: 32 * 1024 * 1024,
    encoding: 'utf8',
    env: engineEnv,
  });
  let parsed;
  try {
    parsed = JSON.parse(String(stdout).trim());
  } catch {
    throw new Error('engine.py 输出不是 JSON：' + String(stdout).slice(0, 200));
  }
  if (!parsed || parsed.ok !== true) {
    throw new Error('engine.py 返回失败：' + String((parsed && parsed.error) || 'unknown error'));
  }
  return parsed;
}

async function engineNext(app, stateFile, advance, noLesson = false) {
  const args = ['next', '--state', stateFile];
  if (advance) args.push('--advance');
  /* 「下一题」= 我就要题目：跳过教学环节卡与 primer 卡，否则连出两张课/卡很烦。 */
  if (noLesson) args.push('--no-lesson');
  return runEngine(app, args);
}

function pickEncouragement() {
  return ENCOURAGEMENTS[Math.floor(Math.random() * ENCOURAGEMENTS.length)];
}

function questionMeta(question) {
  const parts = [question && question.chapter, question && question.unit].filter(Boolean);
  return parts.length ? '（' + parts.join(' · ') + '）' : '';
}

/** 「之前 / 现在 / 之后」一行：让每道题在课程里有明确位置。 */
/**
 * 把题库里的出处串（`lib/Order.sokonanoda:57/61`、`units/I.1/unit60-x.sokonanoda:80`）
 * 变成 DSH 可点的文件链接：`[courses/set-theory/...#L57](courses/set-theory/...#L57)`。
 * 路径相对会话工作目录（仓库根），DSH 的聊天会把这种链接渲染成可打开的本地文件。
 */
/**
 * 数学公式自动换行：KaTeX 的 display 模式不会自动折行，长公式在手机上会横向溢出。
 * 做法：按顶层逻辑分隔符（\iff / \to / \qquad …）切段，套进 \begin{aligned}…\\…\end{aligned}，
 * 每行不超过 maxChars 个字符；短公式原样返回。
 */
export function wrapMath(latex, maxChars = 44) {
  const raw = typeof latex === 'string' ? latex.trim() : '';
  if (raw === '') return '';
  if (raw.length <= maxChars) return raw;
  /* 顶层断点：只在花括号深度 0、且后面不是字母处断开（避免切坏 \to / \top、\iff / \iffx）。 */
  const tokens = ['\\iff', '\\Leftrightarrow', '\\implies', '\\to', '\\rightarrow', '\\qquad', '\\quad'];
  const pieces = [];
  let depth = 0;
  let start = 0;
  for (let index = 0; index < raw.length; index += 1) {
    const ch = raw[index];
    if (ch === '{') depth += 1;
    else if (ch === '}') depth -= 1;
    else if (ch === '\\' && depth === 0) {
      let matched = '';
      for (const token of tokens) {
        if (raw.startsWith(token, index) && !/[A-Za-z]/.test(raw[index + token.length] || '')) {
          matched = token;
          break;
        }
      }
      if (matched !== '' && index > start) {
        pieces.push(raw.slice(start, index));
        start = index;
        index += matched.length - 1;
      }
    }
  }
  pieces.push(raw.slice(start));
  /* 贪心打包：每行不超过 maxChars。 */
  const lines = [];
  let buffer = '';
  for (const piece of pieces) {
    const candidate = buffer === '' ? piece : buffer + piece;
    if (candidate.length > maxChars && buffer !== '') {
      lines.push(buffer.trim());
      buffer = piece;
    } else {
      buffer = candidate;
    }
  }
  if (buffer.trim() !== '') lines.push(buffer.trim());
  if (lines.length <= 1) return raw;
  return ['\\begin{aligned}', lines.join(' \\\\\n'), '\\end{aligned}'].join('\n');
}

export function formatSourceLink(raw) {
  const text = typeof raw === 'string' ? raw.trim() : '';
  if (text === '') return '';
  const match = text.match(/^([A-Za-z0-9_.\/-]+\.sokonanoda)(?::([0-9]+(?:\/[0-9]+)*))?/);
  if (!match) return text;
  const relative = 'courses/set-theory/' + match[1].replace(/^\.\//, '');
  const lines = typeof match[2] === 'string' && match[2] !== '' ? match[2].split('/') : [];
  const first = lines.length > 0 ? Number(lines[0]) : NaN;
  const anchor = Number.isFinite(first) && first > 0
    ? (lines.length > 1 ? `#L${lines[0]}-L${lines[lines.length - 1]}` : `#L${lines[0]}`)
    : '';
  const label = match[1] + (lines.length > 0 ? ':' + match[2] : '');
  return `[${label}](${relative}${anchor})`;
}

export function formatPosition(position) {
  const p = position && typeof position === 'object' ? position : null;
  if (!p || typeof p.chapter !== 'string' || p.chapter === '') return '';
  const unitIndex = Number(p.unitIndex) || 0;
  const unitTotal = Number(p.unitTotal) || 0;
  const qIndex = Number(p.questionIndex) || 0;
  const qTotal = Number(p.questionTotal) || 0;
  const parts = [];
  if (unitIndex > 0 && unitTotal > 0) parts.push(`单元 ${unitIndex}/${unitTotal}`);
  if (qIndex > 0 && qTotal > 0) parts.push(`本单元第 ${qIndex}/${qTotal} 题`);
  const head = parts.length ? '📍 ' + parts.join(' · ') : '📍';
  const before = p.prevUnit ? `上一站：${p.prevUnit}` : '（这是第一站）';
  const after = p.nextUnit ? `下一站：${p.nextUnit}` : '（最后一站）';
  return `${head}｜${before}｜${after}`;
}

/** 「教学环节」渲染：一个单元的微课（目标 / 直觉 / 关键卡 / 范围）。 */
export function formatLesson(lesson) {
  const item = lesson && typeof lesson === 'object' ? lesson : null;
  if (!item) return '📚 本单元简介暂时不可用。';
  const lines = [];
  const unit = typeof item.unit === 'string' ? item.unit : '';
  const chapter = typeof item.chapter === 'string' ? item.chapter : '';
  lines.push(`📚 教学环节 · ${[chapter, unit].filter(Boolean).join(' ')}`);
  if (typeof item.goal === 'string' && item.goal !== '') lines.push('', item.goal);
  const points = Array.isArray(item.knowledgePoints) ? item.knowledgePoints.slice(0, 6) : [];
  if (points.length > 0) {
    lines.push('', '**这一节的知识点**');
    for (const point of points) {
      const title = point && typeof point.title === 'string' ? point.title : '';
      if (title !== '') lines.push('- ' + title);
    }
  }
  if (typeof item.intuition === 'string' && item.intuition !== '') {
    lines.push('', '**一句话直觉**', item.intuition);
  }
  const cards = Array.isArray(item.cards) ? item.cards.slice(0, 2) : [];
  for (const card of cards) {
    const title = card && typeof card.title === 'string' ? card.title : '';
    const oneLiner = card && typeof card.oneLiner === 'string' ? card.oneLiner : '';
    if (title !== '' || oneLiner !== '') lines.push('', `**${title}**`, oneLiner);
  }
  if (Number(item.questionTotal) > 0) {
    lines.push('', `本单元共 ${Number(item.questionTotal)} 道题；上完这几分钟就开始练。`);
  }
  lines.push('', '看完回「懂了」，或者直接回「开始」/「下一题」，我就出第一道题。');
  return lines.join('\n');
}

/**
 * 探针行：只留三件事 —— 知识点（公式用行内代码）、本知识点进度、认知步。
 * 旧版把「L3 · 加压 · 第 2 题 · 第 4/5 题 · 边界」堆在一起，读不懂也没必要。
 */
export function formatProbeLine(probe) {
  if (!probe || typeof probe !== 'object') return '';
  const title = typeof probe.kpTitle === 'string' ? probe.kpTitle : '';
  const continuity = probe.continuity && typeof probe.continuity === 'object' ? probe.continuity : null;
  const index = continuity !== null ? Number(continuity.kpIndex) : NaN;
  const total = continuity !== null ? Number(continuity.kpTotal) : NaN;
  const cognitive = continuity !== null && typeof continuity.cognitiveLabel === 'string' ? continuity.cognitiveLabel : '';
  const parts = [];
  if (title !== '') parts.push(formatKpTitle(title));
  if (Number.isFinite(index) && Number.isFinite(total) && total > 0) {
    parts.push('第 ' + index + '/' + total + ' 题');
  }
  if (cognitive !== '') parts.push(cognitive);
  if (parts.length === 0) return '';
  return '🎯 ' + parts.join(' · ');
}

/** `差集（A\B = A ∧ ¬B）` → ``差集 `A \ B = A ∧ ¬B` ``：公式放进行内代码。 */
export function formatKpTitle(title) {
  const text = typeof title === 'string' ? title.trim() : '';
  if (text === '') return '';
  const matched = text.match(/^(.+?)（(.+)）$/);
  if (matched !== null) {
    const head = matched[1].trim();
    const formula = matched[2].trim();
    if (formula !== '') return head + ' `' + formula + '`';
  }
  return text;
}

export function formatQuestion(question, heading = '📝 题目', position = null, refs = null) {
  /* 极简卡片：题干（完整一句话）+ 可选 Unicode 公式 + 选项 + 一行作答提示。
   * 不显示标题头、位置行、库声明区（用户要求清爽；位置/声明改用「地图」「不懂」按需查）。 */
  const q = question || {};
  const lines = [];
  const plain = typeof q.plain === 'string' && q.plain.trim() !== '' ? q.plain.trim() : '';
  lines.push(`**${plain !== '' ? plain : String(q.prompt || '')}**`);

  const mathOn = String(process.env.QUIZ_PROXY_MATH || '0') === '1';
  if (mathOn && typeof q.math === 'string' && q.math.trim() !== '') {
    lines.push('', '```math', wrapMath(q.math), '```');
  } else if (typeof q.formula === 'string' && q.formula.trim() !== '') {
    lines.push('', '▸ ' + q.formula.trim());
  }

  const options = Array.isArray(q.options) ? q.options.filter((item) => typeof item === 'string') : [];
  const kind = q.kind || 'choice';
  if (kind === 'choice' && options.length) {
    options.slice(0, 4).forEach((option, index) => {
      lines.push('', `**${'ABCD'[index]}.** ${option}`);
    });
    /* 不显示「回复字母作答」提示（用户要求去掉；题型本身已足够明显）。 */
  } else {
    const label =
      kind === 'short' ? '简答' : kind === 'explain' ? '解释' : kind === 'proof' ? '证明' : kind;
    lines.push('', `_这是一道${label}题：直接写出你的回答（会按要点即时判分）_`);
  }
  return lines.join('\n');
}

export function formatCard(card) {
  const value = card || {};
  const lines = [`📇 知识卡：${value.title || value.id || ''}`];
  if (value.oneLiner) lines.push(value.oneLiner);
  const definition = value.definition;
  if (definition && definition.code) {
    lines.push('', '```' + (definition.lang || '') + '\n' + definition.code + '\n```');
  }
  for (const section of (value.sections || []).slice(0, 3)) {
    if (!section || !section.heading) continue;
    lines.push('', `【${section.heading}】`);
    if (section.kind === 'paragraph' && section.text) lines.push(String(section.text));
    else if (section.kind === 'list' && Array.isArray(section.items)) {
      lines.push(...section.items.slice(0, 4).map((item) => '• ' + String(item)));
    } else if (section.kind === 'table' && Array.isArray(section.rows)) {
      if (Array.isArray(section.columns) && section.columns.length) {
        lines.push(' ｜ '.join(section.columns.map(String)));
      }
      lines.push(...section.rows.slice(0, 3).map((row) => (Array.isArray(row) ? row.join(' ｜ ') : String(row))));
    }
  }
  return lines.join('\n');
}

export function formatCodeExercise(exercise) {
  const value = exercise || {};
  const meta = [value.chapter, value.unit].filter(Boolean);
  return [
    `🧩 代码练习${meta.length ? '（' + meta.join(' · ') + '）' : ''}`,
    value.title || '',
    String(value.prompt || ''),
    '',
    '（这道题在编辑器里做；完成后告诉我，我会交给慢思考老师看）',
  ]
    .filter((line) => line !== '')
    .join('\n');
}

export function formatFeedback(answer) {
  const value = answer || {};
  const feedback = value.feedback || {};
  const kind = value.kind || 'choice';
  const correct = value.correct === true;
  const lines = [(correct ? '✅ 对了！' : '差一点。') + pickEncouragement()];
  if (kind === 'choice' && feedback.answerIndex != null) {
    const index = Number(feedback.answerIndex);
    const letter = index >= 1 && index <= 4 ? 'ABCD'[index - 1] : '?';
    lines.push(`正确答案：${letter}. ${feedback.answerText || ''}`.replace(/\s+$/, ''));
  }
  if (feedback.why) lines.push(`解析：${feedback.why}`);
  if (feedback.source) lines.push(`出处：${formatSourceLink(feedback.source)}`);
  return lines.join('\n');
}

/**
 * 连续取步骤直到拿到一道题：知识卡（primer）会先展示，再自动跟一道题。
 * 拿到题目后把 (qid, kind) 记进内存缓存，字母判卷时少起一次 python。
 */
async function nextBlocks(app, stateFile, fingerprint, { advance, heading, noLesson = false }) {
  const blocks = [];
  let step = await engineNext(app, stateFile, advance, noLesson);
  for (let guard = 0; guard < 6; guard++) {
    if (step.kind === 'lesson') {
      blocks.push(formatLesson(step.lesson));
      return blocks;
    }
    if (step.kind === 'question') {
      const question = step.question || {};
      app.issuedKinds.set(stateKeyOf(app, fingerprint), {
        qid: String(question.id || ''),
        kind: question.kind || 'choice',
      });
      rememberQuestionHandle(app, stateKeyOf(app, fingerprint), question); // I2：屏幕上的题 = qid+nonce
      const probeLine = formatProbeLine((step.decision || {}).probe);
      if (probeLine !== '') blocks.push(probeLine);
      blocks.push(formatQuestion(question, heading, step.position, step.lemmas));
      return blocks;
    }
    if (step.kind === 'card') {
      blocks.push(formatCard(step.card));
      step = await engineNext(app, stateFile, true);
      continue;
    }
    if (step.kind === 'code') {
      blocks.push(formatCodeExercise(step.exercise));
      return blocks;
    }
    throw new Error('engine.py 返回了未知步骤：' + String(step && step.kind));
  }
  throw new Error('engine.py 连续返回非题目步骤');
}

/**
 * 开放题（short / explain / proof）的**本地判分**：
 * 以前这类题落到慢路径，模型既要判分又要出下一题 ⇒ 它自己「编」题，
 * 于是「显示的题」和引擎记的 issued 题对不上（用户选 A 却被回「这是简答题」）。
 * 现在：文本直接交给引擎 rubric 判分，再由引擎推进下一题 —— 题目只从引擎来。
 */
function formatOpenFeedback(question, result) {
  const feedback = (result && result.feedback) || {};
  const correct = result && result.correct === true;
  const score = Number(result && result.score);
  const matched = Array.isArray(feedback.matched) ? feedback.matched : [];
  const missed = Array.isArray(feedback.missed) ? feedback.missed : [];
  const reference = typeof feedback.reference === 'string' ? feedback.reference.trim() : '';
  const lines = [];
  lines.push(correct ? '✅ 要点都答到了。' : '⚠️ 要点还不全，再看看。');
  if (Number.isFinite(score)) lines.push('得分：' + Math.round(score * 100) + '%（要点覆盖）');
  if (matched.length > 0) {
    lines.push('答到的要点：' + matched.map((item) => item.id || item.note || '').filter(Boolean).join('、'));
  }
  if (missed.length > 0) {
    lines.push('还差的要点：' + missed.map((item) => item.note || item.id || '').filter(Boolean).join('；'));
  }
  if (reference !== '') lines.push('参考：' + reference);
  const source = typeof question.source === 'string' ? question.source : '';
  if (source !== '') lines.push('出处：' + formatSourceLink(source));
  return lines.join('\n');
}

async function openAnswerFlow(app, fingerprint, stateFile, text) {
  let resume = null;
  try {
    resume = await engineNext(app, stateFile, false);
  } catch {
    return null;
  }
  if (!resume || resume.kind !== 'question') return null;
  const question = resume.question || {};
  const kind = question.kind || 'choice';
  if (kind === 'choice') return null;
  if (text.trim() === '') return null;
  rememberQuestionHandle(app, stateKeyOf(app, fingerprint), question); // I2：屏幕上那道开放题
  /* 先「试判」（dry-run，不记账）：
   *   要点不全 ⇒ 不推进、不记分，把「还差哪几个要点」告诉用户，让他再答一次；
   *   同题连错 2 次 ⇒ 给参考答法并照常判分推进（避免卡死）。
   * 这是用户 2026-11-23 的要求：「答案不全先指出来，给我重答的机会」。 */
  const attemptKey = stateKeyOf(app, fingerprint) + ':' + String(question.id);
  const attempts = (app.openAttempts ||= new Map());
  const tried = attempts.get(attemptKey) || 0;
  let trial = null;
  try {
    trial = await runEngine(app, ['answer', '--state', stateFile, '--qid', String(question.id),
      '--text', text, '--dry-run']);
  } catch {
    trial = null;
  }
  const retryable = trial !== null && trial.dryRun === true && trial.correct !== true && tried < 1;
  if (retryable) {
    attempts.set(attemptKey, tried + 1);
    const feedback = trial.feedback || {};
    const missed = Array.isArray(feedback.missed) ? feedback.missed : [];
    const lines = ['⚠️ 还没答全，先别急着看答案 —— 补上这几点再试一次：'];
    for (const item of missed) {
      const note = typeof item.note === 'string' && item.note !== '' ? item.note : item.id;
      if (note) lines.push('- ' + note);
    }
    lines.push('');
    lines.push(formatQuestion(question, '', null, question.lemmas));
    return { kind: 'answer', text: lines.join('\n') };
  }
  attempts.delete(attemptKey);

  let result = null;
  try {
    result = await runEngine(app, ['answer', '--state', stateFile, '--qid', String(question.id), '--text', text]);
  } catch {
    return null;
  }
  const blocks = [formatOpenFeedback(question, result)];
  blocks.push(...(await nextBlocks(app, stateFile, fingerprint, { advance: true, heading: '' })));
  return { kind: 'answer', text: blocks.join('\n\n') };
}

/**
 * 选择题「先给提示、再给一次机会」：
 * 第一次答错 ⇒ 不推进、不记分，回一句提示 + **同一道题**；
 * 第二次答错 ⇒ 照常揭示答案并推进。
 * 用户 2026-11-23：「不要太快下一道题，选择题也给我提示，再给我一次机会。」
 */
async function choiceRetryHint(app, fingerprint, stateFile, question, choice, resolved) {
  const attemptKey = stateKeyOf(app, fingerprint) + ':' + String(resolved.qid);
  const attempts = (app.choiceAttempts ||= new Map());
  const tried = attempts.get(attemptKey) || 0;
  if (tried >= 1) {
    attempts.delete(attemptKey);
    return null;
  }
  let trial = null;
  try {
    trial = await runEngine(app, ['answer', '--state', stateFile, '--qid', resolved.qid,
      '--choice', String(choice), '--dry-run']);
  } catch {
    return null;
  }
  if (trial === null || trial.dryRun !== true || trial.correct === true) {
    attempts.delete(attemptKey);
    return null;
  }
  attempts.set(attemptKey, tried + 1);
  const hint = typeof trial.hint === 'string' && trial.hint.trim() !== ''
    ? trial.hint.trim()
    : '回到定义：把题干里的符号展开成「逐个元素」的命题，再看哪个选项与它一字不差。';
  const lines = ['❌ 这个不对，再想一次 —— 先看提示，别急着换题：', '', '💡 ' + hint, '', '---', ''];
  lines.push(formatQuestion(question, '', null, question.lemmas));
  return { kind: 'answer', text: lines.join('\n') };
}

async function answerFlow(app, fingerprint, stateFile, letter, requestHandle = null) {
  if (!fs.existsSync(stateFile)) {
    return { kind: 'answer', text: '这轮还没有正在做的题目。想练题就回我「开始做题」～' };
  }
  const stateKey = stateKeyOf(app, fingerprint);
  let state = null;
  try {
    state = JSON.parse(fs.readFileSync(stateFile, 'utf8'));
  } catch {
    state = null;
  }
  if (!state || !state.issued_qid) {
    const blocks = await nextBlocks(app, stateFile, fingerprint, {
      advance: true,
      heading: '',
    });
    return { kind: 'answer', text: ['这轮还没有在做的题，先来一道：', ...blocks].join('\n\n') };
  }
  /* I3：显式句柄的重复判分幂等（同一 qid+nonce 只记一次历史）。
   * 必须在句柄校验之前查：重试请求里的句柄指向「上一题」，而 engine 已推进到下一题。 */
  if (requestHandle) {
    const cachedReply = readGradeCache(app, stateKey, requestHandle.qid, requestHandle.nonce);
    if (cachedReply) return cachedReply;
  }
  /* I2：判分前先把「屏幕上的题（句柄）」和 engine 当前 issued 题对齐；对不上就不记分。 */
  const resolved = await resolveGradeTarget(app, stateKey, stateFile, requestHandle);
  if (!resolved.ok) {
    return buildStaleQuestionReply(app, stateKey, stateFile, resolved, false);
  }
  const kind = resolved.question.kind || 'choice';
  if (kind !== 'choice') {
    return {
      kind: 'answer',
      text: `这题是「${kind}」题，不是选择题：直接写下你的回答，我会交给慢思考老师看。`,
    };
  }
  const choice = 'ABCD'.indexOf(String(letter).toUpperCase()) + 1;
  /* 首次答错：给提示 + 同一道题（不记分、不推进）。 */
  try {
    const issued = await engineNext(app, stateFile, false);
    const issuedQuestion = issued && issued.kind === 'question' ? issued.question : null;
    if (issuedQuestion) {
      const retry = await choiceRetryHint(app, fingerprint, stateFile, issuedQuestion, choice, resolved);
      if (retry !== null) return retry;
    }
  } catch {
    /* 提示通道失败就照常走原有反馈，不影响主流程 */
  }

  let answer = null;
  try {
    answer = await runEngine(app, [
      'answer',
      '--state',
      stateFile,
      '--qid',
      resolved.qid,
      '--choice',
      String(choice),
    ]);
  } catch {
    const blocks = await nextBlocks(app, stateFile, fingerprint, {
      advance: true,
      heading: '',
    });
    return { kind: 'answer', text: ['这道题的状态对不上，先换一道：', ...blocks].join('\n\n') };
  }
  // 反馈块 + 分隔线 + 自动附上的下一题（模板见 README）
  const feedbackBlock = formatFeedback(answer);
  const next = await nextBlocks(app, stateFile, fingerprint, {
    advance: true,
    heading: '📝 下一题',
  });
  const reply = {
    kind: 'answer',
    text: [feedbackBlock, '---', ...next].join('\n\n'),
    handle: app.questionHandles.get(stateKey) || null,
  };
  if (requestHandle) writeGradeCache(app, stateKey, resolved.qid, resolved.nonce, reply);
  return reply;
}

function renderKnowledgeCard(result) {
  const card = (result && result.card) || {};
  const lines = [];
  const kicker = typeof card.kicker === 'string' && card.kicker !== '' ? card.kicker : '';
  const title = typeof card.title === 'string' && card.title !== '' ? card.title : '知识卡';
  lines.push('📖 ' + (kicker !== '' ? kicker + ' · ' : '') + title);
  if (typeof card.oneLiner === 'string' && card.oneLiner !== '') lines.push('', card.oneLiner);
  const definition = card.definition && typeof card.definition === 'object' ? card.definition : null;
  if (definition && typeof definition.code === 'string' && definition.code !== '') {
    lines.push('', '```lean', definition.code, '```');
  }
  const sections = Array.isArray(card.sections) ? card.sections.slice(0, 2) : [];
  for (const section of sections) {
    if (!section || typeof section !== 'object') continue;
    const heading = typeof section.heading === 'string' ? section.heading : '';
    if (heading !== '') lines.push('', '**' + heading + '**');
    if (typeof section.text === 'string' && section.text !== '') lines.push(section.text);
    if (Array.isArray(section.items)) {
      for (const item of section.items.slice(0, 4)) {
        if (typeof item === 'string' && item !== '') lines.push('- ' + item);
      }
    }
  }
  if (typeof card.source === 'string' && card.source !== '') lines.push('', '出处：' + formatSourceLink(card.source));
  const types = Array.isArray(card.types) ? card.types.slice(0, 4) : [];
  if (types.length > 0) {
    lines.push('', '**涉及的术语**');
    for (const type of types) {
      if (type && typeof type.term === 'string') lines.push('- `' + type.term + '`：' + String(type.oneLiner || ''));
    }
  }
  lines.push('', '看完就继续答这道题（回复 A / B / C / D），或者回「下一题」换一道。');
  return lines.join('\n');
}

async function mapReply(app, stateFile) {
  try {
    const output = await runEngine(app, ['map', '--state', stateFile]);
    const lines = Array.isArray(output.lines) ? output.lines : [];
    return { kind: 'map', text: lines.join('\n') };
  } catch (error) {
    return { kind: 'map', text: '学习地图暂时取不到，回「开始做题」继续练吧。' };
  }
}


/**
 * Build the display-only "progress / hotspot" card payload from the engine's
 * `state` and `map` snapshots. The card is a pure rendering of these numbers;
 * it never changes quiz state.
 * @param app - quiz app (enginePath/repoRoot/timeout).
 * @param stateFile - current quiz state JSON.
 * @returns payload, or null when the state cannot be read.
 */
export async function buildProgressCardPayload(app, stateFile) {
  if (!fs.existsSync(stateFile)) return null;
  let state;
  let map;
  try {
    state = await runEngine(app, ['state', '--state', stateFile]);
    map = await runEngine(app, ['map', '--state', stateFile]);
  } catch {
    return null;
  }
  const progress = state.progress || {};
  const positive = state.positive || {};
  const coverage = state.coverage || {};
  const theory = coverage.theory || {};
  const heat = state.heatmap || {};
  const history = Array.isArray(state.history_tail) ? state.history_tail : [];
  const mapSnapshot = map.map || {};
  const mapUnits = Array.isArray(mapSnapshot.units) ? mapSnapshot.units : [];
  const chapters = Object.entries(coverage.chapters || {})
    .map(([id, row]) => ({
      id,
      title: (row && row.title) || id,
      completion: Number(row && row.completion) || 0,
      asked: Number(row && row.asked) || 0,
      touched: Number(row && row.tags_touched) || 0,
      total: Number(row && row.tags_total) || 0,
    }))
    .sort((a, b) => String(a.id).localeCompare(String(b.id)));
  const heatRows = (Array.isArray(heat.rows) ? heat.rows : []).map((row) => ({
    chapter: row.chapter,
    title: row.title || row.chapter,
    units: (Array.isArray(row.units) ? row.units : []).map((unit) => ({
      unit: unit.unit,
      touched: Number(unit.touched) || 0,
      total: Number(unit.total) || 0,
      mastery: Number(unit.mastery) || 0,
      level: Number(unit.level) || 0,
    })),
  }));
  /* Hotspots: units already started but weak, plus the engine's due tags. */
  const weak = mapUnits
    .filter((unit) => unit && Number(unit.touched) > 0
      && (unit.status === '刚起步' || (unit.status === '在学' && Number(unit.mastery) < 0.6)))
    .map((unit) => ({
      kind: 'unit',
      label: String(unit.unit || ''),
      chapter: String(unit.chapter || ''),
      mastery: Number(unit.mastery) || 0,
      level: unit.status === '刚起步' ? 1 : 2,
      reason: unit.status === '刚起步' ? '刚起步' : '掌握偏弱',
      touched: Number(unit.touched) || 0,
      total: Number(unit.total) || 0,
    }))
    .sort((a, b) => a.mastery - b.mastery);
  const due = Array.isArray(progress.due_tags) ? progress.due_tags : [];
  const dueHotspots = due.slice(0, 2).map((tag) => ({
    kind: 'tag', label: String(tag), reason: '待复习',
  }));
  const hotspots = [...dueHotspots, ...weak].slice(0, 4);
  const recent = history.slice(-12).map((entry) => ({
    qid: String(entry.qid || ''),
    correct: entry.correct === true,
    at: String(entry.at || ''),
  }));
  return {
    generatedAt: new Date().toISOString(),
    summary: {
      asked: Number(progress.asked) || 0,
      correct: Number(progress.correct) || 0,
      accuracy: Number(progress.accuracy) || 0,
      streak: Number(positive.streak !== undefined ? positive.streak : progress.streak) || 0,
      streakDays: Number(positive.streak_days) || 0,
      served: Number(state.flow && state.flow.served) || 0,
      tagsTouched: Number(positive.tags_touched !== undefined ? positive.tags_touched : theory.touched) || 0,
      tagsTotal: Number(theory.total) || 0,
      coverage: Number(positive.coverage_ratio !== undefined ? positive.coverage_ratio : theory.ratio) || 0,
      chaptersStarted: Number(positive.chapters_started) || 0,
      currentUnit: String(mapSnapshot.currentUnit || ''),
      unitDone: Number(mapSnapshot.unitDone) || 0,
      unitTotal: Number(mapSnapshot.unitTotal) || 0,
      dueCount: due.length,
    },
    chapters,
    heatRows,
    hotspots,
    recent,
  };
}

/** 「进度/地图」：发一张进展热点卡（纯展示，不改状态）。 */
async function cardMapFlow(app, stateKey, stateFile) {
  const payload = await buildProgressCardPayload(app, stateFile);
  if (payload === null) {
    return { kind: 'map', text: '学习进展暂时取不到，回「开始做题」继续练吧。', toolCall: null };
  }
  return { kind: 'map', text: '', toolCall: buildLearnCardToolCall('progress', payload) };
}

async function explainReply(app, stateFile) {
  let state = {};
  try {
    state = JSON.parse(fs.readFileSync(stateFile, 'utf8'));
  } catch (error) {
    state = {};
  }
  const qid = typeof state.issued_qid === 'string' && state.issued_qid !== '' ? state.issued_qid : '';
  if (qid === '') {
    return { kind: 'explain', text: '现在还没有正在做的题目。回我「开始做题」我就给你出第一题 😊' };
  }
  try {
    const output = await runEngine(app, ['explain', '--state', stateFile, '--qid', qid]);
    return { kind: 'explain', text: renderKnowledgeCard(output) };
  } catch (error) {
    return { kind: 'explain', text: '这张卡片暂时没找到匹配的讲解，可以直接问我这题为什么。' };
  }
}

// ───────────────────────── 卡片模式流程 ─────────────────────────
/**
 * 连续取步骤直到拿到一道题（知识卡 primer 先并入正文）。
 * 返回 { blocks, question }；question 为 null 表示这轮没有题目（如代码练习）。
 */
async function cardNextQuestion(app, stateFile, stateKey, { advance, noLesson = false }) {
  const blocks = [];
  const learn = app.cardsMode === 'learn';
  let step = await engineNext(app, stateFile, advance, noLesson);
  for (let guard = 0; guard < 6; guard += 1) {
    if (step.kind === 'lesson') {
      if (learn) {
        /* learn_card 模式：教学环节本身出一张 lesson 卡片，题目等用户回「继续」再发
         * （引擎已把 after_lesson_qid 记好，下一次 --advance 会出那道题）。 */
        return { blocks, question: null, position: step.position || null, lessonObject: step.lesson };
      }
      blocks.push(formatLesson(step.lesson));
      return { blocks, question: null, position: step.position || null };
    }
    if (step.kind === 'question') {
      const question = step.question || {};
      app.issuedKinds.set(stateKey, {
        qid: String(question.id || ''),
        kind: question.kind || 'choice',
      });
      rememberQuestionHandle(app, stateKey, question); // I2：屏幕上的题 = qid+nonce
      return { blocks, question, position: step.position || null };
    }
    if (step.kind === 'card') {
      if (learn) {
        /* learn_card 模式：知识卡 primer 单独出一张 knowledge 卡片；
         * 引擎已记录 after_primer_qid，下一次 --advance 会出配对的题。 */
        return { blocks, question: null, position: step.position || null, knowledgeCard: step.card };
      }
      blocks.push(formatCard(step.card));
      step = await engineNext(app, stateFile, true);
      continue;
    }
    if (step.kind === 'code') {
      blocks.push(formatCodeExercise(step.exercise));
      return { blocks, question: null, position: step.position || null };
    }
    throw new Error('engine.py 返回了未知步骤：' + String(step && step.kind));
  }
  throw new Error('engine.py 连续返回非题目步骤');
}

/**
 * 用 next 步骤拼一条卡片模式回复：题干只进 tool_call（不进正文，避免重复），
 * 知识卡 primer 原文进正文；非选择题退回纯文本。
 */
function cardReplyFromNext(app, stateKey, stateFile, next, leadingText, kind, heading) {
  const blocks = [...next.blocks];
  const learn = app.cardsMode === 'learn';
  let toolCall = null;
  if (next.question) {
    if (isCardQuestion(next.question) || (learn && !isChoiceQuestion(next.question))) {
      toolCall = learn
        ? buildLearnQuestionCard(next.question, next.position)
        : buildQuestionCard(app, stateKey, stateFile, next.question);
    } else {
      blocks.push(formatQuestion(next.question, heading || '📝 题目', next.position));
    }
  } else if (learn && next.knowledgeCard) {
    toolCall = buildLearnCardToolCall('knowledge', next.knowledgeCard);
  } else if (learn && next.lessonObject) {
    toolCall = buildLearnCardToolCall('lesson', next.lessonObject);
  }
  const text = [leadingText, ...blocks]
    .filter((part) => typeof part === 'string' && part !== '')
    .join('\n\n');
  return { kind, text, toolCall };
}

/** 「开始做题 / 下一题」：发一张卡片；content 留空（或只带知识卡原文）。 */
async function cardNextFlow(app, stateKey, stateFile, noLesson = false) {
  const next = await cardNextQuestion(app, stateFile, stateKey, {
    advance: fs.existsSync(stateFile),
    noLesson,
  });
  return cardReplyFromNext(app, stateKey, stateFile, next, '', 'next', '📝 题目');
}

/** 用户直接打字回字母（没点卡片）：判卷 + 下一张卡片。 */
/**
 * 卡片模式下的「首次答错」：dry-run 判卷 + 提示 + 重发同一张题卡，
 * 不进 history、不推进 issued。第二次仍错才走正常判卷/公布答案。
 * 与文本模式的 choiceRetryHint 同一语义，只是输出 learn_card 题卡。
 */
async function cardChoiceRetryHint(app, stateKey, stateFile, question, choice, resolved, position) {
  const attemptKey = stateKey + ':' + String(resolved.qid);
  const attempts = (app.choiceAttempts ||= new Map());
  const tried = attempts.get(attemptKey) || 0;
  if (tried >= 1) {
    attempts.delete(attemptKey);
    return null;
  }
  let trial = null;
  try {
    trial = await runEngine(app, ['answer', '--state', stateFile, '--qid', resolved.qid,
      '--choice', String(choice), '--dry-run']);
  } catch {
    return null;
  }
  if (trial === null || trial.dryRun !== true || trial.correct === true) {
    attempts.delete(attemptKey);
    return null;
  }
  attempts.set(attemptKey, tried + 1);
  const hint = typeof trial.hint === 'string' && trial.hint.trim() !== ''
    ? trial.hint.trim()
    : '回到定义：把题干里的符号展开成「逐个元素」的命题，再看哪个选项与它一字不差。';
  const lines = [
    '❌ 这个不对，再想一次 —— 先看提示，别急着换题：',
    '',
    '💡 ' + hint,
  ];
  let toolCall = null;
  if (isCardQuestion(question)) {
    toolCall = app.cardsMode === 'learn'
      ? buildLearnQuestionCard(question, position)
      : buildQuestionCard(app, stateKey, stateFile, question);
  }
  return { kind: 'answer', text: lines.join('\n'), toolCall };
}

/**
 * 卡片模式下的开放性/简答题判卷：dry-run 先指出还差的要点并重发同一张
 * 问答题卡；完整/第二次后才记账并附带下一张卡片。与 openAnswerFlow 同一判据。
 */
async function cardOpenAnswerFlow(app, stateKey, stateFile, text, position = null) {
  if (String(text).trim() === '') return null;
  let resume = null;
  try {
    resume = await engineNext(app, stateFile, false);
  } catch {
    return null;
  }
  if (!resume || resume.kind !== 'question') return null;
  const question = resume.question || {};
  if ((question.kind || 'choice') === 'choice') return null;
  rememberQuestionHandle(app, stateKey, question);
  const attemptKey = stateKey + ':' + String(question.id);
  const attempts = (app.openAttempts ||= new Map());
  const tried = attempts.get(attemptKey) || 0;
  let trial = null;
  try {
    trial = await runEngine(app, ['answer', '--state', stateFile, '--qid', String(question.id),
      '--text', text, '--dry-run']);
  } catch {
    trial = null;
  }
  const retryable = trial !== null && trial.dryRun === true && trial.correct !== true && tried < 1;
  if (retryable) {
    attempts.set(attemptKey, tried + 1);
    const feedback = trial.feedback || {};
    const missed = Array.isArray(feedback.missed) ? feedback.missed : [];
    const lines = ['⚠️ 还没答全，先别急着看答案 —— 补上这几点再试一次：'];
    for (const item of missed) {
      const note = typeof item.note === 'string' && item.note !== '' ? item.note : item.id;
      if (note) lines.push('- ' + note);
    }
    return {
      kind: 'answer',
      text: lines.join('\n'),
      toolCall: buildLearnQuestionCard(question, resume.position || position),
    };
  }
  attempts.delete(attemptKey);

  let result = null;
  try {
    result = await runEngine(app, ['answer', '--state', stateFile, '--qid', String(question.id), '--text', text]);
  } catch {
    return null;
  }
  const lead = formatOpenFeedback(question, result);
  const next = await cardNextQuestion(app, stateFile, stateKey, { advance: true });
  return cardReplyFromNext(app, stateKey, stateFile, next, lead, 'answer', '📝 下一题');
}

async function cardAnswerByLetterFlow(app, stateKey, stateFile, letter, requestHandle = null) {
  const state = readStateJson(stateFile);
  if (!state || !state.issued_qid) {
    return { kind: 'answer', text: '先回我「开始做题」，我给你发第一张卡片 😊', toolCall: null };
  }
  /* I3：显式句柄重复判分幂等（先查缓存，再校验句柄 —— 重试时 engine 已推进到下一题）。 */
  if (requestHandle) {
    const cachedReply = readGradeCache(app, stateKey, requestHandle.qid, requestHandle.nonce);
    if (cachedReply) return cachedReply;
  }
  /* I2：句柄校验（卡片模式下屏幕上的题 = learn_card/ask 卡里的那道）。 */
  const resolved = await resolveGradeTarget(app, stateKey, stateFile, requestHandle);
  if (!resolved.ok) {
    return buildStaleQuestionReply(app, stateKey, stateFile, resolved, true);
  }
  const kind = resolved.question.kind || 'choice';
  if (kind !== 'choice') {
    return {
      kind: 'answer',
      text: `这题是「${kind}」题，不是选择题：直接写下你的回答，我会交给慢思考老师看。`,
      toolCall: null,
    };
  }
  const choice = 'ABCD'.indexOf(String(letter).toUpperCase()) + 1;
  /* 首次答错：给提示 + 重发同一张题卡（不记分、不推进）；第二次才正常判卷。 */
  try {
    const issued = await engineNext(app, stateFile, false);
    const issuedQuestion = issued && issued.kind === 'question' ? issued.question : null;
    if (issuedQuestion) {
      const retry = await cardChoiceRetryHint(app, stateKey, stateFile, issuedQuestion, choice, resolved, resolved.position);
      if (retry !== null) return retry;
    }
  } catch {
    /* 提示通道失败就照常判卷，不影响主流程 */
  }

  let answer = null;
  try {
    answer = await runEngine(app, [
      'answer',
      '--state',
      stateFile,
      '--qid',
      resolved.qid,
      '--choice',
      String(choice),
    ]);
  } catch {
    const next = await cardNextQuestion(app, stateFile, stateKey, { advance: true });
    return cardReplyFromNext(
      app,
      stateKey,
      stateFile,
      next,
      '这道题的状态对不上，先换一张卡片：',
      'answer',
      '📝 下一题',
    );
  }
  const next = await cardNextQuestion(app, stateFile, stateKey, { advance: true });
  const reply = cardReplyFromNext(app, stateKey, stateFile, next, formatFeedback(answer), 'answer', '📝 下一题');
  reply.handle = app.questionHandles.get(stateKey) || null;
  if (requestHandle) writeGradeCache(app, stateKey, resolved.qid, resolved.nonce, reply);
  return reply;
}

/** 当前正在做的那道题再发一张卡片（explain / 自定义提问后用，不消耗题目）。 */
async function currentQuestionCard(app, stateKey, stateFile) {
  const state = readStateJson(stateFile);
  if (!state || !state.issued_qid) return null;
  let step = null;
  try {
    step = await engineNext(app, stateFile, false);
  } catch {
    return null;
  }
  if (
    step &&
    step.kind === 'question' &&
    step.decision &&
    step.decision.reason === 'resume' &&
    isCardQuestion(step.question)
  ) {
    app.issuedKinds.set(stateKey, {
      qid: String(step.question.id || ''),
      kind: step.question.kind || 'choice',
    });
    rememberQuestionHandle(app, stateKey, step.question);
    return buildQuestionCard(app, stateKey, stateFile, step.question);
  }
  return null;
}

/** 当前 issued 题的 engine explain 原始输出（含 card 对象），取不到返回 null。 */
async function explainCardObject(app, stateFile) {
  const state = readStateJson(stateFile) || {};
  const qid = typeof state.issued_qid === 'string' && state.issued_qid !== '' ? state.issued_qid : '';
  if (qid === '') return null;
  try {
    const output = await runEngine(app, ['explain', '--state', stateFile, '--qid', qid]);
    return output && output.card && typeof output.card === 'object' ? output.card : null;
  } catch {
    return null;
  }
}

/** 「不懂/讲讲」：输出知识卡（不判卷、不换题）。 */
async function cardExplainFlow(app, stateKey, stateFile) {
  if (app.cardsMode === 'learn') {
    /* learn_card：知识卡也走真卡片（engine 的 card 对象原样透传），content 留空。 */
    const card = await explainCardObject(app, stateFile);
    if (card) {
      return { kind: 'explain', text: '', toolCall: buildLearnCardToolCall('knowledge', card) };
    }
    const explain = await explainReply(app, stateFile);
    return { kind: 'explain', text: explain.text, toolCall: null };
  }
  /* legacy ask 模式：知识卡文本 + 同一道题再发一张点选卡片。 */
  const explain = await explainReply(app, stateFile);
  const toolCall = await currentQuestionCard(app, stateKey, stateFile);
  return { kind: 'explain', text: explain.text, toolCall };
}

/** 处理「上一张卡片被用户点了」的 tool 结果：本地判卷 + 下一张卡片。 */
async function cardAnswerFromToolFlow(app, stateKey, stateFile, cardAnswer) {
  const cacheKey = cardAnswer.toolCallId;
  const cachedReply = app.cardReplies.get(cacheKey);
  if (cachedReply) return cachedReply; // 幂等：同一 tool_call_id 重试不重复判分

  const state = readStateJson(stateFile);
  const pending = app.pendingCards.get(cacheKey) || null;
  let qid = pending ? pending.qid : state && state.issued_qid ? String(state.issued_qid) : '';
  let options = pending ? pending.options : null;
  if (!pending && qid && !options) {
    // 服务重启 / 内存丢失：用引擎 resume「当前发出的题」（不推进）补出选项。
    // 只认 reason=resume 的题，避免把引擎顺手新发的题当成本次卡片。
    let step = null;
    try {
      step = await engineNext(app, stateFile, false);
    } catch {
      step = null;
    }
    if (
      step &&
      step.kind === 'question' &&
      step.decision &&
      step.decision.reason === 'resume' &&
      isCardQuestion(step.question)
    ) {
      options = step.question.options;
      qid = String(step.question.id || qid);
      app.issuedKinds.set(stateKey, {
        qid,
        kind: step.question.kind || 'choice',
      });
      rememberQuestionHandle(app, stateKey, step.question);
    }
  }
  if (!qid) {
    const guidance = {
      kind: 'card-miss',
      text: '这张卡片对应的题目已经找不到了。回我「开始做题」重新发一张吧。',
      toolCall: null,
    };
    rememberCardReply(app, cacheKey, guidance);
    return guidance;
  }
  /* I2：点击路径同样校验「屏幕上的卡」= engine 当前 issued 题；对不上不记分。 */
  const resolved = await resolveGradeTarget(app, stateKey, stateFile, null);
  if (!resolved.ok || resolved.qid !== qid) {
    const staleReply = await buildStaleQuestionReply(app, stateKey, stateFile, resolved, true);
    rememberCardReply(app, cacheKey, staleReply);
    return staleReply;
  }

  const choiceIndex = resolveCardChoice(options || [], cardAnswer.selected, cardAnswer.custom);
  if (choiceIndex < 1) {
    if (cardAnswer.custom.trim() !== '') {
      // custom 不是字母 ⇒ 当作用户提问/吐槽：本地 explain 知识卡 + 原题卡片再来一张
      const explain = await explainReply(app, stateFile);
      const toolCall = await currentQuestionCard(app, stateKey, stateFile);
      const reply = {
        kind: 'card-explain',
        text: `你问的是：「${cardAnswer.custom.trim().slice(0, 200)}」\n\n${explain.text}`,
        toolCall,
      };
      rememberCardReply(app, cacheKey, reply);
      return reply;
    }
    const guidance = {
      kind: 'card-miss',
      text: '没认出这次选择的选项（可能题库刚换了一题）。回我「开始做题」重新发一张卡片吧。',
      toolCall: null,
    };
    rememberCardReply(app, cacheKey, guidance);
    return guidance;
  }

  let answer = null;
  try {
    answer = await runEngine(app, [
      'answer',
      '--state',
      stateFile,
      '--qid',
      qid,
      '--choice',
      String(choiceIndex),
    ]);
  } catch {
    const next = await cardNextQuestion(app, stateFile, stateKey, { advance: true });
    const reply = cardReplyFromNext(
      app,
      stateKey,
      stateFile,
      next,
      '这道题的状态对不上，先换一张卡片：',
      'answer',
      '📝 下一题',
    );
    rememberCardReply(app, cacheKey, reply);
    return reply;
  }
  const next = await cardNextQuestion(app, stateFile, stateKey, { advance: true });
  const reply = cardReplyFromNext(
    app,
    stateKey,
    stateFile,
    next,
    formatFeedback(answer),
    'answer',
    '📝 下一题',
  );
  app.pendingCards.delete(cacheKey); // 这张卡已消费
  rememberCardReply(app, cacheKey, reply);
  return reply;
}

async function cardFastReply(app, fingerprint, fast, requestHandle = null) {
  const stateKey = stateKeyOf(app, fingerprint);
  const stateFile = stateFileOf(app, fingerprint);
  if (fast.type === 'explain') return cardExplainFlow(app, stateKey, stateFile);
  if (fast.type === 'map') return cardMapFlow(app, stateKey, stateFile);
  if (fast.type === 'next') return cardNextFlow(app, stateKey, stateFile, fast.noLesson === true);
  return cardAnswerByLetterFlow(app, stateKey, stateFile, fast.letter, requestHandle);
}

/** 「我会了 / 太简单」：把这个知识点标记为已会，直接换下一个知识点。 */
async function skipFlow(app, stateFile, fingerprint) {
  let skipped = '';
  try {
    const result = await runEngine(app, ['skip-kp', '--state', stateFile]);
    skipped = typeof result.skipped === 'string' ? result.skipped : '';
  } catch {
    skipped = '';
  }
  const blocks = await nextBlocks(app, stateFile, fingerprint, { advance: true, heading: '' });
  const lead = skipped !== '' ? '好，`' + skipped + '` 记为已掌握，换下一个知识点：' : '好，换下一个知识点：';
  return { kind: 'skip', text: [lead, '', ...blocks].join('\n') };
}

async function fastReply(app, fingerprint, fast, rawText, requestHandle = null) {
  const stateFile = stateFileOf(app, fingerprint);
  if (fast === null) {
    /* 不是命令也不是字母：若当前是开放题，这段文字就是它的回答 —— 本地 rubric 判分，
     * 绝不再把这轮交给慢模型（否则模型会自己「编」下一题，displayed ≠ issued）。 */
    const answered = await openAnswerFlow(app, fingerprint, stateFile, String(rawText || ''));
    if (answered !== null) return answered;
  }
  if (fast.type === 'next') {
    const blocks = await nextBlocks(app, stateFile, fingerprint, {
      advance: fs.existsSync(stateFile),
      heading: '',
      noLesson: fast.noLesson === true,
    });
    return { kind: 'next', text: blocks.join('\n\n') };
  }
  if (fast.type === 'skip') return skipFlow(app, stateFile, fingerprint);
  if (fast.type === 'map') return mapReply(app, stateFile);
  if (fast.type === 'explain') return explainReply(app, stateFile);
  return answerFlow(app, fingerprint, stateFile, fast.letter, requestHandle);
}

export function classifyFast(model, lastUserText) {
  if (typeof lastUserText !== 'string') return null;
  if (model && model !== QUIZ_MODEL) return null; // 其它 model 一律直连转发
  const choice = parseChoiceLetter(lastUserText);
  if (choice >= 1) return { type: 'answer', letter: 'ABCD'[choice - 1], index: choice };
  const normalized = normalizeUserText(lastUserText);
  if (START_RE.test(normalized)) {
    /* 「下一题 / 换一题」= 我就要题：跳过教学环节与 primer 卡；
     * 「开始做题 / 继续」= 允许先上课（教学环节是设计的一部分）。 */
    const wantsQuestion = /下一题|下一道|换一题|换一道|再来一题|再来一道/.test(normalized);
    return { type: 'next', noLesson: wantsQuestion };
  }
  if (SKIP_RE.test(normalized)) return { type: 'skip' };
  if (MAP_RE.test(normalized)) return { type: 'map' };
  if (EXPLAIN_RE.test(normalized)) return { type: 'explain' };
  return null;
}

// ───────────────────────── OpenAI 兼容响应（快路径） ─────────────────────────
function splitForStream(text, maxChars = 160) {
  const raw = String(text);
  if (!raw) return [''];
  const pieces = [];
  for (let offset = 0; offset < raw.length; offset += maxChars) {
    pieces.push(raw.slice(offset, offset + maxChars));
  }
  return pieces;
}

function fastCompletionJson(text, promptChars, toolCall = null, emptyToolCalls = false) {
  const content = text == null ? '' : String(text);
  const promptTokens = estimateTokens(promptChars);
  const completionTokens = estimateTokens(content.length);
  const message = { role: 'assistant', content };
  if (toolCall) message.tool_calls = [toolCall];
  else if (emptyToolCalls) message.tool_calls = [];
  return {
    id: completionId(),
    object: 'chat.completion',
    created: epochSeconds(),
    model: QUIZ_MODEL,
    choices: [
      {
        index: 0,
        message,
        // OpenAI 规定：带 tool_calls 时 finish_reason 必须是 tool_calls（不能混 stop）
        finish_reason: toolCall ? 'tool_calls' : 'stop',
      },
    ],
    usage: {
      prompt_tokens: promptTokens,
      completion_tokens: completionTokens,
      total_tokens: promptTokens + completionTokens,
    },
  };
}

function writeFastStream(res, text, includeUsage = false, promptChars = 0, toolCall = null, extraHeaders = {}) {
  const id = completionId();
  const created = epochSeconds();
  const chunk = (delta, finishReason = null) => ({
    id,
    object: 'chat.completion.chunk',
    created,
    model: QUIZ_MODEL,
    choices: [{ index: 0, delta, finish_reason: finishReason }],
  });
  res.writeHead(200, {
    'content-type': 'text/event-stream; charset=utf-8',
    'cache-control': 'no-cache',
    connection: 'keep-alive',
    'x-accel-buffering': 'no',
    'x-quiz-proxy-path': 'fast',
    ...extraHeaders,
  });
  res.write(`data: ${JSON.stringify(chunk({ role: 'assistant' }))}\n\n`);
  const content = text == null ? '' : String(text);
  for (const piece of splitForStream(content)) {
    if (piece === '') continue;
    res.write(`data: ${JSON.stringify(chunk({ content: piece }))}\n\n`);
  }
  if (toolCall) {
    // 整段 arguments 一次发（OpenAI 允许，pi-ai 会拼接后 JSON.parse）
    res.write(
      `data: ${JSON.stringify(
        chunk({
          tool_calls: [
            {
              index: 0,
              id: toolCall.id,
              type: 'function',
              function: {
                name: toolCall.function.name,
                arguments: toolCall.function.arguments,
              },
            },
          ],
        }),
      )}\n\n`,
    );
  }
  res.write(`data: ${JSON.stringify(chunk({}, toolCall ? 'tool_calls' : 'stop'))}\n\n`);
  if (includeUsage) {
    const promptTokens = estimateTokens(promptChars);
    const completionTokens = estimateTokens(content.length);
    res.write(
      `data: ${JSON.stringify({
        id,
        object: 'chat.completion.chunk',
        created,
        model: QUIZ_MODEL,
        choices: [],
        usage: {
          prompt_tokens: promptTokens,
          completion_tokens: completionTokens,
          total_tokens: promptTokens + completionTokens,
        },
      })}\n\n`,
    );
  }
  res.write('data: [DONE]\n\n');
  res.end();
}

/**
 * 统一发送快路径回复（JSON / SSE），并带上 I2 的 `x-quiz-qid` / `x-quiz-nonce` 句柄头
 * （对用户不可见；DSH 会忽略，但测试与排障可以据此确认「屏幕上是哪道题」）。
 */
function emitFastReply(res, app, reply, options = {}) {
  const { stream = false, includeUsage = false, promptChars = 0, handle = null, headers = {} } = options;
  const extra = {
    'x-quiz-proxy-path': 'fast',
    ...questionHandleHeaders(handle),
    ...(reply.stale ? { 'x-quiz-stale': '1' } : {}),
    ...headers,
  };
  if (stream) {
    writeFastStream(res, reply.text, includeUsage, promptChars, reply.toolCall, extra);
  } else {
    sendJson(
      res,
      200,
      fastCompletionJson(reply.text, promptChars, reply.toolCall, reply.emptyToolCalls === true),
      extra,
    );
  }
}

// ───────────────────────── 慢路径（转发 opencode-go） ─────────────────────────
function mapModel(model, app) {
  if (!model || model === QUIZ_MODEL) return app.upstreamModel;
  return model;
}

/** 慢路径需要把 opencode 的路由头带过去（否则上游 400：missing x-opencode-session）。 */
function forwardableHeaders(incomingHeaders, app) {
  const forwarded = {};
  for (const [name, value] of Object.entries(incomingHeaders || {})) {
    if (typeof value === 'string' && name.toLowerCase().startsWith('x-opencode-')) {
      forwarded[name] = value;
    }
  }
  const session = app.opencodeSession;
  if (session) {
    const existing = Object.keys(forwarded).find((name) => name.toLowerCase() === 'x-opencode-session');
    if (!existing) forwarded['x-opencode-session'] = session;
  }
  return forwarded;
}

async function forwardSlow(res, app, body, stream, startedAt, incomingHeaders, guard = null) {
  if (!app.apiKey) {
    sendJson(
      res,
      500,
      errorPayload(
        500,
        '缺少 ' + API_KEY_NAME + '（环境变量或 ~/.dsh/.credentials.yaml）',
        'server_error',
        'missing_api_key',
      ),
    );
    return;
  }
  const outbound = { ...body, model: mapModel(body.model, app) };
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), app.upstreamTimeoutMs);
  let upstreamRes;
  try {
    upstreamRes = await fetch(app.upstream, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        authorization: 'Bearer ' + app.apiKey,
        ...forwardableHeaders(incomingHeaders, app),
      },
      body: JSON.stringify(outbound),
      signal: controller.signal,
    });
  } catch (error) {
    clearTimeout(timer);
    const aborted = error && (error.name === 'AbortError' || error.code === 'ABORT_ERR');
    const status = aborted ? 504 : 502;
    app.log(
      `slow error fp=? ${aborted ? 'timeout' : 'network'} status=${status} ${Date.now() - startedAt}ms`,
    );
    sendJson(
      res,
      status,
      errorPayload(
        status,
        aborted
          ? `上游超时（${app.upstreamTimeoutMs}ms）`
          : '上游不可达：' + String((error && error.message) || error),
        'upstream_error',
        aborted ? 'upstream_timeout' : 'upstream_unreachable',
      ),
    );
    return;
  }

  if (!upstreamRes.ok) {
    clearTimeout(timer);
    let message = '';
    try {
      message = await upstreamRes.text();
    } catch {
      message = '';
    }
    let shortMessage = message.slice(0, 500);
    try {
      const parsed = JSON.parse(message);
      if (parsed && parsed.error) {
        shortMessage = String(parsed.error.message || parsed.error).slice(0, 500);
      }
    } catch {
      /* 非 JSON 错误体，保留原文前 500 字 */
    }
    app.log(
      `slow upstream-status=${upstreamRes.status} ${Date.now() - startedAt}ms model=${outbound.model}`,
    );
    sendJson(
      res,
      upstreamRes.status,
      errorPayload(
        upstreamRes.status,
        `上游 opencode-go 返回 ${upstreamRes.status}：${shortMessage}`,
        'upstream_error',
        'upstream_http_' + upstreamRes.status,
      ),
    );
    return;
  }

  if (!stream) {
    let text = '';
    try {
      text = await upstreamRes.text();
    } finally {
      clearTimeout(timer);
    }
    /* I4：有未答选择题时，慢回复若自己出了「题干 + A/B/C/D 选项」⇒ 整段丢弃，改回引擎题。 */
    if (guard && looksLikeModelQuestion(upstreamTextOfJson(text))) {
      app.log(
        `guard:model-question mode=json qid=${String((guard.question || {}).id || '')} bytes=${Buffer.byteLength(text)}`,
      );
      const guardedReply = buildGuardedQuestionReply(app, guard);
      emitFastReply(res, app, guardedReply, {
        stream: false,
        promptChars: 0,
        handle: guardedReply.handle,
        headers: { 'x-quiz-guard': 'model-question' },
      });
      return;
    }
    const bodyBuffer = Buffer.from(text, 'utf8');
    app.log(
      `slow json bytes=${bodyBuffer.length} model=${outbound.model} ${Date.now() - startedAt}ms`,
    );
    res.writeHead(200, {
      'content-type': 'application/json; charset=utf-8',
      'content-length': String(bodyBuffer.length),
      'x-quiz-proxy-path': 'slow',
    });
    res.end(bodyBuffer);
    return;
  }

  // 流式。
  const reader = upstreamRes.body.getReader();
  if (guard) {
    /* I4：有未答选择题时先缓冲整段上游 SSR 回复（模型若自己出题，整段丢弃换引擎题）。
     * 代价：这一轮慢回复不再边生成边显示（正确性优先，且只在有未答题时生效）。 */
    res.on('close', () => {
      if (!res.writableEnded) {
        controller.abort();
        reader.cancel().catch(() => {});
      }
    });
    const collected = [];
    try {
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        collected.push(Buffer.from(value));
      }
    } catch {
      /* 上游流异常：按空处理，下面照常判定 */
    } finally {
      clearTimeout(timer);
    }
    const buffered = Buffer.concat(collected);
    const upstreamText = upstreamTextOfSse(buffered.toString('utf8'));
    if (looksLikeModelQuestion(upstreamText)) {
      app.log(
        `guard:model-question mode=sse qid=${String((guard.question || {}).id || '')} bytes=${buffered.length}`,
      );
      const guardedReply = buildGuardedQuestionReply(app, guard);
      emitFastReply(res, app, guardedReply, {
        stream: true,
        promptChars: 0,
        handle: guardedReply.handle,
        headers: { 'x-quiz-guard': 'model-question' },
      });
      return;
    }
    res.writeHead(200, {
      'content-type': 'text/event-stream; charset=utf-8',
      'cache-control': 'no-cache',
      connection: 'keep-alive',
      'x-accel-buffering': 'no',
      'x-quiz-proxy-path': 'slow',
    });
    res.write(buffered);
    res.end();
    app.log(
      `slow stream bytes=${buffered.length} model=${outbound.model} guard-pass ${Date.now() - startedAt}ms`,
    );
    return;
  }

  // 无守卫：逐块透传（保持 SSE 顺序与 [DONE]）。
  res.writeHead(200, {
    'content-type': 'text/event-stream; charset=utf-8',
    'cache-control': 'no-cache',
    connection: 'keep-alive',
    'x-accel-buffering': 'no',
    'x-quiz-proxy-path': 'slow',
  });
  let bytes = 0;
  let clientGone = false;
  res.on('close', () => {
    if (!res.writableEnded) {
      clientGone = true;
      controller.abort();
      reader.cancel().catch(() => {});
    }
  });
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      if (res.writableEnded) break;
      bytes += value.length;
      res.write(Buffer.from(value));
    }
  } catch {
    /* 客户端断开 / 上游流异常：直接收尾 */
  } finally {
    clearTimeout(timer);
    if (!res.writableEnded) res.end();
    app.log(
      `slow stream bytes=${bytes} model=${outbound.model} ${clientGone ? 'client-gone ' : ''}${Date.now() - startedAt}ms`,
    );
  }
}

// ───────────────────────── 路由 ─────────────────────────
function modelsPayload() {
  return {
    object: 'list',
    data: [
      { id: QUIZ_MODEL, object: 'model', created: 0, owned_by: 'local-quiz' },
      { id: SLOW_MODEL, object: 'model', created: 0, owned_by: 'opencode-go' },
    ],
  };
}

async function handleChat(req, res, app) {
  const startedAt = Date.now();
  let raw;
  try {
    raw = await readBody(req, app.maxBodyBytes);
  } catch (error) {
    if (error && error.code === 'too_large') {
      sendJson(
        res,
        413,
        errorPayload(
          413,
          `请求体超过上限（${app.maxBodyBytes} 字节）`,
          'invalid_request_error',
          'body_too_large',
        ),
        { connection: 'close' },
      );
      return;
    }
    sendJson(res, 400, errorPayload(400, '读取请求体失败：' + String(error && error.message), 'invalid_request_error', 'body_read_error'));
    return;
  }

  let body;
  try {
    body = JSON.parse(raw.toString('utf8'));
  } catch {
    sendJson(res, 400, errorPayload(400, '请求体不是合法 JSON', 'invalid_request_error', 'invalid_json'));
    return;
  }
  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    sendJson(res, 400, errorPayload(400, '请求体必须是 JSON 对象', 'invalid_request_error', 'invalid_body'));
    return;
  }
  if (!Array.isArray(body.messages)) {
    sendJson(res, 400, errorPayload(400, 'messages 必须是数组', 'invalid_request_error', 'invalid_messages'));
    return;
  }

  const stream = body.stream === true;
  const model = typeof body.model === 'string' ? body.model : '';
  const messages = body.messages;
  /* I2：客户端可回显句柄（响应头 x-quiz-qid/x-quiz-nonce 原样带回即可）。 */
  const requestHandle = extractRequestHandle(req, body);
  /* DSH 会在工具结果之后追加 runtime-context 用户消息（<system-reminder> 等），
   * 所以「最后一条」不能直接取 messages[末尾]：跳过这些注入再看。 */
  const lastMessage = messages.length > 0 ? messages[messages.length - 1] : null;
  const endsWithTool = endsWithToolResult(messages);
  const quizModel = isQuizModel(model);
  const cardsMode = app.cardsMode;
  const learnToolDeclared = cardsMode === 'learn' && hasLearnCardTool(body.tools);
  const askToolDeclared = cardsMode === 'ask' && hasAskUserQuestionTool(body.tools);
  // ① learn_card 的执行回显（纯展示工具）：按 tool_call_id 认领，回空 assistant + stop
  const learnCardEcho =
    quizModel && endsWithTool && cardsMode === 'learn' && isLearnCardToolResult(messages);
  // ② legacy ask 卡片结果（quiz- 前缀 id）→ 本地判卷 + 下一张卡片
  const cardAnswer =
    quizModel && endsWithTool && cardsMode === 'ask' ? lastQuizCardAnswer(messages) : null;
  // ③ 卡片模式：请求 tools 里声明了对应卡片工具（learn_card / ask_user_question）
  const cardMode =
    app.cardsEnabled &&
    quizModel &&
    !endsWithTool &&
    (learnToolDeclared || askToolDeclared) &&
    cardCallsAllowed(body, cardsMode === 'learn' ? LEARN_CARD_TOOL_NAME : ASK_TOOL_NAME);
  const conversation = extractConversation(messages);
  const fast = endsWithTool ? null : classifyFast(model, conversation.lastUserText);
  const includeUsage = Boolean(body.stream_options && body.stream_options.include_usage);
  const preview = String(conversation.lastUserText || '').replace(/\s+/g, ' ').slice(0, 60);
  const route = learnCardEcho
    ? 'card:echo'
    : cardAnswer
      ? 'card:answer'
      : fast
        ? (cardMode ? 'card:' : 'fast:') + fast.type
        : 'slow';
  const toolCount = Array.isArray(body.tools) ? body.tools.length : 0;
  app.log(
    `route=${route} model=${model || '?'} tools=${toolCount} cards=${cardsMode} learn=${learnToolDeclared ? 1 : 0} ask=${askToolDeclared ? 1 : 0} lastUser="${preview}"`,
  );

  const sendFastReply = async (fingerprint, reply, kindTag, handleBefore = null) => {
    const stateKey = stateKeyOf(app, fingerprint);
    const currentHandle = app.questionHandles.get(stateKey) || null;
    /* 这一轮发过新题（issueId 变了）才带句柄头；没发题（map/explain/纯反馈）不带，避免误导。 */
    const handle =
      reply.handle ||
      (currentHandle && (!handleBefore || currentHandle.issueId !== handleBefore.issueId)
        ? currentHandle
        : null);
    /* 一次性升级提醒：拼在这条回复最前面 */
    if (app.announcePending === undefined) {
      app.announcePending = takeAnnouncement(app.runDir, { rev: app.buildRev || '', mode: app.cardsMode });
    }
    if (typeof app.announcePending === 'string' && app.announcePending !== '') {
      reply.text = app.announcePending + '\n\n' + reply.text;
      app.announcePending = '';
    }
    app.stats.fastturns += 1;
    app.log(
      `${kindTag} fp=${fingerprint.slice(0, 8)} kind=${reply.kind} chars=${reply.text.length}${reply.toolCall ? ' card=1' : ''}${handle ? ' qid=' + handle.qid : ''}${reply.stale ? ' stale=1' : ''} ${Date.now() - startedAt}ms`,
    );
    emitFastReply(res, app, reply, {
      stream,
      includeUsage,
      promptChars: conversation.charCount,
      handle,
    });
  };

  if (learnCardEcho) {
    /* learn_card 是纯展示工具：DSH 执行完会再请求一次模型。这里必须直接回
     * {content:'', tool_calls:[], finish_reason:'stop'}，**绝不能**转发慢路径
     * （否则每张卡后面都会多一条慢模型回复，还会耗 key）。 */
    app.stats.fastturns += 1;
    const echoToolMessage = lastNonRuntimeMessage(messages);
    const callId = String((echoToolMessage && echoToolMessage.tool_call_id) || '');
    const echoed = (app.echoedCalls ||= new Set()).has(callId) ? 'again' : 'first';
    app.echoedCalls.add(callId);
    if (app.echoedCalls.size > 200) {
      app.echoedCalls.delete(app.echoedCalls.values().next().value);
    }
    app.log(`card echo call=${callId.slice(0, 16)} ${echoed} ${Date.now() - startedAt}ms`);
    /* content 必须非空（见 ECHO_SILENT_CONTENT 注释）：否则 DSH 不记录 assistant
     * 消息、带同一条 tool 结果重试，界面变成「卡片渲染失败」。 */
    const silence = ECHO_SILENT_CONTENT;
    if (stream) {
      writeFastStream(res, silence, includeUsage, conversation.charCount, null);
    } else {
      sendJson(res, 200, fastCompletionJson(silence, conversation.charCount, null, true), {
        'x-quiz-proxy-path': 'fast',
      });
    }
    return;
  }

  if (cardAnswer) {
    const fingerprint = fingerprintOf(messages);
    const stateKey = stateKeyOf(app, fingerprint);
    const handleBefore = app.questionHandles.get(stateKey) || null;
    try {
      const reply = await withStateLock(app, stateKey, () =>
        cardAnswerFromToolFlow(app, stateKey, stateFileOf(app, fingerprint), cardAnswer),
      );
      await sendFastReply(fingerprint, reply, 'card', handleBefore);
    } catch (error) {
      app.log(`card error fp=${fingerprint.slice(0, 8)} ${String((error && error.message) || error)}`);
      sendJson(
        res,
        502,
        errorPayload(
          502,
          '本地题库引擎出错：' + String((error && error.message) || error),
          'server_error',
          'engine_error',
        ),
      );
    }
    return;
  }

  /* 非命令、非字母：如果当前 issued 题是开放题，这段文字就是它的回答 —— 本地判分，
   * 不交给慢模型（否则模型会自己编下一题，displayed ≠ issued）。 */
  if (fast === null && quizModel && !endsWithTool) {
    const fingerprint = fingerprintOf(messages);
    const stateKey = stateKeyOf(app, fingerprint);
    const handleBefore = app.questionHandles.get(stateKey) || null;
    try {
      const openReply = await withStateLock(app, stateKey, () =>
        cardMode
          ? cardOpenAnswerFlow(app, stateKey, stateFileOf(app, fingerprint), String(conversation.lastUserText || ''))
          : openAnswerFlow(app, fingerprint, stateFileOf(app, fingerprint), String(conversation.lastUserText || '')),
      );
      if (openReply !== null) {
        app.log(`route=fast:open model=${model} tools=${Array.isArray(body.tools) ? body.tools.length : 0} cards=${app.cardsMode} lastUser="${String(conversation.lastUserText || '').replace(/\s+/g, ' ').slice(0, 60)}"`);
        await sendFastReply(fingerprint, openReply, 'fast', handleBefore);
        return;
      }
    } catch (error) {
      app.log(`open error fp=${fingerprint.slice(0, 8)} ${String((error && error.message) || error)}`);
    }
  }

  if (fast !== null) {
    const fingerprint = fingerprintOf(messages);
    const stateKey = stateKeyOf(app, fingerprint);
    const handleBefore = app.questionHandles.get(stateKey) || null;
    try {
      const reply = await withStateLock(app, stateKey, () =>
        cardMode
          ? cardFastReply(app, fingerprint, fast, requestHandle)
          : fastReply(app, fingerprint, fast, conversation.lastUserText, requestHandle),
      );
      await sendFastReply(fingerprint, reply, cardMode ? 'card' : 'fast', handleBefore);
    } catch (error) {
      app.log(`fast error fp=${fingerprint.slice(0, 8)} ${String((error && error.message) || error)}`);
      sendJson(
        res,
        502,
        errorPayload(
          502,
          '本地题库引擎出错：' + String((error && error.message) || error),
          'server_error',
          'engine_error',
        ),
      );
    }
    return;
  }

  app.stats.slowturns += 1;
  /* I4：慢路径转发前记下「当前是否有未答选择题」；有则对上游回复做「模型自作题」守卫。
   * 只在 state 里确实有 issued 题时才 resume（否则会顺手让引擎发新题 = 副作用）。 */
  let modelQuestionGuard = null;
  if (quizModel) {
    const fingerprint = fingerprintOf(messages);
    const stateFile = stateFileOf(app, fingerprint);
    const stateSnapshot = readStateJson(stateFile);
    if (stateSnapshot && stateSnapshot.issued_qid) {
      try {
        const current = await currentIssuedQuestion(app, stateFile);
        if (current && isChoiceQuestion(current.question)) {
          modelQuestionGuard = {
            stateKey: stateKeyOf(app, fingerprint),
            stateFile,
            question: current.question,
            position: current.position,
            cardMode,
          };
        }
      } catch {
        modelQuestionGuard = null;
      }
    }
  }
  await forwardSlow(res, app, body, stream, startedAt, req.headers, modelQuestionGuard);
}

async function handleRequest(req, res, app) {
  const url = new URL(req.url || '/', 'http://127.0.0.1');
  const pathname = url.pathname.replace(/\/+$/, '') || '/';
  if (req.method === 'GET' && (pathname === '/health' || pathname === '/v1/health')) {
    sendJson(res, 200, {
      ok: true,
      fastturns: app.stats.fastturns,
      slowturns: app.stats.slowturns,
      cards: app.cardsEnabled,
      cardsMode: app.cardsMode,
    });
    return;
  }
  if (req.method === 'GET' && (pathname === '/v1/models' || pathname === '/models')) {
    sendJson(res, 200, modelsPayload());
    return;
  }
  if (req.method === 'POST' && (pathname === '/v1/chat/completions' || pathname === '/chat/completions')) {
    await handleChat(req, res, app);
    return;
  }
  sendJson(
    res,
    404,
    errorPayload(404, `没有这个接口：${req.method} ${url.pathname}`, 'not_found', 'not_found'),
  );
}

// ───────────────────────── per-state-key 串行锁（I3：单一写者） ─────────────────────────
/** 锁键 = state key（不是 fingerprint）：同一状态文件同一时刻只有一个写者。 */
function withStateLock(app, stateKey, fn) {
  const key = stateKey;
  const previous = app.locks.get(key) || Promise.resolve();
  const run = previous.then(fn);
  const tail = run.then(
    () => undefined,
    () => undefined,
  );
  app.locks.set(key, tail);
  tail.then(() => {
    if (app.locks.get(key) === tail) app.locks.delete(key);
  });
  return run;
}

/** 兼容旧名（内部调用一律走 withStateLock）。 */
function withFingerprintLock(app, fingerprint, fn) {
  return withStateLock(app, stateKeyOf(app, fingerprint), fn);
}

// ───────────────────────── 组装 ─────────────────────────
/**
 * 创建（但不监听）代理服务器。
 * @param {object} options
 *  - repoRoot             仓库根（默认自动解析）
 *  - stateDir             状态目录（默认 <repo>/.cache/quiz-proxy/state）
 *  - upstream             慢路径上游 URL
 *  - upstreamModel        quiz-auto 映射到的上游模型名
 *  - apiKey               慢路径 key（默认读 env / ~/.dsh/.credentials.yaml）
 *  - credentialsPath      credentials.yaml 路径
 *  - maxBodyBytes         请求体上限（默认 8MB）
 *  - upstreamTimeoutMs    上游超时（默认 120s）
 *  - engineTimeoutMs      引擎子进程超时（默认 30s）
 *  - log                  (line:string)=>void 日志函数（默认写 stderr）
 */
export function createQuizProxy(options = {}) {
  const repoRoot = resolveRepoRoot(options.repoRoot);
  const enginePath = path.join(repoRoot, ENGINE_REL);
  if (!fs.existsSync(enginePath)) {
    throw new Error('找不到题库引擎：' + enginePath);
  }
  const stateDir = path.resolve(
    options.stateDir ||
      process.env.QUIZ_PROXY_STATE_DIR ||
      path.join(repoRoot, '.cache', 'quiz-proxy', 'state'),
  );
  fs.mkdirSync(stateDir, { recursive: true });
  const credentialsPath =
    options.credentialsPath ||
    process.env.QUIZ_PROXY_CREDENTIALS ||
    defaultCredentialsPath();
  const cardsMode = resolveCardsMode(options);
  const app = {
    repoRoot,
    runDir: options.runDir || path.join(repoRoot, '.cache', 'quiz-proxy', 'run'),
    buildRev: buildRev(__filename),
    enginePath,
    stateDir,
    upstream:
      options.upstream || process.env.QUIZ_PROXY_UPSTREAM || DEFAULT_UPSTREAM,
    upstreamModel:
      options.upstreamModel || process.env.QUIZ_PROXY_UPSTREAM_MODEL || SLOW_MODEL,
    opencodeSession:
      options.opencodeSession !== undefined
        ? String(options.opencodeSession)
        : String(process.env.QUIZ_PROXY_OPENCODE_SESSION || 'quiz-proxy'),
    apiKey: options.apiKey !== undefined ? String(options.apiKey) : loadApiKey({ credentialsPath }),
    credentialsPath,
    maxBodyBytes: Number.isFinite(options.maxBodyBytes)
      ? options.maxBodyBytes
      : Number(process.env.QUIZ_PROXY_MAX_BODY_BYTES || DEFAULT_MAX_BODY_BYTES),
    upstreamTimeoutMs: Number.isFinite(options.upstreamTimeoutMs)
      ? options.upstreamTimeoutMs
      : Number(process.env.QUIZ_PROXY_UPSTREAM_TIMEOUT_MS || DEFAULT_UPSTREAM_TIMEOUT_MS),
    engineTimeoutMs: Number.isFinite(options.engineTimeoutMs)
      ? options.engineTimeoutMs
      : Number(process.env.QUIZ_PROXY_ENGINE_TIMEOUT_MS || DEFAULT_ENGINE_TIMEOUT_MS),
    stats: { fastturns: 0, slowturns: 0 },
    // 卡片模式：learn = 真卡片插件 learn_card（QUIZ_PROXY_CARDS=learn 才开）；
    // ask = 旧的点选卡片 ask_user_question（QUIZ_PROXY_CARDS=1，legacy）；
    // off = 纯文本快路径（**默认**，DSH 不认 tool_call 时的安全档）。
    cardsMode,
    cardsEnabled: cardsMode !== 'off',
    /* I2/I3：默认单学习流（stateKey='default'）；perConversation 供隔离/调试用。 */
    perConversation: options.perConversation === true,
    questionHandles: new Map(), // stateKey → { qid, nonce, issueId, at }（屏幕上的题）
    gradeResults: new Map(), // stateKey+\u0001+qid+\u0001+nonce → 第一次判分结果（I3 幂等）
    pendingCards: new Map(),
    cardReplies: new Map(),
    cardSeqs: new Map(),
    locks: new Map(),
    issuedKinds: new Map(),
    log: typeof options.log === 'function' ? options.log : defaultLogger,
  };

  const server = http.createServer((req, res) => {
    handleRequest(req, res, app).catch((error) => {
      app.log('unhandled ' + String((error && error.stack) || error));
      if (!res.headersSent) {
        sendJson(res, 500, errorPayload(500, '代理内部错误', 'server_error', 'proxy_internal_error'));
      } else if (!res.writableEnded) {
        res.end();
      }
    });
  });
  return { server, app };
}

// ───────────────────────── CLI 入口 ─────────────────────────
function main() {
  const host = process.env.QUIZ_PROXY_HOST || DEFAULT_HOST;
  const port = Number(process.env.QUIZ_PROXY_PORT || DEFAULT_PORT);
  const { server, app } = createQuizProxy();
  server.on('error', (error) => {
    process.stderr.write(
      `[quiz-proxy] 启动失败：${String((error && error.message) || error)}\n`,
    );
    process.exitCode = 1;
  });
  server.listen(port, host, () => {
    const address = server.address();
    const actualPort = address && typeof address === 'object' ? address.port : port;
    /* 记录这次构建；指纹变了 ⇒ 下一条回复带「服务已更新」提醒 */
    const announced = recordBuild(app.runDir, { rev: app.buildRev, mode: app.cardsMode, pid: process.pid });
    if (announced) app.log(`build changed → announce pending (rev=${app.buildRev})`);
    process.stdout.write(
      JSON.stringify({
        ok: true,
        listening: `http://${host}:${actualPort}`,
        repo: app.repoRoot,
        stateDir: app.stateDir,
        upstream: app.upstream,
        keyLoaded: Boolean(app.apiKey),
      }) + '\n',
    );
    app.log(
      `quiz-proxy 已启动 http://${host}:${actualPort}（repo=${app.repoRoot} state=${app.stateDir} upstream=${app.upstream} cards=${app.cardsMode} key=${app.apiKey ? 'loaded' : 'MISSING'}）`,
    );
  });
  const shutdown = () => {
    server.close(() => process.exit(0));
    setTimeout(() => process.exit(0), 3000).unref();
  };
  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);
}

if (process.argv[1] && path.resolve(process.argv[1]) === __filename) {
  main();
}


/**
 * Start the quiz fast-path service inside the current process on a loopback
 * ephemeral port. The unified plugin uses this instead of a separate
 * `quiz-proxy` process / `quiz-auto` model provider: the same engine, state
 * machine and I2/I3/I4 invariants run inside the DSH Host.
 *
 * @param options - forwarded to {@link createQuizProxy}; `repoRoot`,
 *   `stateDir`, `runDir` and `cardsMode` are the ones the plugin sets.
 * @returns `{ port, url, close }` once the server is accepting connections.
 */
export async function startQuizService(options = {}) {
  const { server, app } = createQuizProxy(options);
  await new Promise((resolve, reject) => {
    const onError = (error) => {
      server.off('error', onError);
      reject(error);
    };
    server.once('error', onError);
    server.listen(
      Number.isInteger(options.port) ? options.port : 0,
      options.host || '127.0.0.1',
      () => {
        server.off('error', onError);
        resolve();
      },
    );
  });
  const address = server.address();
  const port = address !== null && typeof address === 'object' ? address.port : null;
  if (!Number.isInteger(port)) {
    await new Promise((resolve) => server.close(() => resolve()));
    throw new Error('quiz fast-path service failed to bind a loopback port');
  }
  return {
    port,
    url: `http://127.0.0.1:${port}`,
    app,
    close: () => new Promise((resolve) => server.close(() => resolve())),
  };
}
