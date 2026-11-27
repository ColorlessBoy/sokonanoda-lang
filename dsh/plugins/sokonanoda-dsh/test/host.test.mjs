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
import { EventEmitter } from 'node:events'
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
  parseFileResource,
  isPathWithin,
  resolveAllowedStatePath,
  runSokoCommand,
  LEARN_CARD_TOOL_DECLARATION,
} from '../index.js'
import { assembleSubmission, evaluateGrade, expectedDeclarationName } from '../lib/code-grade.mjs'
import {
  PANEL_EXERCISE_PATH,
  PANEL_GRADE_PATH,
  normalizeExerciseId,
  resolvePanelExercise,
  panelExerciseView,
  gradePanelSubmission,
  handleExerciseRequest,
  handleGradeRequest,
} from '../lib/exercise-panel.mjs'
import {
  NOTES_SCHEMA,
  attachHistory,
  emptyBook,
  extractKnowledgeCards,
  matchNoteForQuestion,
  noteFromCard,
  readLedger,
  readQuizHistory,
  upsertKnowledgeCard,
} from '../lib/class-notes.mjs'
import { renderNotesHtml, renderNotesMarkdown } from '../lib/class-notes-view.mjs'
import { allowedFigureFiles } from '../lib/class-notes-figures.mjs'

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

    /* Pick the actually-correct letter from the bank instead of assuming "A":
     * the bank was revised (2026-11-27) and answer positions moved. */
    const stateFile = path.join(dataDir, 'quiz', 'default.json')
    const issuedQid = JSON.parse(fs.readFileSync(stateFile, 'utf8')).issued_qid
    const bank = JSON.parse(fs.readFileSync(
      path.join(REPO, 'courses/set-theory/knowledge/data/questions.json'), 'utf8'))
    const issuedQuestion = bank.questions.find((question) => question.id === issuedQid)
    assert.ok(issuedQuestion !== undefined, 'issued question must exist in the bank: ' + issuedQid)
    const correctLetter = 'ABCD'[Number(issuedQuestion.answer) - 1]
    const second = [systemMsg(), userMsg('开始做题'), assistantMsg(startReply), userMsg(correctLetter)]
    assert.equal(decideFast({ messages: second }, core).fast.type, 'answer')
    const answerReply = composeReplyText(await callProxy(settings, {
      model: 'quiz-auto', stream: false, tools: [LEARN_CARD_TOOL_DECLARATION],
      messages: toProxyMessages({ sessionId: 'unit-test', messages: second }, core),
    }, undefined))
    assert.match(answerReply, /```learn-card\n/)

    /* Progress / hotspot card: the engine snapshots become a display-only payload. */
    const progress = await core.buildProgressCardPayload(service.app, stateFile)
    assert.ok(progress !== null, 'progress payload must exist after one answer')
    assert.ok(progress.summary.asked >= 1, 'progress must count the answered question')
    assert.ok(Array.isArray(progress.chapters) && progress.chapters.length > 0, 'chapter progress missing')
    assert.ok(Array.isArray(progress.heatRows) && progress.heatRows.length > 0, 'mastery heat rows missing')
    assert.ok(Array.isArray(progress.hotspots), 'hotspots must be an array')
    assert.ok(Array.isArray(progress.recent) && progress.recent.length >= 1, 'recent answers missing')
    /* C9.5 learning map fields come from engine.py map (one source of truth). */
    assert.ok(Number(progress.summary.kpsTotal) > 0, 'learning map KP total missing')
    assert.ok(Number(progress.summary.solvedKps) >= 1, 'learning map must count the solved KP')
    assert.ok(Number(progress.summary.codeTotal) > 0, 'learning map must merge code-exercise totals')
    assert.ok(progress.chapters.every((chapter) => 'solved' in chapter && 'todo' in chapter && 'weak' in chapter),
      'chapter progress must carry learning-map counts')

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

test('classifyFast: 我不知道 is a local explain request, not an answer', () => {
  const fast = core.classifyFast('quiz-auto', '我不知道')
  assert.equal(fast.type, 'explain')
  const withPrefix = core.classifyFast('quiz-auto', '我不知道这题怎么做')
  assert.equal(withPrefix, null, 'longer sentences must stay with the model')
})

test('classifyFast: 上一题 is a read-only review request', () => {
  for (const text of ['上一题', '回到刚才那道题', '回看上一题']) {
    const fast = core.classifyFast('quiz-auto', text)
    assert.equal(fast.type, 'back', text + ' must route to review')
  }
})

test('composeReplyText: card sources gain sidebar file coordinates', () => {
  const reply = composeReplyText({
    choices: [{
      message: {
        content: '',
        tool_calls: [{
          type: 'function',
          function: {
            name: 'learn_card',
            arguments: JSON.stringify({
              kind: 'knowledge',
              payload: { title: '量词换序', source: 'units/I.2/unit07-functions.sokonanoda:194' },
            }),
          },
        }],
      },
    }],
  })
  const match = reply.match(/```learn-card\n([\s\S]*?)\n```/)
  assert.ok(match !== null, 'composeReplyText must emit a learn-card fence')
  const card = JSON.parse(match[1])
  const link = card.payload.sourceLink
  assert.ok(link !== null && typeof link === 'object', 'sourceLink missing')
  assert.equal(link.line, 194)
  assert.equal(link.relativePath, 'courses/set-theory/units/I.2/unit07-functions.sokonanoda')
  assert.ok(link.path.endsWith('/courses/set-theory/units/I.2/unit07-functions.sokonanoda'))
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

test('card mode: 下一题 replays an unanswered question instead of advancing', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-replay-'))
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
    messages: toProxyMessages({ sessionId: 'replay-test', messages: [systemMsg(), ...texts.map((text) => userMsg(text))] }, core),
  }, undefined))
  try {
    let startReply = await ask(['开始做题'])
    let startCard = fenceOf(startReply)
    for (let i = 0; i < 3 && (startCard === null || startCard.kind !== 'question'); i += 1) {
      startReply = await ask(['继续'])
      startCard = fenceOf(startReply)
    }
    assert.ok(startCard !== null && startCard.kind === 'question', 'quiz should reach a question')
    const firstStem = startCard.payload.stem
    const firstQid = readState().issued_qid

    const replayReply = await ask(['下一题'])
    assert.match(replayReply, /还没作答/, 'plain 下一题 must explain that the current question is still open')
    const replayCard = fenceOf(replayReply)
    assert.ok(replayCard !== null && replayCard.kind === 'question')
    assert.equal(replayCard.payload.stem, firstStem, 'plain 下一题 must replay the same question')
    assert.equal(readState().issued_qid, firstQid, 'replaying must not advance')

    const explainReply = await ask(['我不知道'])
    const explainCard = fenceOf(explainReply)
    assert.ok(explainCard !== null && explainCard.kind === 'knowledge', '我不知道 must return a knowledge card')
    const afterExplain = await ask(['下一题'])
    const afterExplainCard = fenceOf(afterExplain)
    assert.equal(afterExplainCard.payload.stem, firstStem, '下一题 after a knowledge card must return to the unanswered question')

    const forcedReply = await ask(['换一题'])
    const forcedCard = fenceOf(forcedReply)
    assert.ok(forcedCard !== null && forcedCard.kind === 'question')
    assert.notEqual(forcedCard.payload.stem, firstStem, '换一题 must advance even while the current question is open')
  } finally {
    await service.close()
  }
})

test('card mode: 上一题 re-opens the previous question as a read-only review', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-back-'))
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
    messages: toProxyMessages({ sessionId: 'back-test', messages: [systemMsg(), ...texts.map((text) => userMsg(text))] }, core),
  }, undefined))
  try {
    let first = null
    let reply = await ask(['开始做题'])
    for (let i = 0; i < 3; i += 1) {
      const card = fenceOf(reply)
      if (card !== null && card.kind === 'question') { first = card.payload.stem; break }
      reply = await ask(['继续'])
    }
    assert.ok(first !== null, 'quiz should reach a question card')
    await ask(['换一题'])
    const before = readState()
    const reviewReply = await ask(['上一题'])
    const reviewCard = fenceOf(reviewReply)
    assert.ok(reviewCard !== null && reviewCard.kind === 'question', 'review must re-send a question card')
    assert.equal(reviewCard.payload.review, true, 'previous card must be marked as a review')
    assert.equal(reviewCard.payload.stem, first, 'review must show the previous question')
    assert.ok(typeof reviewCard.payload.correctKey === 'string' && 'ABCD'.includes(reviewCard.payload.correctKey))
    assert.ok(typeof reviewCard.payload.why === 'string' && reviewCard.payload.why !== '', 'review must carry the explanation')
    const after = readState()
    assert.equal(after.issued_qid, before.issued_qid, 'review must not advance the issued question')
    assert.equal(after.history.length, before.history.length, 'review must not record history')
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
    const retryReply = await ask(['这是随便写的一句'])
    assert.match(retryReply, /还没答全/, 'incomplete open answer must ask for a retry')
    assert.ok(!retryReply.includes('参考：'), 'retry must not reveal the reference answer yet')
    const retryCard = fenceOf(retryReply)
    assert.ok(retryCard !== null && retryCard.kind === 'question', 'retry must re-send the open question card')
    assert.equal(retryCard.payload.questionKind, 'short')
    assert.equal(retryCard.payload.stem, '用一句话解释：为什么证 `A ⊆ B` 只需要一个方向？')
    const afterRetry = readState()
    assert.equal(afterRetry.issued_qid, 'I.1.open-1', 'open retry must not advance the question')
    assert.equal(afterRetry.history.length, before.history.length, 'open retry must not record history')

    const secondReply = await ask(['这是随便写的一句'])
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

test('decideFast: 代码题 opens a session; fenced code is local only with an issued exercise', () => {
  const codeCommand = decideFast({ messages: [systemMsg(), userMsg('代码题')] }, core)
  assert.equal(codeCommand.fast.type, 'code-next', '代码题 must be a fast command even without prior start')
  const fence = '看我的证明：\n```lean\ntheorem foo : True := by\n  trivial\n```'
  const withoutIssued = decideFast({ messages: [systemMsg(), userMsg(fence)] }, core)
  assert.equal(withoutIssued, null, 'fenced code without an issued exercise must reach the model')
  const withIssued = decideFast({ messages: [systemMsg(), userMsg(fence)] }, core, { codeIssued: true })
  assert.equal(withIssued.fast.type, 'code', 'fenced code with an issued exercise must be graded locally')
  assert.equal(core.extractFencedCode('no fence'), null)
  assert.equal(core.extractFencedCode(fence).includes('theorem foo'), true)
})

test('card mode: 代码题 issues a real dataset exercise; wrong, retry and pass use the real kernel', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-code-'))
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
    proxyUrl: `${service.url}/v1/chat/completions`, timeoutMs: 30000,
  }, {
    model: 'quiz-auto', stream: false, tools: [LEARN_CARD_TOOL_DECLARATION],
    messages: toProxyMessages({ sessionId: 'code-test', messages: [systemMsg(), ...texts.map((text) => userMsg(text))] }, core),
  }, undefined))
  try {
    const issueReply = await ask(['代码题'])
    const codeCard = fenceOf(issueReply)
    assert.ok(codeCard !== null, '代码题 must return a learn-card fence: ' + issueReply.slice(0, 200))
    assert.equal(codeCard.kind, 'code')
    assert.equal(typeof codeCard.payload.id, 'string')
    assert.ok(codeCard.payload.template.includes('sorry'), 'code card must show the template with sorry')
    assert.ok(Array.isArray(codeCard.payload.hints) && codeCard.payload.hints.length >= 1, 'code card must show hints')
    assert.equal(codeCard.payload.solution, undefined, 'the solution must never reach the card')
    const issued = codeCard.payload.id
    assert.equal(readState().issued_code, issued, 'issuing a code card must set issued_code')

    const record = datasetRecord(issued)
    assert.ok(record !== undefined, 'issued exercise must exist in the dataset: ' + issued)

    /* 1) wrong submission: kernel diagnostic, no mark-code, same exercise retryable. */
    const wrongCode = '```lean\n' + String(record.template).replace('sorry', 'exact h') + '\n```'
    const wrongReply = await ask(['代码题', wrongCode])
    assert.match(wrongReply, /还没通过/)
    assert.match(wrongReply, /诊断/)
    assert.ok(!wrongReply.includes('```learn-card\n'), 'feedback is text, not a fake card')
    const afterWrong = readState()
    assert.equal(afterWrong.issued_code, issued, 'a failed submission must keep the exercise issued')
    assert.ok(!afterWrong.code_done || afterWrong.code_done[issued] !== true, 'a failed submission must not mark it solved')

    /* 2) retry the SAME exercise with the dataset solution: real kernel passes + progress. */
    const correctCode = '```lean\n' + String(record.solution) + '\n```'
    const passReply = await ask(['代码题', wrongCode, correctCode])
    assert.match(passReply, /通过/)
    assert.match(passReply, /checked/)
    const afterPass = readState()
    assert.equal(afterPass.issued_code, null, 'a passing submission must clear issued_code')
    assert.equal(afterPass.code_done[issued], true, 'a passing submission must be recorded by mark-code')
    assert.ok(passReply.includes('已解 1 道代码练习'), 'pass feedback must show progress: ' + passReply)

    /* 3) skip the next code exercise: recorded as skipped, not solved. */
    const nextReply = await ask(['代码题'])
    const nextCard = fenceOf(nextReply)
    assert.ok(nextCard !== null && nextCard.kind === 'code')
    const nextId = nextCard.payload.id
    assert.notEqual(nextId, issued, 'the next code exercise must be different')
    const skipReply = await ask(['跳过这道代码题'])
    assert.match(skipReply, /跳过/)
    const afterSkip = readState()
    assert.equal(afterSkip.issued_code, null, 'skip-code must clear issued_code')
    assert.equal(afterSkip.code_done[nextId], 'skipped', 'skip-code must record skipped')
  } finally {
    await service.close()
  }
})

