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
const tabRegistrations = []
const injectedSlotNames = []
const dictionaries = {}
const openedTabs = []
const openedResources = []
const panelTabDefinitions = []
const sidebarRightStub = {
  mounted: { getSnapshot: () => 'session-panel-test' },
  openTab(kind, options) { openedTabs.push({ kind, options }) },
  openResource(address, options) { openedResources.push({ address, options }) },
}
const sidebarRightTabsStub = {
  register(definition) {
    tabRegistrations.push(definition)
    panelTabDefinitions.push(definition)
    return () => {}
  },
}
const pluginApplyCtx = {
  effect(callback) {
    const cleanup = callback()
    return typeof cleanup === 'function' ? cleanup : () => {}
  },
  get(name) {
    return name === 'sidebarRightTabs' ? sidebarRightTabsStub : undefined
  },
  sidebarRight: sidebarRightStub,
  sidebarRightTabs: sidebarRightTabsStub,
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

const ClassNotesPanel = plugin.__test.ClassNotesPanel
const ClassNotesPanelTitle = plugin.__test.ClassNotesPanelTitle
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
  points: ['反例：常数函数不是单射。', '单射不要求满射。', '这条第三点也必须显示，不再截断。'],
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
    kpsTotal: 195, kpsWithQuestions: 96, solvedKps: 13, probingKps: 37, todoKps: 45, weakKps: 1,
    codeSolved: 2, codeSkipped: 0, codeTotal: 916,
  },
  chapters: [{ id: 'I.1', title: '集合与隶属', completion: 0.861, asked: 40, touched: 31, total: 36, solved: 8, probing: 8, todo: 0, weak: 1, codeSolved: 1, codeTotal: 115 }],
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
  expect(JSON.stringify(plugin.inject) === JSON.stringify(['slots', 'locale', 'sidebarRight', 'sidebarRightTabs']),
    'client plugin must inject slots + locale + the sidebar services, got ' + JSON.stringify(plugin.inject))
  const pkg = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8'))
  expect(pkg.name === loaded.id, 'package.json name must match the ModuleLoader id')
  expect(pkg.dsh.bundle.patch === './cordis.patch.yml', 'dsh.bundle.patch must point at cordis.patch.yml')
  expect(pkg.dsh.client.platform === 'web', 'dsh.client.platform must be web')
  expect(JSON.stringify(pkg.exports['./client']) === JSON.stringify('./client.js'),
    'exports["./client"] must point at client.js (the loader reads it)')
  expect(pkg.files.includes('client.js') && pkg.files.includes('index.js') && pkg.files.includes('lib/quiz-proxy.mjs'),
    'both halves and the in-process quiz core must be published')
  expect(pkg.files.includes('lib/exercise-panel.mjs'), 'the panel Host bridge must be published')
  const patch = fs.readFileSync(path.join(__dirname, '..', 'cordis.patch.yml'), 'utf8')
  expect(patch.includes('name: sokonanoda-dsh'), 'cordis.patch.yml must insert sokonanoda-dsh')
})

