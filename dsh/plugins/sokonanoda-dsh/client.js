/**
 * Client half of `sokonanoda-dsh` — display-only cards and Lean highlighting.
 *
 * It registers three `conversation.chat.markdown.fence` layers:
 *   - `learn-card`: a fenced `{kind,payload}` JSON block becomes a real card
 *     (this is what the Host fast path emits) — the underlying Markdown text is
 *     left exactly as the model wrote it; only the rendering changes;
 *   - `lean` / `sokonanoda`: highlighted Lean code.
 *
 * Cards are display-first: the learner can still answer by typing in the
 * composer. Question cards additionally make their option rows clickable — a
 * click sends the option's letter as an ordinary user message (same composer
 * submission as typing), so the character stream stays the single source of
 * truth. Knowledge / lesson / progress cards have no interaction.
 *
 *   question   stem (bold) → optional plain-words line → A–D option rows
 *   knowledge  title → one-line summary → definition block → ≤2 points → source
 *   lesson     title → goal → knowledge points → intuition
 *
 * Option rows are the only boxes: a 2-column grid (`26px` letter badge + `1fr`
 * text), 6px apart, 48px minimum height, top-aligned so a two-line option keeps
 * its badge aligned with the first line.
 *
 * Content source (real DSH shapes, verified against
 * `@deepseek-ai/dsh-client-ui-conversation` contract/records.d.ts and
 * `@deepseek-ai/dsh-client-ui-tool`): the settled owner supplies
 * `{phase, block, toolName, callId, t, ...}` where `block` is a
 * `ToolResultNode` with `block.call.argsRaw` (the tool-call arguments, exactly
 * `{kind, payload}`), `block.content` (the rendered result text) and no
 * `output` field. The view reads `block.call.argsRaw` first, then
 * `block.argsRaw` (start phase), then defensively JSON in `block.output` /
 * `block.content` for a window-truncated call.
 */