test('card mode: code grading infrastructure failure returns 503 so the Host can fall back', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-code-fail-'))
  const service = await core.startQuizService({
    repoRoot: REPO,
    stateDir: path.join(dataDir, 'quiz'),
    runDir: path.join(dataDir, 'logs'),
    cardsMode: 'learn',
    codeGradeTimeoutMs: 1,
    codeComposeTimeoutMs: 1,
  })
  const ask = async (texts) => callProxy({
    proxyUrl: `${service.url}/v1/chat/completions`, timeoutMs: 30000,
  }, {
    model: 'quiz-auto', stream: false, tools: [LEARN_CARD_TOOL_DECLARATION],
    messages: toProxyMessages({ sessionId: 'code-fail-test', messages: [systemMsg(), ...texts.map((text) => userMsg(text))] }, core),
  }, undefined)
  try {
    const issue = await ask(['代码题'])
    const card = JSON.parse(composeReplyText(issue).match(/```learn-card\n([\s\S]*?)\n```/)[1])
    await assert.rejects(
      () => ask(['代码题', '```lean\n' + card.payload.template + '\n```']),
      /HTTP 503/,
      'grading infrastructure failure must surface as 503 (Host falls back to next())',
    )
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

test('code-grade: self-contained bank templates get their name from a multi-line template', () => {
  const exercise = {
    id: 'code.i1.subset-trans',
    template: 'import lib.Set\n\ntheorem subset_trans (α : Type) (A B C : Set α)\n    (h1 : A ⊆ B) (h2 : B ⊆ C) : A ⊆ C := by\n  sorry\n',
    anonymous: false,
  }
  assert.equal(expectedDeclarationName(exercise), 'subset_trans')
  const assembled = assembleSubmission(exercise.template, 'by\n  intro x hx\n  exact h2 x (h1 x hx)', {
    expectedName: 'subset_trans',
  })
  assert.equal(assembled.ok, true)
  assert.ok(assembled.text.startsWith('import lib.Set'), 'trusted prefix must stay')
  assert.ok(!assembled.text.includes('sorry'), 'user proof must replace the sorry')
  const passed = evaluateGrade([
    { type: 'decl.checked', name: 'subset_trans' },
  ], exercise, 'subset_trans')
  assert.equal(passed.passed, true)
  const failed = evaluateGrade([
    { type: 'decl.checked', name: 'subset_trans' },
    { type: 'diagnostic', code: 'x', message: 'bad' },
  ], exercise, 'subset_trans')
  assert.equal(failed.passed, false, 'any diagnostic must block a pass')
})

test('apply(): a fenced code submission is graded locally when a code exercise is issued', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-apply-code-'))
  const host = captureHost()
  await apply(host.ctx, { dataDir, timeoutMs: 20000 })
  const request = (messages) => Object.freeze({
    provider: 'opencode-go-7',
    model: 'deepseek-v4.1-flash',
    messages,
    sessionId: 'apply-code-test',
    signal: new AbortController().signal,
  })
  const textOfChunks = (chunks) => chunks
    .filter((chunk) => chunk.type === 'text-delta')
    .map((chunk) => chunk.text).join('')
  try {
    host.commands[0].handler({ agent: { session: { id: 'apply-code-test' } }, rawInput: 'on' })
    const issued = await collect(host.listeners[0](request([systemMsg(), userMsg('代码题')]), slowStream()))
    const issuedText = textOfChunks(issued)
    const card = JSON.parse(issuedText.match(/```learn-card\n([\s\S]*?)\n```/)[1])
    assert.equal(card.kind, 'code')
    const record = datasetRecord(card.payload.id)
    assert.ok(record !== undefined)

    const wrongCode = '```lean\n' + String(record.template).replace('sorry', 'exact h') + '\n```'
    const wrongSlow = slowStream()
    const wrongChunks = await collect(host.listeners[0](request([
      systemMsg(), userMsg('代码题'), userMsg(wrongCode),
    ]), wrongSlow))
    const wrongText = textOfChunks(wrongChunks)
    assert.match(wrongText, /还没通过/, 'code submission must be answered locally: ' + wrongText.slice(0, 200))
    assert.equal(wrongSlow.called, false, 'a code submission must not reach the model')

    const correctCode = '```lean\n' + String(record.solution) + '\n```'
    const passSlow = slowStream()
    const passChunks = await collect(host.listeners[0](request([
      systemMsg(), userMsg('代码题'), userMsg(wrongCode), userMsg(correctCode),
    ]), passSlow))
    assert.match(textOfChunks(passChunks), /通过/)
    assert.equal(passSlow.called, false, 'a passing code submission must not reach the model')
  } finally {
    await host.disposeAll()
  }
})

/* ------------------------------------------------------- exercise panel */

test('exercise panel: the exercise comes from the repository, never the client, and never leaks the solution', () => {
  const id = 'set-theory/I.1/unit01-sets-membership/mem_of_subset'
  const resolved = resolvePanelExercise(REPO, id)
  assert.equal(resolved.ok, true, 'dataset exercise must resolve')
  assert.equal(resolved.exercise.dataset, true, 'dataset exercise must carry the compose path flag')
  assert.match(resolved.exercise.template, /mem_of_subset/)
  assert.match(resolved.exercise.template, /sorry/)
  assert.equal(resolved.exercise.solution, undefined, 'internal exercise must not carry the solution')
  assert.ok(resolved.exercise.hints.length > 0, 'trusted hints must survive')
  const view = panelExerciseView(resolved.exercise)
  assert.equal(view.id, id)
  assert.ok(!JSON.stringify(view).includes('solution'), 'panel view must never carry the solution')
  assert.ok(!JSON.stringify(view).includes('fun (ha : a ∈ A)'), 'the answer proof must not be sent to the client')

  const bank = resolvePanelExercise(REPO, 'code.i1.subset-trans')
  assert.equal(bank.ok, true, 'engine-bank exercise must resolve')
  assert.equal(bank.exercise.dataset, false, 'engine-bank template is self-contained')
  assert.match(bank.exercise.template, /theorem subset_trans/)

  assert.equal(normalizeExerciseId('../etc/passwd'), null, 'path traversal must be rejected')
  assert.equal(normalizeExerciseId('set-theory/../../x'), null, 'relative segments must be rejected')
  assert.equal(resolvePanelExercise(REPO, 'set-theory/nope/nope').code, 'not-found')
  assert.equal(resolvePanelExercise(REPO, '').code, 'bad-id')
})

test('exercise panel: real-kernel grading rejects a wrong proof and passes the course solution', async () => {
  const id = 'set-theory/I.1/unit01-sets-membership/mem_of_subset'
  const record = datasetRecord(id)
  assert.ok(record !== undefined, 'fixture missing: ' + id)
  const wrong = '```lean\n' + String(record.template).replace('sorry', 'exact h') + '\n```'
  const rejected = await gradePanelSubmission(REPO, { exerciseId: id, code: wrong })
  assert.equal(rejected.ok, true, 'a kernel rejection is a completed attempt, not an infrastructure failure')
  assert.equal(rejected.result.ok, true)
  assert.equal(rejected.result.passed, false, 'the wrong proof must not pass')
  assert.ok(rejected.result.diagnostics.length > 0, 'the kernel diagnostic must be reported')
  assert.ok(!JSON.stringify(rejected).includes(record.solution), 'the response must not embed the solution')

  const accepted = await gradePanelSubmission(REPO, { exerciseId: id, code: String(record.solution) })
  assert.equal(accepted.ok, true)
  assert.equal(accepted.result.passed, true, 'the course solution must pass: ' + JSON.stringify(accepted.result.diagnostics))
  assert.equal(accepted.result.target, 'mem_of_subset')

  const tooLarge = await gradePanelSubmission(REPO, { exerciseId: id, code: 'x'.repeat(70 * 1024) })
  assert.equal(tooLarge.ok, true, 'oversize input is a user error, not a service failure')
  assert.equal(tooLarge.result.ok, false)
  assert.equal(tooLarge.result.code, 'too-large')
})

test('exercise panel routes: metadata, content-type, body limits and grading', async () => {
  const id = 'set-theory/I.1/unit01-sets-membership/mem_of_subset'
  const record = datasetRecord(id)

  const meta = await callPanelRoute(handleExerciseRequest, {
    method: 'GET', url: '/sokonanoda-exercise?id=' + encodeURIComponent(id),
  })
  assert.equal(meta.status, 200)
  assert.equal(meta.json.ok, true)
  assert.equal(meta.json.exercise.id, id)
  assert.ok(!JSON.stringify(meta.json).includes('solution'), 'metadata must not leak the solution')

  const missing = await callPanelRoute(handleExerciseRequest, { method: 'GET', url: '/sokonanoda-exercise?id=nope' })
  assert.equal(missing.status, 404)
  assert.equal(missing.json.ok, false)

  const wrongMethod = await callPanelRoute(handleExerciseRequest, { method: 'POST', url: '/sokonanoda-exercise' })
  assert.equal(wrongMethod.status, 405)

  const wrongType = await callPanelRoute(handleGradeRequest, {
    method: 'POST', url: '/sokonanoda-grade', headers: { 'content-type': 'text/plain' }, body: '{}',
  })
  assert.equal(wrongType.status, 415)

  const badJson = await callPanelRoute(handleGradeRequest, {
    method: 'POST', url: '/sokonanoda-grade', headers: { 'content-type': 'application/json' }, body: '{oops',
  })
  assert.equal(badJson.status, 400)
  assert.equal(badJson.json.ok, false)

  const graded = await callPanelRoute(handleGradeRequest, {
    method: 'POST',
    url: '/sokonanoda-grade',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ exerciseId: id, code: String(record.solution) }),
  })
  assert.equal(graded.status, 200)
  assert.equal(graded.json.ok, true)
  assert.equal(graded.json.result.passed, true, 'route grading must come from the real kernel')
  assert.equal(graded.json.exercise.id, id)
})

