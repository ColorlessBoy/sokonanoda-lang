/**
 * Client-half regression test for `dsh-inline-image`.
 *
 * It pins the user-visible contract of the `dsh-images` deck against the seven
 * bundled `public/ch-01.png … ch-07.png` samples:
 *   - the module registers both fence layers and keeps the package id;
 *   - a settled deck renders seven slides, dots, page counter, caption, an
 *     eager first page and lazy later pages, and a polite live announcer;
 *   - a dot click / keyboard press changes the visible page and caption;
 *   - the two neighbouring pages are prefetched (never the whole deck);
 *   - a failed page keeps its path and offers a retry that busts the cache;
 *   - the lightbox opens from the deck, zooms, closes with Escape and restores
 *     focus to the deck stage;
 *   - a newer Client bundle on disk produces the refresh hint;
 *   - malformed fences fail visibly instead of crashing the chat.
 *
 *   node test/client.test.mjs      # 0 = all PASS, 1 = a FAIL
 */
import fs from 'node:fs'
import path from 'node:path'
import zlib from 'node:zlib'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const TEST_DIR = dirname(fileURLToPath(import.meta.url))
const PLUGIN_DIR = join(TEST_DIR, '..')
const PUBLIC_DIR = join(PLUGIN_DIR, 'public')
const CLIENT = join(PLUGIN_DIR, 'client.js')
const source = fs.readFileSync(CLIENT, 'utf8')

let passes = 0
const failures = []
function expect(condition, message) {
  if (!condition) throw new Error(message)
}
async function test(name, body) {
  try {
    await body()
    passes += 1
    console.log('PASS  ' + name)
  } catch (error) {
    failures.push(name)
    console.error('FAIL  ' + name)
    console.error('      ' + (error instanceof Error ? error.message : String(error)))
  }
}

/* ------------------------------------------------------------ sample pages */

const SAMPLE_PATHS = ['ch-01.png', 'ch-02.png', 'ch-03.png', 'ch-04.png', 'ch-05.png', 'ch-06.png', 'ch-07.png']
const CAPTIONS = [
  '封面：无穷之间，还有没有第三档？',
  '一样多 = 能一一配对',
  '对角线：D 不在名单里',
  '连续统假设：不存在中间基数',
  '可数无穷 ℵ₀',
  '实数比自然数多',
  '没有最大的无穷',
]
const SAMPLE_SPEC = {
  title: '连续统假设 · 7 页',
  pages: SAMPLE_PATHS.map((file, index) => ({ path: file, caption: CAPTIONS[index] })),
}
const SAMPLE_JSON = JSON.stringify(SAMPLE_SPEC)

/* The darkPath fixtures: one shared 1147×588 canvas, a white light plate and a
 * #141414 dark plate with the same text layout. */
const DARK_SPEC = {
  title: '深浅讲解图',
  pages: [
    { path: 's7-light.png', darkPath: 's7-dark.png', caption: '同画布 · 深浅两套' },
    { path: 'ch-02.png', darkPath: 'ch-05.png', caption: '邻近页也带深色图' },
    { path: 'ch-03.png', caption: '没有深色图的一页' },
  ],
}
const DARK_JSON = JSON.stringify(DARK_SPEC)

/** Read a PNG's IHDR so the regression pins the shared 1147×588 canvas. */
function pngSize(file) {
  const bytes = fs.readFileSync(file)
  const signature = bytes.subarray(0, 8).toString('hex')
  expect(signature === '89504e470d0a1a0a', file + ' is not a PNG')
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) }
}

/** First pixel of a PNG (inside the first scanline, after its filter byte). */
function pngFirstPixel(file) {
  const bytes = fs.readFileSync(file)
  const chunks = []
  let offset = 8
  while (offset < bytes.length) {
    const length = bytes.readUInt32BE(offset)
    const type = bytes.toString('ascii', offset + 4, offset + 8)
    if (type === 'IDAT') chunks.push(bytes.subarray(offset + 8, offset + 8 + length))
    if (type === 'IEND') break
    offset += 12 + length
  }
  const raw = zlib.inflateSync(Buffer.concat(chunks))
  return [raw[1], raw[2], raw[3]]
}

/* --------------------------------------------------------- browser harness */

const realDateNow = Date.now.bind(Date)
let clockOffsetMs = 0
Date.now = () => realDateNow() + clockOffsetMs

const preloaded = []
class FakeImage {
  constructor() {
    this.decoding = ''
    this.src = ''
    preloaded.push(this)
  }
}

let reducedMotion = false
let fetchResponse = null
const fetchCalls = []
let reloads = 0

/* Theme fakes: the real Client reads the DSH body attribute plus the media
 * query and must react to changes, so the harness models both signals. */
const THEME_ATTR = 'data-ds-dark-theme'
const DARK_QUERY = '(prefers-color-scheme: dark)'
const bodyAttributes = new Map()
const bodyObservers = []
let prefersDark = false
const mediaQueries = new Map()

