/**
 * `dsh-inline-image` Client half.
 *
 * Registers one Chat Markdown fence:
 *
 *   ```dsh-image
 *   {"path":"cb-step1-v2.png","alt":"f 和 g 的映射"}
 *   ```
 *
 * The image is fetched from the Host's owner-authenticated
 * `/dsh-inline-image/public` route, so it works through a remote link too. This
 * plugin is generic: it knows nothing about sokonanoda or any course.
 */
window.__ModuleLoader__.load({
  id: 'dsh-inline-image',
  factory(require) {
    'use strict'
    const React = require('react')
    const h = React.createElement

    const NS = 'dshInlineImage'

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
      const src = '/dsh-inline-image/public?path=' + encodeURIComponent(spec.path)
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

    function apply(ctx) {
      ctx.effect(() => ctx.slots.inject('conversation.chat.markdown.fence', () => {
        return ctx.slots.register({
          name: 'conversation.chat.markdown.fence',
          key: 'dsh-image',
          id: 'dsh-inline-image/fence/image',
        }, ImageFence)
      }), 'dsh-inline-image: image fence')
    }

    return {
      inject: ['slots'],
      apply,
      __test: { ImageFence, parseSpec, NS },
    }
  },
})
