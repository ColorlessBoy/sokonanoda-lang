/**
 * Read-only renderings of the class-notes ledger.
 *
 * `/sokonanoda-notes-view` serves this self-contained HTML page inside the
 * right-sidebar tab: it mirrors the Host's theme tokens (same origin), polls
 * the ledger's `updatedAt` once a second and reloads itself when a note or an
 * answered question changed. `/sokonanoda-notes.md` serves the same book as
 * Markdown for copy/export.
 *
 * Everything is escaped here: card text comes from the course repository, but
 * the ledger is user state and a future card could contain markup.
 */

const SYNC_MS = 1000
const SCROLL_KEY = 'sokonanoda-notes-scroll'
const THEME_TOKENS = [
  '--dsw-alias-bg-base',
  '--dsw-alias-bg-layer-1',
  '--dsw-alias-bg-layer-2',
  '--dsw-alias-bg-module-platform',
  '--dsw-alias-label-primary',
  '--dsw-alias-label-secondary',
  '--dsw-alias-label-tertiary',
  '--dsw-alias-border-l1',
  '--dsw-alias-border-l2',
  '--dsw-alias-state-business-primary',
  '--dsw-alias-state-success-primary',
  '--dsw-alias-state-error-primary',
  '--dsw-alias-state-warn-primary',
  '--dsw-alias-markdown-code-block',
  '--dsw-alias-markdown-inline-code',
  '--dsw-alias-interactive-bg-hover',
]

/** HTML-escape one text node. */
export function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')
}

/** Escape and render the inline subset card text uses: `code` and **bold**. */
export function inlineHtml(value) {
  const escaped = escapeHtml(value)
  return escaped
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
}

function timeLabel(iso) {
  if (typeof iso !== 'string' || iso === '') return ''
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return iso
  try {
    return date.toLocaleString('zh-CN', { hour12: false })
  } catch {
    return iso
  }
}

function answerLine(question) {
  const answer = question.answer !== '' ? `答案 ${question.answer}` : ''
  const chosen = question.chosen !== '' ? `你选 ${question.chosen}` : ''
  return [answer, chosen].filter((part) => part !== '').join(' · ')
}

function questionItem(question) {
  const mark = question.correct === true ? '✓' : '×'
  const state = question.correct === true ? 'correct' : 'wrong'
  const line = answerLine(question)
  return [
    `<li class="cn-q" data-state="${state}">`,
    `<span class="cn-mark" aria-hidden="true">${mark}</span>`,
    '<div class="cn-q-body">',
    `<p class="cn-q-prompt">${inlineHtml(question.prompt)}</p>`,
    line !== '' ? `<p class="cn-q-answer">${escapeHtml(line)}</p>` : '',
    question.why !== '' ? `<p class="cn-q-why">${inlineHtml(question.why)}</p>` : '',
    '</div>',
    '</li>',
  ].join('')
}

function figureBlock(figure) {
  const src = typeof figure?.src === 'string' ? figure.src : ''
  if (src === '') return ''
  const title = typeof figure.title === 'string' ? figure.title : ''
  return [
    '<figure class="cn-figure">',
    `<img src="${escapeHtml(src)}" alt="${escapeHtml(figure.alt || title)}" loading="lazy" decoding="async">`,
    title !== '' ? `<figcaption>${inlineHtml(title)}</figcaption>` : '',
    '</figure>',
  ].join('')
}

