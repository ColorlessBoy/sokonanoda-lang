/**
 * Host-half tests for `sokonanoda-dsh`.
 *
 * They pin the user-visible contract:
 *   - the data directory is user-level (`~/.dsh/state/sokonanoda-dsh/`) and
 *     never inside the repository;
 *   - quiz decisions match the course engine's `classifyFast` (the same helper
 *     the in-process service uses);
 *   - the in-process service answers a quiz turn and the reply becomes a
 *     `learn-card` fence (display-only);
 *   - the `llm/stream` listener short-circuits a quiz request and falls through
 *     to `next()` for everything else;
 *   - `apply()` registers exactly one fast-path listener and one `sokonanoda_fast_think`
 *     tool, and never breaks Host startup when the engine/service is missing;
 *   - the learner command and the agent tool share one per-session switch.
 *
 *   node --test test/host.test.mjs
 */
import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

import {
  defaultDataDir,
  readFileConfig,
  normalizeConfig,
  loadQuizCore,
  decideFast,
  toProxyMessages,
  callProxy,
  composeReplyText,
  isLoopRequestShape,
  apply,
  findRepoRoot,
  LEARN_CARD_TOOL_DECLARATION,
} from '../index.js'

const core = await loadQuizCore()
const REPO = findRepoRoot()

const text = (value) => [{ type: 'text', text: value }]
const systemMsg = (value = 'SYS') => ({ role: 'system', source: { kind: 'system-prompt' }, content: text(value) })
const userMsg = (value) => ({ role: 'user', source: { kind: 'user' }, content: text(value) })
const assistantMsg = (value) => ({ role: 'assistant', source: { kind: 'model' }, content: text(value) })
const toolMsg = (value) => ({ role: 'tool', source: { kind: 'tool' }, content: text(value) })

function withEnv(name, value, body) {
  const saved = process.env[name]
  try {
    if (value === undefined) delete process.env[name]
    else process.env[name] = value
    return body()
  } finally {
    if (saved === undefined) delete process.env[name]
    else process.env[name] = saved
  }
}

test('defaultDataDir: user-level only, never inside the repository', () => {
  withEnv('SOKO_DSH_DATA', '/tmp/sokonanoda-dsh-test-data', () => {
    assert.equal(defaultDataDir(), '/tmp/sokonanoda-dsh-test-data')
  })
  withEnv('SOKO_DSH_DATA', undefined, () => {
    const dir = defaultDataDir()
    assert.ok(dir.endsWith(path.join('state', 'sokonanoda-dsh')), 'unexpected default: ' + dir)
    assert.ok(!path.resolve(dir).startsWith(path.resolve(REPO) + path.sep), 'data dir must not be inside the repo: ' + dir)
  })
})

test('normalizeConfig: config.json is a base, the bundle row wins, paths are derived', () => {
  const settings = normalizeConfig(
    { enabled: false, timeoutMs: 1234 },
    { dataDir: '/tmp/base-data', decisionsLimit: 5, enabled: true },
  )
  assert.equal(settings.enabled, false)
  assert.equal(settings.timeoutMs, 1234)
  assert.equal(settings.dataDir, path.resolve('/tmp/base-data'))
  assert.equal(settings.decisionsLimit, 5)
  assert.equal(settings.historyPath, path.join(path.resolve('/tmp/base-data'), 'history', 'decisions.jsonl'))
  assert.equal(settings.quizStateDir, path.join(path.resolve('/tmp/base-data'), 'quiz'))
  assert.equal(settings.cards, true)
})

test('readFileConfig: missing or malformed files are ignored', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-config-'))
  assert.deepEqual(readFileConfig(dir), {})
  fs.writeFileSync(path.join(dir, 'config.json'), '{oops')
  assert.deepEqual(readFileConfig(dir), {})
  fs.writeFileSync(path.join(dir, 'config.json'), '{"enabled":false}')
  assert.deepEqual(readFileConfig(dir), { enabled: false })
})

test('decideFast: start / continue / letter gating follows the engine classifier', () => {
  assert.equal(decideFast({ messages: [systemMsg(), userMsg('开始做题')] }, core).fast.type, 'next')
  assert.equal(decideFast({ messages: [systemMsg(), userMsg('继续')] }, core), null)
  assert.equal(decideFast({ messages: [systemMsg(), userMsg('A')] }, core), null)
  const answer = decideFast({
    messages: [systemMsg(), userMsg('开始做题'), assistantMsg('📝 题目…'), userMsg('B')],
  }, core)
  assert.equal(answer.fast.type, 'answer')
  assert.equal(answer.fast.letter, 'B')
  assert.equal(decideFast({
    messages: [systemMsg(), userMsg('开始做题'), assistantMsg('x'), toolMsg('tool result')],
  }, core), null)
})

