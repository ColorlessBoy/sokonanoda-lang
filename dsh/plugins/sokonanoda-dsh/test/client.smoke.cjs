/**
 * Client-half smoke test for `sokonanoda-dsh`.
 *
 * It pins the user-visible rendering contract:
 *   - module id/package manifest match the profile loader;
 *   - question options send their letter as a normal composer message on click;
 *     knowledge / lesson / progress cards stay display-only;
 *   - exactly three Markdown fence layers (`learn-card`, `lean`, `sokonanoda`)
 *     are registered, and no tool view / no tool is registered (cards are
 *     display-only; the chat character stream stays the source of truth);
 *   - a settled `learn-card` fence renders the same real card as before
 *     (question with 4 options, knowledge with highlighted definition, lesson);
 *   - streaming / malformed fences fail open to the raw code block;
 *   - `lean` fences highlight keywords/tactics/comments without changing text;
 *   - the card surface, Lean block and token CSS rules exist.
 *
 *   node test/client.smoke.cjs      # 0 = all PASS, 1 = a FAIL
 */
const fs = require('node:fs')
const path = require('node:path')

const CLIENT = path.join(__dirname, '..', 'client.js')
const source = fs.readFileSync(CLIENT, 'utf8')

let passes = 0
const failures = []
function expect(condition, message) {
  if (!condition) throw new Error(message)
}
function test(name, body) {
  try {
    body()
    passes += 1
    console.log('PASS  ' + name)
  } catch (error) {
    failures.push(name)
    console.error('FAIL  ' + name)
    console.error('      ' + (error instanceof Error ? error.message : String(error)))
  }
}

/* ------------------------------------------------------- module execution */

let loaded = null
const injectedStyles = []
globalThis.window = {
  __ModuleLoader__: { load: (mod) => { loaded = mod; } },
  addEventListener: () => {},
  removeEventListener: () => {},
}
globalThis.document = {
  querySelector: () => null,
  createElement: () => ({ dataset: {}, textContent: '' }),
  head: { appendChild: (tag) => { injectedStyles.push(tag); } },
  addEventListener: () => {},
  removeEventListener: () => {},
}

/** Minimal hook runtime: one synchronous render pass, enough for a stateless view. */
const hookRuntime = {
  hooks: [],
  cursor: 0,
  useRef(value) {
    const index = this.cursor++
    if (!(index in this.hooks)) this.hooks[index] = { current: value }
    return this.hooks[index]
  },
  useState(initial) {
    const index = this.cursor++
    if (!(index in this.hooks)) this.hooks[index] = typeof initial === 'function' ? initial() : initial
    return [this.hooks[index], () => {}]
  },
  useMemo(factory) { return factory() },
  useEffect(fn) { if (typeof fn === 'function') fn() },
  useLayoutEffect() {},
}

const React = {
  Fragment: Symbol('Fragment'),
  createElement: (type, props, ...children) => ({ type, props: props || {}, children }),
  useRef: (value) => hookRuntime.useRef(value),
  useState: (initial) => hookRuntime.useState(initial),
  useMemo: (factory) => hookRuntime.useMemo(factory),
  useEffect: (fn) => { if (typeof fn === 'function') fn() },
  useLayoutEffect: (fn) => hookRuntime.useLayoutEffect(fn),
}
const requireStub = (name) => {
  if (name === 'react') return React
  throw new Error('unexpected require: ' + name)
}

eval(source) // eslint-disable-line no-eval
const plugin = loaded.factory(requireStub)

/** Evaluate every function component so tests see what a browser would render. */
function render(node) {
  if (node === null || node === undefined || typeof node !== 'object') return node
  if (Array.isArray(node)) return node.map(render)
  if (typeof node.type === 'function') {
    hookRuntime.hooks = []
    hookRuntime.cursor = 0
    return render(node.type(node.props || {}))
  }
  const copy = Object.assign({}, node)
  if (copy.children !== undefined) copy.children = render(copy.children)
  return copy
}