test('apply(): the Host registers both exercise-panel routes when the web server exists', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-panel-routes-'))
  const host = captureHost()
  await apply(host.ctx, { dataDir, timeoutMs: 20000 })
  try {
    const paths = host.webServer.routes.map((route) => route.path)
    assert.ok(paths.includes(PANEL_EXERCISE_PATH), 'metadata route missing: ' + JSON.stringify(paths))
    assert.ok(paths.includes(PANEL_GRADE_PATH), 'grade route missing: ' + JSON.stringify(paths))
    for (const path of [PANEL_EXERCISE_PATH, PANEL_GRADE_PATH]) {
      const route = host.webServer.routes.find((candidate) => candidate.path === path)
      assert.equal(route.kind, 'exact')
      assert.equal(route.auth, 'anonymous')
      assert.equal(typeof route.handler, 'function')
    }
  } finally {
    await host.disposeAll()
  }
})

/* ---------------------------------------------------------------- helpers */

/** Drive one panel route handler with a fake Node request/response. */
async function callPanelRoute(handler, input) {
  const request = new EventEmitter()
  request.method = input.method || 'GET'
  request.url = input.url || '/'
  request.headers = input.headers || {}
  const response = {
    status: 0,
    headers: {},
    body: '',
    writeHead(status, headers) {
      this.status = status
      this.headers = headers || {}
    },
    end(body) {
      this.body += typeof body === 'string' ? body : ''
    },
  }
  if (input.method === 'POST') {
    const body = input.body === undefined ? '' : input.body
    process.nextTick(() => {
      if (body !== '') request.emit('data', Buffer.from(body))
      request.emit('end')
    })
  }
  await handler(REPO, request, response)
  let json = null
  try { json = JSON.parse(response.body) } catch { json = null }
  return { status: response.status, headers: response.headers, body: response.body, json }
}