test('decideFast: map only on an exact short progress command, never a sentence containing 进度', () => {
  const exact = ['进度', '看看进度', '学习地图', '学到哪了']
  for (const text of exact) {
    const decision = decideFast({ messages: [systemMsg(), userMsg(text)] }, core)
    assert.ok(decision !== null && decision.fast.type === 'map', text + ' must be a map command')
  }
  const sentence = '请创建产品agent和老师agent两个人，另一个根据学生水平调整课程进度'
  assert.equal(decideFast({ messages: [systemMsg(), userMsg(sentence)] }, core), null,
    'a sentence that merely contains 进度 must fall through to the model')
  for (const text of ['下一题', '下一道', '换一题', '再来一道']) {
    const decision = decideFast({ messages: [systemMsg(), userMsg(text)] }, core)
    assert.ok(decision !== null && decision.fast.type === 'next', text + ' must open a quiz session')
    assert.equal(decision.fast.noLesson, true, text + ' should request a question directly')
  }
})

test('isLoopRequestShape: frozen, purpose-less, session-scoped requests only', () => {
  assert.equal(isLoopRequestShape(Object.freeze({ messages: [userMsg('x')], sessionId: 's1', provider: 'p', model: 'm' })), true)
  assert.equal(isLoopRequestShape({ messages: [userMsg('x')], sessionId: 's1' }), false)
  assert.equal(isLoopRequestShape(Object.freeze({ messages: [userMsg('x')], sessionId: 's1', purpose: 'compaction' })), false)
})

test('composeReplyText: learn_card tool calls become a display-only fence', () => {
  const reply = composeReplyText({
    choices: [{
      message: {
        role: 'assistant',
        content: '✅ 对了！',
        tool_calls: [{
          type: 'function',
          function: { name: 'learn_card', arguments: JSON.stringify({ kind: 'question', payload: { stem: '下一题' } }) },
        }],
      },
    }],
  })
  assert.match(reply, /^✅ 对了！\n\n```learn-card\n/)
  const json = reply.slice(reply.indexOf('```learn-card\n') + '```learn-card\n'.length, reply.lastIndexOf('\n```'))
  assert.deepEqual(JSON.parse(json), { kind: 'question', payload: { stem: '下一题' } })
  assert.equal(composeReplyText({ choices: [{ message: { content: '纯文本' } }] }), '纯文本')
  assert.equal(composeReplyText({ choices: [{ message: { tool_calls: [{ type: 'function', function: { name: 'other' } }] } }] }), '')
})

test('in-process quiz service + fence reply + letter grading', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-quiz-'))
  const service = await core.startQuizService({
    repoRoot: REPO,
    stateDir: path.join(dataDir, 'quiz'),
    runDir: path.join(dataDir, 'logs'),
    cardsMode: 'learn',
  })
  try {
    const settings = { proxyUrl: `${service.url}/v1/chat/completions`, timeoutMs: 20000 }
    const first = [systemMsg(), userMsg('开始做题')]
    assert.equal(decideFast({ messages: first }, core).fast.type, 'next')
    const startReply = composeReplyText(await callProxy(settings, {
      model: 'quiz-auto', stream: false, tools: [LEARN_CARD_TOOL_DECLARATION],
      messages: toProxyMessages({ sessionId: 'unit-test', messages: first }, core),
    }, undefined))
    assert.match(startReply, /```learn-card\n/)
    const startCard = JSON.parse(startReply.match(/```learn-card\n([\s\S]*?)\n```/)[1])
    assert.equal(startCard.kind, 'question')
    assert.equal(startCard.payload.options.length, 4)

    const second = [systemMsg(), userMsg('开始做题'), assistantMsg(startReply), userMsg('A')]
    assert.equal(decideFast({ messages: second }, core).fast.type, 'answer')
    const answerReply = composeReplyText(await callProxy(settings, {
      model: 'quiz-auto', stream: false, tools: [LEARN_CARD_TOOL_DECLARATION],
      messages: toProxyMessages({ sessionId: 'unit-test', messages: second }, core),
    }, undefined))
    assert.match(answerReply, /```learn-card\n/)

    /* Progress / hotspot card: the engine snapshots become a display-only payload. */
    const stateFile = path.join(dataDir, 'quiz', 'default.json')
    const progress = await core.buildProgressCardPayload(service.app, stateFile)
    assert.ok(progress !== null, 'progress payload must exist after one answer')
    assert.ok(progress.summary.asked >= 1, 'progress must count the answered question')
    assert.ok(Array.isArray(progress.chapters) && progress.chapters.length > 0, 'chapter progress missing')
    assert.ok(Array.isArray(progress.heatRows) && progress.heatRows.length > 0, 'mastery heat rows missing')
    assert.ok(Array.isArray(progress.hotspots), 'hotspots must be an array')
    assert.ok(Array.isArray(progress.recent) && progress.recent.length >= 1, 'recent answers missing')

    /* 「进度」 is a fast-path map turn that returns the progress card. */
    const progressMessages = [systemMsg(), userMsg('进度')]
    assert.equal(decideFast({ messages: progressMessages }, core).fast.type, 'map')
    const progressText = composeReplyText(await callProxy(settings, {
      model: 'quiz-auto', stream: false, tools: [LEARN_CARD_TOOL_DECLARATION],
      messages: toProxyMessages({ sessionId: 'unit-test', messages: progressMessages }, core),
    }, undefined))
    const progressFence = progressText.match(/```learn-card\n([\s\S]*?)\n```/)
    assert.ok(progressFence, 'progress turn must return a learn-card fence: ' + progressText.slice(0, 200))
    const progressCard = JSON.parse(progressFence[1])
    assert.equal(progressCard.kind, 'progress')
    assert.ok(progressCard.payload.summary.asked >= 1)
  } finally {
    await service.close()
  }
})