/** Flatten one rendered tree into host nodes (arrays included). */
function flatten(node, out) {
  const nodes = out === undefined ? [] : out
  if (node === null || node === undefined) return nodes
  if (Array.isArray(node)) {
    for (const child of node) flatten(child, nodes)
    return nodes
  }
  if (typeof node !== 'object') return nodes
  if (typeof node.type === 'function') {
    flatten(render(node), nodes)
    return nodes
  }
  nodes.push(node)
  if (node.children !== undefined) flatten(node.children, nodes)
  return nodes
}

/** Plain text of a rendered tree, in document order. */
function textOf(node) {
  if (node === null || node === undefined) return ''
  if (typeof node === 'string' || typeof node === 'number') return String(node)
  if (Array.isArray(node)) return node.map(textOf).join('')
  if (typeof node !== 'object') return ''
  return textOf(node.children)
}

function nodesOf(rendered) { return flatten(rendered) }
function byClass(rendered, className) {
  return nodesOf(rendered).filter((node) => typeof node.props?.className === 'string'
    && node.props.className.split(' ').includes(className))
}
function oneByClass(rendered, className) {
  const found = byClass(rendered, className)
  expect(found.length === 1, 'expected exactly one .' + className + ', got ' + found.length)
  return found[0]
}

/** Extract one flat CSS rule block from the client stylesheet. */
function cssRule(selector) {
  const start = cardCss.indexOf(selector + '{')
  if (start < 0) throw new Error('missing CSS rule ' + selector)
  const end = cardCss.indexOf('}', start)
  if (end < 0) throw new Error('unterminated CSS rule ' + selector)
  return cardCss.slice(start + selector.length + 1, end)
}

/* --------------------------------------------------------- slot mounting */

const registrations = []
const injectedSlotNames = []
const dictionaries = {}
const pluginApplyCtx = {
  effect(callback) {
    const cleanup = callback()
    return typeof cleanup === 'function' ? cleanup : () => {}
  },
  locale: {
    register(namespace, value) {
      dictionaries[namespace] = value
      return () => {}
    },
  },
  slots: {
    inject(name, callback) {
      injectedSlotNames.push(name)
      return callback()
    },
    register(definition, component) {
      registrations.push({ definition, component })
      return () => {}
    },
  },
}
expect(typeof plugin.apply === 'function', 'client module must export apply()')
expect(Array.isArray(plugin.inject), 'client module must export inject[]')
plugin.apply(pluginApplyCtx)

const LearnCardFence = plugin.__test.LearnCardFence
const LeanFence = plugin.__test.LeanFence
const highlightLean = plugin.__test.highlightLean
const h = React.createElement
const zh = plugin.__test.zh
const en = plugin.__test.en
const cardCss = plugin.__test.cardCss

function translator(dict) {
  return (key) => (typeof dict[key] === 'string' ? dict[key] : key)
}

/** Props shaped like the real keyed toolview owner (phase + settled result node). */
function resultProps(kind, payload, extra) {
  const args = extra !== undefined && extra.args !== undefined ? extra.args : { kind, payload }
  return {
    phase: 'result',
    callId: 'call_learn_card_1',
    toolName: 'learn_card',
    useDisclosure: () => ({ expanded: true, setExpanded: () => {}, toggle: () => {} }),
    call: { name: 'learn_card', argsRaw: JSON.stringify(args) },
    block: {
      kind: 'tool-result',
      callId: 'call_learn_card_1',
      name: 'learn_card',
      call: { name: 'learn_card', argsRaw: JSON.stringify(args) },
      args: {},
      content: [{ type: 'text', text: 'Card shown in the chat.' }],
      isError: false,
      subCalls: [],
    },
    t: translator(zh),
  }
}