function fakeMatchMedia(query) {
  let entry = mediaQueries.get(query)
  if (entry === undefined) {
    const listeners = new Set()
    entry = {
      listeners,
      addEventListener(type, listener) { if (type === 'change') listeners.add(listener) },
      removeEventListener(type, listener) { listeners.delete(listener) },
      addListener(listener) { listeners.add(listener) },
      removeListener(listener) { listeners.delete(listener) },
    }
    Object.defineProperty(entry, 'matches', {
      get() {
        if (query === DARK_QUERY) return prefersDark
        if (query.includes('prefers-reduced-motion')) return reducedMotion
        return false
      },
    })
    mediaQueries.set(query, entry)
  }
  return entry
}

class FakeMutationObserver {
  constructor(callback) {
    this.callback = callback
    this.disconnected = false
  }
  observe(target, options) {
    this.target = target
    this.options = options
    bodyObservers.push(this)
  }
  disconnect() {
    this.disconnected = true
  }
}

/** Flip the DSH `<body data-ds-dark-theme>` attribute and notify observers. */
function setDarkAttribute(active) {
  if (active) bodyAttributes.set(THEME_ATTR, '')
  else bodyAttributes.delete(THEME_ATTR)
  const record = { type: 'attributes', attributeName: THEME_ATTR, target: documentStub.body }
  for (const observer of bodyObservers) {
    if (observer.disconnected !== true && typeof observer.callback === 'function') observer.callback([record])
  }
}

/** Flip the OS colour-scheme preference and fire the media query listeners. */
function setPrefersDark(active) {
  prefersDark = active
  const entry = mediaQueries.get(DARK_QUERY)
  if (entry === undefined) return
  for (const listener of entry.listeners) listener({ matches: active, media: DARK_QUERY })
}

const documentListeners = new Map()
const documentStub = {
  body: {
    style: { overflow: '' },
    hasAttribute(name) { return bodyAttributes.has(name) },
    getAttribute(name) { return bodyAttributes.has(name) ? bodyAttributes.get(name) : null },
  },
  activeElement: null,
  visibilityState: 'visible',
  addEventListener(type, listener) {
    const list = documentListeners.get(type) || []
    list.push(listener)
    documentListeners.set(type, list)
  },
  removeEventListener(type, listener) {
    const list = documentListeners.get(type) || []
    documentListeners.set(type, list.filter((entry) => entry !== listener))
  },
  createElement() { return { style: {}, dataset: {} } },
  querySelector() { return null },
}

const windowStub = {
  __ModuleLoader__: { load(module) { windowStub.__loaded = module } },
  matchMedia: fakeMatchMedia,
  MutationObserver: FakeMutationObserver,
  Image: FakeImage,
  location: { reload: () => { reloads += 1 } },
  fetch: (url, options) => {
    fetchCalls.push({ url, options })
    if (fetchResponse === null) return Promise.reject(new Error('offline'))
    return Promise.resolve({
      ok: true,
      json: () => Promise.resolve(fetchResponse),
    })
  },
}

globalThis.window = windowStub
globalThis.document = documentStub

/* Minimal hook runtime: one component instance per function type. */
const hookStore = new Map()
const runtime = { cursor: 0, hooks: [] }
let dirty = false
let pendingEffects = []
let renderedTypes = new Set()

function depsChanged(previous, next) {
  if (previous === undefined || next === undefined) return true
  if (previous.length !== next.length) return true
  for (let index = 0; index < previous.length; index += 1) {
    if (previous[index] !== next[index]) return true
  }
  return false
}

const React = {
  Fragment: Symbol('Fragment'),
  createElement: (type, props, ...children) => {
    const nextProps = Object.assign({}, props)
    if (children.length === 1) nextProps.children = children[0]
    else if (children.length > 1) nextProps.children = children
    return { type, props: nextProps, children }
  },
  useState(initial) {
    const hooks = runtime.hooks
    const index = runtime.cursor
    runtime.cursor += 1
    if (hooks[index] === undefined) hooks[index] = { value: typeof initial === 'function' ? initial() : initial }
    const slot = hooks[index]
    return [slot.value, (next) => {
      const value = typeof next === 'function' ? next(slot.value) : next
      if (value !== slot.value) {
        slot.value = value
        dirty = true
      }
    }]
  },
  useRef(initial) {
    const hooks = runtime.hooks
    const index = runtime.cursor
    runtime.cursor += 1
    if (hooks[index] === undefined) hooks[index] = { current: initial }
    return hooks[index]
  },
  useEffect(effect, deps) {
    const hooks = runtime.hooks
    const index = runtime.cursor
    runtime.cursor += 1
    const slot = hooks[index] || (hooks[index] = { deps: undefined, cleanup: undefined })
    if (depsChanged(slot.deps, deps)) {
      slot.deps = deps
      pendingEffects.push({ slot, effect })
    }
  },
  useMemo(factory, deps) {
    const hooks = runtime.hooks
    const index = runtime.cursor
    runtime.cursor += 1
    const slot = hooks[index] || (hooks[index] = { deps: undefined, value: undefined })
    if (depsChanged(slot.deps, deps)) {
      slot.deps = deps
      slot.value = factory()
    }
    return slot.value
  },
  useLayoutEffect(effect, deps) {
    React.useEffect(effect, deps)
  },
}