/** One dataset record by id (fresh read; exercises are generated, not hand-edited). */
function datasetRecord(id) {
  const course = String(id).split('/')[0]
  const payload = JSON.parse(fs.readFileSync(
    path.join(REPO, 'courses', 'exercise-dataset', 'data', course + '.json'), 'utf8'))
  return (payload.exercises || []).find((exercise) => exercise.id === id)
}

function captureHost(options = {}) {
  const listeners = []
  const tools = []
  const commands = []
  const disposers = []
  const services = { ...(options.services || {}) }
  const webServer = options.webServer || captureWebServer()
  if (options.deferWebServer !== true && services.webServer === undefined) services.webServer = webServer
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
    get(name) {
      return services[name]
    },
    inject(deps, callback) {
      if (options.deferWebServer === true && deps.includes('webServer')) callback({ webServer })
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
    webServer,
    async disposeAll() {
      for (const disposer of disposers.reverse()) {
        try { disposer() } catch { /* best effort */ }
      }
      await new Promise((resolve) => setTimeout(resolve, 30))
    },
  }
}

/** Fake named-route table: registration order is preserved for route lookup. */
function captureWebServer() {
  const routes = []
  return {
    routes,
    register(route) {
      routes.push(route)
      return () => {
        const index = routes.indexOf(route)
        if (index >= 0) routes.splice(index, 1)
      }
    },
  }
}

