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

    let sidebarRight = null
    /* The owning Cordis context, kept so a source link can resolve the Sidebar
     * service even when the card was clicked before its late injection ran. */
    let hostCtx = null
    let sourcePreviewReady = false
    const SOURCE_PREVIEW_ID = 'sokonanoda-dsh/source'

    const NS = 'dshLearnCard'
    const KINDS = ['question', 'knowledge', 'lesson', 'progress', 'code']
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
      '.dslc-bold{font-weight:700}',
      '.dslc-options{display:flex;flex-direction:column;gap:6px;margin:0;padding:0;list-style:none;min-width:0}',
      '.dslc-opt{display:grid;grid-template-columns:26px minmax(0,1fr);align-items:start;column-gap:10px;width:100%;max-width:100%;min-width:0;min-height:48px;box-sizing:border-box;padding:10px 12px;border:1px solid var(--dsw-alias-border-l1,#e6e8ec);border-radius:10px;background:var(--dsw-alias-bg-layer-2,#f7f8fa);color:inherit;overflow-wrap:anywhere}',
      '.dslc-opt-skip{border-style:dashed;background:transparent}',
      '.dslc-opt-correct{border:1px solid var(--dsw-alias-state-success-primary,#0f7b4a);background:rgba(15,123,74,.10)}',
      '.dslc-review{display:flex;flex-direction:column;gap:4px;padding:8px 10px;border-radius:8px;background:rgba(127,127,127,.06)}',
      '.dslc-review-note{margin:0;font-size:12.5px;font-weight:600;color:var(--dsw-alias-state-success-primary,#0f7b4a)}',
      '.dslc-skip-options{margin-top:-4px}',
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
      '.dslc-map{margin:0;font-size:11.5px;line-height:1.5;color:var(--dsw-alias-label-secondary,#4b5563);overflow-wrap:anywhere}',
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
      '.dslc-code-hint{margin:0;padding:8px 10px;border-radius:8px;border:1px dashed var(--dsw-alias-border-l1,#e6e8ec);background:var(--dsw-alias-bg-base,rgba(127,127,127,0.06));color:var(--dsw-alias-label-secondary,#4b5563);font-size:12.5px;line-height:1.5;overflow-wrap:anywhere}',
      '.dslc-code-meta{margin:0;font-size:11.5px;line-height:1.45;color:var(--dsw-alias-label-tertiary,var(--dsw-alias-label-secondary,#6b7280));overflow-wrap:anywhere}',
      /* Read-only source preview: active cursor row + goal panel. */
      '.dsls-source-scroll:focus-visible{outline:2px solid var(--dsw-alias-state-business-primary,#0b57d0);outline-offset:-2px}',
      '.dsls-source-toolbar{display:flex;align-items:center;justify-content:space-between;gap:8px;flex:none;box-sizing:border-box;padding:3px 10px;border-bottom:1px solid var(--dsw-alias-border-l1,#e6e8ec);background:var(--dsw-alias-bg-layer-2,#f7f8fa);color:var(--dsw-alias-label-secondary,#4b5563);font-family:var(--dsw-font-family,inherit);font-size:11.5px;line-height:1.4}',
      '.dsls-source-meta{font-family:var(--dsw-font-mono,ui-monospace,monospace);font-size:11px;color:var(--dsw-alias-label-tertiary,var(--dsw-alias-label-secondary,#6b7280))}',
      '.dsls-source-copy{flex:none;padding:1px 8px;border:1px solid var(--dsw-alias-border-l2,#d5d8dd);border-radius:var(--dsw-radius-sm,6px);background:var(--dsw-alias-bg-base,#fff);color:var(--dsw-alias-label-primary,#15171a);font:inherit;cursor:pointer}',
      '.dsls-source-copy:disabled{cursor:default;color:var(--dsw-alias-label-tertiary,#6b7280)}',
      '.dsls-source-copy[data-copy-state="failed"]{color:var(--dsw-alias-state-error-primary,#c4553d);border-color:var(--dsw-alias-state-error-primary,#c4553d)}',
      '.dsls-source-empty{margin:0;padding:18px 12px;color:var(--dsw-alias-label-secondary,#4b5563);text-align:center;font-size:12.5px;line-height:1.6}',
      '.dsls-source-line[data-textpreview-target]{background:var(--dsw-alias-interactive-bg-hover-accent,rgba(11,87,208,.12))}',
      '.dsls-source-line[data-source-active="1"]{background:var(--dsw-alias-interactive-bg-hover,rgba(127,127,127,.14));box-shadow:inset 3px 0 0 var(--dsw-alias-state-business-primary,#0b57d0)}',
      '.dsls-source-line:hover{background:var(--dsw-alias-interactive-bg-hover,rgba(127,127,127,.08))}',
      '.dsls-source-panel{display:flex;flex-direction:column;gap:6px;flex:0 0 auto;max-height:44%;overflow-y:auto;box-sizing:border-box;padding:8px 12px 10px;border-top:1px solid var(--dsw-alias-border-l1,#e6e8ec);background:var(--dsw-alias-bg-layer-2,#f7f8fa);color:var(--dsw-alias-label-primary,#15171a);font-family:var(--dsw-font-family,inherit);font-size:12px;line-height:1.5}',
      '.dsls-state-head{display:flex;flex-wrap:wrap;align-items:center;gap:6px;min-width:0;font-size:11.5px;color:var(--dsw-alias-label-secondary,#4b5563)}',
      '.dsls-state-line{font-weight:700;color:var(--dsw-alias-label-primary,#15171a);font-variant-numeric:tabular-nums;white-space:nowrap}',
      '.dsls-state-decl{font-weight:600;color:var(--dsw-alias-label-primary,#15171a);overflow-wrap:anywhere}',
      '.dsls-state-chip{display:inline-flex;align-items:center;padding:0 6px;border-radius:999px;border:1px solid var(--dsw-alias-border-l1,#e6e8ec);background:var(--dsw-alias-bg-base,#fff);font-size:10.5px;line-height:16px;white-space:nowrap}',
      '.dsls-state-chip[data-status="checked"]{color:var(--dsw-alias-state-success-primary,#0f7b4a);border-color:var(--dsw-alias-state-success-primary,#0f7b4a)}',
      '.dsls-state-chip[data-status="failed"]{color:var(--dsw-alias-state-error-primary,#c4553d);border-color:var(--dsw-alias-state-error-primary,#c4553d)}',
      '.dsls-state-chip[data-status="open"]{color:var(--dsw-alias-state-warn-primary,#9a5b00);border-color:var(--dsw-alias-state-warn-primary,#9a5b00)}',
      '.dsls-state-updating{color:var(--dsw-alias-label-tertiary,#6b7280);white-space:nowrap}',
      '.dsls-state-body{display:flex;flex-direction:column;gap:5px;min-width:0}',
      '.dsls-state-hint,.dsls-state-note{margin:0;color:var(--dsw-alias-label-secondary,#4b5563);overflow-wrap:anywhere}',
      '.dsls-state-note[data-kind="closed"]{color:var(--dsw-alias-state-success-primary,#0f7b4a);font-weight:600}',
      '.dsls-state-note[data-kind="error"]{color:var(--dsw-alias-state-error-primary,#c4553d)}',
      '.dsls-goal{display:flex;flex-direction:column;gap:2px;min-width:0;padding:5px 0}',
      '.dsls-goal+.dsls-goal{border-top:1px dashed var(--dsw-alias-border-l1,#e6e8ec)}',
      '.dsls-goal-head{margin:0;font-size:10.5px;font-weight:700;color:var(--dsw-alias-label-tertiary,#6b7280)}',
      '.dsls-hyp{display:flex;gap:5px;min-width:0;align-items:baseline;overflow-wrap:anywhere}',
      '.dsls-hyp-name{flex:none;font-weight:600;color:var(--dsw-alias-label-primary,#15171a)}',
      '.dsls-hyp-ty{min-width:0;font-family:var(--dsw-font-mono,ui-monospace,monospace);font-size:12px;overflow-wrap:anywhere}',
      '.dsls-goal-text{display:flex;gap:5px;min-width:0;align-items:baseline}',
      '.dsls-turnstile{flex:none;color:var(--dsw-alias-label-secondary,#4b5563)}',
      '.dsls-goal-ty{min-width:0;font-family:var(--dsw-font-mono,ui-monospace,monospace);font-size:12px;font-weight:600;overflow-wrap:anywhere}',
      '.dsls-run-name{color:var(--dsw-alias-state-business-primary,#0b57d0);font-weight:600}',
      '.dsls-run-use{color:var(--dsw-alias-state-business-primary,#0b57d0)}',
      '.dsls-run-binder{color:var(--dsw-alias-label-primary,#15171a)}',
      '.dsls-run-sort{color:var(--dsw-alias-label-primary,#15171a);font-weight:600}',
      '.dsls-run-hole{color:var(--dsw-alias-state-warn-primary,#9a5b00);font-weight:600}',
      '.dsls-run-num{color:var(--dsw-alias-label-secondary,#4b5563)}',
      '.dsls-run-keyword{color:var(--dsw-alias-state-business-primary,#0b57d0);font-weight:600}',
      '.dsls-run-unknown{color:var(--dsw-alias-state-warn-primary,#9a5b00)}',
      '.dsls-state-diag{margin:0;color:var(--dsw-alias-state-error-primary,#c4553d);overflow-wrap:anywhere}',
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
      progressMapTitle: '学习地图',
      progressMapSolved: '已解',
      progressMapTodo: '待做',
      progressMapWeak: '薄弱',
      progressMapCode: '代码',
      fastBadge: '快思考',
      openAnswerHint: '直接在输入框写下你的回答，不用选字母。',
      codeLabel: '代码练习',
      codeTemplateLabel: '模板',
      codeSubmitHint: '把完整声明贴回来（从 theorem/def 那行到证明结束），我用真内核判卷；想放弃回「跳过这道代码题」。',
      dontKnowLabel: '我不知道',
      whyLabel: '解析',
      stateLine: '第 {line} 行',
      stateNoLine: '未选择行',
      stateIdleHint: '↑/↓ 移动光标行，点击任意行查看该位置的 goal',
      stateLoading: '正在读取内核状态…',
      stateUpdating: '更新中…',
      stateOutside: '光标不在任何声明内',
      stateFileErrors: '源码有错误',
      stateClosed: '已无目标（证明完成）',
      stateNoGoals: '当前没有目标',
      stateError: '状态后端不可用',
      stateStep: '步骤',
      stateGoal: '目标',
      stateStatusChecked: '已检查',
      stateStatusOpen: '待证明',
      stateStatusFailed: '失败',
      stateAria: '只读源码预览：上下方向键移动光标行并显示该位置的 goal',
      stateLoadMore: '文件较长：先滚动到底部加载更多行',
      sourceCopy: '复制',
      sourceCopied: '已复制',
      sourceCopyFailed: '复制失败',
      sourceEmpty: '空文件（0 行）',
      sourceToolbarLabel: 'Lean · 只读',
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
      progressMapTitle: 'Learning map',
      progressMapSolved: 'solved',
      progressMapTodo: 'todo',
      progressMapWeak: 'weak',
      progressMapCode: 'code',
      fastBadge: 'fast path',
      openAnswerHint: 'Type your answer in the composer — no letter needed.',
      codeLabel: 'Code exercise',
      codeTemplateLabel: 'Template',
      codeSubmitHint: 'Paste the full declaration back (from the theorem/def line to the end of the proof); the real kernel grades it. Say 「跳过这道代码题」 to skip.',
      dontKnowLabel: "I don't know",
      whyLabel: 'Why',
      stateLine: 'Line {line}',
      stateNoLine: 'No line',
      stateIdleHint: 'Use ↑/↓ to move the cursor line; click any line to show its goal',
      stateLoading: 'Reading kernel state…',
      stateUpdating: 'Updating…',
      stateOutside: 'The cursor is outside every declaration',
      stateFileErrors: 'The source has errors',
      stateClosed: 'No goals left (proof complete)',
      stateNoGoals: 'No goals here',
      stateError: 'Goal state backend unavailable',
      stateStep: 'Step',
      stateGoal: 'Goal',
      stateStatusChecked: 'checked',
      stateStatusOpen: 'open',
      stateStatusFailed: 'failed',
      stateAria: 'Read-only source preview: arrow keys move the cursor line and show its goal',
      stateLoadMore: 'Long file: scroll to the bottom to load more lines',
      sourceCopy: 'Copy',
      sourceCopied: 'Copied',
      sourceCopyFailed: 'Copy failed',
      sourceEmpty: 'Empty file (0 lines)',
      sourceToolbarLabel: 'Lean · read-only',
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

    /** Encode a path for a dsh-resource://file/... address. */
    function encodeResourcePath(value) {
      return String(value || '').replace(/\\/g, '/').replace(/^\/+/, '')
        .split('/').filter((segment) => segment !== '').map(encodeURIComponent).join('/')
    }

    /**
     * Read the Sidebar service, retrying through the owning context. A plugin
     * can apply before the Sidebar's client service exists; the late `inject`
     * in {@link apply} normally fills it, and this covers a click in between.
     * @returns the service, or null when this host has none.
     */
    function resolveSidebarRight() {
      if (sidebarRight !== null) return sidebarRight
      if (hostCtx === null) return null
      try {
        const service = hostCtx.sidebarRight !== undefined
          ? hostCtx.sidebarRight
          : (typeof hostCtx.get === 'function' ? hostCtx.get('sidebarRight') : undefined)
        if (service !== undefined && service !== null) sidebarRight = service
      } catch (error) { /* service not available in this host */ }
      return sidebarRight
    }

    /** Build the sidebar address for one source link, preferring the session scope. */
    function sourceAddress(link) {
      if (link === null || typeof link !== 'object') return ''
      let sessionId = ''
      const service = resolveSidebarRight()
      try {
        if (service !== null && service.mounted !== undefined && typeof service.mounted.getSnapshot === 'function') {
          sessionId = service.mounted.getSnapshot() || ''
        }
      } catch (error) { sessionId = '' }
      if (sessionId !== '' && typeof link.relativePath === 'string' && link.relativePath !== '') {
        return 'dsh-resource://file/session/' + encodeURIComponent(sessionId) + '/' + encodeResourcePath(link.relativePath)
      }
      if (typeof link.path === 'string' && link.path !== '') {
        return 'dsh-resource://file/absolute/' + encodeResourcePath(link.path)
      }
      return ''
    }

    /** Open a card source in the right-sidebar file viewer at its line. */
    function openSourceLink(link) {
      if (link === null || typeof link !== 'object') return
      const address = sourceAddress(link)
      if (address === '') return
      const service = resolveSidebarRight()
      try {
        if (service !== null && typeof service.openResource === 'function') {
          /* The file resource params are `{ line }` (SidebarRightResourceParamsMap
           * `file: WorkspaceFileParams`); a nested `{ file: { line } }` is ignored. */
          const line = Number(link.line)
          const params = Number.isFinite(line) && line > 0 ? { line } : undefined
          if (sourcePreviewReady) {
            try {
              service.openResource(address, { kind: SOURCE_PREVIEW_ID, params })
              return
            } catch (error) { /* fall back to the stock preview type */ }
          }
          service.openResource(address, params === undefined ? {} : { params })
          return
        }
      } catch (error) {
        report('source-open-error', 'message=' + encodeURIComponent(String(error && error.message ? error.message : error).slice(0, 200)))
      }
      report('source-open-missing', 'key=' + encodeURIComponent(address.slice(0, 120)))
    }

        function sourceOf(raw, link) {
      const sourceLink = link !== null && typeof link === 'object' && typeof link.path === 'string' ? link : null
      const address = sourceLink === null ? '' : sourceAddress(sourceLink)
      if (typeof raw === 'string' && raw.trim() !== '') {
        return { label: raw.trim(), url: address, link: sourceLink }
      }
      if (isRecord(raw)) {
        const label = textOf(raw.label) !== '' ? textOf(raw.label) : (textOf(raw.text) !== '' ? textOf(raw.text) : textOf(raw.title))
        const rawUrl = /^https?:\/\//i.test(textOf(raw.url)) ? textOf(raw.url) : ''
        const url = rawUrl !== '' ? rawUrl : address
        if (label !== '' || url !== '') return { label: label !== '' ? label : url, url, link: sourceLink }
      }
      if (sourceLink !== null && address !== '') return { label: textOf(sourceLink.path), url: address, link: sourceLink }
      return null
    }

    /** Split `inline code` spans into React text and `<code>` nodes. */
    function inlineNodes(text) {
      const value = typeof text === 'string' ? text : ''
      if (value === '') return []
      /* Card data (questions/cards/lessons) uses inline markdown: `code` and
       * **bold**. Parse both here; a bare `**` pair must become <strong>, not
       * leak literal asterisks into the card. */
      const pattern = /(`[^`]*`|\*\*[\s\S]*?\*\*)/g
      const nodes = []
      let last = 0
      let index = 0
      let match = pattern.exec(value)
      while (match !== null) {
        if (match.index > last) nodes.push(value.slice(last, match.index))
        const token = match[0]
        if (token.length >= 2 && token.startsWith('`') && token.endsWith('`')) {
          nodes.push(h('code', { className: 'dslc-code', key: 'code-' + index, style: S.code }, token.slice(1, -1)))
        } else {
          const inner = token.slice(2, -2)
          nodes.push(h('strong', { className: 'dslc-bold', key: 'bold-' + index, style: S.bold }, inlineNodes(inner)))
        }
        index += 1
        last = match.index + token.length
        match = pattern.exec(value)
      }
      if (last < value.length) nodes.push(value.slice(last))
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
      bold: { fontWeight: 700 },
      list: { display: 'flex', flexDirection: 'column', gap: '6px', margin: 0, padding: 0, listStyle: 'none' },
      option: { display: 'grid', gridTemplateColumns: '26px minmax(0,1fr)', alignItems: 'start', columnGap: '10px', width: '100%', maxWidth: '100%', minWidth: 0, minHeight: '48px', boxSizing: 'border-box', padding: '10px 12px', border: '1px solid rgba(127,127,127,0.28)', borderRadius: '10px', background: 'rgba(127,127,127,0.08)', overflowWrap: 'anywhere' },
      optionSkip: { borderStyle: 'dashed', background: 'transparent' },
      optionCorrect: { border: '1px solid var(--dsw-alias-state-success-primary,#0f7b4a)', background: 'rgba(15,123,74,0.10)' },
      review: { display: 'flex', flexDirection: 'column', gap: '4px', padding: '8px 10px', borderRadius: '8px', background: 'rgba(127,127,127,0.06)' },
      reviewNote: { margin: 0, fontSize: '12.5px', fontWeight: 600, color: 'var(--dsw-alias-state-success-primary,#0f7b4a)' },
      optionSkipMuted: { opacity: 0.55 },
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
      sourceRoot: { display: 'flex', flexDirection: 'column', width: '100%', height: '100%', minHeight: 0, background: 'var(--dsw-alias-bg-base,#fff)', color: 'var(--dsw-alias-label-primary,#15171a)' },
      sourceToolbar: { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px', flex: '0 0 auto', boxSizing: 'border-box', padding: '3px 10px', borderBottom: '1px solid var(--dsw-alias-border-l1,#e6e8ec)', background: 'var(--dsw-alias-bg-layer-2,#f7f8fa)', color: 'var(--dsw-alias-label-secondary,#4b5563)', fontSize: '11.5px', lineHeight: 1.4 },
      sourceMeta: { fontFamily: 'var(--dsw-font-mono,ui-monospace,monospace)', fontSize: '11px', color: 'var(--dsw-alias-label-tertiary,var(--dsw-alias-label-secondary,#6b7280))' },
      sourceCopy: { flex: '0 0 auto', padding: '1px 8px', border: '1px solid var(--dsw-alias-border-l2,#d5d8dd)', borderRadius: '6px', background: 'var(--dsw-alias-bg-base,#fff)', color: 'var(--dsw-alias-label-primary,#15171a)', font: 'inherit', cursor: 'pointer' },
      sourceCopyDisabled: { cursor: 'default', opacity: 0.6 },
      sourceEmpty: { margin: 0, padding: '18px 12px', color: 'var(--dsw-alias-label-secondary,#4b5563)', textAlign: 'center', fontSize: '12.5px', lineHeight: 1.6 },
      sourceScroll: { flex: 1, minHeight: 0, position: 'relative', overflowY: 'auto', font: '12.5px/1.7 ui-monospace,SFMono-Regular,Menlo,monospace' },
      sourceLines: { display: 'flex', flexDirection: 'column', minWidth: 'max-content', padding: '8px 0' },
      sourceLine: { display: 'flex', alignItems: 'flex-start', minHeight: '1.7em' },
      sourceLineFlash: { background: 'rgba(255,206,84,0.25)' },
      sourceNo: { flex: '0 0 auto', boxSizing: 'border-box', padding: '0 12px 0 10px', textAlign: 'right', color: 'var(--dsw-alias-label-tertiary,var(--dsw-alias-label-secondary,#9aa0a6))', userSelect: 'none' },
      sourceText: { flex: '1 1 auto', minWidth: 0, paddingRight: '12px', whiteSpace: 'inherit' },
      hlKw: { color: 'var(--dsw-alias-state-business-primary,#0b57d0)', fontWeight: 600 },
      hlTactic: { color: 'var(--dsw-alias-state-warn-primary,#9a5b00)' },
      hlType: { color: 'var(--dsw-alias-label-primary,#15171a)', fontWeight: 600 },
      hlStr: { color: 'var(--dsw-alias-state-success-primary,#0f7b4a)' },
      hlCom: { color: 'var(--dsw-alias-label-tertiary,#6b7280)', fontStyle: 'italic' },
      hlNum: { color: 'var(--dsw-alias-label-secondary,#4b5563)' },
      hlOp: { color: 'var(--dsw-alias-label-secondary,#4b5563)' },
      sourcePanel: { display: 'flex', flexDirection: 'column', gap: '6px', flex: '0 0 auto', maxHeight: '44%', overflowY: 'auto', boxSizing: 'border-box', padding: '8px 12px 10px', borderTop: '1px solid var(--dsw-alias-border-l1,#e6e8ec)', background: 'var(--dsw-alias-bg-layer-2,#f7f8fa)' },
      sourcePanelHead: { display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: '6px', minWidth: 0, fontSize: '11.5px', color: 'var(--dsw-alias-label-secondary,#4b5563)' },
      sourcePanelBody: { display: 'flex', flexDirection: 'column', gap: '5px', minWidth: 0 },
      sourcePanelLineStyle: { fontWeight: 700, color: 'var(--dsw-alias-label-primary,#15171a)', fontVariantNumeric: 'tabular-nums', whiteSpace: 'nowrap' },
      sourcePanelDecl: { fontWeight: 600, color: 'var(--dsw-alias-label-primary,#15171a)', overflowWrap: 'anywhere' },
      sourceGoalHead: { margin: 0, fontSize: '10.5px', fontWeight: 700, color: 'var(--dsw-alias-label-tertiary,#6b7280)' },
      sourceHint: { margin: 0, color: 'var(--dsw-alias-label-secondary,#4b5563)', overflowWrap: 'anywhere' },
      sourceGoal: { display: 'flex', flexDirection: 'column', gap: '2px', minWidth: 0, padding: '5px 0' },
      sourceHyp: { display: 'flex', gap: '5px', minWidth: 0, alignItems: 'baseline', overflowWrap: 'anywhere' },
      sourceHypName: { flex: 'none', fontWeight: 600 },
      sourceHypTy: { minWidth: 0, fontFamily: 'var(--dsw-font-mono,ui-monospace,monospace)', fontSize: '12px', overflowWrap: 'anywhere' },
      sourceGoalText: { display: 'flex', gap: '5px', minWidth: 0, alignItems: 'baseline' },
      sourceTurnstile: { flex: 'none', color: 'var(--dsw-alias-label-secondary,#4b5563)' },
      sourceGoalTy: { minWidth: 0, fontFamily: 'var(--dsw-font-mono,ui-monospace,monospace)', fontSize: '12px', fontWeight: 600, overflowWrap: 'anywhere' },
      sourceDiag: { margin: 0, color: 'var(--dsw-alias-state-error-primary,#c4553d)', overflowWrap: 'anywhere' },
      progressHead: { display: 'flex', flexDirection: 'column', gap: '4px' },
      statsGrid: { display: 'grid', gridTemplateColumns: 'repeat(4, minmax(0,1fr))', gap: '8px' },
      stat: { display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '2px', padding: '8px 4px', border: '1px solid rgba(127,127,127,0.25)', borderRadius: '10px', background: 'rgba(127,127,127,0.06)', minWidth: 0 },
      statValue: { fontSize: '17px', fontWeight: 700, lineHeight: 1.1 },
      statLabel: { fontSize: '10.5px', color: 'var(--dsw-alias-label-secondary,#4b5563)', whiteSpace: 'nowrap' },
      mapLine: { margin: 0, fontSize: '11.5px', lineHeight: 1.5, color: 'var(--dsw-alias-label-secondary,#4b5563)', overflowWrap: 'anywhere' },
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
      codeHint: { margin: 0, padding: '8px 10px', borderRadius: '8px', border: '1px dashed rgba(127,127,127,0.32)', background: 'rgba(127,127,127,0.06)', color: 'var(--dsw-alias-label-secondary,#4b5563)', fontSize: '12.5px', lineHeight: 1.5, overflowWrap: 'anywhere' },
      codeMeta: { margin: 0, fontSize: '11.5px', lineHeight: 1.45, color: 'var(--dsw-alias-label-tertiary,var(--dsw-alias-label-secondary,#6b7280))', overflowWrap: 'anywhere' },
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

    function questionCard(payload, onPick, picked, t, onSkip) {
      const stem = textOf(payload.stem) !== '' ? textOf(payload.stem) : textOf(payload.prompt)
      const plain = textOf(payload.plain)
      const options = normalizeOptions(payload.options)
      const questionKind = textOf(payload.questionKind)
      const review = payload.review === true
      const correctKey = textOf(payload.correctKey)
      const reviewWhy = textOf(payload.why)
      const open = options.length === 0 && questionKind !== '' && questionKind !== 'choice'
      const clickable = !review && typeof onPick === 'function'
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
            const active = clickable && picked === null && !review
            const isCorrect = review && correctKey !== '' && option.key === correctKey
            return h('li', {
              className: isCorrect ? 'dslc-opt dslc-opt-correct' : 'dslc-opt',
              key: 'option-' + index,
              'data-letter': option.key,
              'data-pickable': active ? '1' : undefined,
              'data-picked': picked === option.key ? '1' : undefined,
              role: active ? 'button' : undefined,
              tabIndex: active ? 0 : undefined,
              'aria-label': active ? '发送选项 ' + option.key : undefined,
              style: isCorrect
                ? Object.assign({}, S.option, S.optionCorrect)
                : picked === option.key
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
        options.length > 0 && !review && typeof onSkip === 'function'
          ? h('ul', { className: 'dslc-options dslc-skip-options', key: 'skip', style: S.list }, [
            h('li', {
              className: 'dslc-opt dslc-opt-skip',
              key: 'skip-row',
              'data-letter': '?',
              'data-pickable': picked === null ? '1' : undefined,
              role: picked === null ? 'button' : undefined,
              tabIndex: picked === null ? 0 : undefined,
              'aria-label': '我不知道，显示知识卡',
              style: picked === null
                ? Object.assign({}, S.option, S.optionSkip)
                : Object.assign({}, S.option, S.optionSkip, S.optionSkipMuted),
              onClick: picked === null ? () => onSkip() : undefined,
              onKeyDown: picked === null ? (event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  onSkip()
                }
              } : undefined,
            },
            h('span', { className: 'dslc-letter', style: S.letter }, '?'),
            h('span', { className: 'dslc-opttext', style: S.opttext },
              t !== undefined && typeof t === 'function' ? t('dontKnowLabel', '我不知道') : '我不知道')),
          ])
          : null,
        review
          ? h('div', { className: 'dslc-review', key: 'review', style: S.review },
            h('p', { className: 'dslc-review-note', style: S.reviewNote },
              '回看 · ' + (correctKey !== '' ? '正确答案 ' + correctKey : '开放题') + '（不计分）'),
            reviewWhy !== ''
              ? h('p', { className: 'dslc-plain', style: S.plain },
                (t !== undefined && typeof t === 'function' ? t('whyLabel', '解析') : '解析') + '：',
                inlineNodes(reviewWhy))
              : null)
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
      const source = sourceOf(payload.source, payload.sourceLink)
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
              ? (source.link !== null && source.link !== undefined
                ? h('a', {
                  className: 'dslc-link',
                  href: source.url,
                  title: source.link.line !== undefined
                    ? '在右侧栏打开并定位到第 ' + source.link.line + ' 行'
                    : '在右侧栏打开',
                  onClick: (event) => {
                    if (typeof event.preventDefault === 'function') event.preventDefault()
                    openSourceLink(source.link)
                  },
                }, source.label)
                : h('a', { className: 'dslc-link', href: source.url }, source.label))
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
      /* C9.5 学习地图：与 engine.py map 同一份结构计数（已解/待做/薄弱 + 代码题）。 */
      const showMap = Number(summary.kpsTotal) > 0
      const mapParts = [
        t('progressMapSolved', '已解') + ' ' + (Number(summary.solvedKps) || 0),
        t('progressMapTodo', '待做') + ' ' + (Number(summary.todoKps) || 0),
        t('progressMapWeak', '薄弱') + ' ' + (Number(summary.weakKps) || 0),
      ]
      if (Number(summary.codeTotal) > 0) {
        mapParts.push(t('progressMapCode', '代码') + ' ' + (Number(summary.codeSolved) || 0)
          + '/' + Number(summary.codeTotal))
      }
      const mapLine = showMap
        ? h('p', { className: 'dslc-map', key: 'map', style: S.mapLine },
          t('progressMapTitle', '学习地图') + '：' + mapParts.join(' · '))
        : null
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
        mapLine,
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

    /** 代码练习卡：题干 + 提示 + 可信模板（带 sorry）；提交仍在输入框里贴围栏代码。 */
    function codeCard(t, payload) {
      const title = textOf(payload.title)
      const prompt = textOf(payload.prompt)
      const chapter = [textOf(payload.chapter), textOf(payload.unit)].filter((part) => part !== '').join(' · ')
      const template = textOf(payload.template)
      const hints = (Array.isArray(payload.hints) ? payload.hints : []).slice(0, 4).map((hint) => (
        typeof hint === 'string' ? { label: '', body: hint } : hint
      )).filter(isRecord)
      const submitHint = textOf(payload.submitHint)
        || (t !== undefined && typeof t === 'function' ? t('codeSubmitHint', '') : '')
      return cardShell('code', [
        chapter !== ''
          ? h('p', { className: 'dslc-code-meta', key: 'meta', style: S.codeMeta },
            '🧩 ' + (t !== undefined && typeof t === 'function' ? t('codeLabel', '代码练习') : '代码练习') + ' · ' + chapter)
          : null,
        title !== '' ? h('p', { className: 'dslc-heading', key: 'title', style: S.heading }, inlineNodes(title)) : null,
        prompt !== '' ? h('p', { className: 'dslc-plain', key: 'prompt', style: S.plain }, inlineNodes(prompt)) : null,
        hints.length > 0
          ? section(t, 'hints', t !== undefined && typeof t === 'function' ? t('pointsLabel', '提示') : '提示',
            h('ul', { className: 'dslc-slist', style: S.slist }, hints.map((hint, index) => {
              const label = textOf(hint.label)
              const body = textOf(hint.body)
              return h('li', { key: 'hint-' + index }, (label !== '' ? label + '：' : '') + body)
            })))
          : null,
        template !== ''
          ? section(t, 'template', t !== undefined && typeof t === 'function' ? t('codeTemplateLabel', '模板') : '模板',
            h('pre', { className: 'dslc-def', key: 'template', style: S.def },
              h('code', {}, highlightLean(template))))
          : null,
        submitHint !== ''
          ? h('p', { className: 'dslc-code-hint', key: 'submit-hint', style: S.codeHint }, submitHint)
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
      const isReviewCard = isRecord(card.payload) && card.payload.review === true
      const canSend = card.kind === 'question' && !isReviewCard && picked === null
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
      /* "我不知道" is a support exit, not an answer: send the phrase so the
       * fast path returns a knowledge card; do not mark any letter as picked. */
      const onSkip = canSend
        ? () => {
          try {
            inputActions.setDraft('我不知道')
            inputActions.submit()
          } catch (error) { /* composer unavailable: stay display-only */ }
        }
        : null
      let body
      try {
        body = card.kind === 'question' ? questionCard(card.payload, onPick, picked, t, onSkip)
          : card.kind === 'knowledge' ? knowledgeCard(t, card.payload)
            : card.kind === 'lesson' ? lessonCard(t, card.payload)
              : card.kind === 'progress' ? progressCard(t, card.payload)
                : card.kind === 'code' ? codeCard(t, card.payload)
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

    /* ------------------------------------------------- right-sidebar source body
     * `.sokonanoda` is not in DSH's built-in code-highlight extension list, so the
     * stock preview falls back to plain text (no gutter, no Lean highlighting).
     * This document-preview body gives the extension a real code view: line
     * numbers, the fence highlighter, wrapping, source-line navigation, and a
     * read-only cursor: ↑/↓ (or a click) picks the active line and the panel
     * below shows the kernel goal at that exact position. */

    /** Semantic run kind → presentation class (the run text is never re-tokenized). */
    const RUN_CLASSES = {
      keyword: 'dsls-run-keyword',
      sort: 'dsls-run-sort',
      number: 'dsls-run-num',
      hole: 'dsls-run-hole',
      def_name: 'dsls-run-name',
      theorem_name: 'dsls-run-name',
      axiom_name: 'dsls-run-name',
      inductive_name: 'dsls-run-name',
      ctor_name: 'dsls-run-name',
      def_use: 'dsls-run-use',
      theorem_use: 'dsls-run-use',
      axiom_use: 'dsls-run-use',
      inductive_use: 'dsls-run-use',
      ctor_use: 'dsls-run-use',
      binder: 'dsls-run-binder',
      unknown_ident: 'dsls-run-unknown',
    }

    /**
     * Render kernel semantic runs in place of plain goal/hypothesis text. The
     * runs must concatenate back to the field text byte-for-byte; otherwise the
     * plain text wins, so a drifted wire shape can never garble a goal.
     * @param runs - `goal_runs` / `ty_runs` from `soko/stateAt`.
     * @param text - the field's own text.
     * @returns React nodes or the original text.
     */
    function semanticRuns(runs, text) {
      const value = typeof text === 'string' ? text : ''
      if (value === '' || !Array.isArray(runs) || runs.length === 0) return value
      let joined = ''
      for (const run of runs) {
        if (!isRecord(run) || typeof run.text !== 'string') return value
        joined += run.text
      }
      if (joined !== value) return value
      const nodes = []
      for (let index = 0; index < runs.length; index++) {
        const run = runs[index]
        if (run.text === '') continue
        const className = RUN_CLASSES[run.kind]
        nodes.push(className === undefined
          ? run.text
          : h('span', { className: className, key: 'run-' + index, 'data-run-kind': run.kind }, run.text))
      }
      return nodes.length === 0 ? value : nodes
    }

    /** Localized label for a declaration status (`checked` / `open` / `failed`). */
    function statusLabelOf(t, status) {
      if (status === 'checked') return t('stateStatusChecked', '已检查')
      if (status === 'open') return t('stateStatusOpen', '待证明')
      if (status === 'failed') return t('stateStatusFailed', '失败')
      return typeof status === 'string' && status !== '' ? status : ''
    }

    /** Goals as a list; older `goal`/`binders` fields are the single-goal fallback. */
    function goalsOf(state) {
      if (!isRecord(state)) return []
      if (Array.isArray(state.goals)) return state.goals.filter(isRecord)
      if (typeof state.goal === 'string' && state.goal !== '') {
        return [{ goal: state.goal, goal_runs: state.goal_runs, binders: state.binders }]
      }
      return []
    }

    /**
     * Read-only goal panel: declaration name/kind/status, step/total, hypotheses
     * and the goal, all rendered from kernel semantic runs.
     * @param props - `line`, `phase`, last `state`, `error`, diagnostics and `t`.
     * @returns the panel element.
     */
    function SourceGoalPanel(props) {
      const input = isRecord(props) ? props : {}
      const rawT = typeof input.t === 'function' ? input.t : null
      const t = function (key, fallback, vars) {
        let text = fallback
        if (rawT !== null) {
          let value
          try {
            value = vars === undefined ? rawT(key) : rawT(key, vars)
          } catch (error) {
            value = undefined
          }
          if (typeof value === 'string' && value !== '' && value !== key) text = value
        }
        if (isRecord(vars)) {
          text = String(text).replace(/\{(\w+)\}/g, function (whole, name) {
            return vars[name] === undefined ? whole : String(vars[name])
          })
        }
        return text
      }
      const line = Number.isFinite(Number(input.line)) ? Number(input.line) : 0
      const phase = typeof input.phase === 'string' ? input.phase : 'idle'
      const state = isRecord(input.state) ? input.state : null
      const error = isRecord(input.error) ? input.error : null
      const diagnostics = Array.isArray(input.diagnostics) ? input.diagnostics.filter(isRecord) : []
      const decl = state !== null && isRecord(state.decl) ? state.decl : null
      const goals = goalsOf(state)
      const step = state !== null ? Number(state.step) : NaN
      const total = state !== null ? Number(state.total) : NaN
      const stepText = Number.isInteger(step) && Number.isInteger(total) && total > 0
        ? t('stateStep', '步骤') + ' ' + step + '/' + total
        : ''
      const updating = phase === 'loading' && state !== null
      const status = decl !== null && typeof decl.status === 'string' ? decl.status : ''
      const statusLabel = statusLabelOf(t, status)
      const head = h('div', { className: 'dsls-state-head', key: 'state-head', style: S.sourcePanelHead },
        h('span', { className: 'dsls-state-line', style: S.sourcePanelLineStyle },
          line > 0 ? t('stateLine', '第 {line} 行', { line }) : t('stateNoLine', '未选择行')),
        decl !== null && typeof decl.name === 'string' && decl.name !== ''
          ? h('span', { className: 'dsls-state-decl', key: 'decl', style: S.sourcePanelDecl }, decl.name)
          : null,
        decl !== null && typeof decl.kind === 'string' && decl.kind !== ''
          ? h('span', { className: 'dsls-state-chip', key: 'kind', 'data-kind': decl.kind }, decl.kind)
          : null,
        statusLabel !== ''
          ? h('span', {
            className: 'dsls-state-chip',
            key: 'status',
            'data-status': status,
            'data-state-status': status,
          }, statusLabel)
          : null,
        stepText !== ''
          ? h('span', { className: 'dsls-state-step', key: 'step' }, stepText)
          : null,
        updating
          ? h('span', { className: 'dsls-state-updating', key: 'updating' }, t('stateUpdating', '更新中…'))
          : null)
      let body = null
      if (phase === 'idle') {
        body = [h('p', { className: 'dsls-state-hint', key: 'hint', style: S.sourceHint },
          t('stateIdleHint', '↑/↓ 移动光标行，点击任意行查看该位置的 goal'))]
      } else if (phase === 'loading' && state === null) {
        body = [h('p', { className: 'dsls-state-note', key: 'loading', style: S.sourceHint }, t('stateLoading', '正在读取内核状态…'))]
      } else if (phase === 'outside') {
        body = [h('p', { className: 'dsls-state-note', key: 'outside', style: S.sourceHint }, t('stateOutside', '光标不在任何声明内'))]
      } else if (phase === 'file-errors') {
        body = [
          h('p', { className: 'dsls-state-note', key: 'file-errors', 'data-kind': 'error', style: S.sourceHint },
            (error !== null && typeof error.message === 'string' && error.message !== '')
              ? error.message
              : t('stateFileErrors', '源码有错误')),
          diagnostics.map(function (item, index) {
            const where = Number(item.line) > 0 ? '第 ' + Number(item.line) + ' 行：' : ''
            return h('p', { className: 'dsls-state-diag', key: 'diag-' + index, style: S.sourceDiag },
              where + String(item.message || ''))
          }),
        ]
      } else if (phase === 'error') {
        const message = error !== null && typeof error.message === 'string' && error.message !== ''
          ? error.message
          : t('stateError', '状态后端不可用')
        body = [h('p', { className: 'dsls-state-note', key: 'error', 'data-kind': 'error', style: S.sourceHint }, message)]
      } else if (state === null) {
        body = [h('p', { className: 'dsls-state-hint', key: 'hint', style: S.sourceHint },
          t('stateIdleHint', '↑/↓ 移动光标行，点击任意行查看该位置的 goal'))]
      } else if (goals.length === 0) {
        const closed = status === 'checked'
        body = [h('p', {
          className: 'dsls-state-note',
          key: 'closed',
          'data-kind': closed ? 'closed' : 'empty',
          style: S.sourceHint,
        }, closed ? t('stateClosed', '已无目标（证明完成）') : t('stateNoGoals', '当前没有目标'))]
      } else {
        body = goals.map(function (goal, index) {
          const binders = Array.isArray(goal.binders) ? goal.binders.filter(isRecord) : []
          const label = goals.length > 1
            ? h('p', { className: 'dsls-goal-head', key: 'goal-head', style: S.sourceGoalHead },
              t('stateGoal', '目标') + ' ' + (index + 1) + '/' + goals.length)
            : null
          const hyps = binders.length > 0
            ? h('div', { className: 'dsls-hyps', key: 'hyps' },
              binders.map(function (binder, binderIndex) {
                return h('div', { className: 'dsls-hyp', key: 'hyp-' + binderIndex, style: S.sourceHyp },
                  h('span', { className: 'dsls-hyp-name', style: S.sourceHypName }, String(binder.name || '')),
                  h('span', { className: 'dsls-hyp-sep' }, ' : '),
                  h('span', { className: 'dsls-hyp-ty', style: S.sourceHypTy },
                    semanticRuns(binder.ty_runs, binder.ty)))
              }))
            : null
          return h('div', { className: 'dsls-goal', key: 'goal-' + index, 'data-goal-index': index, style: S.sourceGoal },
            label,
            hyps,
            h('div', { className: 'dsls-goal-text', key: 'goal-text', style: S.sourceGoalText },
              h('span', { className: 'dsls-turnstile', style: S.sourceTurnstile }, '⊢ '),
              h('span', { className: 'dsls-goal-ty', style: S.sourceGoalTy },
                semanticRuns(goal.goal_runs, goal.goal))))
        })
      }
      if (input.loadHint === true) {
        body = (Array.isArray(body) ? body : [body]).concat([
          h('p', { className: 'dsls-state-hint', key: 'load-more', style: S.sourceHint },
            t('stateLoadMore', '文件较长：先滚动到底部加载更多行')),
        ])
      }
      return h('div', {
        className: 'dsls-source-panel dsls-state-panel',
        style: S.sourcePanel,
        'data-state-panel': phase,
        'data-line': line > 0 ? line : undefined,
      }, [
        head,
        h('div', { className: 'dsls-state-body', key: 'state-body', style: S.sourcePanelBody }, body),
      ])
    }

    /**
     * One loaded page's source lines. A zero-line page is how the Host reports
     * an empty file, so it contributes no rows; a one-line empty page keeps the
     * blank line it stands for.
     * @param page - document page `{offset, text, lines}`.
     * @returns its lines in source order.
     */
    function linesOfPage(page) {
      if (!isRecord(page)) return []
      const lines = Number(page.lines)
      if (Number.isFinite(lines) && Math.floor(lines) <= 0) return []
      const text = typeof page.text === 'string' ? page.text : ''
      return text === '' ? [''] : text.split('\n')
    }

    /**
     * Flatten the owner's loaded pages into absolute-numbered rows. A page's
     * offset is the file's real 1-based line, so a lazily appended page keeps
     * every row's true number instead of restarting at 1. `content.text` is only
     * a fallback for a host that delivers no page table; it is never re-split
     * when pages are present.
     * @param content - document content delivered to the body.
     * @returns rows and the loaded prefix's last line number.
     */
    function sourceRowsOf(content) {
      const rows = []
      if (isRecord(content) && Array.isArray(content.pages) && content.pages.length > 0) {
        const pages = content.pages.slice().sort(function (left, right) {
          const a = isRecord(left) ? Number(left.offset) : NaN
          const b = isRecord(right) ? Number(right.offset) : NaN
          return (Number.isFinite(a) ? a : 0) - (Number.isFinite(b) ? b : 0)
        })
        for (const page of pages) {
          if (!isRecord(page)) continue
          const offset = Number(page.offset)
          const start = Number.isFinite(offset) && offset > 0 ? Math.floor(offset) : 1
          const lines = linesOfPage(page)
          for (let index = 0; index < lines.length; index += 1) {
            rows.push({ number: start + index, text: lines[index] })
          }
        }
      } else if (isRecord(content) && typeof content.text === 'string') {
        const lines = content.text.split('\n')
        for (let index = 0; index < lines.length; index += 1) rows.push({ number: index + 1, text: lines[index] })
      }
      return { rows: rows, lastLine: rows.length === 0 ? 0 : rows[rows.length - 1].number }
    }

    /**
     * Classic selection copy, used when the async Clipboard API is unavailable
     * or refused. Nothing is left in the document.
     * @param value - text to copy.
     * @param doc - owning document.
     * @returns whether the copy command reported success.
     */
    function legacyCopySource(value, doc) {
      if (doc === undefined || doc === null || typeof doc.createElement !== 'function') return false
      try {
        const area = doc.createElement('textarea')
        area.value = value
        area.setAttribute('readonly', '')
        area.style.position = 'fixed'
        area.style.top = '0'
        area.style.opacity = '0'
        doc.body.appendChild(area)
        area.select()
        const copied = typeof doc.execCommand === 'function' && doc.execCommand('copy') === true
        doc.body.removeChild(area)
        return copied
      } catch (error) {
        return false
      }
    }

    /**
     * Copy the loaded source (the same text the native code body copies) and
     * report success instead of failing silently.
     * @param value - loaded source text.
     * @returns resolves to whether any copy path succeeded.
     */
    function copySourceText(value) {
      const doc = typeof document !== 'undefined' ? document : undefined
      try {
        if (typeof navigator !== 'undefined' && navigator !== null && navigator.clipboard !== undefined
          && typeof navigator.clipboard.writeText === 'function') {
          return navigator.clipboard.writeText(value).then(
            function () { return true },
            function () { return legacyCopySource(value, doc) },
          )
        }
      } catch (error) { /* fall through to the selection path */ }
      return Promise.resolve(legacyCopySource(value, doc))
    }

    /**
     * Clamp one arrow-key move to loaded source lines.
     * @param current - active line (0 = none yet).
     * @param direction - +1 down / -1 up.
     * @param lineCount - loaded source lines.
     * @returns the next active line (0 when nothing is loaded).
     */
    function nextActiveLine(current, direction, lineCount) {
      const count = Number.isFinite(Number(lineCount)) ? Math.max(0, Math.floor(Number(lineCount))) : 0
      if (count === 0) return 0
      if (!Number.isInteger(current) || current <= 0) return direction >= 0 ? 1 : count
      const next = current + (direction >= 0 ? 1 : -1)
      return Math.max(1, Math.min(count, next))
    }

    function SokonanodaSourceBody(props) {
      const input = isRecord(props) ? props : {}
      const tabInfo = typeof input.useTabInfo === 'function' ? input.useTabInfo() : null
      const navigation = tabInfo !== null && tabInfo.tab !== undefined ? tabInfo.tab.navigation : null
      const params = navigation !== null && navigation !== undefined ? navigation.params : null
      const targetLine = params !== null && params !== undefined && Number.isFinite(Number(params.line))
        ? Math.max(0, Math.floor(Number(params.line)))
        : 0
      const navRevision = navigation !== null && navigation !== undefined ? navigation.revision : undefined
      const content = input.content
      /* Rows keep the file's absolute line numbers; `lastLine` is the loaded
       * prefix's end, used to clamp arrow moves and to grow the gutter. */
      const source = sourceRowsOf(content)
      const rows = source.rows
      const lastLine = source.lastLine
      const resourceAddress = typeof input.resourceAddress === 'string' ? input.resourceAddress : ''
      const bodyRef = React.useRef(null)
      const navApplied = React.useRef(null)
      const copyTimer = React.useRef(0)
      const requestSeq = React.useRef(0)
      const [activeLine, setActiveLine] = React.useState(targetLine > 0 ? targetLine : 0)
      const [flashLine, setFlashLine] = React.useState(0)
      const [loadHint, setLoadHint] = React.useState(false)
      const [copyState, setCopyState] = React.useState('idle')
      const [panel, setPanel] = React.useState({ phase: 'idle', state: null, error: null, diagnostics: [] })
      const attach = function (node) {
        bodyRef.current = node
        if (typeof input.scrollportRef === 'function') input.scrollportRef(node)
      }
      /* Rows carry `data-textpreview-line`, the anchor the document owner's
       * `scrollToLine`/`markLine` use; `data-source-line` stays for the cursor
       * panel and older tests. */
      const lineRow = function (body, number) {
        if (body === null || body === undefined || typeof body.querySelector !== 'function') return null
        const row = body.querySelector('[data-textpreview-line="' + number + '"]')
        if (row !== null && row !== undefined) return row
        return body.querySelector('[data-source-line="' + number + '"]')
      }
      const revealLine = function (number) {
        const port = bodyRef.current
        const row = lineRow(port, number)
        if (row === null || row === undefined || typeof row.getBoundingClientRect !== 'function'
          || port === null || typeof port.getBoundingClientRect !== 'function') return
        const rowRect = row.getBoundingClientRect()
        const portRect = port.getBoundingClientRect()
        if (rowRect.top < portRect.top) port.scrollTop -= (portRect.top - rowRect.top)
        else if (rowRect.bottom > portRect.bottom) port.scrollTop += (rowRect.bottom - portRect.bottom)
      }
      /* A new navigation target (source link / hole) moves the active cursor and
       * flashes the line once. The document owner reveals the same target
       * through the loaded pages, so this body must not scroll a not-yet-loaded
       * line itself; it waits for the row and re-runs when a page arrives. */
      React.useEffect(function () {
        if (targetLine <= 0) return undefined
        const row = lineRow(bodyRef.current, targetLine)
        if (row === null || row === undefined) return undefined
        const marker = navRevision === undefined ? 'line:' + targetLine : 'rev:' + navRevision
        if (navApplied.current === marker) return undefined
        navApplied.current = marker
        setActiveLine(targetLine)
        setLoadHint(false)
        setFlashLine(targetLine)
        const timer = setTimeout(function () { setFlashLine(0) }, 2000)
        return function () { clearTimeout(timer) }
      }, [targetLine, navRevision, content])
      /* Debounced, single-flight cursor probe; a newer active line aborts the
       * older request and its sequence number drops any late answer. */
      React.useEffect(function () {
        if (activeLine <= 0 || resourceAddress === '' || typeof fetch !== 'function') {
          setPanel(function (previous) {
            return previous.phase === 'idle' ? previous : { phase: 'idle', state: null, error: null, diagnostics: [] }
          })
          return undefined
        }
        const seq = ++requestSeq.current
        let stopped = false
        let controller = null
        setPanel(function (previous) {
          return { phase: 'loading', state: previous.state, error: null, diagnostics: [] }
        })
        const timer = setTimeout(function () {
          if (stopped) return
          const url = '/sokonanoda-state?resource=' + encodeURIComponent(resourceAddress)
            + '&line=' + activeLine + '&col=1'
          const options = { cache: 'no-store' }
          if (typeof AbortController === 'function') {
            controller = new AbortController()
            options.signal = controller.signal
          }
          fetch(url, options)
            .then(function (response) {
              return response.json().then(function (body) { return { status: response.status, body: body } })
            })
            .then(function (result) {
              if (stopped || seq !== requestSeq.current) return
              const body = isRecord(result.body) ? result.body : null
              if (body === null) {
                setPanel({ phase: 'error', state: null, error: { code: 'invalid-response', message: '状态接口没有返回 JSON' }, diagnostics: [] })
                return
              }
              if (body.ok === true && isRecord(body.state)) {
                setPanel({ phase: 'ready', state: body.state, error: null, diagnostics: [] })
                return
              }
              const code = isRecord(body.error) && typeof body.error.code === 'string' ? body.error.code : 'error'
              const message = isRecord(body.error) && typeof body.error.message === 'string'
                ? body.error.message
                : '状态接口返回错误'
              const diagnostics = Array.isArray(body.diagnostics) ? body.diagnostics : []
              if (code === 'outside-declarations') {
                setPanel({ phase: 'outside', state: null, error: { code, message }, diagnostics: [] })
              } else if (code === 'file-errors') {
                setPanel({ phase: 'file-errors', state: null, error: { code, message }, diagnostics })
              } else {
                setPanel({ phase: 'error', state: null, error: { code, message }, diagnostics: [] })
              }
            })
            .catch(function (error) {
              if (stopped || seq !== requestSeq.current) return
              if (error !== null && typeof error === 'object' && error.name === 'AbortError') return
              setPanel({
                phase: 'error',
                state: null,
                error: { code: 'network', message: String(error?.message || error) },
                diagnostics: [],
              })
            })
        }, 120)
        return function () {
          stopped = true
          clearTimeout(timer)
          if (controller !== null) {
            try { controller.abort() } catch (error) { /* already settled */ }
          }
        }
      }, [activeLine, resourceAddress])
      const moveLine = function (direction) {
        const next = nextActiveLine(activeLine, direction, lastLine)
        if (next === activeLine || next === 0) {
          if (direction > 0 && content !== null && content !== undefined && content.eof === false) {
            setLoadHint(true)
            const port = bodyRef.current
            /* Programmatic bottom scroll reaches the owner's paged reader, which
             * requests the next page; the target-line effect then re-runs. */
            if (port !== null && port !== undefined) port.scrollTop = port.scrollHeight
          }
          return
        }
        setLoadHint(false)
        setActiveLine(next)
      }
      /* Keep the active line visible after the panel changes the code area's
       * height (a panel opens/closes on the first arrow), not only at the move. */
      React.useEffect(function () {
        if (activeLine > 0) revealLine(activeLine)
      }, [activeLine, panel.phase, content])
      React.useEffect(function () {
        return function () {
          if (copyTimer.current !== 0) {
            try { clearTimeout(copyTimer.current) } catch (error) { /* no timer */ }
          }
        }
      }, [])
      const t = makeT(input)
      const onCopy = function () {
        if (rows.length === 0) return
        const text = rows.map(function (row) { return row.text }).join('\n')
        copySourceText(text).then(function (ok) {
          setCopyState(ok ? 'copied' : 'failed')
          if (copyTimer.current !== 0) {
            try { clearTimeout(copyTimer.current) } catch (error) { /* no timer */ }
          }
          copyTimer.current = setTimeout(function () { setCopyState('idle') }, 1500)
        })
      }
      const onKeyDown = function (event) {
        if (event === null || typeof event !== 'object') return
        /* Never steal browser/OS chords (Cmd/Ctrl/Alt/Shift + arrows). */
        if (event.ctrlKey === true || event.metaKey === true || event.altKey === true || event.shiftKey === true) return
        if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return
        if (typeof event.preventDefault === 'function') event.preventDefault()
        moveLine(event.key === 'ArrowDown' ? 1 : -1)
      }
      const onLineClick = function (number) {
        let selected = ''
        try {
          if (typeof window !== 'undefined' && typeof window.getSelection === 'function') {
            selected = String(window.getSelection() || '')
          }
        } catch (error) { /* no selection API */ }
        if (selected !== '') return
        setLoadHint(false)
        setActiveLine(number)
        if (bodyRef.current !== null && bodyRef.current !== undefined && typeof bodyRef.current.focus === 'function') {
          /* Focusing must not jump the scroller back to its top. */
          try {
            bodyRef.current.focus({ preventScroll: true })
          } catch (error) {
            bodyRef.current.focus()
          }
        }
      }
      if (!isRecord(content) || content.kind !== 'text') return null
      const wrapped = input.wrap === true
      /* Gutter width follows the loaded prefix's largest real line number. */
      const gutter = Math.max(2, String(Math.max(lastLine, 1)).length)
      const copyLabel = copyState === 'copied'
        ? t('sourceCopied', '已复制')
        : copyState === 'failed'
          ? t('sourceCopyFailed', '复制失败')
          : t('sourceCopy', '复制')
      const empty = rows.length === 0
      return h('div', {
        className: 'dsls-source',
        style: S.sourceRoot,
        'data-active-line': activeLine > 0 ? activeLine : undefined,
      },
      h('div', { className: 'dsls-source-toolbar', key: 'toolbar', style: S.sourceToolbar, 'data-source-toolbar': '1' },
        h('span', { className: 'dsls-source-meta', key: 'meta', style: S.sourceMeta },
          t('sourceToolbarLabel', 'Lean · 只读')),
        h('button', {
          className: 'dsls-source-copy',
          key: 'copy',
          type: 'button',
          'data-copy-state': copyState,
          'aria-label': copyLabel,
          disabled: empty,
          style: empty ? Object.assign({}, S.sourceCopy, S.sourceCopyDisabled) : S.sourceCopy,
          onClick: empty ? undefined : onCopy,
        }, copyLabel)),
      h('div', {
        className: 'dsls-source-scroll',
        key: 'scroll',
        ref: attach,
        tabIndex: 0,
        role: 'region',
        'aria-label': t('stateAria', '只读源码预览：上下方向键移动光标行并显示该位置的 goal'),
        onKeyDown: onKeyDown,
        style: Object.assign({}, S.sourceScroll, {
          whiteSpace: wrapped ? 'pre-wrap' : 'pre',
          overflowX: wrapped ? 'hidden' : 'auto',
        }),
      },
      empty
        ? h('p', { className: 'dsls-source-empty', key: 'empty', style: S.sourceEmpty },
          t('sourceEmpty', '空文件（0 行）'))
        : h('div', { className: 'dsls-source-lines', key: 'lines', style: S.sourceLines },
          rows.map(function (row) {
            const number = row.number
            const marked = number === flashLine
            return h('div', {
              className: 'dsls-source-line',
              key: 'source-line-' + number,
              'data-source-line': number,
              'data-textpreview-line': number,
              'data-textpreview-target': number === targetLine ? number : undefined,
              'data-source-active': number === activeLine ? '1' : undefined,
              onClick: function () { onLineClick(number) },
              style: marked ? Object.assign({}, S.sourceLine, S.sourceLineFlash) : S.sourceLine,
            },
            h('span', { className: 'dsls-source-no', style: Object.assign({}, S.sourceNo, { minWidth: gutter + 'ch' }) }, String(number)),
            h('span', { className: 'dsls-source-text', style: S.sourceText },
              row.text === '' ? ' ' : highlightLean(row.text)))
          }))),
      h(SourceGoalPanel, {
        line: activeLine,
        phase: panel.phase,
        state: panel.state,
        error: panel.error,
        diagnostics: panel.diagnostics,
        loadHint: loadHint,
        t: input.t,
      }))
    }

        const CLIENT_BUILD = '2026-11-27T2140'

    function apply(ctx) {
      hostCtx = ctx
      try {
        if (ctx.sidebarRight !== undefined && ctx.sidebarRight !== null) sidebarRight = ctx.sidebarRight
      } catch (error) { /* service not injected in this host */ }
      if (sidebarRight === null) {
        try {
          const service = ctx.get('sidebarRight')
          if (service !== undefined && service !== null) sidebarRight = service
        } catch (error) { /* service not available */ }
      }
      report('sidebar-right', 'key=' + (sidebarRight === null ? 'missing' : 'ok'))
      let documentPreviews = null
      try {
        if (ctx.documentPreviews !== undefined && ctx.documentPreviews !== null) documentPreviews = ctx.documentPreviews
      } catch (error) { /* service not injected */ }
      if (documentPreviews === null) {
        try {
          const service = typeof ctx.get === 'function' ? ctx.get('documentPreviews') : undefined
          if (service !== undefined && service !== null) documentPreviews = service
        } catch (error) { /* service not available */ }
      }
      report('document-preview', 'key=' + (documentPreviews === null ? 'missing' : 'ok'))
      const registerSourcePreview = function (service) {
        if (sourcePreviewReady || service === null || service === undefined || typeof service.register !== 'function') return
        try {
          ctx.effect(function () {
            return service.register({
              id: SOURCE_PREVIEW_ID,
              extensions: ['sokonanoda'],
              priority: 'extension',
              title: function () { return 'Sokonanoda' },
              loading: 'text-pages',
              wrap: true,
              sourceLines: true,
            })
          }, 'sokonanoda-dsh: .sokonanoda code preview')
          ctx.effect(function () {
            return ctx.slots.inject('sidebar.right.tab.document', function () {
              return ctx.slots.register({
                name: 'sidebar.right.tab.document',
                key: SOURCE_PREVIEW_ID,
                locale: NS,
              }, SokonanodaSourceBody)
            })
          }, 'sokonanoda-dsh: .sokonanoda code body')
          sourcePreviewReady = true
          report('document-preview-body', 'key=registered')
        } catch (error) {
          report('document-preview-error', 'message=' + encodeURIComponent(String(error && error.message ? error.message : error).slice(0, 180)))
        }
      }
      if (sidebarRight === null && typeof ctx.inject === 'function') {
        /* A card source link must open the right-sidebar viewer; that service
         * can also register after this plugin applies. */
        try {
          ctx.inject(['sidebarRight'], function (scope) {
            let service
            try { service = scope?.sidebarRight ?? scope?.get?.('sidebarRight') } catch { service = undefined }
            if (service !== undefined && service !== null) sidebarRight = service
          })
        } catch (error) { /* non-Web client: links stay display-only */ }
      }
      if (documentPreviews !== null) {
        registerSourcePreview(documentPreviews)
      } else if (typeof ctx.inject === 'function') {
        /* The Sidebar document-preview service can register after this plugin
         * applies; wait for it instead of losing the body registration. */
        try {
          ctx.inject(['documentPreviews'], function (scope) {
            let service
            try { service = scope?.documentPreviews ?? scope?.get?.('documentPreviews') } catch { service = undefined }
            registerSourcePreview(service)
          })
        } catch (error) { /* non-Web client: the stock preview still works */ }
      }
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
        SokonanodaSourceBody: SokonanodaSourceBody,
        SourceGoalPanel: SourceGoalPanel,
        semanticRuns: semanticRuns,
        nextActiveLine: nextActiveLine,
        linesOfPage: linesOfPage,
        sourceRowsOf: sourceRowsOf,
        copySourceText: copySourceText,
        resetSidebarRight: function () { sidebarRight = null; hostCtx = null },
        cardCss: CARD_CSS.join(''),
        zh: zh,
        en: en,
        NS: NS,
      },
    }
  },
})