/** Attach the small DOM surface the component calls on host nodes. */
function decorate(node) {
  if (node === null || node === undefined || typeof node !== 'object') return
  const props = node.props || {}
  if (props.ref !== null && props.ref !== undefined && typeof props.ref === 'object') props.ref.current = node
  node.style = props.style || {}
  node.className = props.className || ''
  node.clientWidth = props.className === 'dii-lb-stage' ? 1200 : 800
  node.clientHeight = props.className === 'dii-lb-stage' ? 700 : 410
  node.scrollLeft = 0
  node.addEventListener = (type, effect) => {
    node.__listeners = node.__listeners || {}
    node.__listeners[type] = (node.__listeners[type] || []).concat(effect)
  }
  node.removeEventListener = (type, effect) => {
    if (node.__listeners === undefined || node.__listeners[type] === undefined) return
    node.__listeners[type] = node.__listeners[type].filter((entry) => entry !== effect)
  }
  node.dispatch = (type, event) => {
    for (const effect of (node.__listeners || {})[type] || []) effect(event)
  }
  node.scrollTo = (options) => { node.scrollLeft = options.left }
  node.querySelectorAll = (selector) => {
    if (!selector.includes('button')) return []
    return flatten(node.children).filter((child) => child.type === 'button' && child.props.disabled !== true)
  }
  node.focus = () => { documentStub.activeElement = node }
}

function renderNode(node) {
  if (node === null || node === undefined || typeof node !== 'object') return node
  if (Array.isArray(node)) return node.map(renderNode)
  if (typeof node.type === 'function') {
    renderedTypes.add(node.type)
    const hooks = hookStore.get(node.type) || []
    hookStore.set(node.type, hooks)
    const previousHooks = runtime.hooks
    const previousCursor = runtime.cursor
    runtime.hooks = hooks
    runtime.cursor = 0
    const output = node.type(node.props || {})
    runtime.hooks = previousHooks
    runtime.cursor = previousCursor
    return renderNode(output)
  }
  const copy = { type: node.type, props: Object.assign({}, node.props) }
  copy.children = (node.children || []).map(renderNode)
  decorate(copy)
  return copy
}

function flatten(node, out) {
  const nodes = out === undefined ? [] : out
  if (node === null || node === undefined) return nodes
  if (Array.isArray(node)) {
    for (const child of node) flatten(child, nodes)
    return nodes
  }
  if (typeof node !== 'object') return nodes
  nodes.push(node)
  if (node.children !== undefined) flatten(node.children, nodes)
  return nodes
}

function unmountMissing() {
  for (const [type, hooks] of hookStore) {
    if (renderedTypes.has(type)) continue
    for (const slot of hooks) {
      if (slot !== null && slot !== undefined && typeof slot.cleanup === 'function') slot.cleanup()
    }
    hookStore.delete(type)
  }
}

function flush(makeRoot) {
  let tree = null
  for (let round = 0; round < 20; round += 1) {
    dirty = false
    pendingEffects = []
    renderedTypes = new Set()
    tree = renderNode(makeRoot())
    for (const effect of pendingEffects) {
      const slot = effect.slot
      if (typeof slot.cleanup === 'function') slot.cleanup()
      slot.cleanup = effect.effect()
    }
    unmountMissing()
    if (!dirty) break
  }
  return tree
}

function find(tree, predicate) {
  return flatten(tree).find(predicate) || null
}
function findAll(tree, predicate) {
  return flatten(tree).filter(predicate)
}
function byClass(tree, className) {
  return findAll(tree, (node) => typeof node.className === 'string' && node.className.split(' ').includes(className))
}
function byType(tree, type) {
  return findAll(tree, (node) => node.type === type)
}

const ReactDOM = { createPortal: (node) => node }
const requireStub = (name) => {
  if (name === 'react') return React
  if (name === 'react-dom') return ReactDOM
  throw new Error('unexpected require: ' + name)
}

eval(source) // eslint-disable-line no-eval
const plugin = windowStub.__loaded.factory(requireStub)

/* ---------------------------------------------------------------- helpers */

function reset() {
  hookStore.clear()
  documentStub.activeElement = null
  documentStub.body.style.overflow = ''
  documentListeners.clear()
  bodyAttributes.clear()
  for (const observer of bodyObservers) observer.disconnected = true
  bodyObservers.length = 0
  mediaQueries.clear()
  prefersDark = false
  preloaded.length = 0
  fetchCalls.length = 0
  fetchResponse = null
  clockOffsetMs = 0
  reloads = 0
  reducedMotion = false
}

let lastRoot = null
function mount(makeElement) {
  lastRoot = makeElement()
  return flushLastRoot()
}
function flushLastRoot() {
  expect(lastRoot !== null, 'mount() was not called')
  return flush(() => lastRoot)
}
function renderDeckTracked(code) {
  return mount(() => React.createElement(plugin.__test.DeckFence, { code }))
}
function renderDeck(code) {
  return renderDeckTracked(code)
}

function click(node) {
  expect(node !== null, 'click target not found')
  node.props.onClick({ stopPropagation() {}, preventDefault() {}, target: node, currentTarget: node })
  return flushLastRoot()
}

function key(node, value, extra) {
  expect(node !== null, 'key target not found')
  node.props.onKeyDown(Object.assign({ key: value, preventDefault() {}, shiftKey: false, target: node, currentTarget: node }, extra || {}))
  return flushLastRoot()
}