/** Drive one route handler with a minimal IncomingMessage/ServerResponse pair. */
function callRouteUrl(route, url, peer, method = 'GET') {
  const request = { url, method, headers: {} }
  return new Promise((resolve) => {
    let settled = false
    const response = {
      status: 0,
      headers: null,
      body: '',
      writeHead(status, headers) { this.status = status; this.headers = headers },
      end(text) {
        if (settled) return
        settled = true
        this.body = text === undefined ? '' : String(text)
        resolve(this)
      },
    }
    const result = route.handler(request, response, peer)
    if (result !== null && typeof result === 'object' && typeof result.then === 'function') {
      result.catch(() => {
        if (settled) return
        settled = true
        resolve(response)
      })
    }
  })
}

/** State-route shortcut kept for the existing preview tests. */
function callRoute(route, params, peer) {
  return callRouteUrl(route, '/sokonanoda-state?' + new URLSearchParams(params).toString(), peer)
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

test('apply(): routes wait for a webServer that registers after the plugin', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-late-web-'))
  const host = captureHost({ deferWebServer: true })
  await apply(host.ctx, { dataDir, timeoutMs: 20000 })
  try {
    const paths = host.webServer.routes.map((route) => route.path)
    assert.ok(paths.includes('/sokonanoda-client-report'), 'late webServer must still get the report route')
    assert.ok(paths.includes('/sokonanoda-state'), 'late webServer must still get the read-only state route')
  } finally {
    await host.disposeAll()
  }
})

/* ------------------------------------------------ read-only goal state route */

/** Write a fake `scripts/soko` driven by the queried file's basename. */
function writeFakeSoko(root) {
  const scripts = path.join(root, 'scripts')
  fs.mkdirSync(scripts, { recursive: true })
  const file = path.join(scripts, 'soko')
  fs.writeFileSync(file, `#!/usr/bin/env node
const args = process.argv.slice(2)
const file = args[args.indexOf('--file') + 1] || ''
const base = file.split('/').pop()
if (args[1] === 'state') {
  if (base.startsWith('outside') || base.startsWith('broken')) {
    console.log(JSON.stringify({ schema: 'soko.query/1', op: 'state', version: 1, ok: false, error: { code: 'outside-declarations', message: 'outside' } }))
  } else {
    console.log(JSON.stringify({ schema: 'soko.query/1', op: 'state', version: 1, ok: true, data: { decl: { name: 'demo', kind: 'theorem', status: 'checked' }, binders: [], goals: [{ goal: 'True', goal_runs: [{ text: 'True', kind: 'keyword' }], binders: [] }], goal: 'True', goal_runs: [{ text: 'True', kind: 'keyword' }], step: 1, total: 2, messages: [] } }))
  }
  process.exit(0)
}
if (args[1] === 'check') {
  const failed = base.startsWith('broken') ? [{ start_line: 3, message: 'unexpected token' }] : []
  console.log(JSON.stringify({ ok: true, data: { failed, warnings: [], counts: {} } }))
  process.exit(0)
}
console.log(JSON.stringify({ ok: false, error: { code: 'bad', message: 'unexpected op' } }))
`, 'utf8')
  return file
}

test('state route helpers: address parsing and separator-aware containment', () => {
  const parsed = parseFileResource('dsh-resource://file/session/s1/courses/a.sokonanoda?x=1')
  assert.deepEqual(parsed, { sessionId: 's1', path: 'courses/a.sokonanoda' })
  assert.deepEqual(
    parseFileResource('dsh-resource://file/session/s1//tmp/a.sokonanoda'),
    { sessionId: 's1', path: '/tmp/a.sokonanoda' },
    'an absolute session path keeps its leading slash',
  )
  assert.deepEqual(
    parseFileResource('dsh-resource://file/session/s1/a%20b/c.sokonanoda'),
    { sessionId: 's1', path: 'a b/c.sokonanoda' },
  )
  assert.equal(parseFileResource('dsh-resource://file/absolute/tmp/a.sokonanoda'), null)
  assert.equal(parseFileResource('https://example.com/a'), null)
  assert.equal(parseFileResource('dsh-resource://file/session/s1/%E0%A4%A'), null, 'a malformed escape is not an address')
  assert.equal(isPathWithin('/repo', '/repo/a.sokonanoda'), true)
  assert.equal(isPathWithin('/repo', '/repo'), true)
  assert.equal(isPathWithin('/repo', '/repo-other/a.sokonanoda'), false)
  assert.equal(isPathWithin('/repo', '/etc/passwd'), false)
})

test('state route helpers: only existing files inside the workspace/repo resolve', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-state-root-'))
  const outside = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-state-outside-'))
  fs.writeFileSync(path.join(root, 'demo.sokonanoda'), 'theorem demo : True := by\n  trivial\n')
  fs.writeFileSync(path.join(outside, 'secret.sokonanoda'), 'theorem secret : True := by\n  trivial\n')
  const resource = (value) => 'dsh-resource://file/session/s1/' + value.split('/').map(encodeURIComponent).join('/')
  const inside = resolveAllowedStatePath({ resource: resource('demo.sokonanoda'), workspaceRoot: root, repoRoot: root })
  assert.equal(inside.ok, true)
  assert.equal(inside.path, fs.realpathSync(path.join(root, 'demo.sokonanoda')))
  const escape = resolveAllowedStatePath({ resource: resource('../' + path.basename(outside) + '/secret.sokonanoda'), workspaceRoot: root, repoRoot: root })
  assert.equal(escape.ok, false)
  assert.equal(escape.code, 'forbidden')
  const absolute = resolveAllowedStatePath({
    resource: 'dsh-resource://file/session/s1/' + outside.split('/').map(encodeURIComponent).join('/'),
    workspaceRoot: root,
    repoRoot: root,
  })
  assert.equal(absolute.ok, false, 'an absolute path outside both roots must be refused')
  const symlink = path.join(root, 'link.sokonanoda')
  fs.symlinkSync(path.join(outside, 'secret.sokonanoda'), symlink)
  const throughLink = resolveAllowedStatePath({ resource: resource('link.sokonanoda'), workspaceRoot: root, repoRoot: root })
  assert.equal(throughLink.ok, false, 'a symlink must not escape the workspace')
  assert.equal(throughLink.code, 'forbidden')
  assert.equal(resolveAllowedStatePath({ resource: resource('missing.sokonanoda'), workspaceRoot: root, repoRoot: root }).code, 'not-found')
  assert.equal(resolveAllowedStatePath({ resource: resource('demo.sokonanoda'), workspaceRoot: undefined, repoRoot: root }).code, 'workspace-unavailable')
})