function noteBlock(note, index) {
  const chapter = typeof note.chapter === 'string' && note.chapter !== ''
    ? `<span class="cn-chapter">${escapeHtml(note.chapter)}</span>` : ''
  const points = Array.isArray(note.keyPoints) ? note.keyPoints : []
  const figures = Array.isArray(note.figures) ? note.figures : []
  const questions = Array.isArray(note.questions) ? note.questions : []
  const mistakes = Array.isArray(note.mistakes) ? note.mistakes : []
  const sources = Array.isArray(note.sources) ? note.sources : []
  const correctCount = questions.filter((question) => question.correct === true).length
  /* A corrected question stays in 易错, so the tally follows the mistake list,
   * not the latest score of each question. */
  const wrongCount = mistakes.length
  return [
    `<article class="cn-note" id="note-${escapeHtml(note.id)}" data-note-index="${index}">`,
    '<header class="cn-note-head">',
    chapter,
    `<h2>${inlineHtml(note.title || note.id)}</h2>`,
    '</header>',
    note.oneLiner !== '' ? `<p class="cn-lead">${inlineHtml(note.oneLiner)}</p>` : '',
    points.length > 0
      ? `<section class="cn-section"><h3>要点</h3><ul class="cn-points">${points.map((point) => `<li>${inlineHtml(point)}</li>`).join('')}</ul></section>`
      : '',
    figures.length > 0
      ? `<section class="cn-section"><h3>图解</h3>${figures.map(figureBlock).join('')}</section>`
      : '',
    questions.length > 0
      ? `<section class="cn-section"><h3>做过的题 <span class="cn-tally"><b class="cn-ok">${correctCount}</b> 对 · <b class="cn-bad">${wrongCount}</b> 错</span></h3><ul class="cn-questions">${questions.map(questionItem).join('')}</ul></section>`
      : '',
    mistakes.length > 0
      ? `<section class="cn-section cn-mistakes"><h3>易错</h3><ul class="cn-mistake-list">${mistakes.map((mistake) => `<li><span class="cn-q-prompt">${inlineHtml(mistake.prompt)}</span>${mistake.why !== '' ? `<span class="cn-q-why">${inlineHtml(mistake.why)}</span>` : ''}</li>`).join('')}</ul></section>`
      : '',
    sources.length > 0
      ? `<footer class="cn-sources">出处：${sources.map((source) => escapeHtml(source.label)).join('；')}</footer>`
      : '',
    '</article>',
  ].join('')
}

/** The page shell: mirror theme tokens, then poll the ledger's updatedAt. */
function pageScript(updatedAt) {
  return `<script>
(function () {
  var TOKENS = ${JSON.stringify(THEME_TOKENS)};
  var STAMP = ${JSON.stringify(updatedAt)};
  var SYNC_MS = ${SYNC_MS};
  var SCROLL_KEY = ${JSON.stringify(SCROLL_KEY)};
  function syncTheme() {
    var dark = null;
    try {
      var body = window.parent && window.parent !== window ? window.parent.document.body : null;
      if (body) {
        dark = body.hasAttribute('data-ds-dark-theme');
        var style = window.parent.getComputedStyle(body);
        for (var i = 0; i < TOKENS.length; i += 1) {
          var value = style.getPropertyValue(TOKENS[i]);
          if (value && value.trim() !== '') document.documentElement.style.setProperty(TOKENS[i], value.trim());
        }
        /* Custom properties keep their raw var(...) text; copy the resolved
         * family so the frame uses the same sans-serif face as the app. */
        if (style.fontFamily) document.documentElement.style.setProperty('--cn-font', style.fontFamily);
        if (style.fontSize) document.documentElement.style.setProperty('--cn-font-size', style.fontSize);
      }
    } catch (error) { /* standalone view: fall back to the OS preference */ }
    if (dark === null && typeof window.matchMedia === 'function') {
      dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    }
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
    document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
  }
  function restoreScroll() {
    try {
      var value = Number(window.sessionStorage.getItem(SCROLL_KEY) || 0);
      if (Number.isFinite(value) && value > 0) {
        window.sessionStorage.removeItem(SCROLL_KEY);
        window.scrollTo(0, value);
      }
    } catch (error) { /* storage unavailable */ }
  }
  function setSync(text, ok) {
    var node = document.querySelector('[data-sync-state]');
    if (!node) return;
    node.textContent = text;
    node.dataset.state = ok ? 'ok' : 'offline';
  }
  var busy = false;
  function tick() {
    if (busy || typeof window.fetch !== 'function') return;
    busy = true;
    window.fetch('/sokonanoda-notes?meta=1', { cache: 'no-store' })
      .then(function (response) { return response.ok ? response.json() : null })
      .then(function (meta) {
        if (meta && meta.ok === true && meta.updatedAt && meta.updatedAt !== STAMP) {
          try { window.sessionStorage.setItem(SCROLL_KEY, String(window.scrollY || 0)); } catch (error) { /* ignore */ }
          window.location.reload();
          return;
        }
        setSync('实时同步', true);
      })
      .catch(function () { setSync('同步暂不可用', false); })
      .then(function () { busy = false; });
  }
  syncTheme();
  restoreScroll();
  try {
    var observer = new window.MutationObserver(syncTheme);
    if (window.parent && window.parent !== window) {
      observer.observe(window.parent.document.body, { attributes: true, attributeFilter: ['data-ds-dark-theme', 'class', 'style'] });
    }
  } catch (error) { /* parent is not observable (standalone) */ }
  window.setInterval(tick, SYNC_MS);
  window.setInterval(syncTheme, 5000);
  document.addEventListener('visibilitychange', function () { if (!document.hidden) tick(); });
})();
</script>`
}