test('decideFast: a freeform message is forwarded only while an open question card is on screen', () => {
  const openCard = '```learn-card\n' + JSON.stringify({
    kind: 'question',
    payload: { questionKind: 'short', stem: '用一句话解释：为什么证 `A ⊆ B` 只需要一个方向？' },
  }) + '\n```'
  const choiceCard = '```learn-card\n' + JSON.stringify({
    kind: 'question',
    payload: { stem: '选一个', options: [{ key: 'A', text: 'a' }, { key: 'B', text: 'b' }] },
  }) + '\n```'
  const openAnswer = '因为 ⊆ 展开是一条箭头，对所有元素单向推出。'
  const openDecision = decideFast({
    messages: [systemMsg(), assistantMsg(openCard), userMsg(openAnswer)],
  }, core)
  assert.equal(openDecision.fast.type, 'open', 'freeform after an open card must be graded locally')
  const choiceDecision = decideFast({
    messages: [systemMsg(), assistantMsg(choiceCard), userMsg(openAnswer)],
  }, core)
  assert.equal(choiceDecision, null, 'freeform after a choice card is not a local answer')
})

test('buildLearnQuestionCard: non-choice questions become open cards with no options', () => {
  const short = {
    id: 'I.1.open-1', kind: 'short', chapter: 'I.1', unit: '② 子集、空集与包含三律',
    prompt: '用一句话解释：为什么证 `A ⊆ B` 只需要一个方向？',
  }
  const toolCall = core.buildLearnQuestionCard(short, null)
  const args = JSON.parse(toolCall.function.arguments)
  assert.equal(args.kind, 'question')
  assert.equal(args.payload.questionKind, 'short')
  assert.deepEqual(args.payload.options, [])
  assert.equal(args.payload.stem, short.prompt)
})