function imageByPath(tree, needle) {
  return byClass(tree, 'dii-deck-img').find((node) => node.props.src.includes(needle)) || null
}

/* ------------------------------------------------------------------ tests */

await test('module loads with the package id and registers both fences', async () => {
  expect(plugin !== null && typeof plugin === 'object', 'factory returned nothing')
  expect(plugin.inject.includes('slots'), 'slots service is not injected')
  expect(plugin.__test.NS === 'dshInlineImage', 'namespace changed')
  const registered = []
  const ctx = {
    effect(run) { run(); return () => {} },
    slots: {
      inject(name, run) { run(); return () => {} },
      register(definition) { registered.push(definition); return () => {} },
    },
  }
  plugin.apply(ctx)
  expect(registered.length === 2, 'expected 2 fence layers, got ' + registered.length)
  expect(registered.map((entry) => entry.key).join(',') === 'dsh-image,dsh-images', 'fence keys changed')
})

await test('the seven bundled samples exist on one 1147×588 canvas', async () => {
  for (const file of SAMPLE_PATHS) {
    const absolute = join(PUBLIC_DIR, file)
    expect(fs.existsSync(absolute), file + ' is missing')
    const size = pngSize(absolute)
    expect(size.width === 1147 && size.height === 588, file + ' is ' + size.width + '×' + size.height)
  }
})

await test('parseDeck accepts the seven-page sample and rejects malformed input', async () => {
  const spec = plugin.__test.parseDeck(SAMPLE_JSON)
  expect(spec !== null, 'sample deck was rejected')
  expect(spec.pages.length === 7, 'expected 7 pages, got ' + spec.pages.length)
  expect(spec.ratio === '1147 / 588', 'default ratio changed: ' + spec.ratio)
  expect(spec.pages[0].path === 'ch-01.png', 'first path changed')
  expect(spec.pages[6].caption === CAPTIONS[6], 'last caption changed')
  expect(plugin.__test.parseDeck('not json') === null, 'bad JSON must be rejected')
  expect(plugin.__test.parseDeck('{"pages":[]}') === null, 'empty deck must be rejected')
  expect(plugin.__test.parseDeck('{"pages":[{"path":""}]}') === null, 'blank path must be rejected')
  const many = { pages: Array.from({ length: 30 }, (_, index) => ({ path: 'p' + index + '.png' })) }
  expect(plugin.__test.parseDeck(JSON.stringify(many)).pages.length === 24, 'deck cap changed')
  const custom = plugin.__test.parseDeck('{"ratio":"4 / 3","pages":[{"path":"a.png"}]}')
  expect(custom.ratio === '4 / 3', 'explicit ratio was not kept')
})

await test('parseSpec / imageSrc keep the single-image contract', async () => {
  const spec = plugin.__test.parseSpec('{"path":"a b.png","alt":"说明","width":320}')
  expect(spec.path === 'a b.png' && spec.alt === '说明' && spec.width === 320, 'parseSpec changed')
  expect(plugin.__test.parseSpec('{"alt":"x"}') === null, 'missing path must be rejected')
  expect(plugin.__test.imageSrc('ch-01.png') === '/dsh-inline-image/public?path=ch-01.png', 'image route changed')
  expect(plugin.__test.imageSrc('ch 01.png', 2) === '/dsh-inline-image/public?path=ch%2001.png&retry=2', 'retry query changed')
})

await test('the deck renders seven slides, dots, counter, caption and the eager first page', async () => {
  reset()
  const tree = renderDeckTracked(SAMPLE_JSON)
  const slides = byClass(tree, 'dii-deck-page')
  expect(slides.length === 7, 'expected 7 slides, got ' + slides.length)
  const dots = byClass(tree, 'dii-deck-dot')
  expect(dots.length === 7, 'expected 7 dots, got ' + dots.length)
  expect(find(tree, (node) => node.props && node.props['aria-current'] === 'true') !== null, 'no current dot')
  const images = byClass(tree, 'dii-deck-img')
  expect(images.length === 7, 'expected 7 images, got ' + images.length)
  expect(images[0].props.loading === 'eager', 'first page must load eagerly')
  expect(images.slice(1).every((image) => image.props.loading === 'lazy'), 'later pages must stay lazy')
  expect(images[0].props.src === '/dsh-inline-image/public?path=ch-01.png', 'first src changed: ' + images[0].props.src)
  const count = findAll(tree, (node) => node.props && node.props.style && node.props.style.fontVariantNumeric === 'tabular-nums')
  expect(count.length === 1 && count[0].children.join('') === '1 / 7', 'page counter is not 1 / 7')
  const caption = byClass(tree, 'dii-deck-sr')[0]
  expect(caption !== undefined, 'live announcer is missing')
  expect(caption.children.join('') === '第 1 页，共 7 页：' + CAPTIONS[0], 'announcement changed: ' + caption.children.join(''))
  const stage = byClass(tree, 'dii-deck-stage')[0]
  expect(stage.props.role === 'region' && stage.props.tabIndex === 0, 'stage region is not keyboard reachable')
  expect(stage.props['aria-roledescription'] === '图片集', 'stage roledescription changed')
  const figure = byClass(tree, 'dii-deck')[0]
  expect(figure.props['aria-label'] === '连续统假设 · 7 页', 'figure label changed')
})