const QUESTION = {
  chapter: '集合论',
  unit: '映射与函数',
  unitNo: 4,
  unitTotal: 15,
  questionNo: 13,
  questionTotal: 29,
  stem: '设 `f : A → B` 是单射，下列哪个命题一定成立？',
  plain: '单射就是「不同的输入不会撞到同一个输出」。',
  options: [
    { key: 'A', text: 'A 中的每个元素都有像' },
    { key: 'B', text: 'B 中的每个元素都有原像' },
    { key: 'C', text: '`f` 一定也是满射' },
    { key: 'D', text: 'A 一定比 B 大' },
  ],
}

const KNOWLEDGE = {
  chapter: '集合论',
  unit: '映射与函数',
  title: '单射（injective）',
  summary: '不同的输入一定给出不同的输出。',
  definition: 'Injective f := ∀ a₁ a₂, f a₁ = f a₂ → a₁ = a₂',
  points: ['反例：常数函数不是单射。', '单射不要求满射。', '这条第三点会被截掉，最多显示两条。'],
  source: { label: '课程 · 单元 4', url: 'https://example.com/set-theory/injective' },
}

const LESSON = {
  chapter: '集合论',
  unit: '映射与函数',
  title: '先分清「单射」和「满射」',
  goal: '能用自己的话区分单射、满射与双射。',
  points: ['单射：不同输入 → 不同输出。', '满射：每个输出都有原像。', '双射 = 单射 + 满射。'],
  intuition: '把函数想成两支队伍的配对：单射是「不抢人」，满射是「没人落单」。',
}


const PROGRESS = {
  summary: {
    asked: 49, correct: 44, accuracy: 0.898, streak: 27, coverage: 0.453,
    tagsTouched: 43, tagsTotal: 95, chaptersStarted: 3, currentUnit: '⑦ 函数',
    unitDone: 0, unitTotal: 15, served: 184, dueCount: 0,
  },
  chapters: [{ id: 'I.1', title: '集合与隶属', completion: 0.861, asked: 40, touched: 31, total: 36 }],
  heatRows: [{
    chapter: 'I.2', title: '关系',
    units: [
      { unit: '⑥ 关系', touched: 5, total: 24, mastery: 0.52, level: 1 },
      { unit: '⑦ 函数', touched: 6, total: 20, mastery: 0.567, level: 2 },
    ],
  }],
  hotspots: [{ kind: 'unit', label: '⑥ 关系', reason: '刚起步', mastery: 0.52 }],
  recent: [
    { qid: 'I.1.empty-1', correct: true, at: '2026-11-23T08:35:51Z' },
    { qid: 'I.1.mem-2', correct: false, at: '2026-11-22T13:00:00Z' },
  ],
}

/* -------------------------------------------------------------- the tests */

test('module id + package manifest match what the profile loader expects', () => {
  expect(loaded.id === 'sokonanoda-dsh', 'ModuleLoader id must equal the package name, got ' + loaded.id)
  expect(JSON.stringify(plugin.inject) === JSON.stringify(['slots', 'locale']),
    'client plugin must inject slots + locale')
  const pkg = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8'))
  expect(pkg.name === loaded.id, 'package.json name must match the ModuleLoader id')
  expect(pkg.dsh.bundle.patch === './cordis.patch.yml', 'dsh.bundle.patch must point at cordis.patch.yml')
  expect(pkg.dsh.client.platform === 'web', 'dsh.client.platform must be web')
  expect(JSON.stringify(pkg.exports['./client']) === JSON.stringify('./client.js'),
    'exports["./client"] must point at client.js (the loader reads it)')
  expect(pkg.files.includes('client.js') && pkg.files.includes('index.js') && pkg.files.includes('lib/quiz-proxy.mjs'),
    'both halves and the in-process quiz core must be published')
  const patch = fs.readFileSync(path.join(__dirname, '..', 'cordis.patch.yml'), 'utf8')
  expect(patch.includes('name: sokonanoda-dsh'), 'cordis.patch.yml must insert sokonanoda-dsh')
})