test('card mode: first wrong choice hints and re-sends the same card; second wrong grades and advances', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-retry-'))
  const service = await core.startQuizService({
    repoRoot: REPO,
    stateDir: path.join(dataDir, 'quiz'),
    runDir: path.join(dataDir, 'logs'),
    cardsMode: 'learn',
  })
  const stateFile = path.join(dataDir, 'quiz', 'default.json')
  const readState = () => JSON.parse(fs.readFileSync(stateFile, 'utf8'))
  const fenceOf = (reply) => {
    const match = reply.match(/```learn-card\n([\s\S]*?)\n```/)
    return match === null ? null : JSON.parse(match[1])
  }
  const ask = async (texts) => composeReplyText(await callProxy({
    proxyUrl: `${service.url}/v1/chat/completions`, timeoutMs: 20000,
  }, {
    model: 'quiz-auto', stream: false, tools: [LEARN_CARD_TOOL_DECLARATION],
    messages: toProxyMessages({ sessionId: 'retry-test', messages: [systemMsg(), ...texts.map((text) => userMsg(text))] }, core),
  }, undefined))
  try {
    const startReply = await ask(['开始做题'])
    const startCard = fenceOf(startReply)
    assert.equal(startCard.kind, 'question')
    let currentStem = startCard.payload.stem

    /* Find a wrong letter: on a correct answer the engine advances to the next
     * question; on the first wrong answer it must hint WITHOUT advancing. */
    let found = null
    for (let round = 0; round < 6 && found === null; round += 1) {
      for (const letter of ['A', 'B', 'C', 'D']) {
        const before = readState()
        const reply = await ask([letter])
        if (reply.includes('再想一次')) {
          found = { letter, reply, before, stem: currentStem }
          break
        }
        const nextCard = fenceOf(reply)
        if (nextCard !== null && nextCard.kind === 'question') currentStem = nextCard.payload.stem
      }
      if (found === null) {
        const nextReply = await ask(['下一题'])
        const nextCard = fenceOf(nextReply)
        if (nextCard !== null && nextCard.kind === 'question') currentStem = nextCard.payload.stem
      }
    }
    assert.ok(found !== null, 'expected at least one wrong choice to trigger the retry path')
    const afterRetry = readState()
    assert.equal(afterRetry.issued_qid, found.before.issued_qid, 'a hinted retry must not advance the issued question')
    assert.equal(afterRetry.history.length, found.before.history.length, 'a hinted retry must not record history')
    assert.match(found.reply, /💡/, 'retry feedback must carry a hint')
    const retryCard = fenceOf(found.reply)
    assert.ok(retryCard !== null && retryCard.kind === 'question', 'retry must re-send the same question card')
    assert.equal(retryCard.payload.stem, found.stem, 'retry card must be the same question')

    const secondReply = await ask([found.letter])
    assert.ok(!secondReply.includes('再想一次'), 'second wrong attempt must grade normally, not hint again')
    const afterSecond = readState()
    assert.equal(afterSecond.history.length, found.before.history.length + 1, 'second wrong attempt must be recorded')
    assert.ok(afterSecond.issued_qid !== found.before.issued_qid || fenceOf(secondReply) !== null,
      'second wrong attempt must advance or re-issue a question')
  } finally {
    await service.close()
  }
})