await test('mobile and dark-mode CSS keeps the 24px dot target, safe area and visible edges', async () => {
  reset()
  renderDeckTracked(SAMPLE_JSON)
  expect(source.includes('.dii-deck-dot{box-sizing:border-box;width:24px;height:24px;min-width:24px;min-height:24px'), 'dot hit target is below 24px')
  expect(source.includes('env(safe-area-inset-bottom)'), 'bottom safe area is not reserved')
  expect(source.includes('@media (pointer:coarse)'), 'coarse-pointer rules are missing')
  expect(source.includes('prefers-reduced-motion:reduce'), 'reduced-motion fallback is missing')
  expect(source.includes('--dsw-alias-border-l3'), 'dark/light border token is not used')
  expect(source.includes('--dsw-alias-label-secondary'), 'caption token is not used')
  expect(source.includes('--dsw-alias-bg-mask-photo'), 'lightbox backdrop token is not used')
  const edges = byClass(renderDeckTracked(SAMPLE_JSON), 'dii-deck-edge')
  expect(edges.length === 2, 'expected two edge fades, got ' + edges.length)
  expect(edges[1].props['data-visible'] === 'true', 'right edge is not hinted on page 1')
  expect(edges[0].props['data-visible'] === 'false', 'left edge must stay hidden on page 1')
})

await test('a dot click changes the visible page, caption, live announcement and prefetches ±1', async () => {
  reset()
  let tree = renderDeckTracked(SAMPLE_JSON)
  const dots = byClass(tree, 'dii-deck-dot')
  tree = click(dots[3])
  const count = findAll(tree, (node) => node.props && node.props.style && node.props.style.fontVariantNumeric === 'tabular-nums')
  expect(count[0].children.join('') === '4 / 7', 'counter did not follow the dot: ' + count[0].children.join(''))
  const caption = byClass(tree, 'dii-deck-sr')[0]
  expect(caption.children.join('').includes(CAPTIONS[3]), 'announcement did not follow the dot')
  const current = findAll(tree, (node) => node.props && node.props['aria-current'] === 'true')
  expect(current.length === 1 && current[0].props['aria-label'].includes(CAPTIONS[3]), 'active dot is not the fourth page')
  const left = byClass(tree, 'dii-deck-edge').find((node) => node.props['data-side'] === 'left')
  expect(left.props['data-visible'] === 'true', 'left edge hint is missing after paging')
  const prefetched = preloaded.map((image) => image.src)
  expect(prefetched.some((src) => src.includes('ch-03.png')), 'previous neighbour was not prefetched: ' + prefetched.join(','))
  expect(prefetched.some((src) => src.includes('ch-05.png')), 'next neighbour was not prefetched: ' + prefetched.join(','))
  expect(prefetched.length <= 4, 'preloading fetched too many pages: ' + prefetched.join(','))
})

await test('Home / End / PageUp / PageDown / arrows move one deck page', async () => {
  reset()
  let tree = renderDeckTracked(SAMPLE_JSON)
  const stage = byClass(tree, 'dii-deck-stage')[0]
  tree = key(stage, 'ArrowRight')
  expect(counterText(tree) === '2 / 7', 'ArrowRight did not advance')
  tree = key(byClass(tree, 'dii-deck-stage')[0], 'End')
  expect(counterText(tree) === '7 / 7', 'End did not reach the last page')
  tree = key(byClass(tree, 'dii-deck-stage')[0], 'Home')
  expect(counterText(tree) === '1 / 7', 'Home did not return to the first page')
  tree = key(byClass(tree, 'dii-deck-stage')[0], 'PageDown')
  expect(counterText(tree) === '2 / 7', 'PageDown did not advance')
  tree = key(byClass(tree, 'dii-deck-stage')[0], 'PageUp')
  expect(counterText(tree) === '1 / 7', 'PageUp did not go back')
  const helper = plugin.__test
  expect(helper.keyTarget('ArrowLeft', 0, 7) === 0, 'left at the edge must clamp')
  expect(helper.keyTarget('End', 3, 7) === 6, 'End helper is wrong')
  expect(helper.keyTarget('a', 3, 7) === null, 'unknown key must not navigate')
})

function counterText(tree) {
  const count = findAll(tree, (node) => node.props && node.props.style && node.props.style.fontVariantNumeric === 'tabular-nums')
  return count[0] === undefined ? '' : count[0].children.join('')
}

await test('a failed page keeps the path and retry restores it with a cache-busting query', async () => {
  reset()
  let tree = renderDeckTracked(SAMPLE_JSON)
  const image = byClass(tree, 'dii-deck-img').find((node) => node.props.src.includes('ch-02.png'))
  image.props.onError()
  tree = flushLastRoot()
  const failure = find(tree, (node) => node.props && node.props.role === 'alert')
  expect(failure !== null, 'failure card is missing')
  expect(flatten(failure).some((node) => node.type === 'code' && node.children.join('') === 'ch-02.png'), 'failure card lost the path')
  const retry = byClass(tree, 'dii-deck-retry')[0]
  expect(retry !== undefined, 'retry button is missing')
  tree = click(retry)
  const restored = byClass(tree, 'dii-deck-img').find((node) => node.props.src.includes('ch-02.png'))
  expect(restored !== undefined, 'retry did not bring the image back')
  expect(restored.props.src.endsWith('&retry=1'), 'retry did not bust the image cache: ' + restored.props.src)
})