const PAGE_CSS = `
:root{
  color-scheme: light;
  --cn-bg: var(--dsw-alias-bg-base, #ffffff);
  --cn-surface: var(--dsw-alias-bg-layer-1, #f7f8fa);
  --cn-surface-2: var(--dsw-alias-bg-layer-2, #eef0f4);
  --cn-text: var(--dsw-alias-label-primary, #17181c);
  --cn-text-2: var(--dsw-alias-label-secondary, #4b5058);
  --cn-text-3: var(--dsw-alias-label-tertiary, #70757e);
  --cn-border: var(--dsw-alias-border-l1, #e6e8ec);
  --cn-border-strong: var(--dsw-alias-border-l2, #d6d9df);
  --cn-accent: var(--dsw-alias-state-business-primary, #2f6fed);
  --cn-ok: var(--dsw-alias-state-success-primary, #0f7b4a);
  --cn-bad: var(--dsw-alias-state-error-primary, #c0392b);
  --cn-warn: var(--dsw-alias-state-warn-primary, #9a5b00);
  --cn-code: var(--dsw-alias-markdown-code-block, #f2f3f5);
  --cn-inline-code: var(--dsw-alias-markdown-inline-code, #eef0f3);
  --cn-font: var(--dsw-font-family, -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", sans-serif);
  --cn-mono: var(--ds-font-family-code, ui-monospace, SFMono-Regular, Menlo, monospace);
}
:root[data-theme="dark"]{
  color-scheme: dark;
  --cn-bg: #16171a;
  --cn-surface: #1d1f23;
  --cn-surface-2: #24272c;
  --cn-text: #f2f3f5;
  --cn-text-2: #c3c7ce;
  --cn-text-3: #969ba4;
  --cn-border: rgba(255,255,255,.09);
  --cn-border-strong: rgba(255,255,255,.16);
  --cn-accent: #6f9dff;
  --cn-ok: #4ed17e;
  --cn-bad: #f2736a;
  --cn-warn: #f7ad31;
  --cn-code: #202226;
  --cn-inline-code: #2a2d33;
}
*{box-sizing:border-box}
html,body{margin:0;padding:0;background:var(--cn-bg);color:var(--cn-text)}
body{font:var(--cn-font-size,14px)/1.62 var(--cn-font);padding:18px 18px 32px;overflow-wrap:anywhere}
a{color:var(--cn-accent)}
code{font:11.5px/1.55 var(--cn-mono);background:var(--cn-inline-code);border-radius:5px;padding:1px 4px}
.cn-head{padding:2px 2px 14px;border-bottom:1px solid var(--cn-border);margin-bottom:16px}
.cn-kicker{font-size:11.5px;letter-spacing:.12em;color:var(--cn-accent);text-transform:uppercase;font-weight:600}
.cn-head h1{font-size:20px;line-height:1.35;margin:6px 0 4px;letter-spacing:-.01em}
.cn-meta{margin:0;color:var(--cn-text-3);font-size:12px}
.cn-meta strong{color:var(--cn-text-2);font-weight:600}
[data-sync-state][data-state="offline"]{color:var(--cn-warn)}
.cn-note{border:1px solid var(--cn-border);background:var(--cn-surface);border-radius:14px;padding:16px 16px 14px;margin:0 0 14px}
.cn-note-head{display:flex;align-items:baseline;gap:8px;flex-wrap:wrap;margin:0 0 6px}
.cn-note-head h2{font-size:16px;line-height:1.4;margin:0;letter-spacing:-.01em}
.cn-chapter{font:600 10.5px/1.6 var(--cn-mono);color:var(--cn-accent);background:color-mix(in srgb, var(--cn-accent) 12%, transparent);border-radius:999px;padding:1px 8px;flex:none}
.cn-lead{margin:0 0 10px;color:var(--cn-text-2);font-size:13.5px}
.cn-section{margin-top:12px}
.cn-section h3{font-size:12px;line-height:1.4;letter-spacing:.04em;color:var(--cn-text-3);text-transform:uppercase;margin:0 0 6px;font-weight:600}
.cn-points{margin:0;padding-left:18px}
.cn-points li{margin:0 0 5px;color:var(--cn-text-2)}
.cn-figure{margin:8px 0 4px;padding:10px;background:var(--cn-bg);border:1px solid var(--cn-border);border-radius:10px}
.cn-figure img{display:block;width:100%;height:auto;border-radius:6px}
.cn-figure figcaption{margin-top:7px;color:var(--cn-text-3);font-size:12px;text-align:center}
.cn-tally{margin-left:8px;text-transform:none;letter-spacing:0;font-weight:500;color:var(--cn-text-3)}
.cn-ok{color:var(--cn-ok)}
.cn-bad{color:var(--cn-bad)}
.cn-questions,.cn-mistake-list{list-style:none;margin:0;padding:0}
.cn-q{display:flex;gap:9px;padding:9px 10px;border-radius:10px;background:var(--cn-bg);border:1px solid var(--cn-border);margin:0 0 8px}
.cn-q[data-state="correct"]{border-left:3px solid var(--cn-ok)}
.cn-q[data-state="wrong"]{border-left:3px solid var(--cn-bad)}
.cn-mark{flex:none;width:18px;height:18px;line-height:18px;text-align:center;border-radius:50%;font-size:11px;font-weight:700;margin-top:2px;color:#fff}
.cn-q[data-state="correct"] .cn-mark{background:var(--cn-ok)}
.cn-q[data-state="wrong"] .cn-mark{background:var(--cn-bad)}
.cn-q-body{min-width:0;flex:1}
.cn-q-prompt{margin:0;font-size:13px;line-height:1.55}
.cn-q-answer{margin:4px 0 0;font-size:11.5px;color:var(--cn-text-3)}
.cn-q-why{margin:4px 0 0;font-size:12px;color:var(--cn-text-2)}
.cn-mistakes{border-top:1px dashed var(--cn-border-strong);padding-top:10px}
.cn-mistakes h3{color:var(--cn-bad)}
.cn-mistake-list li{padding:6px 0;border-bottom:1px solid var(--cn-border)}
.cn-mistake-list li:last-child{border-bottom:0}
.cn-mistake-list .cn-q-prompt{display:block}
.cn-mistake-list .cn-q-why{color:var(--cn-text-3);display:block}
.cn-sources{margin-top:12px;color:var(--cn-text-3);font-size:11.5px}
.cn-empty{border:1px dashed var(--cn-border-strong);border-radius:14px;padding:24px 18px;color:var(--cn-text-2);background:var(--cn-surface)}
.cn-empty h2{margin:0 0 6px;font-size:15px;color:var(--cn-text)}
.cn-empty p{margin:0;font-size:13px}
.cn-foot{margin:18px 2px 0;color:var(--cn-text-3);font-size:11.5px}
@media (max-width: 460px){body{padding:14px 12px 24px}.cn-note{padding:13px 12px}.cn-note-head h2{font-size:15px}}
@media (prefers-reduced-motion: reduce){*{scroll-behavior:auto}}
`

