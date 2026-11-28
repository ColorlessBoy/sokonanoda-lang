/**
 * `dsh-inline-image` Client half.
 *
 * Registers two Chat Markdown fences:
 *
 *   ```dsh-image
 *   {"path":"cb-step1-v2.png","alt":"f 和 g 的映射"}
 *   ```
 *
 *   ```dsh-images
 *   {"title":"连续统假设","pages":[
 *     {"path":"ch-01.png","darkPath":"ch-01-dark.png","caption":"封面"},
 *     {"path":"ch-02.png","darkPath":"ch-02-dark.png","caption":"一样多 = 能配对"}]}
 *   ```
 *
 * Both fetch images from the Host's `/dsh-inline-image/public` route, so they
 * work through a remote link too. `dsh-images` is a swipeable deck: one page per
 * slide, scroll-snap + dots + keyboard, a full-screen zoomable lightbox, and the
 * caption of the page in view. Both fences accept an optional page-level
 * `darkPath` used only while the DSH dark theme (or the OS dark preference) is
 * active; without one, or once it fails, the light `path` is used unchanged.
 * The plugin stays generic: it knows nothing about sokonanoda or any course.
 */
window.__ModuleLoader__.load({
  id: 'dsh-inline-image',
  factory(require) {
    'use strict'
    const React = require('react')
    let ReactDOM = null
    try {
      ReactDOM = require('react-dom')
    } catch {
      ReactDOM = null
    }
    const h = React.createElement

    const NS = 'dshInlineImage'
    const IMAGE_ROUTE = '/dsh-inline-image/public?path='
    const HEALTH_ROUTE = '/dsh-inline-image/health'
    const DECK_ASPECT = '1147 / 588'
    const MAX_PAGES = 24
    const ZOOM_MIN = 1
    const ZOOM_MAX = 4
    const ZOOM_STEP = 0.5
    const DOUBLE_ZOOM = 2.5
    /** A file younger than the running bundle by more than this ⇒ reload hint. */
    const STALE_TOLERANCE_MS = 2000
    /** Module evaluation time, compared with the Host's report of `client.js` age. */
    const LOADED_AT = Date.now()

    function clamp(value, min, max) {
      if (value < min) return min
      if (value > max) return max
      return value
    }

    function finite(value, fallback) {
      return typeof value === 'number' && Number.isFinite(value) ? value : fallback
    }

    function imageSrc(path, attempt) {
      const base = IMAGE_ROUTE + encodeURIComponent(path)
      return typeof attempt === 'number' && attempt > 0 ? base + '&retry=' + String(attempt) : base
    }

    /* ------------------------------------------------------------------ theme */

    /** DSH puts this attribute on `<body>` while the dark theme is active. */
    const DARK_THEME_ATTR = 'data-ds-dark-theme'
    const DARK_MEDIA_QUERY = '(prefers-color-scheme: dark)'

    /** True when the DSH theme attribute on `<body>` says dark. */
    function bodyWantsDark() {
      const body = typeof document !== 'undefined' ? document.body : null
      if (body === null || body === undefined) return false
      try {
        if (typeof body.hasAttribute === 'function') return body.hasAttribute(DARK_THEME_ATTR) === true
        const attributes = body.attributes
        if (attributes !== null && attributes !== undefined && typeof attributes.getNamedItem === 'function') {
          return attributes.getNamedItem(DARK_THEME_ATTR) !== null
        }
        return body[DARK_THEME_ATTR] !== undefined
      } catch {
        return false
      }
    }

    /** OS-level dark preference; a browser without `matchMedia` reads as light. */
    function mediaWantsDark() {
      if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return false
      try {
        return window.matchMedia(DARK_MEDIA_QUERY).matches === true
      } catch {
        return false
      }
    }

    /** Dark is active when the DSH body attribute or the media query says so. */
    function isDarkTheme() {
      return bodyWantsDark() || mediaWantsDark()
    }

    /**
     * The path to show for one page: `darkPath` only while dark, and only when it
     * is a usable string; otherwise (and once a broken `darkPath` fell back) the
     * light `path`. Light theme always uses `path`.
     */
    function pagePath(page, dark, fallback) {
      if (page === null || page === undefined) return ''
      const light = typeof page.path === 'string' ? page.path : ''
      if (dark !== true) return light
      if (typeof page.darkPath !== 'string' || page.darkPath === '') return light
      if (fallback !== null && fallback !== undefined && fallback[light] !== undefined) return light
      return page.darkPath
    }

    /** The letterbox behind a dark plate; light plates keep their white. */
    function darkPlate() {
      return 'var(--dsw-alias-bg-base, #141414)'
    }

    /**
     * The dark state, kept live: the DSH body attribute is watched with a
     * MutationObserver and the OS preference with the media query listener, so a
     * runtime theme switch re-renders the deck instead of only reading once.
     */    function useDarkTheme() {
      const [dark, setDark] = React.useState(isDarkTheme)
      React.useEffect(() => {
        let disposed = false
        const sync = () => {
          if (disposed === true) return
          const next = isDarkTheme()
          setDark((previous) => (previous === next ? previous : next))
        }
        sync()
        let media = null
        if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
          try {
            media = window.matchMedia(DARK_MEDIA_QUERY)
          } catch {
            media = null
          }
        }
        if (media !== null && media !== undefined) {
          if (typeof media.addEventListener === 'function') media.addEventListener('change', sync)
          else if (typeof media.addListener === 'function') media.addListener(sync)
        }
        let observer = null
        const body = typeof document !== 'undefined' ? document.body : null
        if (body !== null && body !== undefined && typeof window !== 'undefined' && typeof window.MutationObserver === 'function') {
          try {
            observer = new window.MutationObserver(sync)
            observer.observe(body, { attributes: true, attributeFilter: [DARK_THEME_ATTR] })
          } catch {
            observer = null
          }
        }
        return () => {
          disposed = true
          if (observer !== null && typeof observer.disconnect === 'function') observer.disconnect()
          if (media !== null && media !== undefined) {
            if (typeof media.removeEventListener === 'function') media.removeEventListener('change', sync)
            else if (typeof media.removeListener === 'function') media.removeListener(sync)
          }
        }
      }, [])
      return dark
    }

    function parseSpec(code) {
      if (typeof code !== 'string' || code.trim() === '') return null
      try {
        const value = JSON.parse(code)
        if (value === null || typeof value !== 'object' || Array.isArray(value)) return null
        if (typeof value.path !== 'string' || value.path.trim() === '') return null
        return {
          path: value.path.trim(),
          darkPath: typeof value.darkPath === 'string' && value.darkPath.trim() !== '' ? value.darkPath.trim() : '',
          alt: typeof value.alt === 'string' ? value.alt : '',
          caption: typeof value.caption === 'string' ? value.caption : '',
          width: typeof value.width === 'number' || typeof value.width === 'string' ? value.width : '',
        }
      } catch {
        return null
      }
    }

    function parseDeck(code) {
      if (typeof code !== 'string' || code.trim() === '') return null
      try {
        const value = JSON.parse(code)
        if (value === null || typeof value !== 'object' || Array.isArray(value)) return null
        const input = Array.isArray(value.pages) ? value.pages : []
        const pages = []
        for (const page of input) {
          if (page === null || typeof page !== 'object' || Array.isArray(page)) continue
          if (typeof page.path !== 'string' || page.path.trim() === '') continue
          pages.push({
            path: page.path.trim(),
            darkPath: typeof page.darkPath === 'string' && page.darkPath.trim() !== '' ? page.darkPath.trim() : '',
            alt: typeof page.alt === 'string' ? page.alt : '',
            caption: typeof page.caption === 'string' ? page.caption : '',
          })
        }
        if (pages.length === 0) return null
        return {
          title: typeof value.title === 'string' ? value.title : '',
          ratio: typeof value.ratio === 'string' && /^\d+(\.\d+)?\s*\/\s*\d+(\.\d+)?$/.test(value.ratio) ? value.ratio : DECK_ASPECT,
          pages: pages.slice(0, MAX_PAGES),
        }
      } catch {
        return null
      }
    }

    /**
     * Map a navigation key to the page it should show.
     * @returns the clamped page, or null when the key is not a deck key.
     */
    function keyTarget(key, index, count) {
      if (count <= 0) return null
      let target = null
      if (key === 'ArrowRight' || key === 'PageDown') target = index + 1
      else if (key === 'ArrowLeft' || key === 'PageUp') target = index - 1
      else if (key === 'Home') target = 0
      else if (key === 'End') target = count - 1
      if (target === null) return null
      return clamp(target, 0, count - 1)
    }

    /** Pages to prefetch for one page in view: the previous and the next. */
    function neighborPages(index, count) {
      if (count <= 1) return []
      const result = []
      if (index - 1 >= 0) result.push(index - 1)
      if (index + 1 < count) result.push(index + 1)
      return result
    }

    /**
     * True when the Host's `client.js` is newer than the bundle this page runs.
     * Both inputs are durations (no wall clock): the file age reported by the
     * Host and the time elapsed since this bundle evaluated.
     */
    function isStaleBuild(clientAgeMs, elapsedMs, toleranceMs) {
      if (typeof clientAgeMs !== 'number' || !Number.isFinite(clientAgeMs)) return false
      if (typeof elapsedMs !== 'number' || !Number.isFinite(elapsedMs)) return false
      return clientAgeMs + Math.max(0, finite(toleranceMs, STALE_TOLERANCE_MS)) < elapsedMs
    }

    /** One wheel notch toward the clamped zoom range. */
    function zoomFromWheel(zoom, deltaY) {
      const direction = finite(deltaY, 0) < 0 ? 1 : -1
      return clamp(zoom + direction * ZOOM_STEP, ZOOM_MIN, ZOOM_MAX)
    }

    /** Keep a zoomed image's pan inside its viewport. */
    function clampPan(pan, zoom, viewport) {
      const width = Math.max(0, finite(viewport && viewport.width, 0))
      const height = Math.max(0, finite(viewport && viewport.height, 0))
      const limitX = Math.max(0, ((zoom - 1) * width) / 2)
      const limitY = Math.max(0, ((zoom - 1) * height) / 2)
      return {
        x: clamp(finite(pan && pan.x, 0), -limitX, limitX),
        y: clamp(finite(pan && pan.y, 0), -limitY, limitY),
      }
    }

    function pointerDistance(first, second) {
      const dx = finite(first && first.x, 0) - finite(second && second.x, 0)
      const dy = finite(first && first.y, 0) - finite(second && second.y, 0)
      return Math.sqrt(dx * dx + dy * dy)
    }

    const DECK_CSS = [
      '.dii-deck{--dii-accent:var(--dsw-alias-state-business-primary,#2f5d8a);padding-bottom:max(6px,env(safe-area-inset-bottom))}',
      '.dii-deck-track{scrollbar-width:none;-ms-overflow-style:none;overscroll-behavior-x:contain}',
      '.dii-deck-track::-webkit-scrollbar{display:none}',
      '.dii-deck-page{position:relative}',
      '.dii-deck-img{cursor:zoom-in;touch-action:manipulation}',
      '.dii-deck-open{position:absolute;right:8px;top:8px;box-sizing:border-box;width:30px;height:30px;display:flex;align-items:center;justify-content:center;padding:0;border:1px solid var(--dsw-alias-border-l3,rgba(127,127,127,.28));border-radius:9px;background:var(--dsw-alias-card-fill,rgba(255,255,255,.86));color:var(--dsw-alias-label-primary,#1b1b1b);font-family:inherit;font-size:15px;line-height:1;cursor:zoom-in;opacity:0;transition:opacity .15s ease}',
      '.dii-deck-page:hover .dii-deck-open,.dii-deck-page:focus-within .dii-deck-open{opacity:1}',
      '.dii-deck-open:focus-visible,.dii-deck-btn:focus-visible,.dii-deck-dot:focus-visible,.dii-deck-stage:focus-visible{outline:2px solid var(--dsw-focus-ring-color,var(--dsw-alias-state-business-primary,#2f5d8a));outline-offset:2px}',
      '.dii-deck-btn:hover:not(:disabled){background:rgba(127,127,127,.14)}',
      '.dii-deck-btn:disabled{opacity:.32;cursor:default}',
      '.dii-deck-dot{box-sizing:border-box;width:24px;height:24px;min-width:24px;min-height:24px;display:inline-flex;align-items:center;justify-content:center;padding:0;border:0;border-radius:50%;background:transparent;cursor:pointer}',
      '.dii-deck-dot:hover .dii-deck-dot-mark{background:rgba(127,127,127,.55);background:color-mix(in srgb,var(--dsw-alias-label-primary,#111) 55%,transparent)}',
      '.dii-deck-dot-mark{width:8px;height:8px;border-radius:50%;background:rgba(127,127,127,.35);background:color-mix(in srgb,var(--dsw-alias-label-primary,#111) 28%,transparent);transition:background .18s ease,transform .18s ease}',
      '.dii-deck-dot[aria-current="true"] .dii-deck-dot-mark{background:var(--dii-accent);transform:scale(1.35)}',
      '.dii-deck-edge{position:absolute;top:0;bottom:0;width:26px;pointer-events:none;opacity:0;transition:opacity .18s ease;z-index:1}',
      '.dii-deck-edge[data-side="left"]{left:0;background:linear-gradient(to right,rgba(0,0,0,.16),rgba(0,0,0,0))}',
      '.dii-deck-edge[data-side="right"]{right:0;background:linear-gradient(to left,rgba(0,0,0,.16),rgba(0,0,0,0))}',
      '.dii-deck-edge[data-visible="true"]{opacity:1}',
      '.dii-deck-sr{position:absolute;width:1px;height:1px;margin:-1px;padding:0;border:0;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}',
      '.dii-deck-fail{box-sizing:border-box;width:100%;height:100%;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:6px;padding:16px;background:var(--dsw-alias-card-fill,#fff);color:var(--dsw-alias-label-secondary,#4b5563);font-size:13px;line-height:1.6;text-align:center}',
      '.dii-deck-retry{box-sizing:border-box;margin-top:4px;padding:5px 16px;border:1px solid var(--dsw-alias-border-l3,rgba(127,127,127,.28));border-radius:9px;background:transparent;color:var(--dsw-alias-label-primary,#1b1b1b);font:inherit;font-size:13px;cursor:pointer}',
      '.dii-deck-retry:hover{background:rgba(127,127,127,.12)}',
      '.dii-deck-stale{display:flex;align-items:center;justify-content:space-between;gap:10px;padding:6px 10px;border-radius:9px;background:var(--dsw-alias-card-fill-hover,rgba(127,127,127,.12));color:var(--dsw-alias-label-secondary,#4b5563);font-size:12.5px;line-height:1.5}',
      '.dii-deck-stale-refresh{box-sizing:border-box;padding:3px 10px;border:1px solid var(--dsw-alias-border-l3,rgba(127,127,127,.28));border-radius:8px;background:transparent;color:var(--dsw-alias-label-primary,#1b1b1b);font:inherit;font-size:12.5px;cursor:pointer}',
      '.dii-lb{position:fixed;inset:0;z-index:2147483000;box-sizing:border-box;display:flex;flex-direction:column;padding:max(8px,env(safe-area-inset-top)) max(8px,env(safe-area-inset-right)) max(8px,env(safe-area-inset-bottom)) max(8px,env(safe-area-inset-left));background:var(--dsw-alias-bg-mask-photo,rgba(0,0,0,.92));color:#fff;overscroll-behavior:contain;-webkit-backdrop-filter:blur(10px) saturate(.9);backdrop-filter:blur(10px) saturate(.9)}',
      '.dii-lb-top{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:4px 6px;font-size:13px;color:rgba(255,255,255,.78);font-variant-numeric:tabular-nums}',
      '.dii-lb-stage{position:relative;flex:1 1 auto;min-height:0;display:flex;align-items:center;justify-content:center;overflow:hidden;touch-action:none;cursor:grab}',
      '.dii-lb-stage[data-zoomed="true"]{cursor:grabbing}',
      '.dii-lb-img{max-width:100%;max-height:100%;width:auto;height:auto;object-fit:contain;background:#fff;border-radius:10px;user-select:none;-webkit-user-drag:none;transform-origin:center center;will-change:transform}',
      '.dii-lb-controls{display:flex;align-items:center;justify-content:center;gap:8px;padding:8px 6px 2px;flex-wrap:wrap}',
      '.dii-lb-btn{box-sizing:border-box;min-width:40px;height:40px;display:inline-flex;align-items:center;justify-content:center;padding:0 12px;border:1px solid rgba(255,255,255,.28);border-radius:12px;background:rgba(255,255,255,.08);color:#fff;font-family:inherit;font-size:15px;line-height:1;cursor:pointer}',
      '.dii-lb-btn:hover:not(:disabled){background:rgba(255,255,255,.18)}',
      '.dii-lb-btn:disabled{opacity:.35;cursor:default}',
      '.dii-lb-btn:focus-visible{outline:2px solid #fff;outline-offset:2px}',
      '.dii-lb-zoom{min-width:68px;font-variant-numeric:tabular-nums;font-size:13px;color:rgba(255,255,255,.82)}',
      '.dii-lb-caption{padding:2px 12px 6px;text-align:center;font-size:13.5px;line-height:1.6;color:rgba(255,255,255,.86)}',
      '.dii-lb-close{width:40px;padding:0;border-radius:50%;font-size:22px}',
      'body[data-ds-dark-theme] .dii-deck-stage{border-color:rgba(255,255,255,.22);box-shadow:0 10px 30px rgba(0,0,0,.45)}',
      'body[data-ds-dark-theme] .dii-deck-page{box-shadow:inset 0 0 0 1px rgba(255,255,255,.06)}',
      '@media (pointer:coarse){.dii-deck-btn{width:44px;height:44px}.dii-deck-open{opacity:1}.dii-lb-btn{min-width:44px;height:44px}}',
      '@media (prefers-reduced-motion:reduce){.dii-deck-track{scroll-behavior:auto!important}.dii-deck-edge,.dii-deck-dot-mark,.dii-deck-open{transition:none!important}.dii-lb-img{transition:none!important}}',
    ].join('\n')

    const DECK_STYLE = {
      figure: { margin: '12px 0', maxWidth: '100%', display: 'flex', flexDirection: 'column', gap: '8px' },
      head: { display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', gap: '12px' },
      title: { fontSize: '13px', fontWeight: 600, color: 'var(--dsw-alias-label-primary,#1b1b1b)', minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' },
      count: { fontSize: '12px', fontVariantNumeric: 'tabular-nums', color: 'var(--dsw-alias-label-secondary,#4b5563)', whiteSpace: 'nowrap' },
      stage: { position: 'relative', width: '100%', overflow: 'hidden', borderRadius: '14px', border: '1px solid var(--dsw-alias-border-l3, rgba(127,127,127,.28))', background: '#fff' },
      track: { display: 'flex', width: '100%', height: '100%', overflowX: 'auto', overflowY: 'hidden', scrollSnapType: 'x mandatory', scrollBehavior: 'smooth', WebkitOverflowScrolling: 'touch' },
      page: { flex: '0 0 100%', height: '100%', scrollSnapAlign: 'center', scrollSnapStop: 'always', display: 'flex', alignItems: 'center', justifyContent: 'center', background: '#fff' },
      img: { display: 'block', width: '100%', height: '100%', objectFit: 'contain', background: '#fff' },
      controls: { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '10px' },
      btn: { boxSizing: 'border-box', width: '34px', height: '34px', flex: '0 0 auto', display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 0, borderRadius: '50%', border: '1px solid var(--dsw-alias-border-l3, rgba(127,127,127,.28))', background: 'transparent', color: 'var(--dsw-alias-label-primary,#1b1b1b)', fontSize: '20px', lineHeight: 1, cursor: 'pointer', fontFamily: 'inherit' },
      dots: { display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0px', flex: '1 1 auto', minWidth: 0, flexWrap: 'wrap' },
      caption: { fontSize: '13px', lineHeight: 1.6, color: 'var(--dsw-alias-label-secondary,#4b5563)', minHeight: '20px' },
    }

    /** One page's failure card: path stays visible, retry busts the cache. */
    function FailureCard(props) {
      return h('div', { className: props.className, role: 'alert' },
        h('div', {}, '图片加载失败'),
        h('code', {}, props.path),
        h('div', { style: { opacity: 0.78 } }, '确认文件在 dsh-inline-image/public 下'),
        h('button', { type: 'button', className: props.buttonClass, onClick: props.onRetry }, '重试')
      )
    }

    /** One inline image, or a readable fallback instead of a crashed fence. */
    function ImageFence(props) {
      const streaming = props !== null && typeof props === 'object' && props.streaming === true
      const dark = useDarkTheme()
      const [failed, setFailed] = React.useState(false)
      const [attempt, setAttempt] = React.useState(0)
      const [darkFallback, setDarkFallback] = React.useState({})
      const children = props !== null && typeof props === 'object' ? props.children : undefined
      if (streaming) return children === undefined ? null : children
      const spec = parseSpec(props !== null && typeof props === 'object' ? props.code : '')
      if (spec === null) {
        return h('div', { className: 'dii-error' }, 'dsh-image：需要一个 JSON，例如 {"path":"image.png","alt":"说明"}')
      }
      const path = pagePath(spec, dark, darkFallback)
      const src = imageSrc(path, attempt)
      const figureStyle = {
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: '8px',
        margin: '10px 0',
        maxWidth: '100%',
      }
      const imageStyle = {
        display: 'block',
        maxWidth: '100%',
        height: 'auto',
        borderRadius: '12px',
        border: '1px solid var(--dsw-alias-border-l3, rgba(127,127,127,.28))',
        background: path === spec.path ? '#fff' : darkPlate(),
      }
      if (failed) {
        return h('div', { className: 'dii-error', role: 'alert', style: { color: 'var(--dsw-alias-label-secondary,#4b5563)', fontSize: '13px', lineHeight: 1.6, textAlign: 'center' } },
          h('div', {}, '图片加载失败：'),
          h('code', {}, spec.path),
          h('div', { style: { marginTop: '4px', opacity: 0.75 } }, '请确认文件在工作区内，且当前账号是 owner。'),
          h('button', {
            type: 'button',
            style: { marginTop: '6px', padding: '5px 16px', border: '1px solid var(--dsw-alias-border-l3, rgba(127,127,127,.28))', borderRadius: '9px', background: 'transparent', color: 'var(--dsw-alias-label-primary,#1b1b1b)', font: 'inherit', fontSize: '13px', cursor: 'pointer' },
            onClick: () => { setFailed(false); setDarkFallback({}); setAttempt((value) => value + 1) },
          }, '重试'))
      }
      return h('figure', { className: 'dii-figure', style: figureStyle },
        h('img', {
          className: 'dii-image',
          src,
          alt: spec.alt,
          loading: 'lazy',
          decoding: 'async',
          style: Object.assign({}, imageStyle, spec.width === '' ? {} : { width: spec.width }),
          onError: () => {
            if (pagePath(spec, dark, darkFallback) !== spec.path) setDarkFallback({ [spec.path]: true })
            else setFailed(true)
          },
        }),
        spec.caption !== ''
          ? h('figcaption', { className: 'dii-caption', style: { fontSize: '12.5px', color: 'var(--dsw-alias-label-secondary,#4b5563)', lineHeight: 1.5, textAlign: 'center' } }, spec.caption)
          : (spec.alt !== '' ? h('figcaption', { className: 'dii-caption', style: { fontSize: '12.5px', color: 'var(--dsw-alias-label-secondary,#4b5563)', lineHeight: 1.5, textAlign: 'center' } }, spec.alt) : null))
    }

    /**
     * Full-screen viewer: pinch/wheel zoom, drag to pan, double-click toggles,
     * `Esc` closes, arrows/Home/End page, `+`/`-`/`0` zoom, Tab stays inside.
     */
    function Lightbox(props) {
      const pages = props.pages
      const count = props.count
      const index = clamp(props.index, 0, Math.max(0, count - 1))
      const page = pages[index]
      const [zoom, setZoom] = React.useState(ZOOM_MIN)
      const [pan, setPan] = React.useState({ x: 0, y: 0 })
      const dialogRef = React.useRef(null)
      const viewportRef = React.useRef(null)
      const closeRef = React.useRef(null)
      const pointers = React.useRef({})
      const gesture = React.useRef(null)

      React.useEffect(() => {
        const close = closeRef.current
        if (close !== null && close !== undefined && typeof close.focus === 'function') {
          try { close.focus() } catch { /* the close button stays the intended target */ }
        }
        const body = typeof document !== 'undefined' ? document.body : null
        const previous = body !== null && body !== undefined && body.style !== undefined ? body.style.overflow : ''
        if (body !== null && body !== undefined && body.style !== undefined) body.style.overflow = 'hidden'
        return () => {
          if (body !== null && body !== undefined && body.style !== undefined) body.style.overflow = previous
        }
      }, [])

      React.useEffect(() => {
        setZoom(ZOOM_MIN)
        setPan({ x: 0, y: 0 })
      }, [index])

      React.useEffect(() => {
        const node = viewportRef.current
        if (node === null || node === undefined || typeof node.addEventListener !== 'function') return undefined
        const onWheel = (event) => {
          if (typeof event.preventDefault === 'function') event.preventDefault()
          const next = zoomFromWheel(zoom, event.deltaY)
          setZoom(next)
          if (next === ZOOM_MIN) setPan({ x: 0, y: 0 })
        }
        node.addEventListener('wheel', onWheel, { passive: false })
        return () => { node.removeEventListener('wheel', onWheel) }
      }, [zoom])

      const viewportSize = () => {
        const node = viewportRef.current
        return {
          width: node !== null && node !== undefined && typeof node.clientWidth === 'number' ? node.clientWidth : 0,
          height: node !== null && node !== undefined && typeof node.clientHeight === 'number' ? node.clientHeight : 0,
        }
      }

      const resetZoom = () => {
        setZoom(ZOOM_MIN)
        setPan({ x: 0, y: 0 })
      }

      const pointerList = () => Object.keys(pointers.current).map((id) => pointers.current[id])

      const onPointerDown = (event) => {
        if (event === null || typeof event !== 'object') return
        if (event.pointerType === 'mouse' && event.button !== 0) return
        const node = event.currentTarget
        if (node !== null && node !== undefined && typeof node.setPointerCapture === 'function') {
          try { node.setPointerCapture(event.pointerId) } catch { /* capture is an optimization */ }
        }
        pointers.current[event.pointerId] = { x: finite(event.clientX, 0), y: finite(event.clientY, 0) }
        const list = pointerList()
        if (list.length === 1) {
          gesture.current = { kind: 'pan', x: finite(event.clientX, 0), y: finite(event.clientY, 0), panX: pan.x, panY: pan.y }
        } else if (list.length === 2) {
          gesture.current = { kind: 'pinch', distance: pointerDistance(list[0], list[1]), zoom }
        }
      }

      const onPointerMove = (event) => {
        if (event === null || typeof event !== 'object') return
        if (pointers.current[event.pointerId] === undefined) return
        pointers.current[event.pointerId] = { x: finite(event.clientX, 0), y: finite(event.clientY, 0) }
        const list = pointerList()
        const mode = gesture.current
        if (list.length >= 2 && mode !== null && mode !== undefined && mode.kind === 'pinch') {
          const distance = pointerDistance(list[0], list[1])
          if (mode.distance > 0) {
            const next = clamp((mode.zoom * distance) / mode.distance, ZOOM_MIN, ZOOM_MAX)
            setZoom(next)
            if (next === ZOOM_MIN) setPan({ x: 0, y: 0 })
            else setPan((previous) => clampPan(previous, next, viewportSize()))
          }
          return
        }
        if (mode !== null && mode !== undefined && mode.kind === 'pan' && zoom > ZOOM_MIN) {
          setPan(clampPan({
            x: mode.panX + (finite(event.clientX, 0) - mode.x),
            y: mode.panY + (finite(event.clientY, 0) - mode.y),
          }, zoom, viewportSize()))
        }
      }

      const onPointerUp = (event) => {
        if (event === null || typeof event !== 'object') return
        delete pointers.current[event.pointerId]
        if (pointerList().length < 2 && gesture.current !== null && gesture.current !== undefined && gesture.current.kind === 'pinch') gesture.current = null
        if (pointerList().length === 0) gesture.current = null
      }

      const onDoubleClick = () => {
        if (zoom > ZOOM_MIN) resetZoom()
        else setZoom(DOUBLE_ZOOM)
      }

      const trapFocus = (event) => {
        const root = dialogRef.current
        if (root === null || root === undefined || typeof root.querySelectorAll !== 'function') return
        const nodes = Array.prototype.slice.call(root.querySelectorAll('button:not([disabled]),[href],[tabindex="0"]'))
          .filter((node) => node !== null && node !== undefined && typeof node.focus === 'function')
        if (nodes.length === 0) return
        const active = typeof document !== 'undefined' ? document.activeElement : null
        const at = nodes.indexOf(active)
        const next = at === -1
          ? (event.shiftKey === true ? nodes[nodes.length - 1] : nodes[0])
          : nodes[(at + (event.shiftKey === true ? -1 : 1) + nodes.length) % nodes.length]
        if (typeof event.preventDefault === 'function') event.preventDefault()
        next.focus()
      }

      const onKeyDown = (event) => {
        if (event === null || typeof event !== 'object') return
        if (event.key === 'Escape') {
          if (typeof event.preventDefault === 'function') event.preventDefault()
          props.onClose()
          return
        }
        if (event.key === 'Tab') {
          trapFocus(event)
          return
        }
        if (event.key === '+' || event.key === '=') {
          if (typeof event.preventDefault === 'function') event.preventDefault()
          setZoom(clamp(zoom + ZOOM_STEP, ZOOM_MIN, ZOOM_MAX))
          return
        }
        if (event.key === '-' || event.key === '_') {
          if (typeof event.preventDefault === 'function') event.preventDefault()
          const next = clamp(zoom - ZOOM_STEP, ZOOM_MIN, ZOOM_MAX)
          setZoom(next)
          if (next === ZOOM_MIN) setPan({ x: 0, y: 0 })
          return
        }
        if (event.key === '0') {
          if (typeof event.preventDefault === 'function') event.preventDefault()
          resetZoom()
          return
        }
        const target = keyTarget(event.key, index, count)
        if (target !== null) {
          if (typeof event.preventDefault === 'function') event.preventDefault()
          props.onIndexChange(target)
        }
      }

      if (page === undefined) return null
      const attempt = props.attempts !== null && props.attempts !== undefined && typeof props.attempts[page.path] === 'number' ? props.attempts[page.path] : 0
      const failed = props.failed !== null && props.failed !== undefined && props.failed[page.path] !== undefined
      const dark = props.dark === true
      const darkFallback = props.darkFallback !== null && props.darkFallback !== undefined ? props.darkFallback : {}
      const shownPath = pagePath(page, dark, darkFallback)
      const transformed = {
        transform: 'translate(' + String(pan.x) + 'px, ' + String(pan.y) + 'px) scale(' + String(zoom) + ')',
        transition: props.reducedMotion === true ? 'none' : 'transform .12s ease-out',
      }
      const alt = page.alt !== '' ? page.alt : (page.caption !== '' ? page.caption : '第 ' + String(index + 1) + ' 页')
      const label = (props.title !== undefined && props.title !== '' ? props.title + ' · ' : '') + '图片集，第 ' + String(index + 1) + ' 页，共 ' + String(count) + ' 页'

      const overlay = h('div', {
        className: 'dii-lb',
        ref: dialogRef,
        role: 'dialog',
        'aria-modal': 'true',
        'aria-label': label,
        tabIndex: -1,
        onKeyDown,
        onClick: (event) => {
          if (event !== null && event !== undefined && event.target === event.currentTarget) props.onClose()
        },
      },
      h('style', { key: 'css' }, DECK_CSS),
      h('div', { key: 'top', className: 'dii-lb-top' },
        h('span', {}, String(index + 1) + ' / ' + String(count)),
        h('button', {
          key: 'close',
          ref: closeRef,
          type: 'button',
          className: 'dii-lb-btn dii-lb-close',
          onClick: props.onClose,
          'aria-label': '关闭（Esc）',
          title: '关闭（Esc）',
        }, '×')
      ),
      h('div', {
        key: 'stage',
        className: 'dii-lb-stage',
        ref: viewportRef,
        'data-zoomed': zoom > ZOOM_MIN ? 'true' : 'false',
        onPointerDown,
        onPointerMove,
        onPointerUp,
        onPointerCancel: onPointerUp,
        onDoubleClick,
      },
      failed
        ? h(FailureCard, { key: 'fail', className: 'dii-deck-fail', buttonClass: 'dii-deck-retry', path: page.path, onRetry: () => props.onRetry(page.path) })
        : h('img', {
          key: 'img',
          className: 'dii-lb-img',
          src: imageSrc(shownPath, attempt),
          alt,
          decoding: 'async',
          draggable: false,
          style: shownPath === page.path ? transformed : Object.assign({}, transformed, { background: darkPlate() }),
          onError: () => {
            if (pagePath(page, dark, darkFallback) !== page.path) props.onDarkFallback(page.path)
            else props.onFail(page.path)
          },
        })
      ),
      h('div', { key: 'controls', className: 'dii-lb-controls' },
        h('button', { key: 'prev', type: 'button', className: 'dii-lb-btn', onClick: () => props.onIndexChange(index - 1), disabled: index <= 0, 'aria-label': '上一页' }, '‹'),
        h('button', { key: 'out', type: 'button', className: 'dii-lb-btn', onClick: () => setZoom(clamp(zoom - ZOOM_STEP, ZOOM_MIN, ZOOM_MAX)), disabled: zoom <= ZOOM_MIN, 'aria-label': '缩小' }, '−'),
        h('button', {
          key: 'zoom',
          type: 'button',
          className: 'dii-lb-btn dii-lb-zoom',
          onClick: resetZoom,
          'aria-label': '恢复 100%',
          title: '恢复 100%',
        }, String(Math.round(zoom * 100)) + '%'),
        h('button', { key: 'in', type: 'button', className: 'dii-lb-btn', onClick: () => setZoom(clamp(zoom + ZOOM_STEP, ZOOM_MIN, ZOOM_MAX)), disabled: zoom >= ZOOM_MAX, 'aria-label': '放大' }, '+'),
        h('button', { key: 'next', type: 'button', className: 'dii-lb-btn', onClick: () => props.onIndexChange(index + 1), disabled: index >= count - 1, 'aria-label': '下一页' }, '›')
      ),
      h('div', { key: 'caption', className: 'dii-lb-caption' }, page.caption !== '' ? page.caption : page.alt)
      )

      if (typeof document !== 'undefined' && ReactDOM !== null && typeof ReactDOM.createPortal === 'function') {
        return ReactDOM.createPortal(overlay, document.body)
      }
      return overlay
    }

    /** Swipeable image deck: one page per slide, native scroll-snap. */
    function DeckFence(props) {
      const streaming = props !== null && typeof props === 'object' && props.streaming === true
      const children = props !== null && typeof props === 'object' ? props.children : undefined
      const spec = parseDeck(props !== null && typeof props === 'object' ? props.code : '')
      const pages = spec === null ? [] : spec.pages
      const count = pages.length
      const dark = useDarkTheme()
      const [index, setIndex] = React.useState(0)
      const [failed, setFailed] = React.useState({})
      const [attempts, setAttempts] = React.useState({})
      const [darkFallback, setDarkFallback] = React.useState({})
      const [lightbox, setLightbox] = React.useState(false)
      const [stale, setStale] = React.useState(false)
      const trackRef = React.useRef(null)
      const stageRef = React.useRef(null)
      const preloaded = React.useRef({})
      const currentIndex = count === 0 ? 0 : clamp(index, 0, count - 1)
      const current = count === 0 ? null : pages[currentIndex]

      const reducedMotion = () => {
        if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return false
        try {
          return window.matchMedia('(prefers-reduced-motion: reduce)').matches === true
        } catch {
          return false
        }
      }

      const goTo = (target) => {
        if (count === 0) return
        const next = clamp(target, 0, count - 1)
        setIndex(next)
        const track = trackRef.current
        if (track !== null && track !== undefined && typeof track.scrollTo === 'function' && typeof track.clientWidth === 'number') {
          try {
            track.scrollTo({ left: next * track.clientWidth, behavior: reducedMotion() ? 'auto' : 'smooth' })
          } catch {
            track.scrollLeft = next * track.clientWidth
          }
        }
      }

      const onScroll = () => {
        const track = trackRef.current
        if (track === null || track === undefined || typeof track.clientWidth !== 'number') return
        const width = Math.max(1, track.clientWidth)
        const next = clamp(Math.round(finite(track.scrollLeft, 0) / width), 0, Math.max(0, count - 1))
        if (next !== currentIndex) setIndex(next)
      }

      const onKeyDown = (event) => {
        if (event === null || typeof event !== 'object') return
        const target = keyTarget(event.key, currentIndex, count)
        if (target === null) return
        if (typeof event.preventDefault === 'function') event.preventDefault()
        goTo(target)
      }

      const openLightbox = (pageIndex) => {
        if (count === 0) return
        setIndex(clamp(pageIndex, 0, count - 1))
        setLightbox(true)
      }

      const closeLightbox = () => {
        setLightbox(false)
        const stage = stageRef.current
        if (stage !== null && stage !== undefined && typeof stage.focus === 'function') {
          try { stage.focus() } catch { /* the deck keeps its place */ }
        }
      }

      const markFailed = (path) => {
        setFailed((previous) => (previous[path] !== undefined ? previous : Object.assign({}, previous, { [path]: true })))
      }

      const markDarkFallback = (path) => {
        setDarkFallback((previous) => (previous[path] !== undefined ? previous : Object.assign({}, previous, { [path]: true })))
      }

      /** A broken dark plate falls back to the light one before it ever fails. */
      const failPage = (page) => {
        if (pagePath(page, dark, darkFallback) !== page.path) markDarkFallback(page.path)
        else markFailed(page.path)
      }

      const retryPage = (path) => {
        setDarkFallback((previous) => {
          if (previous[path] === undefined) return previous
          const next = Object.assign({}, previous)
          delete next[path]
          return next
        })
        setFailed((previous) => {
          if (previous[path] === undefined) return previous
          const next = Object.assign({}, previous)
          delete next[path]
          return next
        })
        setAttempts((previous) => Object.assign({}, previous, { [path]: (previous[path] || 0) + 1 }))
      }

      React.useEffect(() => {
        if (count === 0) return
        for (const target of neighborPages(currentIndex, count)) {
          const page = pages[target]
          if (page === undefined) continue
          if (failed[page.path] !== undefined) continue
          const src = imageSrc(pagePath(page, dark, darkFallback), attempts[page.path] || 0)
          if (preloaded.current[src] === true) continue
          preloaded.current[src] = true
          if (typeof window === 'undefined' || typeof window.Image !== 'function') continue
          try {
            const preload = new window.Image()
            preload.decoding = 'async'
            preload.src = src
          } catch {
            /* prefetch is best effort */
          }
        }
      }, [currentIndex, count, failed, attempts, dark, darkFallback])

      React.useEffect(() => {
        let disposed = false
        const probe = () => {
          if (typeof window === 'undefined' || typeof window.fetch !== 'function') return
          let response
          try {
            response = window.fetch(HEALTH_ROUTE, { cache: 'no-store' })
          } catch {
            return
          }
          if (response === null || response === undefined || typeof response.then !== 'function') return
          response
            .then((value) => (value !== null && value !== undefined && value.ok === true && typeof value.json === 'function' ? value.json() : null))
            .then((body) => {
              if (disposed || body === null || typeof body !== 'object') return
              if (isStaleBuild(body.clientAgeMs, Date.now() - LOADED_AT, STALE_TOLERANCE_MS)) setStale(true)
            })
            .catch(() => { /* a missing health route is not the deck's failure */ })
        }
        probe()
        if (typeof document !== 'undefined' && typeof document.addEventListener === 'function') {
          const onVisible = () => { if (document.visibilityState === 'visible') probe() }
          document.addEventListener('visibilitychange', onVisible)
          return () => {
            disposed = true
            document.removeEventListener('visibilitychange', onVisible)
          }
        }
        return () => { disposed = true }
      }, [])

      if (streaming) return children === undefined ? null : children
      if (spec === null) {
        return h('div', { className: 'dii-error' }, 'dsh-images：需要一个 JSON，例如 {"pages":[{"path":"a.png","caption":"第一页"}]}')
      }

      const renderPage = (page, pageIndex) => {
        if (failed[page.path] !== undefined) {
          return h(FailureCard, { key: 'fail', className: 'dii-deck-fail', buttonClass: 'dii-deck-retry', path: page.path, onRetry: () => retryPage(page.path) })
        }
        const shownPath = pagePath(page, dark, darkFallback)
        return h('img', {
          key: 'img',
          className: 'dii-deck-img',
          src: imageSrc(shownPath, attempts[page.path] || 0),
          alt: page.alt !== '' ? page.alt : (page.caption !== '' ? page.caption : '第 ' + String(pageIndex + 1) + ' 页'),
          loading: pageIndex === 0 ? 'eager' : 'lazy',
          decoding: 'async',
          draggable: false,
          style: shownPath === page.path ? DECK_STYLE.img : Object.assign({}, DECK_STYLE.img, { background: darkPlate() }),
          onClick: () => openLightbox(pageIndex),
          onError: () => failPage(page),
        })
      }

      const caption = current !== null ? (current.caption !== '' ? current.caption : current.alt) : ''
      const announcement = '第 ' + String(currentIndex + 1) + ' 页，共 ' + String(count) + ' 页' + (caption !== '' ? '：' + caption : '')

      return h('figure', { className: 'dii-deck', style: DECK_STYLE.figure, 'aria-label': spec.title === '' ? '图片集' : spec.title },
        h('style', { key: 'css' }, DECK_CSS),
        stale
          ? h('div', { key: 'stale', className: 'dii-deck-stale', role: 'status' },
            h('span', {}, '画册组件已更新，当前页面仍在运行旧版本'),
            h('button', {
              key: 'refresh',
              type: 'button',
              className: 'dii-deck-stale-refresh',
              onClick: () => {
                if (typeof window !== 'undefined' && window.location !== undefined && typeof window.location.reload === 'function') window.location.reload()
              },
            }, '刷新页面'))
          : null,
        h('div', { key: 'head', style: DECK_STYLE.head },
          h('span', { style: DECK_STYLE.title }, spec.title),
          h('span', { key: 'count', style: DECK_STYLE.count }, String(currentIndex + 1) + ' / ' + String(count))
        ),
        h('div', {
          key: 'stage',
          className: 'dii-deck-stage',
          ref: stageRef,
          style: Object.assign({}, DECK_STYLE.stage, { aspectRatio: spec.ratio }),
          tabIndex: 0,
          role: 'region',
          'aria-roledescription': '图片集',
          'aria-label': spec.title === '' ? '图片集' : spec.title,
          onKeyDown,
        },
        h('div', { className: 'dii-deck-track', ref: trackRef, style: DECK_STYLE.track, onScroll },
          pages.map((page, pageIndex) => h('div', {
            key: page.path + ':' + String(pageIndex),
            className: 'dii-deck-page',
            style: DECK_STYLE.page,
            role: 'group',
            'aria-roledescription': '幻灯片',
            'aria-label': '第 ' + String(pageIndex + 1) + ' 页，共 ' + String(count) + ' 页',
            'aria-hidden': pageIndex === currentIndex ? undefined : 'true',
          },
          renderPage(page, pageIndex),
          failed[page.path] === undefined
            ? h('button', {
              key: 'open',
              type: 'button',
              className: 'dii-deck-open',
              onClick: (event) => {
                if (event !== null && event !== undefined && typeof event.stopPropagation === 'function') event.stopPropagation()
                openLightbox(pageIndex)
              },
              'aria-label': '放大查看第 ' + String(pageIndex + 1) + ' 页',
              title: '放大查看',
              tabIndex: pageIndex === currentIndex ? 0 : -1,
            }, '⤢')
            : null
          ))
        ),
        h('div', { key: 'edge-left', className: 'dii-deck-edge', 'data-side': 'left', 'data-visible': currentIndex > 0 ? 'true' : 'false', 'aria-hidden': 'true' }),
        h('div', { key: 'edge-right', className: 'dii-deck-edge', 'data-side': 'right', 'data-visible': currentIndex < count - 1 ? 'true' : 'false', 'aria-hidden': 'true' })
        ),
        h('div', { key: 'controls', style: DECK_STYLE.controls },
          h('button', {
            type: 'button',
            className: 'dii-deck-btn',
            onClick: () => goTo(currentIndex - 1),
            disabled: currentIndex <= 0,
            'aria-label': '上一页',
            style: Object.assign({}, DECK_STYLE.btn),
          }, '‹'),
          h('div', { style: DECK_STYLE.dots },
            pages.map((page, dotIndex) => h('button', {
              key: 'dot:' + String(dotIndex),
              type: 'button',
              className: 'dii-deck-dot',
              onClick: () => goTo(dotIndex),
              'aria-label': page.caption !== '' ? '第 ' + String(dotIndex + 1) + ' 页：' + page.caption : '第 ' + String(dotIndex + 1) + ' 页',
              'aria-current': dotIndex === currentIndex ? 'true' : undefined,
              title: page.caption !== '' ? page.caption : '第 ' + String(dotIndex + 1) + ' 页',
            }, h('span', { key: 'mark', className: 'dii-deck-dot-mark', 'aria-hidden': 'true' })))
          ),
          h('button', {
            type: 'button',
            className: 'dii-deck-btn',
            onClick: () => goTo(currentIndex + 1),
            disabled: currentIndex >= count - 1,
            'aria-label': '下一页',
            style: Object.assign({}, DECK_STYLE.btn),
          }, '›')
        ),
        current !== null
          ? h('figcaption', { key: 'cap', style: DECK_STYLE.caption }, caption)
          : null,
        h('div', { key: 'live', className: 'dii-deck-sr', role: 'status', 'aria-live': 'polite', 'aria-atomic': 'true' }, announcement),
        lightbox && current !== null
          ? h(Lightbox, {
            key: 'lightbox',
            pages,
            count,
            index: currentIndex,
            title: spec.title,
            dark,
            darkFallback,
            failed,
            attempts,
            reducedMotion: reducedMotion(),
            onRetry: retryPage,
            onDarkFallback: markDarkFallback,
            onFail: markFailed,
            onClose: closeLightbox,
            onIndexChange: goTo,
          })
          : null
      )
    }

    function apply(ctx) {
      ctx.effect(() => ctx.slots.inject('conversation.chat.markdown.fence', () => {
        return ctx.slots.register({
          name: 'conversation.chat.markdown.fence',
          key: 'dsh-image',
          id: 'dsh-inline-image/fence/image',
        }, ImageFence)
      }), 'dsh-inline-image: image fence')
      ctx.effect(() => ctx.slots.inject('conversation.chat.markdown.fence', () => {
        return ctx.slots.register({
          name: 'conversation.chat.markdown.fence',
          key: 'dsh-images',
          id: 'dsh-inline-image/fence/deck',
        }, DeckFence)
      }), 'dsh-inline-image: image deck fence')
    }

    return {
      inject: ['slots'],
      apply,
      __test: {
        ImageFence,
        DeckFence,
        Lightbox,
        parseSpec,
        parseDeck,
        keyTarget,
        neighborPages,
        isStaleBuild,
        zoomFromWheel,
        clampPan,
        pointerDistance,
        imageSrc,
        isDarkTheme,
        pagePath,
        DARK_THEME_ATTR,
        DARK_MEDIA_QUERY,
        LOADED_AT,
        NS,
      },
    }
  },
})
