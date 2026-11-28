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
 *     {"path":"ch-01.png","caption":"封面"},
 *     {"path":"ch-02.png","caption":"一样多 = 能配对"}]}
 *   ```
 *
 * Both fetch images from the Host's owner-authenticated
 * `/dsh-inline-image/public` route, so they work through a remote link too.
 * `dsh-images` is a swipeable deck: one page per slide, scroll-snap + dots +
 * arrow keys, with the caption of the page in view. The plugin stays generic:
 * it knows nothing about sokonanoda or any course.
 */
window.__ModuleLoader__.load({
  id: 'dsh-inline-image',
  factory(require) {
    'use strict'
    const React = require('react')
    const h = React.createElement

    const NS = 'dshInlineImage'
    const IMAGE_ROUTE = '/dsh-inline-image/public?path='
    const DECK_ASPECT = '1147 / 588'

    function imageSrc(path) {
      return IMAGE_ROUTE + encodeURIComponent(path)
    }

    function parseSpec(code) {
      if (typeof code !== 'string' || code.trim() === '') return null
      try {
        const value = JSON.parse(code)
        if (value === null || typeof value !== 'object' || Array.isArray(value)) return null
        if (typeof value.path !== 'string' || value.path.trim() === '') return null
        return {
          path: value.path.trim(),
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
            alt: typeof page.alt === 'string' ? page.alt : '',
            caption: typeof page.caption === 'string' ? page.caption : '',
          })
        }
        if (pages.length === 0) return null
        return {
          title: typeof value.title === 'string' ? value.title : '',
          ratio: typeof value.ratio === 'string' && /^\d+(\.\d+)?\s*\/\s*\d+(\.\d+)?$/.test(value.ratio) ? value.ratio : DECK_ASPECT,
          pages: pages.slice(0, 24),
        }
      } catch {
        return null
      }
    }

    /** One inline image, or a readable fallback instead of a crashed fence. */
    function ImageFence(props) {
      const streaming = props !== null && typeof props === 'object' && props.streaming === true
      const [failed, setFailed] = React.useState(false)
      const children = props !== null && typeof props === 'object' ? props.children : undefined
      if (streaming) return children === undefined ? null : children
      const spec = parseSpec(props !== null && typeof props === 'object' ? props.code : '')
      if (spec === null) {
        return h('div', { className: 'dii-error' }, 'dsh-image：需要一个 JSON，例如 {"path":"image.png","alt":"说明"}')
      }
      const src = imageSrc(spec.path)
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
        border: '1px solid var(--dsw-alias-border-secondary, rgba(127,127,127,.25))',
        background: 'var(--dsw-alias-card-fill, #fff)',
      }
      if (failed) {
        return h('div', { className: 'dii-error', style: { color: 'var(--dsw-alias-label-secondary,#6b7280)', fontSize: '13px', lineHeight: 1.6 } },
          h('div', {}, '图片加载失败：'),
          h('code', {}, spec.path),
          h('div', { style: { marginTop: '4px', opacity: 0.75 } }, '请确认文件在工作区内，且当前账号是 owner。'))
      }
      return h('figure', { className: 'dii-figure', style: figureStyle },
        h('img', {
          className: 'dii-image',
          src,
          alt: spec.alt,
          loading: 'lazy',
          decoding: 'async',
          style: Object.assign({}, imageStyle, spec.width === '' ? {} : { width: spec.width }),
          onError: () => setFailed(true),
        }),
        spec.caption !== ''
          ? h('figcaption', { className: 'dii-caption', style: { fontSize: '12.5px', color: 'var(--dsw-alias-label-secondary,#6b7280)', lineHeight: 1.5, textAlign: 'center' } }, spec.caption)
          : (spec.alt !== '' ? h('figcaption', { className: 'dii-caption', style: { fontSize: '12.5px', color: 'var(--dsw-alias-label-secondary,#6b7280)', lineHeight: 1.5, textAlign: 'center' } }, spec.alt) : null))
    }

    const DECK_CSS = [
      '.dii-deck-track{scrollbar-width:none;-ms-overflow-style:none}',
      '.dii-deck-track::-webkit-scrollbar{display:none}',
      '.dii-deck-btn:hover:not(:disabled){background:rgba(127,127,127,.14)}',
      '.dii-deck-dot:hover{background:rgba(127,127,127,.45)}',
      '.dii-deck-btn:focus-visible,.dii-deck-dot:focus-visible,.dii-deck-stage:focus-visible{outline:2px solid var(--dsw-alias-brand-primary,#2f5d8a);outline-offset:2px}',
      '.dii-deck-img{user-select:none;-webkit-user-drag:none}',
      '@media (prefers-reduced-motion: reduce){.dii-deck-track{scroll-behavior:auto!important}}',
    ].join('\n')

    const DECK_STYLE = {
      figure: { margin: '12px 0', maxWidth: '100%', display: 'flex', flexDirection: 'column', gap: '8px' },
      head: { display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', gap: '12px' },
      title: { fontSize: '13px', fontWeight: 600, color: 'var(--dsw-alias-label-primary,#1b1b1b)', minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' },
      count: { fontSize: '12px', fontVariantNumeric: 'tabular-nums', color: 'var(--dsw-alias-label-secondary,#6b7280)', whiteSpace: 'nowrap' },
      stage: { position: 'relative', width: '100%', overflow: 'hidden', borderRadius: '14px', border: '1px solid var(--dsw-alias-border-secondary, rgba(127,127,127,.25))', background: 'var(--dsw-alias-card-fill, #fff)' },
      track: { display: 'flex', width: '100%', height: '100%', overflowX: 'auto', overflowY: 'hidden', scrollSnapType: 'x mandatory', scrollBehavior: 'smooth', WebkitOverflowScrolling: 'touch' },
      page: { flex: '0 0 100%', height: '100%', scrollSnapAlign: 'center', scrollSnapStop: 'always', display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'var(--dsw-alias-card-fill, #fff)' },
      img: { display: 'block', width: '100%', height: '100%', objectFit: 'contain', background: '#fff' },
      fail: { boxSizing: 'border-box', width: '100%', height: '100%', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: '6px', padding: '16px', color: 'var(--dsw-alias-label-secondary,#6b7280)', fontSize: '13px', lineHeight: 1.6, textAlign: 'center' },
      controls: { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '10px' },
      btn: { boxSizing: 'border-box', width: '34px', height: '34px', flex: '0 0 auto', display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 0, borderRadius: '50%', border: '1px solid var(--dsw-alias-border-secondary, rgba(127,127,127,.25))', background: 'transparent', color: 'var(--dsw-alias-label-primary,#1b1b1b)', fontSize: '20px', lineHeight: 1, cursor: 'pointer', fontFamily: 'inherit' },
      btnDisabled: { opacity: 0.32, cursor: 'default' },
      dots: { display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '7px', flex: '1 1 auto', minWidth: 0, flexWrap: 'wrap' },
      dot: { boxSizing: 'border-box', width: '7px', height: '7px', padding: 0, borderRadius: '50%', border: 'none', background: 'rgba(127,127,127,.38)', cursor: 'pointer', transition: 'background .18s ease, transform .18s ease' },
      dotActive: { background: 'var(--dsw-alias-brand-primary,#2f5d8a)', transform: 'scale(1.25)' },
      caption: { fontSize: '12.5px', lineHeight: 1.55, color: 'var(--dsw-alias-label-secondary,#6b7280)', minHeight: '19px' },
    }

    /** Swipeable image deck: one page per slide, native scroll-snap. */
    function DeckFence(props) {
      const [index, setIndex] = React.useState(0)
      const [failed, setFailed] = React.useState({})
      const trackRef = React.useRef(null)
      const spec = parseDeck(props !== null && typeof props === 'object' ? props.code : '')
      const pages = spec === null ? [] : spec.pages
      const count = pages.length
      const streaming = props !== null && typeof props === 'object' && props.streaming === true
      const children = props !== null && typeof props === 'object' ? props.children : undefined
      const current = count > 0 ? pages[Math.max(0, Math.min(index, count - 1))] : null

      const goTo = (target) => {
        const track = trackRef.current
        if (track === null || track === undefined) return
        const next = Math.max(0, Math.min(target, count - 1))
        const left = next * track.clientWidth
        if (typeof track.scrollTo === 'function') track.scrollTo({ left, behavior: 'smooth' })
        else track.scrollLeft = left
      }

      const onScroll = () => {
        const track = trackRef.current
        if (track === null || track === undefined) return
        const width = Math.max(1, track.clientWidth)
        const next = Math.round(track.scrollLeft / width)
        if (next >= 0 && next < count && next !== index) setIndex(next)
      }

      const onKeyDown = (event) => {
        if (event === null || typeof event !== 'object') return
        if (event.key === 'ArrowLeft') {
          event.preventDefault()
          goTo(index - 1)
        } else if (event.key === 'ArrowRight') {
          event.preventDefault()
          goTo(index + 1)
        }
      }

      const markFailed = (path) => {
        setFailed((previous) => (previous[path] === true ? previous : Object.assign({}, previous, { [path]: true })))
      }

      if (streaming) return children === undefined ? null : children
      if (spec === null) {
        return h('div', { className: 'dii-error' }, 'dsh-images：需要一个 JSON，例如 {"pages":[{"path":"a.png","caption":"第一页"}]}')
      }

      return h('figure', { className: 'dii-deck', style: DECK_STYLE.figure },
        h('style', { key: 'css' }, DECK_CSS),
        h('div', { key: 'head', style: DECK_STYLE.head },
          h('span', { style: DECK_STYLE.title }, spec.title),
          h('span', { style: DECK_STYLE.count }, String(Math.min(index, count - 1) + 1) + ' / ' + String(count))
        ),
        h('div', {
          key: 'stage',
          className: 'dii-deck-stage',
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
            'aria-hidden': pageIndex === index ? undefined : 'true',
          },
          failed[page.path] === true
            ? h('div', { style: DECK_STYLE.fail },
              h('div', {}, '图片加载失败'),
              h('code', {}, page.path),
              h('div', { style: { opacity: 0.75 } }, '确认文件在 dsh-inline-image/public 下'))
            : h('img', {
              className: 'dii-deck-img',
              src: imageSrc(page.path),
              alt: page.alt !== '' ? page.alt : (page.caption !== '' ? page.caption : '第 ' + String(pageIndex + 1) + ' 页'),
              loading: pageIndex === 0 ? 'eager' : 'lazy',
              decoding: 'async',
              draggable: false,
              style: DECK_STYLE.img,
              onError: () => markFailed(page.path),
            })
          ))
        )
        ),
        h('div', { key: 'controls', style: DECK_STYLE.controls },
          h('button', {
            type: 'button',
            className: 'dii-deck-btn',
            onClick: () => goTo(index - 1),
            disabled: index <= 0,
            'aria-label': '上一页',
            style: Object.assign({}, DECK_STYLE.btn, index <= 0 ? DECK_STYLE.btnDisabled : {}),
          }, '‹'),
          h('div', { style: DECK_STYLE.dots },
            pages.map((page, dotIndex) => h('button', {
              key: 'dot:' + String(dotIndex),
              type: 'button',
              className: 'dii-deck-dot',
              onClick: () => goTo(dotIndex),
              'aria-label': '第 ' + String(dotIndex + 1) + ' 页',
              'aria-current': dotIndex === index ? 'true' : undefined,
              style: Object.assign({}, DECK_STYLE.dot, dotIndex === index ? DECK_STYLE.dotActive : {}),
            }))
          ),
          h('button', {
            type: 'button',
            className: 'dii-deck-btn',
            onClick: () => goTo(index + 1),
            disabled: index >= count - 1,
            'aria-label': '下一页',
            style: Object.assign({}, DECK_STYLE.btn, index >= count - 1 ? DECK_STYLE.btnDisabled : {}),
          }, '›')
        ),
        current !== null
          ? h('figcaption', { key: 'cap', style: DECK_STYLE.caption }, current.caption !== '' ? current.caption : current.alt)
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
      __test: { ImageFence, DeckFence, parseSpec, parseDeck, NS },
    }
  },
})