await test('the lightbox opens, zooms, pages with arrows, closes with Escape and restores deck focus', async () => {
  reset()
  let tree = renderDeckTracked(SAMPLE_JSON)
  tree = click(byClass(tree, 'dii-deck-img')[0])
  let dialog = find(tree, (node) => node.props && node.props.role === 'dialog')
  expect(dialog !== null, 'image click did not open the lightbox')
  expect(dialog.props['aria-modal'] === 'true', 'lightbox is not modal')
  expect(documentStub.activeElement !== null && documentStub.activeElement.props['aria-label'] === '关闭（Esc）', 'close button did not take focus')
  const plus = findAll(dialog, (node) => node.type === 'button' && node.props['aria-label'] === '放大')[0]
  tree = click(plus)
  dialog = find(tree, (node) => node.props && node.props.role === 'dialog')
  let image = find(dialog, (node) => node.className === 'dii-lb-img')
  expect(image !== null && image.props.style.transform.includes('scale(1.5)'), 'zoom did not reach 150%: ' + (image === null ? 'no image' : image.props.style.transform))
  const resetButton = findAll(dialog, (node) => node.type === 'button' && node.props['aria-label'] === '恢复 100%')[0]
  tree = click(resetButton)
  dialog = find(tree, (node) => node.props && node.props.role === 'dialog')
  image = find(dialog, (node) => node.className === 'dii-lb-img')
  expect(image.props.style.transform.includes('scale(1)'), 'reset did not return to 100%')
  tree = key(dialog, 'ArrowRight')
  dialog = find(tree, (node) => node.props && node.props.role === 'dialog')
  image = find(dialog, (node) => node.className === 'dii-lb-img')
  expect(image.props.src.includes('ch-02.png'), 'lightbox arrow did not page the deck: ' + image.props.src)
  tree = key(dialog, 'Escape')
  expect(find(tree, (node) => node.props && node.props.role === 'dialog') === null, 'Escape did not close the lightbox')
  expect(documentStub.activeElement !== null && documentStub.activeElement.className === 'dii-deck-stage', 'focus was not restored to the deck stage')
})

await test('the lightbox wheel helper clamps between 100% and 400%', async () => {
  const helper = plugin.__test
  expect(helper.zoomFromWheel(1, -1) === 1.5, 'wheel up did not zoom in')
  expect(helper.zoomFromWheel(1, 1) === 1, 'wheel down must clamp at 100%')
  expect(helper.zoomFromWheel(4, -1) === 4, 'wheel must clamp at 400%')
  const pan = helper.clampPan({ x: 900, y: -900 }, 2, { width: 1000, height: 500 })
  expect(pan.x === 500 && pan.y === -250, 'pan clamp changed: ' + JSON.stringify(pan))
  expect(helper.pointerDistance({ x: 0, y: 0 }, { x: 3, y: 4 }) === 5, 'pointer distance changed')
})

await test('a newer Client bundle on disk shows the refresh hint', async () => {
  reset()
  const LOADED_AT = plugin.__test.LOADED_AT
  clockOffsetMs = 10000
  fetchResponse = { clientAgeMs: 1000 }
  const tree = renderDeckTracked(SAMPLE_JSON)
  await new Promise((resolve) => setTimeout(resolve, 0))
  const updated = flushLastRoot()
  const bar = byClass(updated, 'dii-deck-stale')[0]
  expect(bar !== undefined, 'stale build did not show the refresh hint (tree had ' + byClass(updated, 'dii-deck').length + ' decks)')
  expect(flatten(bar).some((node) => node.children.join('') === '刷新页面'), 'refresh button is missing')
  expect(fetchCalls.length >= 1 && fetchCalls[0].url === '/dsh-inline-image/health', 'health probe route changed')
  const refresh = byClass(updated, 'dii-deck-stale-refresh')[0]
  click(refresh)
  expect(reloads === 1, 'refresh button did not reload the page')
  expect(Date.now() - LOADED_AT >= 10000, 'clock harness is not active')
})

await test('an unchanged Client bundle shows no stale hint', async () => {
  reset()
  clockOffsetMs = 0
  fetchResponse = { clientAgeMs: 600000 }
  const tree = renderDeckTracked(SAMPLE_JSON)
  await new Promise((resolve) => setTimeout(resolve, 0))
  const updated = flushLastRoot()
  expect(byClass(updated, 'dii-deck-stale').length === 0, 'an old bundle falsely showed the refresh hint')
})

await test('malformed fences fail visibly instead of crashing', async () => {
  reset()
  const tree = renderDeck('{oops')
  const error = byClass(tree, 'dii-error')[0]
  expect(error !== undefined && error.children.join('').includes('dsh-images'), 'deck bad-JSON error is missing')
  reset()
  const single = mount(() => React.createElement(plugin.__test.ImageFence, { code: '{"path":""}' }))
  expect(byClass(single, 'dii-error').length === 1, 'single-image error is missing')
})