/**
 * Render the complete iframe document.
 * @param book - ledger snapshot.
 */
export function renderNotesHtml(book) {
  const notes = Array.isArray(book?.notes) ? book.notes : []
  const updatedAt = typeof book?.updatedAt === 'string' ? book.updatedAt : ''
  const course = typeof book?.course === 'string' && book.course !== '' ? book.course : 'set-theory'
  const noteCount = notes.length
  const questionCount = notes.reduce((sum, note) => sum + (Array.isArray(note.questions) ? note.questions.length : 0), 0)
  const wrongCount = notes.reduce((sum, note) => sum + (Array.isArray(note.mistakes) ? note.mistakes.length : 0), 0)
  const body = notes.length === 0
    ? `<section class="cn-empty"><h2>还没有课堂笔记</h2><p>做几道题，或点「不懂」看一张知识卡；看过的知识卡和答过的题会自动沉淀到这里，闲聊不会进来。</p></section>`
    : notes.map(noteBlock).join('')
  return `<!doctype html>
<html lang="zh-CN" data-theme="light" data-updated-at="${escapeHtml(updatedAt)}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
<title>课堂笔记</title>
<style>${PAGE_CSS}</style>
</head>
<body>
<header class="cn-head">
  <div class="cn-kicker">课堂笔记 · ${escapeHtml(course)}</div>
  <h1>这节课的复习账本</h1>
  <p class="cn-meta"><strong>${noteCount}</strong> 个知识点 · <strong>${questionCount}</strong> 道做过的题${wrongCount > 0 ? ` · <strong class="cn-bad">${wrongCount}</strong> 道进易错` : ''} · 更新于 ${escapeHtml(timeLabel(updatedAt))} · <span data-sync-state data-state="ok">实时同步</span></p>
</header>
<main>${body}</main>
<footer class="cn-foot">只读 · 数据在 ~/.dsh/state/sokonanoda-dsh/notes/，不在仓库里；闲聊不进笔记。</footer>
${pageScript(updatedAt)}
</body>
</html>`
}