test('card mode: 不懂 returns a knowledge card without advancing the question', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-knowledge-'))
  const service = await core.startQuizService({
    repoRoot: REPO,
    stateDir: path.join(dataDir, 'quiz'),
    runDir: path.join(dataDir, 'logs'),
    cardsMode: 'learn',
  })
  const stateFile = path.join(dataDir, 'quiz', 'default.json')
  const ask = async (texts) => composeReplyText(await callProxy({
    proxyUrl: `${service.url}/v1/chat/completions`, timeoutMs: 20000,
  }, {
    model: 'quiz-auto', stream: false, tools: [LEARN_CARD_TOOL_DECLARATION],
    messages: toProxyMessages({ sessionId: 'knowledge-test', messages: [systemMsg(), ...texts.map((text) => userMsg(text))] }, core),
  }, undefined))
  try {
    await ask(['开始做题'])
    const before = JSON.parse(fs.readFileSync(stateFile, 'utf8'))
    const explainReply = await ask(['不懂'])
    assert.match(explainReply, /```learn-card\n/)
    const card = JSON.parse(explainReply.match(/```learn-card\n([\s\S]*?)\n```/)[1])
    assert.equal(card.kind, 'knowledge', '不懂 must return a knowledge card')
    assert.equal(typeof card.payload.title, 'string')
    const after = JSON.parse(fs.readFileSync(stateFile, 'utf8'))
    assert.equal(after.issued_qid, before.issued_qid, 'knowledge card must not advance the question')
  } finally {
    await service.close()
  }
})

test('card mode: an open question card retries with missed points, then grades and advances', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-open-'))
  const service = await core.startQuizService({
    repoRoot: REPO,
    stateDir: path.join(dataDir, 'quiz'),
    runDir: path.join(dataDir, 'logs'),
    cardsMode: 'learn',
  })
  const stateFile = path.join(dataDir, 'quiz', 'default.json')
  const readState = () => JSON.parse(fs.readFileSync(stateFile, 'utf8'))
  const fenceOf = (reply) => {
    const match = reply.match(/```learn-card\n([\s\S]*?)\n```/)
    return match === null ? null : JSON.parse(match[1])
  }
  const ask = async (texts) => composeReplyText(await callProxy({
    proxyUrl: `${service.url}/v1/chat/completions`, timeoutMs: 20000,
  }, {
    model: 'quiz-auto', stream: false, tools: [LEARN_CARD_TOOL_DECLARATION],
    messages: toProxyMessages({ sessionId: 'open-test', messages: [systemMsg(), ...texts.map((text) => userMsg(text))] }, core),
  }, undefined))
  try {
    const startReply = await ask(['开始做题'])
    assert.ok(fenceOf(startReply) !== null, 'quiz should start with a question card')
    /* Seed a deterministic open question instead of relying on a random draw. */
    const seeded = readState()
    seeded.issued_qid = 'I.1.open-1'
    seeded.issued_card = null
    fs.writeFileSync(stateFile, JSON.stringify(seeded))

    const before = readState()
    const retryReply = await ask(['不知道'])
    assert.match(retryReply, /还没答全/, 'incomplete open answer must ask for a retry')
    assert.ok(!retryReply.includes('参考：'), 'retry must not reveal the reference answer yet')
    const retryCard = fenceOf(retryReply)
    assert.ok(retryCard !== null && retryCard.kind === 'question', 'retry must re-send the open question card')
    assert.equal(retryCard.payload.questionKind, 'short')
    assert.equal(retryCard.payload.stem, '用一句话解释：为什么证 `A ⊆ B` 只需要一个方向？')
    const afterRetry = readState()
    assert.equal(afterRetry.issued_qid, 'I.1.open-1', 'open retry must not advance the question')
    assert.equal(afterRetry.history.length, before.history.length, 'open retry must not record history')

    const secondReply = await ask(['不知道'])
    assert.ok(!secondReply.includes('还没答全'), 'second incomplete attempt must grade normally')
    assert.match(secondReply, /参考：/, 'second attempt should reveal the reference answer')
    assert.ok(fenceOf(secondReply) !== null, 'graded open answer should come with the next card')
    const afterSecond = readState()
    assert.equal(afterSecond.history.length, before.history.length + 1, 'attempts are recorded as one final answer only')
    assert.ok(afterSecond.issued_qid !== 'I.1.open-1', 'graded open answer must advance the question')
  } finally {
    await service.close()
  }
})

test('apply(): /sokonanoda-fast-think gates the fast path per session; quiz hits fast only after opt-in', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-apply-'))
  const host = captureHost()
  await apply(host.ctx, { dataDir, timeoutMs: 20000 })
  try {
    assert.equal(host.listeners.length, 1, 'expected one llm/stream listener')
    assert.equal(host.tools.length, 1, 'expected one tool')
    assert.equal(host.tools[0].name, 'sokonanoda_fast_think')
    assert.equal(host.commands.length, 1, 'expected one /sokonanoda-fast-think command')
    assert.equal(host.commands[0].name, 'sokonanoda-fast-think')
    assert.ok(!JSON.stringify(host.tools).includes('learn_card'), 'cards must be display-only: no learn_card tool')

    /* Default OFF: even a quiz opener falls through to the model. */
    const quizRequest = () => Object.freeze({
      provider: 'opencode-go-7',
      model: 'deepseek-v4.1-flash',
      messages: [systemMsg(), userMsg('开始做题')],
      sessionId: 'apply-test',
      signal: new AbortController().signal,
    })
    const offSlow = slowStream()
    await collect(host.listeners[0](quizRequest(), offSlow))
    assert.equal(offSlow.called, true, 'fast path must stay off until the session opts in')

    /* Both callers share one switch: the learner's command and the agent's tool. */
    const command = (sessionId, rawInput) => host.commands[0].handler({
      agent: { session: { id: sessionId } }, rawInput,
    })
    const statusBefore = command('apply-test', ' status')
    assert.equal(statusBefore.kind, 'success')
    assert.match(statusBefore.text, /已关闭/)

    /* The agent tool cannot toggle without a session identity, and never guesses one. */
    const contextless = await host.tools[0].execute({ action: 'enable' }, {})
    assert.equal(contextless.ok, false)
    assert.match(contextless.error, /session/i)

    /* Agent toggles on the explicit learner request: sokonanoda_fast_think enable. */
    const toolEnabled = await host.tools[0].execute(
      { action: 'enable' },
      { agent: { session: { id: 'apply-test' } } },
    )
    assert.equal(toolEnabled.ok, true)
    assert.equal(toolEnabled.enabledSession, true)
    assert.equal(toolEnabled.sessionId, 'apply-test')
    const sessionsFile = path.join(dataDir, 'sessions.json')
    assert.equal(JSON.parse(fs.readFileSync(sessionsFile, 'utf8'))['apply-test'], true, 'switch must persist')
    assert.match(command('apply-test', ' status').text, /已开启/, 'the human command sees the agent toggle')

    const fastSlow = slowStream()
    const chunks = await collect(host.listeners[0](quizRequest(), fastSlow))
    const reply = chunks.filter((chunk) => chunk.type === 'text-delta').map((chunk) => chunk.text).join('')
    assert.match(reply, /```learn-card\n/)
    assert.equal(fastSlow.called, false, 'enabled quiz turn must not reach the model')
    assert.equal(chunks.at(-1).type, 'finish')

    /* Another session stays off. */
    const otherSlow = slowStream()
    await collect(host.listeners[0](Object.freeze({
      provider: 'opencode-go-7', model: 'deepseek-v4.1-flash',
      messages: [systemMsg(), userMsg('开始做题')], sessionId: 'other-session',
      signal: new AbortController().signal,
    }), otherSlow))
    assert.equal(otherSlow.called, true, 'other sessions must not inherit the switch')

    /* Ordinary text in the enabled session still goes to the model. */
    const ordinary = Object.freeze({
      provider: 'opencode-go-7',
      model: 'deepseek-v4.1-flash',
      messages: [systemMsg(), userMsg('解释一下外延公理')],
      sessionId: 'apply-test',
      signal: new AbortController().signal,
    })
    const slowAgain = slowStream()
    const fallback = await collect(host.listeners[0](ordinary, slowAgain))
    assert.equal(slowAgain.called, true, 'ordinary text must reach the model')
    assert.equal(fallback.filter((chunk) => chunk.type === 'text-delta').map((chunk) => chunk.text).join(''), 'SLOW')

    /* Command errors are explicit; /sokonanoda-fast-think off disables again. */
    assert.equal(command('apply-test', ' maybe').kind, 'error')
    assert.equal(command('apply-test', ' off').kind, 'success')
    assert.equal(JSON.parse(fs.readFileSync(sessionsFile, 'utf8'))['apply-test'], undefined)
    const afterOff = slowStream()
    await collect(host.listeners[0](quizRequest(), afterOff))
    assert.equal(afterOff.called, true, 'switch off must restore the model path')

    /* sokonanoda_fast_think status reports per-session enablement; progress reads the card payload. */
    const status = await host.tools[0].execute({ action: 'status' })
    assert.equal(status.ok, true)
    assert.ok(Array.isArray(status.sessions), 'status must list enabled sessions')
    const progressTool = await host.tools[0].execute({ action: 'progress' })
    assert.equal(progressTool.ok, true)
    assert.ok(progressTool.payload !== null && progressTool.payload !== undefined, 'progress payload missing')
    assert.ok(Array.isArray(progressTool.payload.heatRows) && progressTool.payload.heatRows.length > 0, 'progress heat rows missing')
    assert.ok(progressTool.payload.summary !== null && typeof progressTool.payload.summary === 'object', 'progress summary missing')
    assert.ok(progressTool.payload.summary.unitTotal >= 1, 'progress tool must see the course units')
  } finally {
    await host.disposeAll()
  }
})