test('runSokoCommand: JSON envelope, invalid output and a hard timeout', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-soko-cmd-'))
  const okScript = path.join(root, 'ok.js')
  fs.writeFileSync(okScript, 'console.log(JSON.stringify({ok:true,n:1}))\n')
  const ok = await runSokoCommand({ sokoPath: okScript, cwd: root, args: [], timeoutMs: 2000 })
  assert.equal(ok.ok, true)
  assert.equal(ok.envelope.n, 1)
  const badScript = path.join(root, 'bad.js')
  fs.writeFileSync(badScript, "console.log('not json')\n")
  const bad = await runSokoCommand({ sokoPath: badScript, cwd: root, args: [], timeoutMs: 2000 })
  assert.equal(bad.ok, false)
  assert.equal(bad.error.code, 'invalid-response')
  const slowScript = path.join(root, 'slow.js')
  fs.writeFileSync(slowScript, 'setTimeout(() => console.log(JSON.stringify({ok:true})), 5000)\n')
  const started = Date.now()
  const slow = await runSokoCommand({ sokoPath: slowScript, cwd: root, args: [], timeoutMs: 250 })
  assert.equal(slow.ok, false)
  assert.equal(slow.error.code, 'timeout')
  assert.ok(Date.now() - started < 3000, 'timeout must kill the query promptly')
})

test('GET /sokonanoda-state: owner only, workspace-contained, structured answers', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-state-route-'))
  const soko = writeFakeSoko(root)
  for (const name of ['demo.sokonanoda', 'outside.sokonanoda', 'broken.sokonanoda']) {
    fs.writeFileSync(path.join(root, name), 'theorem demo : True := by\n  trivial\n')
  }
  const host = captureHost({
    services: {
      workspaceFiles: {
        async scopeOf(sessionId) {
          return sessionId === 's1' ? { sessionId, workspaceRoot: root } : undefined
        },
      },
    },
  })
  await apply(host.ctx, {
    dataDir: path.join(root, 'data'),
    repoRoot: root,
    sokoPath: soko,
    stateTimeoutMs: 2000,
  })
  const route = host.webServer.routes.find((candidate) => candidate.path === '/sokonanoda-state')
  assert.ok(route !== undefined, 'the read-only state route must register')
  assert.equal(route.auth, 'required', 'the route must require a live Peer')
  const owner = { principal: { kind: 'owner' } }
  const resource = (name) => 'dsh-resource://file/session/s1/' + encodeURIComponent(name)
  try {
    const answered = await callRoute(route, { resource: resource('demo.sokonanoda'), line: '2', col: '1' }, owner)
    assert.equal(answered.status, 200)
    const body = JSON.parse(answered.body)
    assert.equal(body.ok, true)
    assert.equal(body.line, 2)
    assert.equal(body.state.decl.name, 'demo')
    assert.equal(body.state.step, 1)

    const anonymous = await callRoute(route, { resource: resource('demo.sokonanoda'), line: '2' }, { principal: { kind: 'guest' } })
    assert.equal(anonymous.status, 403, 'only the Host owner may read goal state')

    const notAnAddress = await callRoute(route, { resource: '../etc/passwd', line: '1' }, owner)
    assert.equal(notAnAddress.status, 400, 'a non-resource string is a bad request')
    const escaped = await callRoute(route, { resource: resource('../../etc/passwd.sokonanoda'), line: '1' }, owner)
    assert.equal(escaped.status, 403, 'a traversal inside a session address must be refused')
    const wrongFile = await callRoute(route, { resource: resource('../../etc/passwd'), line: '1' }, owner)
    assert.equal(wrongFile.status, 400, 'the read-only route only serves .sokonanoda source files')

    const outside = await callRoute(route, { resource: resource('outside.sokonanoda'), line: '1' }, owner)
    assert.equal(JSON.parse(outside.body).error.code, 'outside-declarations')

    const broken = await callRoute(route, { resource: resource('broken.sokonanoda'), line: '1' }, owner)
    const brokenBody = JSON.parse(broken.body)
    assert.equal(brokenBody.error.code, 'file-errors')
    assert.equal(brokenBody.diagnostics[0].line, 3)

    const badLine = await callRoute(route, { resource: resource('demo.sokonanoda'), line: '0' }, owner)
    assert.equal(badLine.status, 400)
    const missingParam = await callRoute(route, { line: '1' }, owner)
    assert.equal(missingParam.status, 400)
  } finally {
    await host.disposeAll()
  }
})

/* ---------------------------------------------------------- class notes */

const OWNER = { principal: { kind: 'owner' } }

function courseCards() {
  return JSON.parse(fs.readFileSync(
    path.join(REPO, 'courses', 'set-theory', 'knowledge', 'data', 'cards.json'), 'utf8')).cards
}

function courseQuestions() {
  return JSON.parse(fs.readFileSync(
    path.join(REPO, 'courses', 'set-theory', 'knowledge', 'data', 'questions.json'), 'utf8')).questions
}

function cardPayload(id) {
  const card = courseCards().find((candidate) => candidate.id === id)
  assert.ok(card !== undefined, 'missing fixture card ' + id)
  const payload = {
    choices: [{
      message: {
        content: '',
        tool_calls: [{
          type: 'function',
          function: { name: 'learn_card', arguments: JSON.stringify({ kind: 'knowledge', payload: card }) },
        }],
      },
    }],
  }
  return { card, payload }
}

function routeOf(host, routePath) {
  const route = host.webServer.routes.find((candidate) => candidate.path === routePath)
  assert.ok(route !== undefined, 'missing route ' + routePath)
  return route
}

test('class-notes: a seen knowledge card becomes one note with tags, points and a whitelisted figure', () => {
  const { card, payload } = cardPayload('injective')
  const extracted = extractKnowledgeCards(payload)
  assert.equal(extracted.length, 1)
  assert.equal(extracted[0].id, 'injective')

  const book = emptyBook('set-theory', Date.parse('2026-11-27T00:00:00Z'))
  assert.equal(upsertKnowledgeCard(book, extracted[0], Date.parse('2026-11-27T00:00:00Z')), true)
  assert.equal(book.notes.length, 1)
  const note = book.notes[0]
  assert.equal(note.id, 'injective')
  assert.deepEqual(note.tags, ['function', 'injective'])
  assert.ok(note.keyPoints.length >= 3, 'one-liner + list sections must become review points')
  assert.equal(note.figures.length, 1)
  assert.equal(note.figures[0].file, 'injective.png')
  assert.equal(note.figures[0].src, '/sokonanoda-notes-asset?path=injective.png')
  assert.ok(allowedFigureFiles().has('injective.png'))
  assert.equal(noteFromCard(card, Date.parse('2026-11-27T00:00:00Z')).title, card.title)

  /* Seeing the same card again refreshes it without duplicating the note. */
  assert.equal(upsertKnowledgeCard(book, extracted[0], Date.parse('2026-11-27T01:00:00Z')), true)
  assert.equal(book.notes.length, 1)
  assert.equal(book.notes[0].firstSeenAt, '2026-11-27T00:00:00.000Z')
  assert.equal(book.notes[0].lastSeenAt, '2026-11-27T01:00:00.000Z')

  /* Unknown knowledge kinds never create notes. */
  const questionPayload = {
    choices: [{ message: { content: '', tool_calls: [{
      type: 'function',
      function: { name: 'learn_card', arguments: JSON.stringify({ kind: 'question', payload: { id: 'x' } }) },
    }] } }],
  }
  assert.deepEqual(extractKnowledgeCards(questionPayload), [])
})