/** Markdown rendering of the same ledger (copy/export route). */
export function renderNotesMarkdown(book) {
  const notes = Array.isArray(book?.notes) ? book.notes : []
  const course = typeof book?.course === 'string' && book.course !== '' ? book.course : 'set-theory'
  const lines = [`# 课堂笔记 · ${course}`, '']
  lines.push(`> 更新于 ${timeLabel(book?.updatedAt)} · ${notes.length} 个知识点`, '')
  if (notes.length === 0) {
    lines.push('还没有课堂笔记：做几道题，或点「不懂」看一张知识卡。', '')
    return lines.join('\n')
  }
  for (const note of notes) {
    const title = note.chapter !== '' ? `${note.chapter} · ${note.title}` : note.title
    lines.push(`## ${title}`, '')
    if (note.oneLiner !== '') lines.push(note.oneLiner, '')
    if (Array.isArray(note.keyPoints) && note.keyPoints.length > 0) {
      lines.push('**要点**', '')
      for (const point of note.keyPoints) lines.push(`- ${point}`)
      lines.push('')
    }
    if (Array.isArray(note.figures) && note.figures.length > 0) {
      for (const figure of note.figures) lines.push(`![${figure.alt || figure.title}](${figure.src})`, '')
    }
    if (Array.isArray(note.questions) && note.questions.length > 0) {
      lines.push('**做过的题**', '')
      for (const question of note.questions) {
        const mark = question.correct === true ? '✅' : '❌'
        lines.push(`- ${mark} ${question.prompt}（${answerLine(question)}）`)
        if (question.why !== '') lines.push(`  - ${question.why}`)
      }
      lines.push('')
    }
    if (Array.isArray(note.mistakes) && note.mistakes.length > 0) {
      lines.push('**易错**', '')
      for (const mistake of note.mistakes) {
        lines.push(`- ${mistake.prompt}`)
        if (mistake.why !== '') lines.push(`  - ${mistake.why}`)
      }
      lines.push('')
    }
    if (Array.isArray(note.sources) && note.sources.length > 0) {
      lines.push(`> 出处：${note.sources.map((source) => source.label).join('；')}`, '')
    }
  }
  return lines.join('\n')
}