/* ---------------------------------------------------------------- helpers */

function captureHost() {
  const listeners = []
  const tools = []
  const commands = []
  const disposers = []
  const ctx = {
    effect(callback) {
      const disposer = callback()
      disposers.push(typeof disposer === 'function' ? disposer : () => {})
      return () => {}
    },
    on(event, listener) {
      if (event === 'llm/stream') listeners.push(listener)
      return () => {}
    },
    commands: {
      register(definition) {
        commands.push(definition)
        return () => {}
      },
    },
    tools: {
      register(definition) {
        tools.push(definition)
        return () => {}
      },
    },
  }
  return {
    ctx,
    listeners,
    tools,
    commands,
    async disposeAll() {
      for (const disposer of disposers.reverse()) {
        try { disposer() } catch { /* best effort */ }
      }
      await new Promise((resolve) => setTimeout(resolve, 30))
    },
  }
}

function slowStream() {
  const holder = { called: false }
  const next = () => (async function* () {
    holder.called = true
    yield { type: 'text-delta', index: 0, text: 'SLOW' }
    yield { type: 'finish', reason: { kind: 'stop' } }
  })()
  Object.defineProperty(next, 'called', { get: () => holder.called })
  return next
}

async function collect(iterable) {
  const chunks = []
  for await (const chunk of iterable) chunks.push(chunk)
  return chunks
}