test('client registers exactly three fence layers, no tool view', () => {
  expect(registrations.length === 3, 'expected 3 registrations, got ' + registrations.length)
  const keys = registrations.map((item) => item.definition.key).sort()
  expect(JSON.stringify(keys) === JSON.stringify(['lean', 'learn-card', 'sokonanoda']),
    'fence keys must be lean / learn-card / sokonanoda, got ' + JSON.stringify(keys))
  for (const item of registrations) {
    expect(item.definition.name === 'conversation.chat.markdown.fence',
      'slot name must be conversation.chat.markdown.fence, got ' + item.definition.name)
    expect(typeof item.definition.id === 'string' && item.definition.id !== '', 'hoc layer needs an id')
    expect(typeof item.component === 'function', 'fence layer must be a component')
  }
  const cardFence = registrations.find((item) => item.definition.key === 'learn-card')
  expect(cardFence.definition.locale === plugin.__test.NS, 'the card fence needs the card locale')
  expect(injectedSlotNames.includes('conversation.chat.markdown.fence'), 'must inject the fence slot')
  expect(!registrations.some((item) => item.definition.name === 'tool.call.toolview'),
    'cards are display-only: no tool view may be registered')
})

test('locale dictionaries cover the same chrome keys in zh and en', () => {
  const zhKeys = Object.keys(zh).sort()
  const enKeys = Object.keys(en).sort()
  expect(JSON.stringify(zhKeys) === JSON.stringify(enKeys), 'zh/en key sets differ')
  for (const key of ['pointsLabel', 'knowledgePointsLabel', 'goalLabel', 'intuitionLabel',
    'sourceLabel', 'cardUnavailable', 'cardPreparing', 'unsupportedKind']) {
    expect(typeof zh[key] === 'string' && zh[key] !== '', 'zh missing ' + key)
    expect(typeof en[key] === 'string' && en[key] !== '', 'en missing ' + key)
  }
})

test('the card content stays chrome-free; the fence layer owns the surface', () => {
  expect(injectedStyles.length === 1, 'expected one injected <style>, got ' + injectedStyles.length)
  const rule = cssRule('.dslc-card')
  for (const needle of ['border:0', 'padding:0', 'background:transparent', 'border-radius:0']) {
    expect(rule.includes(needle), '.dslc-card must not paint its own chrome (missing ' + needle + ')')
  }
  const fence = cssRule('.dslc-fence')
  for (const needle of ['border:1px solid', 'border-radius:14px', 'padding:14px 16px', 'background:']) {
    expect(fence.includes(needle), '.dslc-fence surface missing ' + needle)
  }
})

test('mobile-first layout: full width, wrapping, grid option rows, explicit fonts', () => {
  const card = cssRule('.dslc-card')
  for (const needle of ['width:100%', 'max-width:100%', 'box-sizing:border-box', 'overflow-wrap:anywhere']) {
    expect(card.includes(needle), 'card rule missing ' + needle)
  }
  const row = cssRule('.dslc-opt')
  for (const needle of ['display:grid', 'grid-template-columns:26px minmax(0,1fr)', 'min-height:48px', 'border-radius:10px']) {
    expect(row.includes(needle), 'option row missing ' + needle)
  }
})

function fenceProps(kind, payload, extra) {
  return Object.assign({
    lang: 'learn-card',
    code: JSON.stringify({ kind, payload }),
    streaming: false,
    t: translator(zh),
    children: h('pre', { className: 'raw-fence' }, 'RAW CODE'),
  }, extra)
}

test('question fence: option rows render, and a click sends the letter through the composer', () => {
  const calls = []
  const inputActions = {
    setDraft: (value) => calls.push(['setDraft', value]),
    submit: () => calls.push(['submit']),
  }
  const view = render(LearnCardFence(fenceProps('question', QUESTION, {
    inputActions,
    useInput: () => ({ phase: 'plain', draft: '' }),
  })))
  oneByClass(view, 'dslc-fence')
  oneByClass(view, 'dslc-card')
  const options = byClass(view, 'dslc-opt')
  expect(options.length === 4, 'question fence must render 4 options')
  expect(textOf(view).includes('单射'), 'question fence lost the stem')
  expect(!textOf(view).includes('RAW CODE'), 'settled fence must replace the raw code block')
  expect(!textOf(view).includes('```'), 'the fence JSON must not leak into the rendered card')
  expect(typeof options[0].props.onClick === 'function', 'options must be clickable when inputActions exist')
  expect(options[0].props['data-pickable'] === '1', 'clickable option must advertise it')
  options[0].props.onClick()
  expect(JSON.stringify(calls) === JSON.stringify([['setDraft', 'A'], ['submit']]),
    'a click must send exactly the option letter as a normal composer message, got ' + JSON.stringify(calls))
})

