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
globalThis.fetch = undefined

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
  return (key, vars) => {
    const value = typeof dict[key] === 'string' ? dict[key] : key
    if (vars === undefined) return value
    return value.replace(/\{(\w+)\}/g, (whole, name) => vars[name] === undefined ? whole : String(vars[name]))
  }
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
    kpsTotal: 195, kpsWithQuestions: 96, solvedKps: 50, todoKps: 45, weakKps: 1,
    codeSolved: 2, codeSkipped: 0, codeTotal: 916,
  },
  chapters: [{ id: 'I.1', title: '集合与隶属', completion: 0.861, asked: 40, touched: 31, total: 36, solved: 16, todo: 0, weak: 1, codeSolved: 1, codeTotal: 115 }],
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

const CODE = {
  id: 'set-theory/I.1/unit01-sets-membership/mem_of_subset',
  course: 'set-theory',
  chapter: 'I.1',
  unit: '① 集合与隶属',
  title: '元素落在子集里。',
  prompt: '练习 1（T 类）：元素落在子集里。',
  kind: 'theorem',
  hints: [
    { label: '思路', body: '手里 h 是 A ⊆ B，把 h 当函数用。' },
    { label: '关键件', body: '先 intro 掉 a ∈ A。' },
  ],
  template: 'theorem mem_of_subset (α : Type) (A B : Set α) (h : A ⊆ B) (a : α) :\n    a ∈ A → a ∈ B := by\n  sorry',
  submitHint: '把完整声明贴回来（从 theorem/def 那行到证明结束），我用真内核判卷；想放弃回「跳过这道代码题」。',
  source: 'courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda:44',
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

test('a late document-preview service still receives the sokonanoda body', () => {
  const previews = []
  const slots = []
  const late = {
    effect(callback) {
      const cleanup = callback()
      return typeof cleanup === 'function' ? cleanup : () => {}
    },
    locale: { register() { return () => {} } },
    slots: {
      inject(name, callback) { return callback() },
      register(definition, component) { slots.push({ definition, component }); return () => {} },
    },
    get(name) { return name === 'sidebarRight' ? { openResource() {} } : undefined },
    inject(deps, callback) {
      expect(deps.includes('documentPreviews'), 'the client must wait for documentPreviews')
      callback({
        documentPreviews: {
          register(definition) { previews.push(definition); return () => {} },
        },
      })
    },
  }
  plugin.apply(late)
  const preview = previews.find((definition) => definition.id === 'sokonanoda-dsh/source')
  expect(preview !== undefined, 'late documentPreviews must still register the source preview')
  expect(preview.extensions.includes('sokonanoda'), 'preview must claim the sokonanoda suffix')
  expect(preview.loading === 'text-pages' && preview.sourceLines === true, 'preview must use paged text with line anchors')
  const body = slots.find((entry) => entry.definition.name === 'sidebar.right.tab.document')
  expect(body !== undefined, 'the source body must register on the document slot')
  expect(body.definition.locale === plugin.__test.NS, 'the body must receive the card locale namespace')
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
  expect(options.length === 5, 'question fence must render 4 choices + the 我不知道 support row')
  expect(textOf(view).includes('单射'), 'question fence lost the stem')
  expect(!textOf(view).includes('RAW CODE'), 'settled fence must replace the raw code block')
  expect(!textOf(view).includes('```'), 'the fence JSON must not leak into the rendered card')
  const skip = oneByClass(view, 'dslc-opt-skip')
  expect(textOf(skip).includes('我不知道'), 'skip row must say 我不知道')
  expect(typeof skip.props.onClick === 'function', 'skip row must be clickable when inputActions exist')
  skip.props.onClick()
  expect(JSON.stringify(calls) === JSON.stringify([['setDraft', '我不知道'], ['submit']]),
    'the skip row must send the phrase (not a letter), got ' + JSON.stringify(calls))

  const letterCalls = []
  const letterView = render(LearnCardFence(fenceProps('question', QUESTION, {
    inputActions: {
      setDraft: (value) => letterCalls.push(['setDraft', value]),
      submit: () => letterCalls.push(['submit']),
    },
    useInput: () => ({ phase: 'plain', draft: '' }),
  })))
  const letterOptions = byClass(letterView, 'dslc-opt')
  expect(options.length === 5, 'question fence must render 4 choices + skip row')
  expect(typeof letterOptions[0].props.onClick === 'function', 'options must be clickable when inputActions exist')
  expect(letterOptions[0].props['data-pickable'] === '1', 'clickable option must advertise it')
  letterOptions[0].props.onClick()
  expect(JSON.stringify(letterCalls) === JSON.stringify([['setDraft', 'A'], ['submit']]),
    'a click must send exactly the option letter as a normal composer message, got ' + JSON.stringify(letterCalls))
})

test('question fence stays display-only without input actions', () => {
  const quiet = render(LearnCardFence(fenceProps('question', QUESTION)))
  const quietOptions = byClass(quiet, 'dslc-opt')
  expect(quietOptions.length === 4, 'display-only card has no 我不知道 row')
  expect(quietOptions[0].props.onClick === undefined, 'no inputActions => no click handler')
})

test('review question card highlights the correct option and disables answering', () => {
  const review = Object.assign({}, QUESTION, { review: true, correctKey: 'B', why: '自反是每个元素与自己有关系。' })
  const view = render(LearnCardFence(fenceProps('question', review, {
    inputActions: { setDraft: () => {}, submit: () => {} },
    useInput: () => ({ phase: 'plain', draft: '' }),
  })))
  const options = byClass(view, 'dslc-opt')
  expect(options.length === 4, 'review card must still show its four options')
  expect(byClass(view, 'dslc-opt-correct').length === 1, 'review card must mark exactly one correct option')
  expect(textOf(view).includes('回看') && textOf(view).includes('正确答案 B'), 'review note missing')
  expect(textOf(view).includes('不计分'), 'review note must say it is not graded')
  expect(textOf(view).includes('自反是每个元素与自己有关系。'), 'review must show the why explanation')
  expect(options.every((option) => option.props.onClick === undefined), 'review options must not be clickable')
  expect(byClass(view, 'dslc-opt-skip').length === 0, 'review card must not offer 我不知道')
  expect(cardCss.includes('.dslc-opt-correct{'), 'correct-option CSS rule missing')
  expect(cardCss.includes('.dslc-review{'), 'review CSS rule missing')
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

test('knowledge source with sourceLink renders a right-sidebar file link', () => {
  const payload = {
    title: '量词换序',
    oneLiner: '∃∀ ⇒ ∀∃',
    source: 'units/I.2/unit07-functions.sokonanoda:194',
    sourceLink: {
      path: '/tmp/soko/units/I.2/unit07-functions.sokonanoda',
      relativePath: 'courses/set-theory/units/I.2/unit07-functions.sokonanoda',
      line: 194,
    },
  }
  const view = render(LearnCardFence(fenceProps('knowledge', payload)))
  const link = oneByClass(view, 'dslc-link')
  expect(/^dsh-resource:\/\/file\//.test(link.props.href), 'source link must use a dsh-resource address')
  expect(link.props.href.includes('unit07-functions.sokonanoda'), 'source link lost the file path')
  expect(typeof link.props.onClick === 'function', 'source link must handle click through the sidebar service')
  expect(textOf(view).includes('194'), 'source label must keep the line number')
})

test('.sokonanoda source preview renders a line-numbered Lean view', () => {
  const source = 'theorem swap_exists_forall (α β : Type) (P : α → β → Prop) :\n    ∀ (x : α), ∃ (y : β), P x y := by\n  intro x\n  sorry'
  const view = render(h(plugin.__test.SokonanodaSourceBody, {
    content: { kind: 'text', text: source, pages: [], eof: true },
    wrap: false,
    scrollportRef: () => {},
    useTabInfo: () => ({ tab: { navigation: { params: { line: 2 } } } }),
  }))
  const numbers = byClass(view, 'dsls-source-no')
  expect(numbers.length === 4, 'source preview must render one gutter number per line')
  expect(textOf(view).includes('swap_exists_forall'), 'source preview lost the file text')
  expect(byClass(view, 'dsls-source-line').length === 4, 'source preview must render one row per line')
  expect(byClass(view, 'dslc-hl').length >= 1, 'source preview must run the Lean highlighter')
})

test('source cursor: navigation target becomes the active row, focusable for arrows', () => {
  const source = 'theorem demo : True := by\n  trivial'
  const view = render(h(plugin.__test.SokonanodaSourceBody, {
    content: { kind: 'text', text: source, pages: [], eof: true },
    wrap: false,
    resourceAddress: 'dsh-resource://file/session/s1/demo.sokonanoda',
    scrollportRef: () => {},
    useTabInfo: () => ({ tab: { navigation: { revision: 7, params: { line: 2 } } } }),
  }))
  const active = nodesOf(view).filter((node) => node.props?.['data-source-active'] === '1')
  expect(active.length === 1, 'exactly one row must be active')
  expect(active[0].props['data-source-line'] === 2, 'navigation line 2 must be the active row')
  const root = nodesOf(view).find((node) => node.props?.className === 'dsls-source')
  expect(root !== undefined && root.props['data-active-line'] === 2, 'preview root must expose the active line')
  const scroll = nodesOf(view).find((node) => node.props?.className === 'dsls-source-scroll')
  expect(scroll !== undefined && scroll.props.tabIndex === 0, 'preview must be focusable so arrows reach it')
  expect(scroll.props.role === 'region', 'preview must expose a region role')
  expect(typeof scroll.props.onKeyDown === 'function', 'preview must handle arrow keys')
  oneByClass(view, 'dsls-source-panel')
})

test('source cursor: arrow moves clamp to loaded lines without escaping the file', () => {
  const next = plugin.__test.nextActiveLine
  expect(next(0, 1, 10) === 1, 'down from no line starts at line 1')
  expect(next(0, -1, 10) === 10, 'up from no line starts at the last loaded line')
  expect(next(5, 1, 10) === 6, 'down moves one line')
  expect(next(5, -1, 10) === 4, 'up moves one line')
  expect(next(10, 1, 10) === 10, 'down clamps at the last loaded line')
  expect(next(1, -1, 10) === 1, 'up clamps at line 1')
  expect(next(0, 1, 0) === 0, 'an empty file has no active line')
})

test('goal panel renders declaration, step, hypotheses and semantic goal runs', () => {
  const state = {
    decl: { name: 'demo_diag', kind: 'theorem', status: 'checked' },
    binders: [{ name: 'α', ty: 'Type', ty_runs: [{ text: 'Type', kind: 'sort' }] }],
    goals: [{
      goal: 'False',
      goal_runs: [{ text: 'False', kind: 'keyword' }],
      binders: [{ name: 'h', ty: '¬ True', ty_runs: [{ text: '¬', kind: 'keyword' }, { text: ' ' }, { text: 'True', kind: 'keyword' }] }],
    }],
    goal: 'False',
    step: 4,
    total: 6,
  }
  const view = render(h(plugin.__test.SourceGoalPanel, { line: 88, phase: 'ready', state, t: translator(zh) }))
  const text = textOf(view)
  for (const needle of ['第 88 行', 'demo_diag', 'theorem', '已检查', '步骤 4/6', 'h', '¬ True', '⊢', 'False']) {
    expect(text.includes(needle), 'goal panel missing ' + needle + ' in ' + text)
  }
  oneByClass(view, 'dsls-goal')
  oneByClass(view, 'dsls-hyp')
  expect(byClass(view, 'dsls-run-keyword').length >= 2, 'semantic runs must colour the goal/hypothesis')
  const chip = byClass(view, 'dsls-state-chip').find((node) => node.props['data-status'] === 'checked')
  expect(chip !== undefined, 'checked status chip missing')
})

test('goal panel distinguishes closed, outside and broken-file states', () => {
  const closed = render(h(plugin.__test.SourceGoalPanel, {
    line: 4, phase: 'ready', t: translator(zh),
    state: { decl: { name: 'done', kind: 'theorem', status: 'checked' }, goals: [], step: 2, total: 2 },
  }))
  expect(textOf(closed).includes('已无目标'), 'a closed proof must say so')
  const outside = render(h(plugin.__test.SourceGoalPanel, {
    line: 1, phase: 'outside', t: translator(zh),
    error: { code: 'outside-declarations', message: 'x' },
  }))
  expect(textOf(outside).includes('光标不在任何声明内'), 'outside state must be explicit')
  const broken = render(h(plugin.__test.SourceGoalPanel, {
    line: 3, phase: 'file-errors', t: translator(zh),
    error: { code: 'file-errors', message: '源码有 1 处错误' },
    diagnostics: [{ line: 3, message: 'unexpected token' }],
  }))
  expect(textOf(broken).includes('第 3 行'), 'diagnostic line must be visible')
  expect(textOf(broken).includes('unexpected token'), 'diagnostic message must be visible')
})

test('semantic runs never re-tokenize: mismatched runs fall back to the field text', () => {
  const exact = plugin.__test.semanticRuns(
    [{ text: 'α', kind: 'binder' }, { text: ' → ' }, { text: 'β', kind: 'binder' }],
    'α → β',
  )
  expect(textOf(render(h('div', {}, exact))) === 'α → β', 'run rendering must preserve the exact text')
  expect(plugin.__test.semanticRuns([{ text: 'wrong' }], 'α → β') === 'α → β',
    'mismatched runs must fall back to plain text')
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

test('inline markdown bold becomes <strong> in question cards', () => {
  const code = JSON.stringify({ kind: 'question', payload: {
    stem: '两个**不同**的集合能互相包含吗？',
    options: [
      { key: 'A', text: '**不能**，外延性要求元素完全相同' },
      { key: 'B', text: '能，包含是单向的' },
    ],
  } })
  const view = render(LearnCardFence({ lang: 'learn-card', code: code, streaming: false, t: translator(zh) }))
  const strongs = nodesOf(view).filter((node) => node.type === 'strong')
  expect(strongs.length === 2, 'both stem and option bold runs should render as <strong>')
  expect(strongs.some((node) => textOf(node).includes('不同')), 'stem bold text missing')
  expect(!textOf(view).includes('**'), 'literal asterisks leaked into the card')
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

test('code fence renders prompt, hints and the trusted template without interaction', () => {
  const view = render(LearnCardFence(fenceProps('code', CODE)))
  oneByClass(view, 'dslc-fence')
  const text = textOf(view)
  for (const needle of ['代码练习', 'I.1', '元素落在子集里。', '思路', '关键件', 'mem_of_subset', 'sorry', '真内核']) {
    expect(text.includes(needle), 'code card missing ' + needle)
  }
  const template = oneByClass(view, 'dslc-def')
  expect(textOf(template).includes('theorem mem_of_subset'), 'template must keep the declaration')
  expect(nodesOf(view).every((node) => node.props?.onClick === undefined), 'code card stays non-interactive')
  expect(!text.includes('```'), 'the fence JSON must not leak into the code card')
  expect(!text.includes('RAW CODE'), 'settled code fence must replace the raw block')
  expect(byClass(view, 'dslc-fast-badge').length === 1, 'code card must carry the fast-path badge')
  expect(cardCss.includes('.dslc-code-hint{'), 'code submit-hint CSS rule missing')
  expect(cardCss.includes('.dslc-code-meta{'), 'code meta CSS rule missing')
  for (const dict of [zh, en]) {
    for (const key of ['codeLabel', 'codeTemplateLabel', 'codeSubmitHint',
      'progressMapTitle', 'progressMapSolved', 'progressMapTodo', 'progressMapWeak', 'progressMapCode']) {
      expect(typeof dict[key] === 'string' && dict[key] !== '', 'locale missing ' + key)
    }
  }
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
  for (const needle of ['学习进展', '连对', '27', '正确率', '90%', '知识点覆盖', '45%', '已答题', '章节进展', 'I.1', '掌握热力', '建议关注', '⑥ 关系', '刚起步', '最近', '学习地图', '已解 50', '待做 45', '薄弱 1', '代码 2/916']) {
    expect(text.includes(needle), 'progress card missing ' + needle)
  }
  expect(byClass(view, 'dslc-stat').length === 4, 'expected 4 summary stats')
  expect(byClass(view, 'dslc-map').length === 1, 'expected one learning-map line')
  expect(byClass(view, 'dslc-chapter').length === 1, 'expected one chapter row')
  expect(byClass(view, 'dslc-cell').length >= 2, 'expected heat cells')
  expect(byClass(view, 'dslc-hot').length === 1, 'expected one hotspot row')
  expect(byClass(view, 'dslc-dot').length === 2, 'expected two recent dots')
  expect(nodesOf(view).every((node) => node.type !== 'button'), 'cards stay non-interactive')
})

test('progress card CSS rules exist', () => {
  for (const selector of ['.dslc-stats{', '.dslc-stat{', '.dslc-map{', '.dslc-bar{', '.dslc-heat-row{',
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
  expect(cardCss.includes('.dslc-bold{'), 'bold CSS rule missing')
  expect(cardCss.includes('.dslc-opt-skip{'), 'skip CSS rule missing')
})

console.log('')
console.log('CLIENT SMOKE: ' + passes + ' passed, ' + failures.length + ' failed')
if (failures.length > 0) process.exitCode = 1