window.__ModuleLoader__.load({
  id: 'sokonanoda-dsh',
  factory(require) {
    'use strict'
    const React = require('react')
    const h = React.createElement

    /* Closed-loop diagnostic: tell the local Host that this client half actually
     * loaded and how far its fence registration got. Anonymous local route,
     * append-only log under the plugin data dir; never carries card content. */
    function report(stage, extra) {
      try {
        if (typeof fetch !== 'function') return
        const suffix = extra === undefined ? '' : '&' + extra
        fetch('/sokonanoda-client-report?stage=' + encodeURIComponent(stage) + suffix, { cache: 'no-store' })
          .catch(() => {})
      } catch (error) { /* diagnostics never break the UI */ }
    }

    /** Keep one bad card payload from taking down the Chat view; report the real error. */
    const FenceBoundary = typeof React.Component === 'function'
      ? class FenceBoundary extends React.Component {
        constructor(props) {
          super(props)
          this.state = { failed: false }
        }

        static getDerivedStateFromError() {
          return { failed: true }
        }

        componentDidCatch(error) {
          report('render-error', 'message=' + encodeURIComponent(String(error && error.message ? error.message : error).slice(0, 200)))
        }

        render() {
          return this.state.failed === true ? this.props.fallback : this.props.children
        }
      }
      : null

    const NS = 'dshLearnCard'
    const KINDS = ['question', 'knowledge', 'lesson', 'progress']
    const LETTERS = ['A', 'B', 'C', 'D', 'E', 'F']

    /* --------------------------------------------------------------- styles */

    /**
     * Content-only CSS. No rule paints an outer card: `.dslc-card` is a plain
     * flex column so the transcript surface stays visible. All colors use
     * `--dsw-alias-*` theme tokens with light-theme fallbacks. Font sizes are
     * explicit so the card does not drift with inherited chat typography.
     */
    const CARD_CSS = [
      '.dslc-card{display:flex;flex-direction:column;gap:10px;width:100%;max-width:100%;min-width:0;box-sizing:border-box;padding:0;border:0;background:transparent;border-radius:0;color:var(--dsw-alias-label-primary,#15171a);font-size:15px;line-height:1.55;overflow-wrap:anywhere;word-break:break-word}',
      '.dslc-stem{margin:0;font-size:15px;line-height:1.55;font-weight:600;overflow-wrap:anywhere}',
      '.dslc-plain{margin:0;font-size:13px;line-height:1.55;color:var(--dsw-alias-label-secondary,#4b5563);overflow-wrap:anywhere}',
      '.dslc-code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:.92em;padding:1px 4px;border-radius:5px;background:var(--dsw-alias-markdown-code-block,#f2f3f5);overflow-wrap:anywhere}',
      '.dslc-options{display:flex;flex-direction:column;gap:6px;margin:0;padding:0;list-style:none;min-width:0}',
      '.dslc-opt{display:grid;grid-template-columns:26px minmax(0,1fr);align-items:start;column-gap:10px;width:100%;max-width:100%;min-width:0;min-height:48px;box-sizing:border-box;padding:10px 12px;border:1px solid var(--dsw-alias-border-l1,#e6e8ec);border-radius:10px;background:var(--dsw-alias-bg-layer-2,#f7f8fa);color:inherit;overflow-wrap:anywhere}',
      '.dslc-letter{display:inline-flex;align-items:center;justify-content:center;width:26px;height:26px;box-sizing:border-box;border:1px solid var(--dsw-alias-border-l1,#e6e8ec);border-radius:50%;background:var(--dsw-alias-bg-base,#fff);color:var(--dsw-alias-label-secondary,#4b5563);font-size:12.5px;font-weight:700;line-height:1}',
      '.dslc-opttext{min-width:0;font-size:14px;line-height:1.5;overflow-wrap:anywhere;word-break:break-word}',
      '.dslc-heading{margin:0;font-size:15px;line-height:1.5;font-weight:700;overflow-wrap:anywhere}',
      '.dslc-summary{margin:0;min-width:0;max-width:100%;font-size:13.5px;line-height:1.6;overflow-wrap:anywhere;word-break:break-word}',
      '.dslc-def{margin:0;max-width:100%;box-sizing:border-box;padding:10px 12px;border-radius:10px;background:var(--dsw-alias-markdown-code-block,#f2f3f5);color:var(--dsw-alias-label-primary,#15171a);font:12.5px/1.6 ui-monospace,SFMono-Regular,Menlo,monospace;white-space:pre-wrap;overflow-x:auto;overflow-wrap:anywhere}',
      '.dslc-section{display:flex;flex-direction:column;gap:4px;min-width:0;max-width:100%}',
      '.dslc-section>*{min-width:0;max-width:100%}',
      '.dslc-sheading{margin:0;font-size:12px;font-weight:700;letter-spacing:.02em;color:var(--dsw-alias-label-tertiary,var(--dsw-alias-label-secondary,#6b7280))}',
      '.dslc-ptext{margin:0;min-width:0;max-width:100%;font-size:13.5px;line-height:1.6;overflow-wrap:anywhere;word-break:break-word}',
      '.dslc-slist{margin:0;padding-left:18px;font-size:13.5px;line-height:1.6;min-width:0}',
      '.dslc-slist li{overflow-wrap:anywhere;margin:0 0 2px}',
      '.dslc-source{margin:0;font-size:11.5px;line-height:1.45;color:var(--dsw-alias-label-tertiary,var(--dsw-alias-label-secondary,#6b7280));overflow-wrap:anywhere}',
      '.dslc-link{color:var(--dsw-alias-link,var(--dsw-alias-brand-primary,#0b6bcb));text-decoration:underline;overflow-wrap:anywhere}',
      '.dslc-placeholder{margin:0;font-size:13px;line-height:1.5;color:var(--dsw-alias-label-secondary,#6b7280);overflow-wrap:anywhere}',
      /* Fence cards: the surface lives here (the inner .dslc-card stays chrome-free). */
      '.dslc-fence{display:flex;flex-direction:column;gap:10px;width:100%;max-width:100%;min-width:0;box-sizing:border-box;padding:14px 16px;border:1px solid var(--dsw-alias-border-l1,#e6e8ec);border-radius:14px;background:var(--dsw-alias-bg-layer-2,#f7f8fa);color:var(--dsw-alias-label-primary,#15171a);overflow-wrap:anywhere}',
      /* Highlighted Lean code blocks. */
      '.dslc-lean-wrap{width:100%;max-width:100%;box-sizing:border-box;border:1px solid var(--dsw-alias-border-l1,#e6e8ec);border-radius:12px;background:var(--dsw-alias-markdown-code-block,#f2f3f5);overflow:hidden}',
      '.dslc-lean{margin:0;padding:10px 12px;overflow-x:auto;font:12.5px/1.7 ui-monospace,SFMono-Regular,Menlo,monospace;white-space:pre;color:var(--dsw-alias-label-primary,#15171a)}',
      '.dslc-hl-kw{color:var(--dsw-alias-state-business-primary,#0b57d0);font-weight:600}',
      '.dslc-hl-tactic{color:var(--dsw-alias-state-warn-primary,#9a5b00)}',
      '.dslc-hl-type{color:var(--dsw-alias-label-primary,#15171a);font-weight:600}',
      '.dslc-hl-str{color:var(--dsw-alias-state-success-primary,#0f7b4a)}',
      '.dslc-hl-com{color:var(--dsw-alias-label-tertiary,#6b7280);font-style:italic}',
      '.dslc-hl-num{color:var(--dsw-alias-label-secondary,#4b5563)}',
      '.dslc-hl-op{color:var(--dsw-alias-label-secondary,#4b5563)}',
      '.dslc-progress-head{display:flex;flex-direction:column;gap:4px}',
      '.dslc-stats{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:8px}',
      '.dslc-stat{display:flex;flex-direction:column;align-items:center;gap:2px;padding:8px 4px;border:1px solid var(--dsw-alias-border-l1,#e6e8ec);border-radius:10px;background:var(--dsw-alias-bg-base,rgba(127,127,127,0.06));min-width:0}',
      '.dslc-stat-value{font-size:17px;font-weight:700;line-height:1.1}',
      '.dslc-stat-label{font-size:10.5px;color:var(--dsw-alias-label-secondary,#4b5563);white-space:nowrap}',
      '.dslc-chapters{display:flex;flex-direction:column;gap:6px}',
      '.dslc-chapter{display:grid;grid-template-columns:minmax(0,1fr) 72px 36px;align-items:center;column-gap:8px;font-size:12px}',
      '.dslc-chapter-name{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;min-width:0}',
      '.dslc-bar{display:block;height:6px;border-radius:999px;background:rgba(127,127,127,0.18);overflow:hidden}',
      '.dslc-bar-fill{display:block;height:100%;border-radius:999px;background:var(--dsw-alias-state-business-primary,#0b57d0)}',
      '.dslc-chapter-pct{text-align:right;font-size:11px;color:var(--dsw-alias-label-secondary,#4b5563)}',
      '.dslc-heat{display:flex;flex-direction:column;gap:5px}',
      '.dslc-heat-row{display:grid;grid-template-columns:minmax(72px,0.9fr) minmax(0,1.6fr);gap:8px;align-items:center}',
      '.dslc-heat-name{font-size:11px;color:var(--dsw-alias-label-secondary,#4b5563);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}',
      '.dslc-heat-cells{display:flex;flex-wrap:wrap;gap:3px}',
      '.dslc-cell{display:inline-block;width:14px;height:14px;border-radius:4px;background:rgba(127,127,127,0.16)}',
      '.dslc-cell[data-level="1"]{background:#f3c76b}',
      '.dslc-cell[data-level="2"]{background:#8fb8f0}',
      '.dslc-cell[data-level="3"]{background:#4f83d6}',
      '.dslc-cell[data-level="4"]{background:#0f7b4a}',
      '.dslc-legend{display:flex;flex-wrap:wrap;gap:8px;font-size:10px;color:var(--dsw-alias-label-secondary,#6b7280);margin:0}',
      '.dslc-hotspots{display:flex;flex-direction:column;gap:5px}',
      '.dslc-hot{display:grid;grid-template-columns:minmax(0,1fr) auto 64px;gap:8px;align-items:center;font-size:12px}',
      '.dslc-hot-name{font-weight:600;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}',
      '.dslc-hot-reason{font-size:10px;color:var(--dsw-alias-state-warn-primary,#9a5b00);background:rgba(154,91,0,0.10);padding:1px 6px;border-radius:999px;white-space:nowrap}',
      '.dslc-recent{display:flex;align-items:center;gap:6px;flex-wrap:wrap}',
      '.dslc-dots{display:inline-flex;gap:3px;flex-wrap:wrap}',
      '.dslc-dot{display:inline-block;width:10px;height:10px;border-radius:50%}',
      '.dslc-dot[data-correct="1"]{background:#0f7b4a}',
      '.dslc-dot[data-correct="0"]{background:#c4553d}',
      '.dslc-opt[data-pickable="1"]{cursor:pointer}',
      '.dslc-opt[data-picked="1"]{border-color:var(--dsw-alias-state-business-primary,#0b57d0);background:rgba(11,87,208,0.08)}',
      '.dslc-opt:focus-visible{outline:2px solid var(--dsw-alias-state-business-primary,#0b57d0);outline-offset:2px}',
      '.dslc-fast-row{display:flex;justify-content:flex-end;margin:0 0 -6px}',
      '.dslc-fast-badge{display:inline-flex;align-items:center;gap:4px;font-size:10.5px;line-height:1;padding:3px 7px;border-radius:999px;border:1px solid var(--dsw-alias-border-l1,#e6e8ec);color:var(--dsw-alias-label-tertiary,var(--dsw-alias-label-secondary,#6b7280));background:var(--dsw-alias-bg-base,rgba(127,127,127,0.08));white-space:nowrap}',
      '.dslc-open-hint{margin:0;padding:8px 10px;border-radius:8px;border:1px dashed var(--dsw-alias-border-l1,#e6e8ec);background:var(--dsw-alias-bg-base,rgba(127,127,127,0.06));color:var(--dsw-alias-label-secondary,#4b5563);font-size:12.5px;line-height:1.5;overflow-wrap:anywhere}',
    ]

    /* Inject the stylesheet once per page; HMR removes this module's styles
     * before executing the new factory, so the tag is fresh after a reload. */
    try {
      const styleId = '@local/dsh-learn-card/card.css'
      if (typeof document !== 'undefined' && document !== null
        && typeof document.createElement === 'function' && document.head !== undefined && document.head !== null) {
        const existing = typeof document.querySelector === 'function'
          ? document.querySelector('style[data-plugin-css="' + styleId + '"]')
          : null
        if (existing === null) {
          const tag = document.createElement('style')
          tag.dataset = tag.dataset || {}
          tag.dataset.plugin = '@local/dsh-learn-card'
          tag.dataset.pluginCss = styleId
          tag.textContent = CARD_CSS.join('')
          document.head.appendChild(tag)
        }
      }
    } catch (error) {
      /* A missing document (tests, headless render) only means no stylesheet. */
    }

    /* -------------------------------------------------------------- locales */

    const zh = {
      pointsLabel: '要点',
      knowledgePointsLabel: '知识点',
      goalLabel: '目标',
      intuitionLabel: '直觉',
      sourceLabel: '出处',
      cardPreparing: '卡片准备中…',
      cardUnavailable: '这张卡片的内容暂时不可用。',
      unsupportedKind: '未知卡片类型',
      progressTitle: '学习进展',
      progressCurrent: '当前：',
      progressStreak: '连对',
      progressAccuracy: '正确率',
      progressCoverage: '知识点覆盖',
      progressAsked: '已答题',
      progressChapters: '章节进展',
      progressHeat: '掌握热力（单元）',
      progressHotspots: '建议关注',
      progressRecent: '最近',
      progressLevel0: '未接触',
      progressLevel1: '刚起步',
      progressLevel2: '有点感觉',
      progressLevel3: '比较稳',
      progressLevel4: '很熟',
      fastBadge: '快思考',
      openAnswerHint: '直接在输入框写下你的回答，不用选字母。',
    }

    const en = {
      pointsLabel: 'Key points',
      knowledgePointsLabel: 'Knowledge points',
      goalLabel: 'Goal',
      intuitionLabel: 'Intuition',
      sourceLabel: 'Source',
      cardPreparing: 'Preparing the card…',
      cardUnavailable: 'This card has no readable content.',
      unsupportedKind: 'Unknown card kind',
      progressTitle: 'Progress',
      progressCurrent: 'Now: ',
      progressStreak: 'Streak',
      progressAccuracy: 'Accuracy',
      progressCoverage: 'Coverage',
      progressAsked: 'Answered',
      progressChapters: 'Chapters',
      progressHeat: 'Mastery heat (units)',
      progressHotspots: 'Focus',
      progressRecent: 'Recent',
      progressLevel0: 'new',
      progressLevel1: 'starting',
      progressLevel2: 'getting it',
      progressLevel3: 'solid',
      progressLevel4: 'strong',
      fastBadge: 'fast path',
      openAnswerHint: 'Type your answer in the composer — no letter needed.',
    }

    /* -------------------------------------------------------------- helpers */

    function isRecord(value) {
      return value !== null && typeof value === 'object' && !Array.isArray(value)
    }

    function textOf(value) {
      return typeof value === 'string' ? value.trim() : ''
    }

    /** `props.t` is the locale-bound seat; fall back to the Chinese default. */
    function makeT(props) {
      const raw = isRecord(props) && typeof props.t === 'function' ? props.t : null
      return function t(key, fallback) {
        if (raw !== null) {
          let value
          try {
            value = raw(key)
          } catch (error) {
            value = undefined
          }
          if (typeof value === 'string' && value !== '' && value !== key) return value
        }
        return fallback
      }
    }

    function parseObject(raw) {
      if (typeof raw !== 'string' || raw.trim() === '') return null
      try {
        const parsed = JSON.parse(raw)
        return isRecord(parsed) ? parsed : null
      } catch (error) {
        return null
      }
    }

    function cardFromArgsObject(args) {
      if (args === null) return null
      const payload = isRecord(args.payload) ? args.payload : {}
      if (typeof args.kind !== 'string' || args.kind === '') return null
      if (KINDS.includes(args.kind)) return { kind: args.kind, payload }
      return { kind: 'unsupported', payload, unsupportedKind: args.kind }
    }

    function normalizeOptions(raw) {
      if (!Array.isArray(raw)) return []
      const options = []
      for (let index = 0; index < raw.length && index < LETTERS.length; index++) {
        const item = raw[index]
        if (typeof item === 'string') {
          options.push({ key: LETTERS[index], text: item })
          continue
        }
        if (isRecord(item)) {
          const key = textOf(item.key) !== '' ? textOf(item.key).slice(0, 2) : LETTERS[index]
          const text = textOf(item.text) !== '' ? textOf(item.text) : textOf(item.label)
          options.push({ key, text })
        }
      }
      return options
    }

    function normalizePoints(raw) {
      if (!Array.isArray(raw)) return []
      const points = []
      for (const item of raw) {
        if (typeof item === 'string' && item.trim() !== '') points.push(item.trim())
        else if (isRecord(item) && textOf(item.text) !== '') points.push(textOf(item.text))
        else if (isRecord(item) && textOf(item.title) !== '') points.push(textOf(item.title))
      }
      return points
    }

    /** First list section of an engine knowledge card, as points. */
    function pointsFromSections(sections) {
      if (!Array.isArray(sections)) return []
      for (const section of sections) {
        if (isRecord(section) && section.kind === 'list' && Array.isArray(section.items)) {
          return normalizePoints(section.items)
        }
      }
      return []
    }

    function sourceOf(raw) {
      if (typeof raw === 'string' && raw.trim() !== '') return { label: raw.trim(), url: '' }
      if (isRecord(raw)) {
        const label = textOf(raw.label) !== '' ? textOf(raw.label) : (textOf(raw.text) !== '' ? textOf(raw.text) : textOf(raw.title))
        const url = /^https?:\/\//i.test(textOf(raw.url)) ? textOf(raw.url) : ''
        if (label !== '' || url !== '') return { label: label !== '' ? label : url, url }
      }
      return null
    }

    /** Split `inline code` spans into React text and `<code>` nodes. */
    function inlineNodes(text) {
      const value = typeof text === 'string' ? text : ''
      if (value === '') return []
      const parts = value.split(/(`[^`]*`)/g)
      const nodes = []
      for (let index = 0; index < parts.length; index++) {
        const part = parts[index]
        if (part === '') continue
        if (part.length >= 2 && part.startsWith('`') && part.endsWith('`')) {
          nodes.push(h('code', { className: 'dslc-code', key: 'code-' + index, style: S.code }, part.slice(1, -1)))
        } else {
          nodes.push(part)
        }
      }
      return nodes
    }

    function section(t, key, heading, body) {
      return h('div', { className: 'dslc-section', key: key, style: S.section },
        h('p', { className: 'dslc-sheading', style: S.sheading }, heading),
        body,
      )
    }

    /* Inline styles (2026-11-23): the stylesheet injection silently failed in one
     * real page (`<style data-plugin>` absent), so the card rendered as plain
     * paragraphs. Every visible part now carries its own style object; the CSS
     * stylesheet stays as a nicer polish when it does load. */
    const S = {
      card: { display: 'flex', flexDirection: 'column', gap: '10px', width: '100%', maxWidth: '100%', minWidth: 0, boxSizing: 'border-box', color: 'var(--dsw-alias-label-primary,#15171a)', fontSize: '15px', lineHeight: 1.55, overflowWrap: 'anywhere', wordBreak: 'break-word' },
      stem: { margin: 0, fontSize: '15px', lineHeight: 1.55, fontWeight: 600, overflowWrap: 'anywhere' },
      plain: { margin: 0, fontSize: '13px', lineHeight: 1.55, color: 'var(--dsw-alias-label-secondary,#4b5563)', overflowWrap: 'anywhere' },
      code: { fontFamily: 'ui-monospace,SFMono-Regular,Menlo,monospace', fontSize: '0.92em', padding: '1px 4px', borderRadius: '5px', background: 'rgba(127,127,127,0.10)', overflowWrap: 'anywhere' },
      list: { display: 'flex', flexDirection: 'column', gap: '6px', margin: 0, padding: 0, listStyle: 'none' },
      option: { display: 'grid', gridTemplateColumns: '26px minmax(0,1fr)', alignItems: 'start', columnGap: '10px', width: '100%', maxWidth: '100%', minWidth: 0, minHeight: '48px', boxSizing: 'border-box', padding: '10px 12px', border: '1px solid rgba(127,127,127,0.28)', borderRadius: '10px', background: 'rgba(127,127,127,0.08)', overflowWrap: 'anywhere' },
      letter: { display: 'inline-flex', alignItems: 'center', justifyContent: 'center', width: '26px', height: '26px', boxSizing: 'border-box', border: '1px solid rgba(127,127,127,0.32)', borderRadius: '50%', background: 'rgba(127,127,127,0.12)', color: 'var(--dsw-alias-label-secondary,#4b5563)', fontSize: '12.5px', fontWeight: 700, lineHeight: 1 },
      opttext: { minWidth: 0, fontSize: '14px', lineHeight: 1.5, overflowWrap: 'anywhere', wordBreak: 'break-word' },
      heading: { margin: 0, fontSize: '15px', lineHeight: 1.5, fontWeight: 700, overflowWrap: 'anywhere' },
      body: { margin: 0, minWidth: 0, maxWidth: '100%', fontSize: '13.5px', lineHeight: 1.6, overflowWrap: 'anywhere', wordBreak: 'break-word' },
      section: { display: 'flex', flexDirection: 'column', gap: '4px', minWidth: 0, maxWidth: '100%' },
      sheading: { margin: 0, fontSize: '12px', fontWeight: 700, letterSpacing: '.02em', color: 'var(--dsw-alias-label-tertiary,var(--dsw-alias-label-secondary,#6b7280))' },
      slist: { margin: 0, paddingLeft: '18px', fontSize: '13.5px', lineHeight: 1.6, minWidth: 0 },
      def: { margin: 0, maxWidth: '100%', boxSizing: 'border-box', padding: '10px 12px', borderRadius: '10px', background: 'rgba(127,127,127,0.10)', color: 'var(--dsw-alias-label-primary,#15171a)', font: '12.5px/1.6 ui-monospace,SFMono-Regular,Menlo,monospace', whiteSpace: 'pre-wrap', overflowX: 'auto', overflowWrap: 'anywhere' },
      source: { margin: 0, fontSize: '11.5px', lineHeight: 1.45, color: 'var(--dsw-alias-label-tertiary,var(--dsw-alias-label-secondary,#6b7280))', overflowWrap: 'anywhere' },
      placeholder: { margin: 0, fontSize: '13px', lineHeight: 1.5, color: 'var(--dsw-alias-label-secondary,#6b7280)', overflowWrap: 'anywhere' },
      fence: { display: 'flex', flexDirection: 'column', gap: '10px', width: '100%', maxWidth: '100%', minWidth: 0, boxSizing: 'border-box', padding: '14px 16px', border: '1px solid var(--dsw-alias-border-l1,#e6e8ec)', borderRadius: '14px', background: 'var(--dsw-alias-bg-layer-2,#f7f8fa)', color: 'var(--dsw-alias-label-primary,#15171a)', overflowWrap: 'anywhere' },
      leanWrap: { width: '100%', maxWidth: '100%', boxSizing: 'border-box', border: '1px solid var(--dsw-alias-border-l1,#e6e8ec)', borderRadius: '12px', background: 'var(--dsw-alias-markdown-code-block,#f2f3f5)', overflow: 'hidden' },
      lean: { margin: 0, padding: '10px 12px', overflowX: 'auto', font: '12.5px/1.7 ui-monospace,SFMono-Regular,Menlo,monospace', whiteSpace: 'pre', color: 'var(--dsw-alias-label-primary,#15171a)' },
      hlKw: { color: 'var(--dsw-alias-state-business-primary,#0b57d0)', fontWeight: 600 },
      hlTactic: { color: 'var(--dsw-alias-state-warn-primary,#9a5b00)' },
      hlType: { color: 'var(--dsw-alias-label-primary,#15171a)', fontWeight: 600 },
      hlStr: { color: 'var(--dsw-alias-state-success-primary,#0f7b4a)' },
      hlCom: { color: 'var(--dsw-alias-label-tertiary,#6b7280)', fontStyle: 'italic' },
      hlNum: { color: 'var(--dsw-alias-label-secondary,#4b5563)' },
      hlOp: { color: 'var(--dsw-alias-label-secondary,#4b5563)' },
      progressHead: { display: 'flex', flexDirection: 'column', gap: '4px' },
      statsGrid: { display: 'grid', gridTemplateColumns: 'repeat(4, minmax(0,1fr))', gap: '8px' },
      stat: { display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '2px', padding: '8px 4px', border: '1px solid rgba(127,127,127,0.25)', borderRadius: '10px', background: 'rgba(127,127,127,0.06)', minWidth: 0 },
      statValue: { fontSize: '17px', fontWeight: 700, lineHeight: 1.1 },
      statLabel: { fontSize: '10.5px', color: 'var(--dsw-alias-label-secondary,#4b5563)', whiteSpace: 'nowrap' },
      chapterList: { display: 'flex', flexDirection: 'column', gap: '6px' },
      chapterRow: { display: 'grid', gridTemplateColumns: 'minmax(0,1fr) 72px 36px', alignItems: 'center', columnGap: '8px', fontSize: '12px' },
      chapterName: { overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', minWidth: 0 },
      bar: { display: 'block', height: '6px', borderRadius: '999px', background: 'rgba(127,127,127,0.18)', overflow: 'hidden' },
      barFill: { display: 'block', height: '100%', borderRadius: '999px', background: 'var(--dsw-alias-state-business-primary,#0b57d0)' },
      chapterPct: { textAlign: 'right', fontSize: '11px', color: 'var(--dsw-alias-label-secondary,#4b5563)' },
      heatList: { display: 'flex', flexDirection: 'column', gap: '5px' },
      heatRow: { display: 'grid', gridTemplateColumns: 'minmax(72px,0.9fr) minmax(0,1.6fr)', gap: '8px', alignItems: 'center' },
      heatName: { fontSize: '11px', color: 'var(--dsw-alias-label-secondary,#4b5563)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' },
      heatCells: { display: 'flex', flexWrap: 'wrap', gap: '3px' },
      cell: { display: 'inline-block', width: '14px', height: '14px', borderRadius: '4px', background: 'rgba(127,127,127,0.16)' },
      cellLevels: [
        { background: 'rgba(127,127,127,0.16)' },
        { background: '#f3c76b' },
        { background: '#8fb8f0' },
        { background: '#4f83d6' },
        { background: '#0f7b4a' },
      ],
      legend: { display: 'flex', flexWrap: 'wrap', gap: '8px', fontSize: '10px', color: 'var(--dsw-alias-label-secondary,#6b7280)', margin: 0 },
      legendItem: { display: 'inline-flex', alignItems: 'center', gap: '3px' },
      hotList: { display: 'flex', flexDirection: 'column', gap: '5px' },
      hotRow: { display: 'grid', gridTemplateColumns: 'minmax(0,1fr) auto 64px', gap: '8px', alignItems: 'center', fontSize: '12px' },
      hotName: { fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' },
      hotReason: { fontSize: '10px', color: 'var(--dsw-alias-state-warn-primary,#9a5b00)', background: 'rgba(154,91,0,0.10)', padding: '1px 6px', borderRadius: '999px', whiteSpace: 'nowrap' },
      recentRow: { display: 'flex', alignItems: 'center', gap: '6px', flexWrap: 'wrap' },
      dots: { display: 'inline-flex', gap: '3px', flexWrap: 'wrap' },
      dot: { display: 'inline-block', width: '10px', height: '10px', borderRadius: '50%' },
      dotOk: { background: '#0f7b4a' },
      dotMiss: { background: '#c4553d' },
      fastRow: { display: 'flex', justifyContent: 'flex-end', margin: '0 0 -6px' },
      fastBadge: { display: 'inline-flex', alignItems: 'center', gap: '4px', fontSize: '10.5px', lineHeight: 1, padding: '3px 7px', borderRadius: '999px', border: '1px solid rgba(127,127,127,0.22)', color: 'var(--dsw-alias-label-tertiary,var(--dsw-alias-label-secondary,#6b7280))', background: 'rgba(127,127,127,0.08)', whiteSpace: 'nowrap' },
      openHint: { margin: 0, padding: '8px 10px', borderRadius: '8px', border: '1px dashed rgba(127,127,127,0.32)', background: 'rgba(127,127,127,0.06)', color: 'var(--dsw-alias-label-secondary,#4b5563)', fontSize: '12.5px', lineHeight: 1.5, overflowWrap: 'anywhere' },
      optionPick: { cursor: 'pointer' },
      optionPicked: { borderColor: 'var(--dsw-alias-state-business-primary,#0b57d0)', background: 'rgba(11,87,208,0.08)' },
    }

    function cardShell(kind, children) {
      return h('div', {
        className: 'dslc-card',
        'data-learn-card': kind,
        'data-kind': kind,
        role: 'group',
        style: S.card,
      }, children)
    }

    /* ------------------------------------------------------------- renderers */

    function questionCard(payload, onPick, picked, t) {
      const stem = textOf(payload.stem) !== '' ? textOf(payload.stem) : textOf(payload.prompt)
      const plain = textOf(payload.plain)
      const options = normalizeOptions(payload.options)
      const questionKind = textOf(payload.questionKind)
      const open = options.length === 0 && questionKind !== '' && questionKind !== 'choice'
      const clickable = typeof onPick === 'function'
      const openHint = open
        ? h('p', { className: 'dslc-open-hint', key: 'open-hint', style: S.openHint },
          t !== undefined && typeof t === 'function' ? t('openAnswerHint', '直接在输入框写下你的回答，不用选字母。') : '直接在输入框写下你的回答，不用选字母。')
        : null
      return cardShell('question', [
        stem !== '' ? h('p', { className: 'dslc-stem', key: 'stem', style: S.stem }, inlineNodes(stem)) : null,
        plain !== '' ? h('p', { className: 'dslc-plain', key: 'plain', style: S.plain }, inlineNodes(plain)) : null,
        openHint,
        options.length > 0
          ? h('ul', { className: 'dslc-options', key: 'options', style: S.list }, options.map((option, index) => {
            const active = clickable && picked === null
            return h('li', {
              className: 'dslc-opt',
              key: 'option-' + index,
              'data-letter': option.key,
              'data-pickable': active ? '1' : undefined,
              'data-picked': picked === option.key ? '1' : undefined,
              role: active ? 'button' : undefined,
              tabIndex: active ? 0 : undefined,
              'aria-label': active ? '发送选项 ' + option.key : undefined,
              style: picked === option.key
                ? Object.assign({}, S.option, S.optionPick, S.optionPicked)
                : active ? Object.assign({}, S.option, S.optionPick) : S.option,
              onClick: active ? () => onPick(option.key) : undefined,
              onKeyDown: active ? (event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  onPick(option.key)
                }
              } : undefined,
            },
            h('span', { className: 'dslc-letter', style: S.letter }, option.key),
            h('span', { className: 'dslc-opttext', style: S.opttext }, inlineNodes(option.text)))
          }))
          : null,
      ])
    }

    function knowledgeCard(t, payload) {
      const title = textOf(payload.title)
      const summary = textOf(payload.summary) !== '' ? textOf(payload.summary) : textOf(payload.oneLiner)
      const definition = textOf(payload.definition) !== ''
        ? textOf(payload.definition)
        : (isRecord(payload.definition) ? textOf(payload.definition.code) : '')
      const rawPoints = normalizePoints(payload.points)
      const points = (rawPoints.length > 0 ? rawPoints : pointsFromSections(payload.sections)).slice(0, 2)
      const source = sourceOf(payload.source)
      return cardShell('knowledge', [
        title !== '' ? h('p', { className: 'dslc-heading', key: 'title', style: S.heading }, inlineNodes(title)) : null,
        summary !== '' ? h('p', { className: 'dslc-summary', key: 'summary', style: S.body }, inlineNodes(summary)) : null,
        definition !== ''
          ? h('pre', { className: 'dslc-def', key: 'definition', style: S.def }, h('code', {}, highlightLean(definition)))
          : null,
        points.length > 0
          ? section(t, 'points', t('pointsLabel', '要点'),
            h('ul', { className: 'dslc-slist', style: S.slist }, points.map((point, index) =>
              h('li', { key: 'point-' + index }, inlineNodes(point)))))
          : null,
        source !== null
          ? h('p', { className: 'dslc-source', key: 'source', style: S.source },
            t('sourceLabel', '出处') + '：',
            source.url !== ''
              ? h('a', { className: 'dslc-link', href: source.url }, source.label)
              : source.label)
          : null,
      ])
    }

    function lessonCard(t, payload) {
      const title = textOf(payload.title)
      const goal = textOf(payload.goal)
      const intuition = textOf(payload.intuition)
      const rawLessonPoints = normalizePoints(payload.points)
      const points = rawLessonPoints.length > 0 ? rawLessonPoints : normalizePoints(payload.knowledgePoints)
      return cardShell('lesson', [
        title !== '' ? h('p', { className: 'dslc-heading', key: 'title', style: S.heading }, inlineNodes(title)) : null,
        goal !== ''
          ? section(t, 'goal', t('goalLabel', '目标'), h('p', { className: 'dslc-ptext', style: S.body }, inlineNodes(goal)))
          : null,
        points.length > 0
          ? section(t, 'points', t('knowledgePointsLabel', '知识点'),
            h('ul', { className: 'dslc-slist', style: S.slist }, points.map((point, index) =>
              h('li', { key: 'point-' + index }, inlineNodes(point)))))
          : null,
        intuition !== ''
          ? section(t, 'intuition', t('intuitionLabel', '直觉'), h('p', { className: 'dslc-ptext', style: S.body }, inlineNodes(intuition)))
          : null,
      ])
    }

    /** 进展热点卡：把引擎的进度/覆盖/掌握热力/薄弱单元渲染成纯展示卡片。 */
    function progressCard(t, payload) {
      const summary = isRecord(payload.summary) ? payload.summary : {}
      const chapters = Array.isArray(payload.chapters) ? payload.chapters : []
      const heatRows = Array.isArray(payload.heatRows) ? payload.heatRows : []
      const hotspots = Array.isArray(payload.hotspots) ? payload.hotspots : []
      const recent = Array.isArray(payload.recent) ? payload.recent : []
      const pct = (value) => Math.round((Number(value) || 0) * 100)
      const stats = [
        { key: 'streak', value: summary.streak, label: t('progressStreak', '连对') },
        { key: 'accuracy', value: pct(summary.accuracy) + '%', label: t('progressAccuracy', '正确率') },
        { key: 'coverage', value: pct(summary.coverage) + '%', label: t('progressCoverage', '知识点覆盖') },
        { key: 'asked', value: summary.asked, label: t('progressAsked', '已答题') },
      ]
      const legend = [0, 1, 2, 3, 4].map((level) => h('span', {
        className: 'dslc-legend-item', key: 'legend-' + level, style: S.legendItem,
      },
      h('span', { className: 'dslc-cell', 'data-level': level, style: Object.assign({}, S.cell, S.cellLevels[level]) }, ''),
      h('span', {}, t('progressLevel' + level, ['未接触', '刚起步', '有点感觉', '比较稳', '很熟'][level]))))
      return cardShell('progress', [
        h('div', { className: 'dslc-progress-head', key: 'head', style: S.progressHead },
          h('p', { className: 'dslc-heading', style: S.heading }, '📈 ' + t('progressTitle', '学习进展')),
          summary.currentUnit
            ? h('p', { className: 'dslc-plain', style: S.plain }, t('progressCurrent', '当前：') + summary.currentUnit)
            : null),
        h('div', { className: 'dslc-stats', key: 'stats', style: S.statsGrid }, stats.map((stat) =>
          h('div', { className: 'dslc-stat', key: stat.key, style: S.stat },
            h('span', { className: 'dslc-stat-value', style: S.statValue }, String(stat.value)),
            h('span', { className: 'dslc-stat-label', style: S.statLabel }, stat.label)))),
        chapters.length > 0
          ? h('div', { className: 'dslc-chapters', key: 'chapters', style: S.chapterList },
            h('p', { className: 'dslc-sheading', style: S.sheading }, t('progressChapters', '章节进展')),
            chapters.map((chapter, index) => h('div', {
              className: 'dslc-chapter', key: 'chapter-' + index, style: S.chapterRow,
            },
            h('span', { className: 'dslc-chapter-name', style: S.chapterName }, (chapter.id || '') + ' ' + (chapter.title || '')),
            h('span', { className: 'dslc-bar', style: S.bar },
              h('span', { className: 'dslc-bar-fill', style: Object.assign({}, S.barFill, { width: pct(chapter.completion) + '%' }) }, '')),
            h('span', { className: 'dslc-chapter-pct', style: S.chapterPct }, pct(chapter.completion) + '%'))))
          : null,
        heatRows.length > 0
          ? h('div', { className: 'dslc-heat', key: 'heat', style: S.heatList },
            h('p', { className: 'dslc-sheading', style: S.sheading }, t('progressHeat', '掌握热力（单元）')),
            heatRows.map((row, index) => h('div', { className: 'dslc-heat-row', key: 'heat-' + index, style: S.heatRow },
              h('span', { className: 'dslc-heat-name', style: S.heatName }, (row.chapter || '') + ' ' + (row.title || '')),
              h('span', { className: 'dslc-heat-cells', style: S.heatCells },
                (Array.isArray(row.units) ? row.units : []).map((unit, unitIndex) => {
                  const level = Math.max(0, Math.min(4, Number(unit.level) || 0))
                  return h('span', {
                    className: 'dslc-cell',
                    key: 'cell-' + unitIndex,
                    'data-level': level,
                    title: String(unit.unit || '') + ' · ' + (Number(unit.touched) || 0) + '/' + (Number(unit.total) || 0),
                    style: Object.assign({}, S.cell, S.cellLevels[level]),
                  }, '')
                })))),
            h('p', { className: 'dslc-legend', key: 'legend', style: S.legend }, legend))
          : null,
        hotspots.length > 0
          ? h('div', { className: 'dslc-hotspots', key: 'hotspots', style: S.hotList },
            h('p', { className: 'dslc-sheading', style: S.sheading }, t('progressHotspots', '建议关注')),
            hotspots.map((hot, index) => h('div', { className: 'dslc-hot', key: 'hot-' + index, style: S.hotRow },
              h('span', { className: 'dslc-hot-name', style: S.hotName }, String(hot.label || '')),
              h('span', { className: 'dslc-hot-reason', style: S.hotReason }, String(hot.reason || '')),
              Number.isFinite(Number(hot.mastery))
                ? h('span', { className: 'dslc-bar', style: S.bar },
                  h('span', { className: 'dslc-bar-fill', style: Object.assign({}, S.barFill, { width: pct(hot.mastery) + '%' }) }, ''))
                : h('span', {}))))
          : null,
        recent.length > 0
          ? h('div', { className: 'dslc-recent', key: 'recent', style: S.recentRow },
            h('span', { className: 'dslc-sheading', style: S.sheading }, t('progressRecent', '最近')),
            h('span', { className: 'dslc-dots', style: S.dots }, recent.map((entry, index) => h('span', {
              className: 'dslc-dot',
              key: 'dot-' + index,
              'data-correct': entry.correct === true ? '1' : '0',
              title: String(entry.qid || ''),
              style: Object.assign({}, S.dot, entry.correct === true ? S.dotOk : S.dotMiss),
            }, ''))))
          : null,
      ])
    }

    function placeholderCard(t, phase, unsupportedKind) {
      const message = unsupportedKind !== undefined
        ? t('unsupportedKind', '未知卡片类型') + ' · ' + unsupportedKind
        : (phase === 'preparing' || phase === 'start'
          ? t('cardPreparing', '卡片准备中…')
          : t('cardUnavailable', '这张卡片的内容暂时不可用。'))
      return cardShell(unsupportedKind !== undefined ? 'unsupported' : 'pending', [
        h('p', { className: 'dslc-placeholder', key: 'message', style: S.placeholder }, message),
      ])
    }

    /* -------------------------------------------------- Lean syntax highlight */

    const LEAN_KEYWORDS = ['theorem', 'lemma', 'def', 'example', 'import', 'open', 'namespace',
      'section', 'end', 'variable', 'axiom', 'inductive', 'structure', 'class', 'instance',
      'where', 'match', 'with', 'fun', 'let', 'in', 'if', 'then', 'else', 'by', 'have', 'show',
      'from', 'calc', 'at', 'this', 'deriving', 'universe', 'noncomputable', 'private',
      'protected', 'local', 'scoped', 'attribute', 'notation', 'infix', 'infixl', 'infixr',
      'prefix', 'postfix', 'mutual', 'partial', 'termination_by', 'decreasing_by', 'set_option',
      'export', 'abbrev', 'opaque', 'syntax', 'macro', 'elab', 'include']
    const LEAN_TACTICS = ['apply', 'exact', 'intro', 'intros', 'rw', 'simp', 'omega', 'linarith',
      'nlinarith', 'ring', 'norm_num', 'constructor', 'cases', 'induction', 'ext', 'funext',
      'rfl', 'trivial', 'assumption', 'contradiction', 'exfalso', 'sorry', 'decide', 'positivity',
      'gcongr', 'field_simp', 'ring_nf', 'push_neg', 'congr', 'subst', 'injection', 'fin_cases',
      'norm_cast', 'exact_mod_cast', 'use', 'refine', 'obtain', 'rcases', 'specialize', 'symm',
      'trans', 'change', 'convert', 'generalize', 'clear', 'rename_i', 'repeat', 'first', 'try',
      'all_goals', 'any_goals', 'focus', 'rotate_left', 'swap', 'haveI', 'letI']
    const LEAN_TYPES = ['Prop', 'Type', 'Sort', 'Nat', 'Int', 'Rat', 'Real', 'Complex', 'Set',
      'List', 'Finset', 'Multiset', 'Bool', 'String', 'Char', 'Option', 'Array', 'Vector',
      'Subtype', 'Sigma', 'PSet', 'ENat', 'ENNReal', 'NNReal', 'Finset', 'Nonempty']
    const LEAN_SYMBOLS = ['∀', '∃', '→', '↔', '∧', '∨', '¬', '≠', '≤', '≥', '∈', '∉', '⊆', '⊂',
      '∪', '∩', '∅', 'λ', 'Π', 'Σ', '∘', '≈', '≡', '⟨', '⟩', '↦', '⊢', '⊨', 'ℕ', 'ℤ', 'ℚ',
      'ℝ', 'ℂ']

    function escapeRegExp(value) {
      return String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    }
    function alternation(words) {
      return words.slice().sort(function (a, b) { return b.length - a.length }).map(escapeRegExp).join('|')
    }
    /* Comments and strings first; then declarations, tactics, types, symbols, numbers, operators. */
    const LEAN_TOKEN_RE = new RegExp(
      '(/\\-[\\s\\S]*?\\-/)|(--[^\\n]*)|("(?:[^"\\\\]|\\\\.)*")|(\\b(?:' + alternation(LEAN_KEYWORDS) + ')\\b)'
      + '|(\\b(?:' + alternation(LEAN_TACTICS) + ')\\b)|(\\b(?:' + alternation(LEAN_TYPES) + ')\\b)'
      + '|(' + alternation(LEAN_SYMBOLS) + ')|(\\b\\d+(?:\\.\\d+)?\\b)|([=<>+\\-*/^!?&|:]+)',
      'g',
    )

    /**
     * Tokenize Lean-ish source into React spans (no dependency, no HTML parsing).
     * A plain-text fallback keeps the code readable for unknown constructs.
     * @param code - fence/definition source.
     * @returns text nodes and highlighted spans.
     */
    function highlightLean(code) {
      const value = typeof code === 'string' ? code : ''
      if (value === '') return []
      const nodes = []
      let last = 0
      let key = 0
      LEAN_TOKEN_RE.lastIndex = 0
      let match
      while ((match = LEAN_TOKEN_RE.exec(value)) !== null) {
        const token = match[0]
        if (token === '') continue
        if (match.index > last) nodes.push(value.slice(last, match.index))
        let style = null
        if (match[1] !== undefined || match[2] !== undefined) style = S.hlCom
        else if (match[3] !== undefined) style = S.hlStr
        else if (match[4] !== undefined) style = S.hlKw
        else if (match[5] !== undefined) style = S.hlTactic
        else if (match[6] !== undefined) style = S.hlType
        else if (match[7] !== undefined) style = S.hlType
        else if (match[8] !== undefined) style = S.hlNum
        else if (match[9] !== undefined) style = S.hlOp
        nodes.push(h('span', {
          key: 'hl-' + (key++),
          className: 'dslc-hl ' + (style === S.hlCom ? 'dslc-hl-com'
            : style === S.hlStr ? 'dslc-hl-str'
              : style === S.hlKw ? 'dslc-hl-kw'
                : style === S.hlTactic ? 'dslc-hl-tactic'
                  : style === S.hlNum ? 'dslc-hl-num'
                    : style === S.hlOp ? 'dslc-hl-op' : 'dslc-hl-type'),
          style: style,
        }, token))
        last = match.index + token.length
      }
      if (last < value.length) nodes.push(value.slice(last))
      return nodes
    }

    /* -------------------------------------------------------------- fence cards */

    function fenceCardOf(code) {
      return cardFromArgsObject(parseObject(code))
    }

    /**
     * Hoc layer for ```` ```learn-card {kind,payload} ``` ```` in assistant Markdown.
     * While the message streams the raw code block (children) stays visible; a
     * settled fence is replaced by the card. Parse errors fail open to the code.
     */
    function LearnCardFence(props) {
      const [picked, setPicked] = React.useState(null)
      const reported = React.useRef(false)
      const streaming = isRecord(props) && props.streaming === true
      const children = isRecord(props) ? props.children : undefined
      if (streaming) return children === undefined ? null : children
      const card = fenceCardOf(isRecord(props) && typeof props.code === 'string' ? props.code : '')
      if (card === null) return children === undefined ? null : children
      if (reported.current !== true) {
        reported.current = true
        report('fence-rendered', 'key=' + encodeURIComponent(card.kind + '|boundary=' + (FenceBoundary === null ? 'null' : 'ok')))
      }
      const t = makeT(props)
      /* 卡片选项可直接发送字母：走和打字完全相同的 composer 提交路径。 */
      const inputActions = isRecord(props) ? props.inputActions : undefined
      const canSend = card.kind === 'question' && picked === null
        && inputActions !== undefined
        && typeof inputActions.setDraft === 'function'
        && typeof inputActions.submit === 'function'
      const onPick = canSend
        ? (key) => {
          setPicked(key)
          try {
            inputActions.setDraft(key)
            inputActions.submit()
          } catch (error) {
            setPicked(null)
          }
        }
        : null
      let body
      try {
        body = card.kind === 'question' ? questionCard(card.payload, onPick, picked)
          : card.kind === 'knowledge' ? knowledgeCard(t, card.payload)
            : card.kind === 'lesson' ? lessonCard(t, card.payload)
              : card.kind === 'progress' ? progressCard(t, card.payload)
                : placeholderCard(t, '', card.unsupportedKind)
      } catch (error) {
        report('render-error', 'kind=' + encodeURIComponent(card.kind) + '&message=' + encodeURIComponent(String(error && error.message ? error.message : error).slice(0, 180)))
        return children === undefined ? null : children
      }
      const badge = h('div', { className: 'dslc-fast-row', key: 'fast-row', style: S.fastRow },
        h('span', {
          className: 'dslc-fast-badge',
          key: 'fast-badge',
          style: S.fastBadge,
          title: 'sokonanoda 本地快思考（/sokonanoda-fast-think off 关闭）',
        }, '⚡ ' + t('fastBadge', '快思考')))
      const content = h('div', {
        className: 'dslc-fence',
        'data-learn-card-fence': card.kind,
        style: S.fence,
        ref: function (node) {
          if (node === null || node === undefined) return
          let count = -1
          try {
            const doc = node.ownerDocument
            if (doc !== null && doc !== undefined && typeof doc.querySelectorAll === 'function') {
              count = doc.querySelectorAll('.dslc-card').length
            }
          } catch (error) { /* diagnostics */ }
          report('ref-attached', 'key=' + encodeURIComponent(card.kind) + ';cards=' + count)
        },
      }, [badge, body])
      if (FenceBoundary !== null) {
        return h(FenceBoundary, { fallback: children === undefined ? null : children }, content)
      }
      return content
    }

    /** Hoc layer for `lean` / `sokonanoda` fences: highlighted code, streaming-safe. */
    function LeanFence(props) {
      const children = isRecord(props) ? props.children : undefined
      if (isRecord(props) && props.streaming === true) return children === undefined ? null : children
      const code = isRecord(props) && typeof props.code === 'string' ? props.code : ''
      return h('div', { className: 'dslc-lean-wrap', style: S.leanWrap },
        h('pre', { className: 'dslc-lean', style: S.lean }, h('code', {}, highlightLean(code))))
    }

    const CLIENT_BUILD = '2026-11-24T1115'

    function apply(ctx) {
      /* A client update can land in an already-open page through the HMR carrier,
       * but markdown nodes mounted by the previous module instance keep their old
       * component. Bump CLIENT_BUILD when a release needs the page to reconcile;
       * one reload per build per tab, then normal loads are untouched. */
      try {
        const key = 'sokonanoda-client-build'
        if (typeof sessionStorage !== 'undefined' && sessionStorage.getItem(key) !== CLIENT_BUILD) {
          sessionStorage.setItem(key, CLIENT_BUILD)
          if (typeof location !== 'undefined' && typeof location.reload === 'function') {
            location.reload()
            return
          }
        }
      } catch (error) { /* storage unavailable: continue without the one-shot reload */ }
      report('apply', 'key=' + encodeURIComponent('probe=' + CLIENT_BUILD
        + ';comp=' + (typeof React.Component === 'function' ? '1' : '0')
        + ';st=' + (typeof setTimeout === 'function' ? '1' : '0')
        + ';pr=' + (typeof Promise === 'function' ? '1' : '0')
        + ';qm=' + (typeof queueMicrotask === 'function' ? '1' : '0')
        + ';raf=' + (typeof requestAnimationFrame === 'function' ? '1' : '0')
        + ';doc=' + (typeof document !== 'undefined' ? '1' : '0')))
      try {
        if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
          window.addEventListener('error', function (event) {
            report('window-error', 'message=' + encodeURIComponent(String(event && event.message ? event.message : event).slice(0, 180)))
          })
        }
      } catch (error) { /* diagnostics only */ }
      ctx.effect(function () {
        return ctx.locale.register(NS, { zh: zh, en: en })
      }, 'sokonanoda-dsh: dictionaries')
      /**
       * Register one language layer on the Chat Markdown fence hoc slot.
       * @param key - lowercased fence language.
       * @param id - stable layer id (hoc layers are addressed by id).
       * @param component - the layer component.
       * @param label - diagnostics label.
       * @param withLocale - attach the card locale namespace.
       */
      function fence(key, id, component, label, withLocale) {
        ctx.effect(function () {
          return ctx.slots.inject('conversation.chat.markdown.fence', function () {
            report('slot-ready', 'key=' + encodeURIComponent(key))
            const options = { name: 'conversation.chat.markdown.fence', key: key, id: id }
            if (withLocale === true) options.locale = NS
            const dispose = ctx.slots.register(options, component)
            report('fence-registered', 'key=' + encodeURIComponent(key))
            return dispose
          })
        }, label)
      }
      fence('learn-card', 'sokonanoda-dsh/fence/card', LearnCardFence, 'sokonanoda-dsh: card fence', true)
      fence('lean', 'sokonanoda-dsh/fence/lean', LeanFence, 'sokonanoda-dsh: lean fence')
      fence('sokonanoda', 'sokonanoda-dsh/fence/sokonanoda', LeanFence, 'sokonanoda-dsh: sokonanoda fence')
    }

    return {
      inject: ['slots', 'locale'],
      apply: apply,
      __test: {
        LearnCardFence: LearnCardFence,
        LeanFence: LeanFence,
        highlightLean: highlightLean,
        fenceCardOf: fenceCardOf,
        progressCard: progressCard,
        cardCss: CARD_CSS.join(''),
        zh: zh,
        en: en,
        NS: NS,
      },
    }
  },
})