test('question fence stays display-only without input actions', () => {
  const quiet = render(LearnCardFence(fenceProps('question', QUESTION)))
  const quietOptions = byClass(quiet, 'dslc-opt')
  expect(quietOptions[0].props.onClick === undefined, 'no inputActions => no click handler')
})

test('knowledge and progress cards stay non-interactive', () => {
  for (const [kind, payload] of [['knowledge', KNOWLEDGE], ['progress', PROGRESS]]) {
    const view = render(LearnCardFence(fenceProps(kind, payload, {
      inputActions: { setDraft: () => {}, submit: () => {} },
      useInput: () => ({ phase: 'plain', draft: '' }),
    })))
    const clickable = nodesOf(view).filter((node) => typeof node.props?.onClick === 'function')
    expect(clickable.length === 0, kind + ' card must not be clickable')
  }
})

test('knowledge fence renders title, definition tokens and source', () => {
  const view = render(LearnCardFence(fenceProps('knowledge', KNOWLEDGE)))
  oneByClass(view, 'dslc-fence')
  expect(textOf(view).includes('单射（injective）'), 'knowledge fence lost the title')
  expect(textOf(view).includes('∀'), 'highlighted definition must keep its text')
  expect(byClass(view, 'dslc-hl').length >= 1, 'definition must be highlighted')
})

test('lesson fence renders goal, points and intuition', () => {
  const view = render(LearnCardFence(fenceProps('lesson', LESSON)))
  oneByClass(view, 'dslc-fence')
  const content = textOf(view)
  for (const needle of ['目标', '知识点', '直觉']) {
    expect(content.includes(needle), 'lesson fence missing ' + needle)
  }
})

test('streaming and malformed fences fail open to the raw code', () => {
  const streaming = render(LearnCardFence(fenceProps('question', QUESTION, { code: '{"kind":"question"', streaming: true })))
  expect(textOf(streaming).includes('RAW CODE'), 'streaming fence must show the raw code')
  const broken = render(LearnCardFence(fenceProps('question', QUESTION, { code: 'not json' })))
  expect(textOf(broken).includes('RAW CODE'), 'unparsable fence must fail open to the raw code')
  const unknown = render(LearnCardFence(fenceProps('weird', { title: 'x' })))
  expect(textOf(unknown).includes('未知卡片类型'), 'unknown kind must stay a visible placeholder card')
})

test('lean fence highlights code without changing the exact source', () => {
  const source = '-- 注释\ntheorem foo : ∀ n : Nat, n = n := by\n  rfl'
  const view = render(LeanFence({ lang: 'lean', code: source, streaming: false }))
  oneByClass(view, 'dslc-lean-wrap')
  oneByClass(view, 'dslc-lean')
  expect(textOf(view) === source, 'highlight must not change the code text')
  expect(byClass(view, 'dslc-hl-com').length >= 1, 'comment token missing')
  expect(byClass(view, 'dslc-hl-kw').length >= 1, 'keyword token missing')
  expect(byClass(view, 'dslc-hl-tactic').length >= 1, 'tactic token missing')
})

test('highlight CSS rules exist', () => {
  for (const selector of ['.dslc-lean-wrap{', '.dslc-lean{', '.dslc-hl-kw{', '.dslc-hl-tactic{',
    '.dslc-hl-type{', '.dslc-hl-str{', '.dslc-hl-com{']) {
    expect(cardCss.includes(selector), 'stylesheet missing ' + selector)
  }
})