test('class-notes: right-sidebar page tab, guide entry and iframe body', () => {
  const kind = plugin.__test.NOTES_KIND
  expect(kind === 'sokonanoda-notes', 'notes kind must be sokonanoda-notes')
  const definition = tabRegistrations.find((entry) => entry !== null && entry.kind === kind)
  expect(definition !== undefined, 'class-notes page type must be registered')
  expect(definition.id === 'sokonanoda-dsh/class-notes', 'class-notes type id must be stable')
  expect(Array.isArray(definition.guide) && definition.guide.length === 1,
    'the right-sidebar guide must list one class-notes entry')
  expect(definition.guide[0].id === 'open', 'guide entry id must be open')
  expect(typeof definition.guide[0].title === 'function' && definition.guide[0].title() === '课堂笔记',
    'guide entry title must be 课堂笔记')

  const rendered = render(h(ClassNotesPanel, { t: translator(zh) }))
  const frame = nodesOf(rendered).find((node) => node.type === 'iframe')
  expect(frame !== undefined, 'class-notes body must render an iframe')
  expect(frame.props.src === '/sokonanoda-notes-view', 'iframe must point at the Host view route')
  expect(frame.props.title === '课堂笔记', 'iframe needs an accessible title')
  expect(frame.props.className === 'dslc-notes-frame', 'iframe must carry the panel class')

  const title = render(h(ClassNotesPanelTitle, { t: translator(zh) }))
  expect(textOf(title) === '课堂笔记', 'chip title must be 课堂笔记')
  const moved = render(h(ClassNotesPanel, { t: translator(en) }))
  const movedFrame = nodesOf(moved).find((node) => node.type === 'iframe')
  expect(movedFrame.props.title === 'Class notes', 'English frame title must follow the dictionary')

  const css = cssRule('.dslc-notes-frame')
  expect(css.includes('width:100%') && css.includes('height:100%'), 'iframe CSS must fill the pane')
  expect(cssRule('.dslc-notes-body').includes('height:100%'), 'notes body CSS must fill the pane')
  expect(injectedSlotNames.includes('sidebar.right.pane.tab'), 'notes body must use the sidebar tab slot')
  expect(injectedSlotNames.includes('sidebar.right.pane.tab.title'), 'notes chip must use the title slot')
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

test('a late sidebar-right service opens a card source at its line', () => {
  const opened = []
  let sidebarReady = null
  const late = {
    effect(callback) {
      const cleanup = callback()
      return typeof cleanup === 'function' ? cleanup : () => {}
    },
    locale: { register() { return () => {} } },
    slots: {
      inject(name, callback) { return callback() },
      register() { return () => {} },
    },
    get() { return undefined },
    inject(deps, callback) {
      if (deps.includes('sidebarRight')) {
        sidebarReady = callback
        return
      }
      if (deps.includes('documentPreviews')) {
        callback({ documentPreviews: { register() { return () => {} } } })
      }
    },
  }
  plugin.__test.resetSidebarRight()
  plugin.apply(late)
  expect(typeof sidebarReady === 'function', 'the client must wait for documentPreviews and sidebarRight')
  sidebarReady({
    sidebarRight: {
      mounted: { getSnapshot: () => 'session-1' },
      openResource(address, options) { opened.push({ address, options }) },
    },
  })
  const payload = {
    title: '量词换序',
    source: 'units/I.2/unit07-functions.sokonanoda:194',
    sourceLink: {
      path: '/tmp/soko/units/I.2/unit07-functions.sokonanoda',
      relativePath: 'courses/set-theory/units/I.2/unit07-functions.sokonanoda',
      line: 194,
    },
  }
  const view = render(LearnCardFence(fenceProps('knowledge', payload)))
  oneByClass(view, 'dslc-link').props.onClick({ preventDefault() {} })
  expect(opened.length === 1, 'the source link must reach the late sidebar service')
  expect(opened[0].address.startsWith('dsh-resource://file/session/session-1/'),
    'a mounted session must produce a session-scoped file address, got ' + opened[0].address)
  expect(opened[0].address.includes('unit07-functions.sokonanoda'), 'address lost the source file')
  expect(opened[0].options.kind === 'sokonanoda-dsh/source', 'the plugin preview must be requested')
  expect(opened[0].options.params.line === 194, 'the real line must travel as params.line')
})


test('client registers exactly three fence layers, no tool view', () => {
  const fences = registrations.filter((item) => item.definition.name === 'conversation.chat.markdown.fence')
  expect(fences.length === 3, 'expected 3 fence registrations, got ' + fences.length)
  const keys = fences.map((item) => item.definition.key).sort()
  expect(JSON.stringify(keys) === JSON.stringify(['lean', 'learn-card', 'sokonanoda']),
    'fence keys must be lean / learn-card / sokonanoda, got ' + JSON.stringify(keys))
  for (const item of fences) {
    expect(typeof item.definition.id === 'string' && item.definition.id !== '', 'hoc layer needs an id')
    expect(typeof item.component === 'function', 'fence layer must be a component')
  }
  const cardFence = fences.find((item) => item.definition.key === 'learn-card')
  expect(cardFence.definition.locale === plugin.__test.NS, 'the card fence needs the card locale')
  expect(injectedSlotNames.includes('conversation.chat.markdown.fence'), 'must inject the fence slot')
  expect(!registrations.some((item) => item.definition.name === 'tool.call.toolview'),
    'cards are display-only: no tool view may be registered')
})

test('exercise panel registers a sidebar page type + body and chip seats', () => {
  const panelDefinitions = panelTabDefinitions.filter((entry) => entry !== null && entry.kind === plugin.__test.PANEL_KIND)
  expect(panelDefinitions.length === 1, 'expected one exercise-panel tab definition, got ' + panelDefinitions.length)
  const definition = panelDefinitions[0]
  expect(definition.kind === plugin.__test.PANEL_KIND, 'panel kind mismatch: ' + definition.kind)
  expect(definition.id === plugin.__test.PANEL_TYPE_ID, 'panel type id mismatch: ' + definition.id)
  expect(definition.keepMounted === true, 'the editor body should survive tab hiding')
  expect(typeof definition.title === 'function' && definition.title('') !== '', 'panel type needs a chip title')
  const body = registrations.find((item) => item.definition.name === 'sidebar.right.pane.tab')
  expect(body !== undefined, 'panel body must register into sidebar.right.pane.tab')
  expect(body.definition.key === plugin.__test.PANEL_TYPE_ID, 'panel body key must be the tab type id')
  expect(typeof body.component === 'function', 'panel body must be a component')
  const title = registrations.find((item) => item.definition.name === 'sidebar.right.pane.tab.title')
  expect(title !== undefined, 'panel chip title seat must be registered')
  expect(title.definition.key === plugin.__test.PANEL_TYPE_ID, 'chip title key must be the tab type id')
  expect(injectedSlotNames.includes('sidebar.right.pane.tab'), 'must inject the panel body slot')
  expect(injectedSlotNames.includes('sidebar.right.pane.tab.title'), 'must inject the panel title slot')
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
  expect(textOf(view).includes('这条第三点也必须显示，不再截断。'), 'all legacy points must render (no slice(0, 2))')
})

test('knowledge fence renders every section in order (no truncation)', () => {
  const payload = {
    title: '选择公理',
    oneLiner: '一句话不能挤掉。',
    definition: { lang: 'lean', code: 'def Set.pi (ι α : Type) (A : ι → Set α) : Set (ι → α)' },
    sections: [
      { heading: '段一', kind: 'list', items: ['a', 'b', 'c'] },
      { heading: '段二', kind: 'paragraph', text: '这一段必须出现，而且 `choice` 走 inline code。' },
      { heading: '段三', kind: 'list', items: ['d'] },
      {
        heading: '段四',
        kind: 'table',
        columns: ['条件', '人话'],
        rows: [['非自反', '`a < a` 永远不成立'], ['传递', 'a < b、b < c ⇒ a < c']],
      },
    ],
    source: 'lib/SUnion.sokonanoda:115',
  }
  const view = render(LearnCardFence(fenceProps('knowledge', payload)))
  const text = textOf(view)
  for (const heading of ['段一', '段二', '段三', '段四']) {
    expect(text.includes(heading), 'knowledge section heading lost: ' + heading)
  }
  for (const item of ['a', 'b', 'c', 'd']) {
    expect(byClass(view, 'dslc-slist').length === 2 && text.includes(item),
      'knowledge list item lost: ' + item)
  }
  expect(text.includes('这一段必须出现'), 'paragraph section text must render, not only list sections')
  expect(text.includes('一句话不能挤掉。'), 'oneLiner must stay visible')
  expect(text.includes('Set.pi'), 'definition code must stay visible')
  expect(text.includes('lib/SUnion.sokonanoda:115'), 'source must stay visible')
  expect(byClass(view, 'dslc-sheading').length === 4, 'one section heading per section')
  expect(byClass(view, 'dslc-ptext').length === 1, 'paragraph section must render a paragraph')
  expect(byClass(view, 'dslc-code').length >= 1, 'inline code in a paragraph must go through inlineNodes')
  const lists = byClass(view, 'dslc-slist').map(textOf)
  expect(lists.length === 2 && lists[0] === 'abc' && lists[1] === 'd',
    'list sections must keep every item in order, got ' + JSON.stringify(lists))
  expect(byClass(view, 'dslc-table').length === 1, 'table section must render as a table')
  expect(byClass(view, 'dslc-table').filter((node) => node.type === 'table')[0] !== undefined,
    'table section must use a real <table>')
  expect(text.includes('非自反') && text.includes('人话') && text.includes('a < b、b < c ⇒ a < c'),
    'table header cells and body cells must render')
  expect(byClass(view, 'dslc-code').length >= 2, 'inline code inside table cells must go through inlineNodes')
  expect(text.indexOf('段一') < text.indexOf('段二') && text.indexOf('段二') < text.indexOf('段三')
    && text.indexOf('段三') < text.indexOf('段四'), 'sections must keep engine order')
  expect(cardCss.includes('.dslc-table{') && cardCss.includes('.dslc-table td{'),
    'knowledge section table CSS rules missing')
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

test('source preview numbers every row from its page offset, not from 1', () => {
  const view = render(h(plugin.__test.SokonanodaSourceBody, {
    content: {
      kind: 'text',
      text: 'THIS TEXT MUST NOT BE USED',
      pages: [
        { offset: 1, text: 'theorem a : True := by\n  trivial', lines: 2 },
        { offset: 3, text: '  -- page two\n  exact True.intro', lines: 2 },
      ],
      eof: true,
    },
    wrap: false,
    scrollportRef: () => {},
    useTabInfo: () => ({ tab: { navigation: { revision: 1, params: {} } } }),
  }))
  const numbers = byClass(view, 'dsls-source-no').map((node) => textOf(node))
  expect(JSON.stringify(numbers) === JSON.stringify(['1', '2', '3', '4']),
    'absolute line numbers must follow page offsets, got ' + numbers.join(','))
  const rows = byClass(view, 'dsls-source-line')
  expect(rows.length === 4, 'one row per loaded source line')
  expect(rows[0].props['data-textpreview-line'] === 1 && rows[3].props['data-textpreview-line'] === 4,
    'rows must expose the native owner anchor data-textpreview-line')
  expect(!textOf(view).includes('THIS TEXT MUST NOT BE USED'),
    'loaded pages must be the source of truth, not a re-split content.text')
})

test('source preview navigation marks the absolute target row', () => {
  const view = render(h(plugin.__test.SokonanodaSourceBody, {
    content: {
      kind: 'text',
      text: '',
      pages: [
        { offset: 1, text: 'theorem a : True := by', lines: 1 },
        { offset: 41, text: '  exact True.intro\n  -- tail', lines: 2 },
      ],
      eof: true,
    },
    wrap: false,
    scrollportRef: () => {},
    useTabInfo: () => ({ tab: { navigation: { revision: 9, params: { line: 41 } } } }),
  }))
  const rows = byClass(view, 'dsls-source-line')
  const target = rows.find((node) => node.props['data-textpreview-target'] === 41)
  expect(target !== undefined, 'the target row needs data-textpreview-target')
  expect(target.props['data-textpreview-line'] === 41, 'the target anchor must be the absolute line')
  expect(target.props['data-source-active'] === '1', 'the cursor must sit on the navigation target')
  expect(rows.every((node) => node.props['data-textpreview-target'] === undefined || node.props['data-textpreview-line'] === 41),
    'only the target row carries the target marker')
})

test('a target in a not-yet-loaded page activates when that page arrives', () => {
  const firstOnly = render(h(plugin.__test.SokonanodaSourceBody, {
    content: { kind: 'text', text: '', pages: [{ offset: 1, text: 'theorem long : True := by', lines: 1 }], eof: false },
    wrap: true,
    scrollportRef: () => {},
    useTabInfo: () => ({ tab: { navigation: { revision: 2, params: { line: 41 } } } }),
  }))
  expect(byClass(firstOnly, 'dsls-source-line').length === 1, 'only the loaded page renders at first')
  expect(nodesOf(firstOnly).every((node) => node.props?.['data-textpreview-target'] === undefined),
    'a page that is not loaded yet cannot paint its target row')
  const loaded = render(h(plugin.__test.SokonanodaSourceBody, {
    content: {
      kind: 'text',
      text: '',
      pages: [
        { offset: 1, text: 'theorem long : True := by', lines: 1 },
        { offset: 41, text: '  exact True.intro', lines: 1 },
      ],
      eof: true,
    },
    wrap: true,
    scrollportRef: () => {},
    useTabInfo: () => ({ tab: { navigation: { revision: 2, params: { line: 41 } } } }),
  }))
  const target = byClass(loaded, 'dsls-source-line').find((node) => node.props['data-textpreview-line'] === 41)
  expect(target !== undefined && target.props['data-textpreview-target'] === 41,
    'the arrived page must reveal and mark the target line')
  expect(target.props['data-source-active'] === '1', 'the landed target becomes the active cursor line')
})

test('source preview gutter widens with the largest loaded absolute line', () => {
  const view = render(h(plugin.__test.SokonanodaSourceBody, {
    content: { kind: 'text', text: '', pages: [{ offset: 99, text: 'a\nb\nc', lines: 3 }], eof: true },
    wrap: true,
    scrollportRef: () => {},
    useTabInfo: () => ({ tab: { navigation: { params: {} } } }),
  }))
  const numbers = byClass(view, 'dsls-source-no')
  expect(numbers.map((node) => textOf(node)).join(',') === '99,100,101', 'loaded line numbers must stay absolute')
  expect(numbers.every((node) => node.props.style.minWidth === '3ch'), 'a three-digit line needs a three-character gutter')
})

test('an empty file renders an empty state instead of a bogus line 1', () => {
  const view = render(h(plugin.__test.SokonanodaSourceBody, {
    content: { kind: 'text', text: '', pages: [{ offset: 1, text: '', lines: 0 }], eof: true },
    wrap: true,
    scrollportRef: () => {},
    useTabInfo: () => ({ tab: { navigation: { params: {} } } }),
  }))
  expect(byClass(view, 'dsls-source-line').length === 0, 'a zero-line page must not render a row')
  expect(textOf(oneByClass(view, 'dsls-source-empty')).includes('空文件'), 'the empty state must say the file is empty')
  const copy = oneByClass(view, 'dsls-source-copy')
  expect(copy.props.disabled === true, 'copy is disabled when there is nothing to copy')
})

test('copy button copies the loaded pages', () => {
  let copied = ''
  const previous = Object.getOwnPropertyDescriptor(globalThis, 'navigator')
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: { clipboard: { writeText(value) { copied = value; return Promise.resolve() } } },
  })
  try {
    const view = render(h(plugin.__test.SokonanodaSourceBody, {
      content: {
        kind: 'text',
        text: 'unused',
        pages: [{ offset: 1, text: 'theorem a : True := by\n  trivial', lines: 2 }],
        eof: true,
      },
      wrap: false,
      scrollportRef: () => {},
      useTabInfo: () => ({ tab: { navigation: { params: {} } } }),
    }))
    const copy = oneByClass(view, 'dsls-source-copy')
    expect(typeof copy.props.onClick === 'function', 'a non-empty preview must offer copy')
    copy.props.onClick()
    expect(copied === 'theorem a : True := by\n  trivial', 'copy must join exactly the loaded pages')
  } finally {
    if (previous === undefined) delete globalThis.navigator
    else Object.defineProperty(globalThis, 'navigator', previous)
  }
})

test('source preview CSS covers toolbar, copy and empty states', () => {
  for (const selector of ['.dsls-source-toolbar{', '.dsls-source-copy{', '.dsls-source-empty{',
    '.dsls-source-line[data-textpreview-target]{']) {
    expect(cardCss.includes(selector), 'stylesheet missing ' + selector)
  }
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

/** Rebind the module-level sidebar service after the late-service test resets it. */
function restoreSidebar() {
  plugin.apply({
    effect(callback) { const cleanup = callback(); return typeof cleanup === 'function' ? cleanup : () => {} },
    get(name) { return name === 'sidebarRightTabs' ? sidebarRightTabsStub : undefined },
    sidebarRight: sidebarRightStub,
    sidebarRightTabs: sidebarRightTabsStub,
    locale: { register() { return () => {} } },
    slots: {
      inject(name, callback) { return callback() },
      register() { return () => {} },
    },
  })
}

test('code fence renders prompt, hints, template and the panel deep link', () => {
  restoreSidebar()
  const view = render(LearnCardFence(fenceProps('code', CODE)))
  oneByClass(view, 'dslc-fence')
  const text = textOf(view)
  for (const needle of ['代码练习', 'I.1', '元素落在子集里。', '思路', '关键件', 'mem_of_subset', 'sorry', '真内核', '在侧栏面板中打开']) {
    expect(text.includes(needle), 'code card missing ' + needle)
  }
  const template = oneByClass(view, 'dslc-def')
  expect(textOf(template).includes('theorem mem_of_subset'), 'template must keep the declaration')
  const openers = nodesOf(view).filter((node) => node.props?.['data-action'] === 'open-exercise-panel')
  expect(openers.length === 1, 'code card must carry exactly one panel deep-link button, got ' + openers.length)
  expect(openers[0].props['data-exercise-id'] === CODE.id, 'deep link must name the exercise id')
  const before = openedTabs.length
  expect(typeof openers[0].props.onClick === 'function', 'deep link must be clickable')
  openers[0].props.onClick()
  expect(openedTabs.length === before + 1, 'clicking the deep link must open one sidebar tab')
  const opened = openedTabs[openedTabs.length - 1]
  expect(opened.kind === plugin.__test.PANEL_KIND, 'deep link must open the exercise panel kind, got ' + opened.kind)
  expect(opened.options.params.exerciseId === CODE.id, 'deep link params lost the exercise id')
  expect(opened.options.params.title === CODE.title, 'deep link params lost the exercise title')
  expect(nodesOf(view).every((node) => node.props?.onClick === undefined
    || node.props?.['data-action'] === 'open-exercise-panel'), 'only the panel deep link may be clickable')
  expect(!text.includes('```'), 'the fence JSON must not leak into the code card')
  expect(!text.includes('RAW CODE'), 'settled code fence must replace the raw block')
  expect(byClass(view, 'dslc-fast-badge').length === 1, 'code card must carry the fast-path badge')
  expect(cardCss.includes('.dslc-code-hint{'), 'code submit-hint CSS rule missing')
  expect(cardCss.includes('.dslc-code-meta{'), 'code meta CSS rule missing')
  for (const dict of [zh, en]) {
    for (const key of ['codeLabel', 'codeTemplateLabel', 'codeSubmitHint', 'codeOpenPanel',
      'progressMapTitle', 'progressMapSolved', 'progressMapProbing', 'progressMapTodo', 'progressMapWeak', 'progressMapCode']) {
      expect(typeof dict[key] === 'string' && dict[key] !== '', 'locale missing ' + key)
    }
  }
})

test('code card without a sidebar service or exercise id keeps the chat-only path', () => {
  const noId = render(LearnCardFence(fenceProps('code', Object.assign({}, CODE, { id: '' }))))
  expect(nodesOf(noId).every((node) => node.props?.['data-action'] !== 'open-exercise-panel'),
    'a card without an exercise id must not offer the panel')
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
  for (const needle of ['学习进展', '连对', '27', '正确率', '90%', '知识点覆盖', '45%', '已答题', '章节进展', 'I.1', '掌握热力', '建议关注', '⑥ 关系', '刚起步', '最近', '学习地图', '已解 13', '探针中 37', '待做 45', '薄弱 1', '代码 2/916']) {
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

/* --------------------------------------------------------- exercise panel */

const { exercisePanelView, ExercisePanelTitle, PANEL_TYPE_ID } = plugin.__test

function panelView(attempt, extra) {
  return render(exercisePanelView(Object.assign({
    t: translator(zh),
    phase: 'ready',
    exercise: CODE,
    code: CODE.template,
    attempt,
    busy: false,
    errorMessage: '',
    onCode: () => {},
    onRun: () => {},
    onReset: () => {},
    onRetry: () => {},
  }, extra || {})))
}

test('exercise panel: loads the trusted template into a real editor and offers Run', () => {
  const view = panelView(null)
  const root = view
  expect(root.props['data-panel-state'] === 'ready', 'panel must enter the ready state')
  expect(root.props['data-exercise-id'] === CODE.id, 'panel root must name the exercise id')
  const text = textOf(view)
  for (const needle of ['代码练习', '元素落在子集里。', '思路', '编辑声明', '运行（真内核判卷）', '恢复模板']) {
    expect(text.includes(needle), 'panel missing ' + needle)
  }
  const editor = nodesOf(view).find((node) => node.props?.['data-exercise-editor'] === '1')
  expect(editor !== undefined, 'panel must render a real editor')
  expect(editor.props.value === CODE.template, 'editor must start from the trusted template')
  expect(editor.props.readOnly !== true, 'editor must be editable')
  const run = nodesOf(view).find((node) => node.props?.['data-action'] === 'run-exercise')
  expect(run !== undefined && typeof run.props.onClick === 'function', 'Run must be clickable')
  expect(nodesOf(view).every((node) => node.props?.['data-panel-result'] === undefined), 'no result before a run')
})

test('exercise panel: a pass comes only from the kernel result', () => {
  const view = panelView({ phase: 'done', result: {
    ok: true,
    passed: true,
    target: 'mem_of_subset',
    checked: ['mem_of_subset'],
    open: [],
    diagnostics: [],
    exampleChecked: 0,
  } })
  const result = nodesOf(view).find((node) => node.props?.['data-panel-result'] === 'pass')
  expect(result !== undefined, 'a kernel pass must render the pass box')
  const text = textOf(view)
  for (const needle of ['通过（真内核判卷）', 'mem_of_subset', 'open：0', 'diagnostic：0']) {
    expect(text.includes(needle), 'pass box missing ' + needle)
  }
})

test('exercise panel: diagnostics, sorry and service failures never show a pass', () => {
  const failed = panelView({ phase: 'done', result: {
    ok: true,
    passed: false,
    target: 'mem_of_subset',
    checked: [],
    open: [],
    diagnostics: [{ code: 'elab-tactic-failed', message: '期望 x ∈ C，实际是 x ∈ B', line: 6 }],
  } })
  const failBox = nodesOf(failed).find((node) => node.props?.['data-panel-result'] === 'fail')
  expect(failBox !== undefined, 'a kernel rejection must render the fail box')
  const failText = textOf(failed)
  for (const needle of ['还没通过', '目标', 'elab-tactic-failed', '第 6 行', '期望 x ∈ C']) {
    expect(failText.includes(needle), 'fail box missing ' + needle)
  }
  expect(!failText.includes('✅'), 'a rejection must not claim a pass')
  expect(nodesOf(failed).every((node) => node.props?.['data-panel-result'] !== 'pass'), 'a rejection must not render a pass box')

  const sorry = panelView({ phase: 'done', result: {
    ok: true, passed: false, target: 'mem_of_subset', checked: [], open: ['mem_of_subset'], diagnostics: [],
  } })
  expect(textOf(sorry).includes('还是 sorry'), 'an open target must explain the sorry')

  const service = panelView({ phase: 'error', message: 'timeout' })
  const errorBox = nodesOf(service).find((node) => node.props?.['data-panel-result'] === 'error')
  expect(errorBox !== undefined, 'a service failure must render the unavailable box')
  expect(textOf(service).includes('判卷服务不可用'), 'service failure must be explicit')
  expect(textOf(service).includes('timeout'), 'service failure must keep the Host reason')
  expect(!textOf(service).includes('通过（真内核判卷）'), 'a service failure must never look like a pass')
})

test('exercise panel: user-level submission errors are explicit, not infrastructure failures', () => {
  const mismatch = panelView({ phase: 'done', result: { ok: false, code: 'name-mismatch', expectedName: 'mem_of_subset', foundName: 'foo' } })
  expect(textOf(mismatch).includes('声明名对不上'), 'name mismatch must be explained')
  expect(textOf(mismatch).includes('mem_of_subset'), 'name mismatch must name the expected declaration')
  expect(textOf(mismatch).includes('还没通过'), 'a rejected submission stays "not passed yet"')
})

test('exercise panel: load failure is explicit and retryable', () => {
  const view = render(exercisePanelView({
    t: translator(zh), phase: 'error', exercise: null, code: '', attempt: null, busy: false,
    errorMessage: 'not-found', onRetry: () => {},
  }))
  expect(view.props['data-panel-state'] === 'error', 'load failure state missing')
  const text = textOf(view)
  expect(text.includes('题目加载失败') && text.includes('not-found'), 'load failure text missing: ' + text)
  const retry = nodesOf(view).find((node) => node.props?.['data-action'] === 'retry-exercise-load')
  expect(retry !== undefined && typeof retry.props.onClick === 'function', 'load failure must offer retry')
})

test('exercise panel: chip title prefers the exercise title from navigation params', () => {
  const chip = ExercisePanelTitle({
    useTabInfo: () => ({ tab: { navigation: { params: { title: '子集传递' } } } }),
    t: translator(zh),
  })
  expect(textOf(chip) === '子集传递', 'chip must show the exercise title')
  const fallback = ExercisePanelTitle({ useTabInfo: () => ({ tab: { navigation: { params: {} } } }), t: translator(zh) })
  expect(textOf(fallback) === zh.panelTitle, 'chip must fall back to the panel title')
})

test('exercise panel CSS rules exist (surface, editor, result states)', () => {
  for (const selector of ['.dslc-panel{', '.dslc-panel-editor{', '.dslc-panel-run{',
    '.dslc-panel-open{', '.dslc-panel-result[data-state="pass"]{', '.dslc-panel-result[data-state="fail"]{',
    '.dslc-panel-result[data-state="error"]{']) {
    expect(cardCss.includes(selector), 'stylesheet missing ' + selector)
  }
})

console.log('')
console.log('CLIENT SMOKE: ' + passes + ' passed, ' + failures.length + ' failed')
if (failures.length > 0) process.exitCode = 1