test('class-notes: questions attach only to notes whose card the learner saw; latest answer wins', () => {
  const { payload } = cardPayload('injective')
  const book = emptyBook('set-theory', Date.parse('2026-11-27T00:00:00Z'))
  upsertKnowledgeCard(book, extractKnowledgeCards(payload)[0], Date.parse('2026-11-27T00:00:00Z'))
  const questions = courseQuestions()
  const byId = Object.fromEntries(questions.map((question) => [question.id, question]))
  const injectiveQuestion = byId['I.2.inj-1']
  assert.ok(injectiveQuestion !== undefined)
  assert.equal(matchNoteForQuestion(injectiveQuestion, book.notes), 'injective')
  /* A membership question has no shared specific tag with the injective note. */
  assert.equal(matchNoteForQuestion(byId['I.1.mem-1'], book.notes), '')

  const history = [
    { qid: 'I.2.inj-1', choice: 2, correct: false, at: '2026-11-27T01:00:00Z' },
    { qid: 'I.2.inj-1', choice: 1, correct: true, at: '2026-11-27T02:00:00Z' },
    { qid: 'I.1.mem-1', choice: 2, correct: false, at: '2026-11-27T03:00:00Z' },
  ]
  assert.equal(attachHistory(book, history, byId, Date.parse('2026-11-27T04:00:00Z')), true)
  assert.equal(book.notes[0].questions.length, 1, 'only the seen-card question is attached')
  assert.equal(book.notes[0].questions[0].questionId, 'I.2.inj-1')
  assert.equal(book.notes[0].questions[0].correct, true)
  assert.equal(book.notes[0].questions[0].answer, 'A')
  assert.equal(book.notes[0].questions[0].chosen, 'A')
  assert.equal(book.notes[0].mistakes.length, 1, 'a wrong attempt stays in 易错 after a later correct one')
  assert.equal(book.notes[0].mistakes[0].questionId, 'I.2.inj-1')

  /* Flip the latest answer: the same question moves into 易错. */
  assert.equal(attachHistory(book, [
    { qid: 'I.2.inj-1', choice: 2, correct: false, at: '2026-11-27T05:00:00Z' },
  ], byId, Date.parse('2026-11-27T06:00:00Z')), true)
  assert.equal(book.notes[0].questions[0].correct, false)
  assert.equal(book.notes[0].mistakes.length, 1)
  assert.equal(book.notes[0].mistakes[0].questionId, 'I.2.inj-1')
})

test('class-notes: readQuizHistory keeps a wrong attempt after a later correct replay', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-notes-history-'))
  fs.writeFileSync(path.join(dir, 'default.json'), JSON.stringify({
    history: [
      { qid: 'I.2.inj-1', choice: 2, correct: false, at: '2026-11-27T01:00:00Z' },
      { qid: 'I.2.inj-1', choice: 1, correct: true, at: '2026-11-27T02:00:00Z' },
    ],
  }), 'utf8')
  fs.writeFileSync(path.join(dir, 'fingerprint.json'), JSON.stringify({
    history: [{ qid: 'I.2.inj-1', choice: 2, correct: false, at: '2026-11-27T01:00:00Z' }],
  }), 'utf8')
  const history = readQuizHistory(dir)
  assert.equal(history.length, 2, 'both attempts survive; the duplicate across state files is dropped once')
  const { payload } = cardPayload('injective')
  const book = emptyBook('set-theory')
  upsertKnowledgeCard(book, extractKnowledgeCards(payload)[0])
  const byId = Object.fromEntries(courseQuestions().map((question) => [question.id, question]))
  attachHistory(book, history, byId)
  assert.equal(book.notes[0].questions[0].correct, true, 'the latest attempt is the question status')
  assert.equal(book.notes[0].mistakes[0].questionId, 'I.2.inj-1', 'the earlier wrong attempt stays in 易错')
})

test('class-notes: HTML and Markdown render ✓/×, why and the figure', () => {
  const { payload } = cardPayload('injective')
  const book = emptyBook('set-theory', Date.parse('2026-11-27T00:00:00Z'))
  upsertKnowledgeCard(book, extractKnowledgeCards(payload)[0], Date.parse('2026-11-27T00:00:00Z'))
  const byId = Object.fromEntries(courseQuestions().map((question) => [question.id, question]))
  attachHistory(book, [
    { qid: 'I.2.inj-1', choice: 1, correct: true, at: '2026-11-27T01:00:00Z' },
  ], byId, Date.parse('2026-11-27T02:00:00Z'))

  const html = renderNotesHtml(book)
  assert.match(html, /课堂笔记/)
  assert.match(html, /单射 · Injective/)
  assert.match(html, /\/sokonanoda-notes-asset\?path=injective\.png/)
  assert.match(html, /data-state="correct"/)
  assert.match(html, /单射 = 不同输入不会撞到同一输出/)
  assert.match(html, /sokonanoda-notes\?meta=1/, 'the page must poll updatedAt')
  assert.equal(renderNotesHtml({ ...book, notes: [] }).includes('还没有课堂笔记'), true)

  const markdown = renderNotesMarkdown(book)
  assert.match(markdown, /^# 课堂笔记 · set-theory/m)
  assert.match(markdown, /!\[[^\]]*\]\(\/sokonanoda-notes-asset\?path=injective\.png\)/)
  assert.match(markdown, /✅/)
})