await test('streaming fences pass their children through untouched', async () => {
  reset()
  const el = React.createElement
  const code = el('code', {}, '{"pages":[]}')
  const tree = flush(() => el(plugin.__test.DeckFence, { code: '{"pages":[', streaming: true }, code))
  expect(byClass(tree, 'dii-error').length === 0, 'streaming deck must not show an error')
  expect(byType(tree, 'code').length === 1, 'streaming children were not passed through')
})

await test('the single image fence retries without losing its path', async () => {
  reset()
  let tree = mount(() => React.createElement(plugin.__test.ImageFence, { code: '{"path":"solo.png","alt":"单图"}' }))
  let image = byType(tree, 'img')[0]
  expect(image.props.loading === 'lazy' && image.props.alt === '单图', 'single image contract changed')
  image.props.onError()
  tree = flushLastRoot()
  const failure = byClass(tree, 'dii-error')[0]
  expect(failure !== undefined && flatten(failure).some((node) => node.type === 'code' && node.children.join('') === 'solo.png'), 'single-image failure lost the path')
  const retry = find(failure, (node) => node.type === 'button')
  tree = click(retry)
  image = byType(tree, 'img')[0]
  expect(image !== undefined && image.props.src.includes('&retry=1'), 'single-image retry did not bust the cache')
})

/* ------------------------------------------------------------- darkPath */

await test('the darkPath fixtures share one canvas and are a light/dark plate pair', async () => {
  const light = join(PUBLIC_DIR, 's7-light.png')
  const dark = join(PUBLIC_DIR, 's7-dark.png')
  for (const file of [light, dark]) {
    expect(fs.existsSync(file), file + ' is missing')
    const size = pngSize(file)
    expect(size.width === 1147 && size.height === 588, file + ' is ' + size.width + '×' + size.height)
  }
  const lightPixel = pngFirstPixel(light)
  const darkPixel = pngFirstPixel(dark)
  expect(lightPixel.every((value) => value > 200), 'light plate is not white: ' + lightPixel.join(','))
  expect(darkPixel.every((value) => value < 40), 'dark plate is not dark: ' + darkPixel.join(','))
})

await test('parseDeck / parseSpec keep darkPath only when it is a usable string', async () => {
  const spec = plugin.__test.parseDeck(JSON.stringify({ pages: [
    { path: 'a-l01.png', darkPath: 'a-d01.png' },
    { path: 'b.png', darkPath: 42 },
    { path: 'c.png', darkPath: '   ' },
    { path: 'd.png' },
  ] }))
  expect(spec.pages[0].darkPath === 'a-d01.png', 'darkPath was not kept')
  expect(spec.pages[1].darkPath === '' && spec.pages[2].darkPath === '' && spec.pages[3].darkPath === '', 'non-string / blank / missing darkPath must read as absent')
  const single = plugin.__test.parseSpec('{"path":"s-l01.png","darkPath":"s-d01.png"}')
  expect(single.darkPath === 's-d01.png', 'parseSpec dropped darkPath')
  expect(plugin.__test.parseSpec('{"path":"s.png","darkPath":{}}').darkPath === '', 'object darkPath must read as absent')
  expect(plugin.__test.pagePath({ path: 'l.png', darkPath: 'd.png' }, false, {}) === 'l.png', 'light theme must always use path')
})

await test('light uses path, dark uses darkPath, a missing darkPath falls back to path', async () => {
  reset()
  let tree = renderDeckTracked(DARK_JSON)
  expect(imageByPath(tree, 's7-light.png') !== null, 'light theme did not use path')
  expect(imageByPath(tree, 's7-dark.png') === null, 'light theme used darkPath')
  reset()
  setDarkAttribute(true)
  tree = renderDeckTracked(DARK_JSON)
  expect(imageByPath(tree, 's7-dark.png') !== null, 'dark theme did not use darkPath')
  expect(imageByPath(tree, 's7-light.png') === null, 'dark theme still used the light plate')
  expect(imageByPath(tree, 'ch-03.png') !== null, 'dark theme dropped a page without darkPath')
  expect(imageByPath(tree, 'ch-02.png') === null && imageByPath(tree, 'ch-05.png') !== null, 'dark theme did not swap the second page')
  reset()
  setPrefersDark(true)
  tree = renderDeckTracked(DARK_JSON)
  expect(imageByPath(tree, 's7-dark.png') !== null, 'media-query dark preference did not pick darkPath')
})

await test('a runtime theme switch re-renders the deck through the body attribute and the media query', async () => {
  reset()
  let tree = renderDeckTracked(DARK_JSON)
  expect(imageByPath(tree, 's7-light.png') !== null, 'expected the light plate initially')
  setDarkAttribute(true)
  tree = flushLastRoot()
  expect(imageByPath(tree, 's7-dark.png') !== null, 'body attribute change did not switch to darkPath')
  expect(imageByPath(tree, 's7-light.png') === null, 'body attribute change left the light plate')
  setDarkAttribute(false)
  tree = flushLastRoot()
  expect(imageByPath(tree, 's7-light.png') !== null, 'removing the body attribute did not return to path')
  setPrefersDark(true)
  tree = flushLastRoot()
  expect(imageByPath(tree, 's7-dark.png') !== null, 'media query change did not switch to darkPath')
  setPrefersDark(false)
  tree = flushLastRoot()
  expect(imageByPath(tree, 's7-light.png') !== null, 'media query clearing did not return to path')
})