test('open question fence renders the typing hint instead of option rows', () => {
  const code = JSON.stringify({ kind: 'question', payload: {
    questionKind: 'short',
    stem: '用一句话解释：为什么证 `A ⊆ B` 只需要一个方向？',
  } })
  const view = render(LearnCardFence({ lang: 'learn-card', code: code, streaming: false, t: translator(zh) }))
  oneByClass(view, 'dslc-fence')
  expect(byClass(view, 'dslc-opt').length === 0, 'an open question has no option rows')
  const hint = oneByClass(view, 'dslc-open-hint')
  expect(textOf(hint).includes('输入框'), 'open card must tell the learner to type the answer')
  expect(textOf(view).includes('只需要一个方向'), 'open card lost the stem')
  expect(nodesOf(view).every((node) => node.type !== 'button'), 'open card stays non-interactive')
})

test('progress fence renders summary, chapter bars, heat cells, hotspots and recent dots', () => {
  const view = render(LearnCardFence({
    lang: 'learn-card',
    code: JSON.stringify({ kind: 'progress', payload: PROGRESS }),
    streaming: false,
    t: translator(zh),
  }))
  oneByClass(view, 'dslc-fence')
  const text = textOf(view)
  for (const needle of ['学习进展', '连对', '27', '正确率', '90%', '知识点覆盖', '45%', '已答题', '章节进展', 'I.1', '掌握热力', '建议关注', '⑥ 关系', '刚起步', '最近']) {
    expect(text.includes(needle), 'progress card missing ' + needle)
  }
  expect(byClass(view, 'dslc-stat').length === 4, 'expected 4 summary stats')
  expect(byClass(view, 'dslc-chapter').length === 1, 'expected one chapter row')
  expect(byClass(view, 'dslc-cell').length >= 2, 'expected heat cells')
  expect(byClass(view, 'dslc-hot').length === 1, 'expected one hotspot row')
  expect(byClass(view, 'dslc-dot').length === 2, 'expected two recent dots')
  expect(nodesOf(view).every((node) => node.type !== 'button'), 'cards stay non-interactive')
})

test('progress card CSS rules exist', () => {
  for (const selector of ['.dslc-stats{', '.dslc-stat{', '.dslc-bar{', '.dslc-heat-row{',
    '.dslc-cell{', '.dslc-legend{', '.dslc-hotspots{', '.dslc-hot{', '.dslc-dot{']) {
    expect(cardCss.includes(selector), 'stylesheet missing ' + selector)
  }
  expect(cssRule('.dslc-cell').includes('width:14px'), 'heat cell must be a compact square')
})

test('fast-path cards carry a top-right provenance badge', () => {
  for (const [kind, payload] of [['question', QUESTION], ['knowledge', KNOWLEDGE], ['progress', PROGRESS]]) {
    const view = render(LearnCardFence(fenceProps(kind, payload)))
    const badges = byClass(view, 'dslc-fast-badge')
    expect(badges.length === 1, kind + ' card must carry exactly one fast-path badge, got ' + badges.length)
    expect(textOf(badges[0]).includes('快思考'), 'badge must read 快思考')
    expect(typeof badges[0].props.title === 'string' && badges[0].props.title.includes('sokonanoda'),
      'badge must explain that this is the local sokonanoda fast path')
  }
  const streaming = render(LearnCardFence(fenceProps('question', QUESTION, {
    code: '{"kind":"question"',
    streaming: true,
  })))
  expect(byClass(streaming, 'dslc-fast-badge').length === 0, 'streaming raw code must not claim the fast path')
  expect(cardCss.includes('.dslc-fast-badge{'), 'badge CSS rule missing')
  expect(cardCss.includes('.dslc-open-hint{'), 'open-hint CSS rule missing')
})

console.log('')
console.log('CLIENT SMOKE: ' + passes + ' passed, ' + failures.length + ' failed')
if (failures.length > 0) process.exitCode = 1