test('GET /sokonanoda-notes*: owner-only, read-only and whitelisted PNG assets', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-notes-routes-'))
  const host = captureHost()
  await apply(host.ctx, { dataDir, timeoutMs: 20000 })
  try {
    const paths = host.webServer.routes.map((route) => route.path)
    for (const routePath of ['/sokonanoda-notes', '/sokonanoda-notes-view', '/sokonanoda-notes.md', '/sokonanoda-notes-asset']) {
      assert.ok(paths.includes(routePath), 'missing notes route ' + routePath)
    }

    const ledgerRoute = routeOf(host, '/sokonanoda-notes')
    const mine = await callRouteUrl(ledgerRoute, '/sokonanoda-notes', OWNER)
    assert.equal(mine.status, 200)
    const body = JSON.parse(mine.body)
    assert.equal(body.ok, true)
    assert.equal(body.book.schema, NOTES_SCHEMA)
    assert.deepEqual(body.book.notes, [])

    const meta = await callRouteUrl(ledgerRoute, '/sokonanoda-notes?meta=1', OWNER)
    assert.equal(meta.status, 200)
    assert.equal(JSON.parse(meta.body).ok, true)
    assert.equal(typeof JSON.parse(meta.body).updatedAt, 'string')
    assert.equal(JSON.parse(meta.body).count, 0)

    const forbidden = await callRouteUrl(ledgerRoute, '/sokonanoda-notes', { principal: { kind: 'visitor' } })
    assert.equal(forbidden.status, 403)
    const method = await callRouteUrl(ledgerRoute, '/sokonanoda-notes', OWNER, 'POST')
    assert.equal(method.status, 405)
    assert.match(method.headers.allow || '', /GET/)

    const view = await callRouteUrl(routeOf(host, '/sokonanoda-notes-view'), '/sokonanoda-notes-view', OWNER)
    assert.equal(view.status, 200)
    assert.match(view.headers['content-type'], /text\/html/)
    assert.match(view.body, /课堂笔记/)
    assert.match(view.body, /sokonanoda-notes\?meta=1/)

    const markdown = await callRouteUrl(routeOf(host, '/sokonanoda-notes.md'), '/sokonanoda-notes.md', OWNER)
    assert.equal(markdown.status, 200)
    assert.match(markdown.headers['content-type'], /text\/markdown/)
    assert.match(markdown.body, /^# 课堂笔记/m)

    const assetRoute = routeOf(host, '/sokonanoda-notes-asset')
    const png = await callRouteUrl(assetRoute, '/sokonanoda-notes-asset?path=injective.png', OWNER)
    assert.equal(png.status, 200)
    assert.equal(png.headers['content-type'], 'image/png')
    assert.ok(Number(png.headers['content-length']) > 1000, 'real PNG bytes expected')
    const traversal = await callRouteUrl(assetRoute, '/sokonanoda-notes-asset?path=' + encodeURIComponent('../injective.png'), OWNER)
    assert.equal(traversal.status, 400)
    const outside = await callRouteUrl(assetRoute, '/sokonanoda-notes-asset?path=hero-vscode.png', OWNER)
    assert.equal(outside.status, 404)
    const notPng = await callRouteUrl(assetRoute, '/sokonanoda-notes-asset?path=notes.txt', OWNER)
    assert.equal(notPng.status, 400)
    const assetForbidden = await callRouteUrl(assetRoute, '/sokonanoda-notes-asset?path=injective.png', { principal: { kind: 'visitor' } })
    assert.equal(assetForbidden.status, 403)
    const assetPost = await callRouteUrl(assetRoute, '/sokonanoda-notes-asset?path=injective.png', OWNER, 'POST')
    assert.equal(assetPost.status, 405)
  } finally {
    await host.disposeAll()
  }
})

test('apply(): startup reconciliation attaches questions answered before this Host loaded', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-notes-startup-'))
  fs.mkdirSync(path.join(dataDir, 'quiz'), { recursive: true })
  fs.writeFileSync(path.join(dataDir, 'quiz', 'default.json'), JSON.stringify({
    version: 1,
    history: [{ qid: 'I.2.inj-1', choice: 2, correct: false, at: '2026-11-27T01:00:00Z' }],
  }), 'utf8')
  const { payload } = cardPayload('injective')
  const book = emptyBook('set-theory')
  upsertKnowledgeCard(book, extractKnowledgeCards(payload)[0])
  fs.mkdirSync(path.join(dataDir, 'notes'), { recursive: true })
  fs.writeFileSync(path.join(dataDir, 'notes', 'set-theory.json'), JSON.stringify(book), 'utf8')

  const host = captureHost()
  await apply(host.ctx, { dataDir, timeoutMs: 20000 })
  try {
    const response = await callRouteUrl(routeOf(host, '/sokonanoda-notes'), '/sokonanoda-notes', OWNER)
    assert.equal(response.status, 200)
    const body = JSON.parse(response.body)
    assert.equal(body.book.notes.length, 1)
    assert.equal(body.book.notes[0].questions.length, 1)
    assert.equal(body.book.notes[0].questions[0].questionId, 'I.2.inj-1')
    assert.equal(body.book.notes[0].questions[0].correct, false)
    assert.equal(body.book.notes[0].mistakes.length, 1)
    const ledger = readLedger(path.join(dataDir, 'notes', 'set-theory.json'), 'set-theory')
    assert.equal(ledger.notes[0].questions[0].questionId, 'I.2.inj-1', 'reconciliation must persist')
  } finally {
    await host.disposeAll()
  }
})

test('apply(): a fast knowledge-card reply lands in the class-notes ledger', async () => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sokonanoda-dsh-notes-fast-'))
  const host = captureHost()
  await apply(host.ctx, { dataDir, timeoutMs: 20000 })
  try {
    const command = (rawInput) => host.commands[0].handler({
      agent: { session: { id: 'notes-fast' } }, rawInput,
    })
    assert.equal(command('on').kind, 'success')
    const base = {
      provider: 'opencode-go-7',
      model: 'deepseek-v4.1-flash',
      sessionId: 'notes-fast',
    }
    /* Turn 1 opens the quiz (the service persists issued_qid). */
    const opened = await collect(host.listeners[0](Object.freeze({
      ...base,
      messages: [systemMsg(), userMsg('开始做题')],
      signal: new AbortController().signal,
    }), slowStream()))
    assert.ok(opened.some((chunk) => chunk.type === 'text-delta'), 'quiz opener must answer locally')
    /* Turn 2 asks for the explanation card of the issued question. */
    const request = Object.freeze({
      ...base,
      messages: [systemMsg(), userMsg('开始做题'), assistantMsg('…'), userMsg('不懂')],
      signal: new AbortController().signal,
    })
    const slow = slowStream()
    const chunks = await collect(host.listeners[0](request, slow))
    const reply = chunks.filter((chunk) => chunk.type === 'text-delta').map((chunk) => chunk.text).join('')
    assert.match(reply, /```learn-card\n/, '快路径 must answer locally: ' + reply.slice(0, 200))
    assert.equal(slow.called, false)

    const response = await callRouteUrl(routeOf(host, '/sokonanoda-notes'), '/sokonanoda-notes', OWNER)
    assert.equal(response.status, 200)
    const body = JSON.parse(response.body)
    assert.ok(body.book.notes.length >= 1, 'a seen knowledge card must create a note')
    const note = body.book.notes[0]
    assert.ok(note.id !== '' && typeof note.title === 'string' && note.title !== '')
    assert.ok(Array.isArray(note.keyPoints) && note.keyPoints.length > 0, 'review points must be extracted')
    assert.ok(fs.existsSync(path.join(dataDir, 'notes', 'set-theory.json')), 'ledger must persist on disk')
  } finally {
    await host.disposeAll()
  }
})