await test('dark theme prefetches the neighbouring dark plate, not the light one', async () => {
  reset()
  setDarkAttribute(true)
  renderDeckTracked(DARK_JSON)
  const prefetched = preloaded.map((image) => image.src)
  expect(prefetched.some((src) => src.includes('ch-05.png')), 'neighbour dark plate was not prefetched: ' + prefetched.join(','))
  expect(prefetched.some((src) => src.includes('ch-02.png')) === false, 'light neighbour was prefetched in dark mode: ' + prefetched.join(','))
  expect(prefetched.length <= 4, 'preload fetched too many pages: ' + prefetched.join(','))
})

await test('a broken darkPath falls back to path and never shows a broken plate', async () => {
  reset()
  setDarkAttribute(true)
  let tree = renderDeckTracked(DARK_JSON)
  const darkImage = imageByPath(tree, 's7-dark.png')
  expect(darkImage !== null, 'dark plate was not rendered')
  darkImage.props.onError()
  tree = flushLastRoot()
  expect(byClass(tree, 'dii-deck-fail').length === 0, 'a broken darkPath showed the failure card instead of falling back')
  expect(imageByPath(tree, 's7-light.png') !== null, 'a broken darkPath did not fall back to path')
  expect(imageByPath(tree, 's7-dark.png') === null, 'the broken darkPath is still requested')
  imageByPath(tree, 's7-light.png').props.onError()
  tree = flushLastRoot()
  const failure = byClass(tree, 'dii-deck-fail')[0]
  expect(failure !== undefined, 'a broken path no longer shows the failure card')
  expect(flatten(failure).some((node) => node.type === 'code' && node.children.join('') === 's7-light.png'), 'failure card lost the path')
  tree = click(byClass(tree, 'dii-deck-retry')[0])
  const restored = imageByPath(tree, 's7-dark.png')
  expect(restored !== null && restored.props.src.endsWith('&retry=1'), 'retry did not retry the dark plate with a cache-busting query: ' + (restored === null ? 'no image' : restored.props.src))
})

await test('the lightbox follows the theme and falls back from a broken darkPath', async () => {
  reset()
  setDarkAttribute(true)
  let tree = renderDeckTracked(DARK_JSON)
  tree = click(imageByPath(tree, 's7-dark.png'))
  let dialog = find(tree, (node) => node.props && node.props.role === 'dialog')
  expect(dialog !== null, 'lightbox did not open in dark theme')
  let image = find(dialog, (node) => node.className === 'dii-lb-img')
  expect(image !== null && image.props.src.includes('s7-dark.png'), 'lightbox did not use darkPath: ' + (image === null ? 'no image' : image.props.src))
  setDarkAttribute(false)
  tree = flushLastRoot()
  dialog = find(tree, (node) => node.props && node.props.role === 'dialog')
  image = find(dialog, (node) => node.className === 'dii-lb-img')
  expect(image.props.src.includes('s7-light.png'), 'lightbox did not follow a runtime theme switch: ' + image.props.src)
  setDarkAttribute(true)
  tree = flushLastRoot()
  dialog = find(tree, (node) => node.props && node.props.role === 'dialog')
  image = find(dialog, (node) => node.className === 'dii-lb-img')
  image.props.onError()
  tree = flushLastRoot()
  dialog = find(tree, (node) => node.props && node.props.role === 'dialog')
  expect(find(dialog, (node) => node.props && node.props.role === 'alert') === null, 'broken darkPath showed a failure card in the lightbox')
  image = find(dialog, (node) => node.className === 'dii-lb-img')
  expect(image !== null && image.props.src.includes('s7-light.png'), 'lightbox darkPath did not fall back to path')
})

await test('the single dsh-image fence also honours darkPath and its fallback', async () => {
  reset()
  setDarkAttribute(true)
  const code = JSON.stringify({ path: 's7-light.png', darkPath: 's7-dark.png', alt: '深色单图' })
  let tree = mount(() => React.createElement(plugin.__test.ImageFence, { code }))
  let image = byType(tree, 'img')[0]
  expect(image.props.src.includes('s7-dark.png'), 'single image did not use darkPath in dark theme: ' + image.props.src)
  image.props.onError()
  tree = flushLastRoot()
  image = byType(tree, 'img')[0]
  expect(image !== undefined && image.props.src.includes('s7-light.png'), 'single image darkPath did not fall back to path')
  expect(byClass(tree, 'dii-error').length === 0, 'single image darkPath fallback showed an error')
  setDarkAttribute(false)
  tree = flushLastRoot()
  image = byType(tree, 'img')[0]
  expect(image.props.src.includes('s7-light.png'), 'single image light theme changed')
  expect(image.props.src.includes('s7-dark.png') === false, 'single image light theme used darkPath')
})

/* ----------------------------------------------------------------- report */

console.log('')
console.log(passes + ' passed, ' + failures.length + ' failed')
if (failures.length > 0) process.exitCode = 1
