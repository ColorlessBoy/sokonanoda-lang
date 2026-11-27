/**
 * Class-notes ledger for `sokonanoda-dsh`.
 *
 * The chat is the lesson; these notes are the review copy. Only *seen*
 * knowledge cards create notes (a knowledge card the learner opened in chat),
 * and only answered questions are attached to them. Chit-chat never reaches
 * this module: the Host calls it for quiz replies, never for model replies.
 *
 * The ledger is user state, not repository content:
 *   ~/.dsh/state/sokonanoda-dsh/notes/<course>.json
 *
 * Everything here is pure/filesystem-only and synchronous so a route can read
 * the same settled snapshot the fast path just wrote. Writes are atomic
 * (temp file + rename), so the iframe poller can never observe half a book.
 */

import fs from 'node:fs'
import path from 'node:path'

import { figuresForNote } from './class-notes-figures.mjs'

export const NOTES_SCHEMA = 'soko.class-notes/1'
export const NOTES_VERSION = 1

/** Tags too generic to prove a card is about a question (same policy as engine.py). */
export const GENERIC_TAGS = new Set(['function', 'logic', 'types', 'glossary', 'order'])

/** Keep the review copy readable: newest N items per note. */
export const MAX_QUESTIONS_PER_NOTE = 12
export const MAX_MISTAKES_PER_NOTE = 8
export const MAX_KEY_POINTS = 12

const LETTERS = ['A', 'B', 'C', 'D', 'E', 'F']
const NOTE_ID_RE = /[^a-z0-9._-]+/g

/** ISO timestamp used by every ledger mutation. */
export function nowIso(now = Date.now()) {
  return new Date(now).toISOString()
}

/** Ledger-safe note id (payload ids are mostly slugs already). */
export function sanitizeNoteId(value) {
  const raw = typeof value === 'string' ? value.trim().toLowerCase() : ''
  return raw.replace(NOTE_ID_RE, '-').replace(/^-+|-+$/g, '')
}

/** `I.2` out of a card kicker like `卷 I · I.2 关系`. */
export function chapterOf(card) {
  const candidates = [card?.chapter, card?.kicker, card?.unit]
  for (const candidate of candidates) {
    if (typeof candidate !== 'string') continue
    const match = /([IVX]+\.\d+)/.exec(candidate)
    if (match !== null) return match[1]
  }
  return ''
}

/** 1-based option index → letter. */
export function letterOf(index) {
  const value = Number(index)
  if (!Number.isInteger(value) || value < 1 || value > LETTERS.length) return ''
  return LETTERS[value - 1]
}

/** One empty ledger for a course. */
export function emptyBook(course, now = Date.now()) {
  return {
    schema: NOTES_SCHEMA,
    version: NOTES_VERSION,
    course: typeof course === 'string' && course !== '' ? course : 'set-theory',
    updatedAt: nowIso(now),
    notes: [],
  }
}

/**
 * Pull display-only knowledge cards out of one quiz reply.
 * The fast path gets the raw OpenAI-shaped payload, before it becomes a fence.
 * @param payload - quiz-service chat-completions payload.
 * @returns knowledge card objects (deduplicated by id).
 */
export function extractKnowledgeCards(payload) {
  const message = payload !== null && typeof payload === 'object'
    ? payload.choices?.[0]?.message
    : undefined
  const calls = Array.isArray(message?.tool_calls) ? message.tool_calls : []
  const cards = []
  const seen = new Set()
  for (const call of calls) {
    const fn = call !== null && typeof call === 'object' ? call.function : undefined
    if (fn === null || typeof fn !== 'object' || fn.name !== 'learn_card' || typeof fn.arguments !== 'string') continue
    let args = null
    try { args = JSON.parse(fn.arguments) } catch { args = null }
    if (args === null || typeof args !== 'object' || args.kind !== 'knowledge') continue
    const card = args.payload
    if (card === null || typeof card !== 'object' || Array.isArray(card)) continue
    const id = sanitizeNoteId(card.id)
    if (id === '' || seen.has(id)) continue
    seen.add(id)
    cards.push(card)
  }
  return cards
}

/** Flatten the readable parts of a knowledge card into short review points. */
export function keyPointsOf(card) {
  const points = []
  const push = (value) => {
    if (typeof value !== 'string') return
    const text = value.trim()
    if (text === '' || points.includes(text)) return
    points.push(text)
  }
  push(card?.oneLiner)
  const sections = Array.isArray(card?.sections) ? card.sections : []
  for (const section of sections) {
    if (section === null || typeof section !== 'object') continue
    if (section.kind === 'list' && Array.isArray(section.items)) {
      for (const item of section.items) push(item)
    } else if (section.kind === 'paragraph') {
      push(section.text)
    }
  }
  return points.slice(0, MAX_KEY_POINTS)
}

/** One ledger note from a knowledge-card payload. */
export function noteFromCard(card, now = Date.now()) {
  const stamp = nowIso(now)
  const id = sanitizeNoteId(card?.id)
  return {
    id,
    chapter: chapterOf(card),
    title: typeof card?.title === 'string' && card.title.trim() !== '' ? card.title.trim() : id,
    kicker: typeof card?.kicker === 'string' ? card.kicker : '',
    oneLiner: typeof card?.oneLiner === 'string' ? card.oneLiner.trim() : '',
    keyPoints: keyPointsOf(card),
    definition: card?.definition !== null && typeof card?.definition === 'object'
      ? { lang: String(card.definition.lang || 'lean'), code: String(card.definition.code || '') }
      : null,
    questions: [],
    mistakes: [],
    figures: figuresForNote(id),
    sources: typeof card?.source === 'string' && card.source.trim() !== ''
      ? [{ label: card.source.trim() }]
      : [],
    tags: Array.isArray(card?.tags)
      ? card.tags.filter((tag) => typeof tag === 'string' && tag.trim() !== '').map((tag) => tag.trim())
      : [],
    firstSeenAt: stamp,
    lastSeenAt: stamp,
  }
}

/** True when two ledger values differ, ignoring the ordering of object keys. */
function stable(value) {
  if (Array.isArray(value)) return '[' + value.map(stable).join(',') + ']'
  if (value !== null && typeof value === 'object') {
    return '{' + Object.keys(value).sort().map((key) => JSON.stringify(key) + ':' + stable(value[key])).join(',') + '}'
  }
  return JSON.stringify(value)
}

function sameContent(left, right) {
  return stable(left) === stable(right)
}

/**
 * Insert or refresh one knowledge note. Seeing the same card again refreshes
 * its review copy and `lastSeenAt`; `firstSeenAt` and answered questions stay.
 * @returns true when the ledger changed.
 */
export function upsertKnowledgeCard(book, card, now = Date.now()) {
  const fresh = noteFromCard(card, now)
  if (fresh.id === '') return false
  const index = book.notes.findIndex((note) => note.id === fresh.id)
  if (index < 0) {
    book.notes.push(fresh)
    return true
  }
  const previous = book.notes[index]
  const merged = {
    ...previous,
    chapter: fresh.chapter || previous.chapter,
    title: fresh.title,
    kicker: fresh.kicker || previous.kicker,
    oneLiner: fresh.oneLiner,
    keyPoints: fresh.keyPoints.length > 0 ? fresh.keyPoints : previous.keyPoints,
    definition: fresh.definition !== null ? fresh.definition : previous.definition,
    figures: fresh.figures,
    sources: fresh.sources.length > 0 ? fresh.sources : previous.sources,
    tags: fresh.tags.length > 0 ? fresh.tags : previous.tags,
    lastSeenAt: fresh.lastSeenAt,
  }
  if (sameContent(merged, previous)) return false
  book.notes[index] = merged
  return true
}

/**
 * Pick the note a question belongs to, using the same tag rule as
 * `engine.py` explain_for: two shared tags, or at least one specific tag.
 * `notes` is the set the learner has actually seen, so an unseen card never
 * gains questions.
 * @returns note id or ''.
 */
export function matchNoteForQuestion(question, notes) {
  const tags = new Set(Array.isArray(question?.tags) ? question.tags : [])
  if (tags.size === 0 || !Array.isArray(notes)) return ''
  let best = null
  for (const note of notes) {
    const noteTags = Array.isArray(note?.tags) ? note.tags : []
    const matched = noteTags.filter((tag) => tags.has(tag))
    if (matched.length === 0) continue
    const specific = matched.some((tag) => !GENERIC_TAGS.has(tag))
    if (matched.length < 2 && !specific) continue
    const score = matched.length
    const specificCount = matched.filter((tag) => !GENERIC_TAGS.has(tag)).length
    if (best === null || score > best.score
      || (score === best.score && specificCount > best.specificCount)
      || (score === best.score && specificCount === best.specificCount && note.id < best.id)) {
      best = { id: note.id, score, specificCount }
    }
  }
  return best === null ? '' : best.id
}

/** One history row → the review question shape (answer letters, never indices). */
export function questionFromHistory(entry, question) {
  const correct = entry?.correct === true
  const choice = Number(entry?.choice)
  return {
    questionId: String(entry?.qid || ''),
    prompt: typeof question?.prompt === 'string' ? question.prompt : '',
    options: Array.isArray(question?.options) ? question.options.filter((option) => typeof option === 'string') : [],
    answer: letterOf(question?.answer),
    chosen: letterOf(choice),
    why: typeof question?.why === 'string' ? question.why : '',
    correct,
    at: typeof entry?.at === 'string' ? entry.at : '',
  }
}

function byNewest(left, right) {
  return String(right.at).localeCompare(String(left.at))
}

function mistakeOf(question) {
  return {
    questionId: question.questionId,
    prompt: question.prompt,
    why: question.why,
    at: question.at,
  }
}

/**
 * Attach the latest answer of every question to the note it explains.
 * The quiz service writes history for every answered question; this is the
 * only path that makes a question appear in the notes (no model guessing).
 * @returns true when the ledger changed.
 */
export function attachHistory(book, history, questionsById, now = Date.now()) {
  if (!Array.isArray(book?.notes) || !Array.isArray(history)) return false
  const latest = new Map()
  const wrong = new Map()
  for (const entry of history) {
    if (entry === null || typeof entry !== 'object' || typeof entry.qid !== 'string') continue
    const previous = latest.get(entry.qid)
    if (previous === undefined || String(entry.at || '') >= String(previous.at || '')) latest.set(entry.qid, entry)
    if (entry.correct === false) {
      const previousWrong = wrong.get(entry.qid)
      if (previousWrong === undefined || String(entry.at || '') >= String(previousWrong.at || '')) wrong.set(entry.qid, entry)
    }
  }
  const grouped = new Map(book.notes.map((note) => [note.id, { questions: [], mistakes: [] }]))
  const bank = questionsById !== null && typeof questionsById === 'object' ? questionsById : {}
  for (const [qid, entry] of latest) {
    const question = bank[qid]
    if (question === null || typeof question !== 'object' || qid === '') continue
    const noteId = matchNoteForQuestion(question, book.notes)
    if (noteId === '' || !grouped.has(noteId)) continue
    grouped.get(noteId).questions.push(questionFromHistory(entry, question))
    /* A wrong attempt stays in 易错 even after the learner later fixes it:
     * the review page is a record of what happened, not only the last score. */
    const wrongEntry = wrong.get(qid)
    if (wrongEntry !== undefined) grouped.get(noteId).mistakes.push(questionFromHistory(wrongEntry, question))
  }
  let changed = false
  for (const note of book.notes) {
    const attached = grouped.get(note.id) || { questions: [], mistakes: [] }
    const questions = attached.questions.slice().sort(byNewest).slice(0, MAX_QUESTIONS_PER_NOTE)
    const mistakes = attached.mistakes.slice().sort(byNewest).slice(0, MAX_MISTAKES_PER_NOTE).map(mistakeOf)
    if (!sameContent(note.questions, questions)) {
      note.questions = questions
      changed = true
    }
    if (!sameContent(note.mistakes, mistakes)) {
      note.mistakes = mistakes
      changed = true
    }
  }
  if (changed) {
    /* Stable display order: newest seen first, but chapters keep first-seen order. */
    book.notes.sort((left, right) => String(left.firstSeenAt).localeCompare(String(right.firstSeenAt))
      || left.id.localeCompare(right.id))
  }
  return changed
}

/** Read a ledger; a missing or malformed file becomes a new empty book. */
export function readLedger(file, course, now = Date.now()) {
  try {
    const parsed = JSON.parse(fs.readFileSync(file, 'utf8'))
    if (parsed === null || typeof parsed !== 'object' || !Array.isArray(parsed.notes)) {
      return emptyBook(course, now)
    }
    return {
      schema: NOTES_SCHEMA,
      version: NOTES_VERSION,
      course: typeof parsed.course === 'string' && parsed.course !== '' ? parsed.course : course,
      updatedAt: typeof parsed.updatedAt === 'string' ? parsed.updatedAt : nowIso(now),
      notes: parsed.notes.filter((note) => note !== null && typeof note === 'object'
        && typeof note.id === 'string' && note.id !== ''),
    }
  } catch {
    return emptyBook(course, now)
  }
}

/** Atomic ledger write: a reader sees the old book or the new one, never half. */
export function writeLedger(file, book) {
  const dir = path.dirname(file)
  fs.mkdirSync(dir, { recursive: true })
  const tmp = `${file}.${process.pid}.${Date.now()}.tmp`
  try {
    fs.writeFileSync(tmp, `${JSON.stringify(book, null, 2)}\n`, 'utf8')
    fs.renameSync(tmp, file)
  } catch (error) {
    try { fs.unlinkSync(tmp) } catch { /* the temp file may never have been created */ }
    throw error
  }
}

/**
 * Read every quiz-state file the service may have written (`default.json` plus
 * fingerprint files) and return **every attempt**. The mistake ledger needs a
 * wrong attempt even when the learner later answered the same question right;
 * `attachHistory` is the place that picks the latest attempt per question.
 */
export function readQuizHistory(quizStateDir) {
  let names = []
  try {
    names = fs.readdirSync(quizStateDir).filter((name) => name.endsWith('.json'))
  } catch {
    return []
  }
  const entries = []
  const seen = new Set()
  for (const name of names) {
    let parsed
    try {
      parsed = JSON.parse(fs.readFileSync(path.join(quizStateDir, name), 'utf8'))
    } catch {
      continue
    }
    const history = Array.isArray(parsed?.history) ? parsed.history : []
    for (const entry of history) {
      if (entry === null || typeof entry !== 'object' || typeof entry.qid !== 'string') continue
      const key = [entry.qid, entry.at || '', entry.choice === undefined ? '' : entry.choice, entry.correct === true ? '1' : '0'].join('|')
      if (seen.has(key)) continue
      seen.add(key)
      entries.push(entry)
    }
  }
  return entries
}

/** Question bank (`questions.json`) indexed by id; absent files yield an empty bank. */
export function loadQuestionBank(repoRoot, course = 'set-theory') {
  try {
    const file = path.join(repoRoot, 'courses', course, 'knowledge', 'data', 'questions.json')
    const parsed = JSON.parse(fs.readFileSync(file, 'utf8'))
    const questions = Array.isArray(parsed?.questions) ? parsed.questions : []
    const byId = {}
    for (const question of questions) {
      if (question !== null && typeof question === 'object' && typeof question.id === 'string') byId[question.id] = question
    }
    return { questions, byId }
  } catch {
    return { questions: [], byId: {} }
  }
}

/** Ledger metadata the client poller reads (cheap, no note bodies). */
export function bookMeta(book) {
  return {
    updatedAt: typeof book?.updatedAt === 'string' ? book.updatedAt : '',
    count: Array.isArray(book?.notes) ? book.notes.length : 0,
  }
}

/**
 * Create the per-Host notes runtime. One instance owns one ledger file and
 * keeps its in-memory book in sync with whatever the routes serve.
 * @param options - dataDir, repoRoot, course, now.
 */
export function createClassNotes(options = {}) {
  const dataDir = typeof options.dataDir === 'string' ? options.dataDir : ''
  const repoRoot = typeof options.repoRoot === 'string' ? options.repoRoot : ''
  const quizStateDir = typeof options.quizStateDir === 'string' && options.quizStateDir !== ''
    ? options.quizStateDir
    : path.join(dataDir, 'quiz')
  const course = typeof options.course === 'string' && options.course !== '' ? options.course : 'set-theory'
  const file = path.join(dataDir, 'notes', `${course}.json`)
  const now = typeof options.now === 'function' ? options.now : () => Date.now()
  const bank = loadQuestionBank(repoRoot, course)
  let book = readLedger(file, course, now())

  const persist = () => {
    book.updatedAt = nowIso(now())
    writeLedger(file, book)
  }

  return {
    file,
    course,
    book() { return book },
    meta() { return bookMeta(book) },

    /**
     * Record knowledge cards from one quiz reply and reconcile answered
     * questions. Returns what changed; throws only on an unwritable ledger so
     * the caller can record the failure without breaking the reply.
     */
    recordReply(payload) {
      const cards = extractKnowledgeCards(payload)
      let changed = false
      for (const card of cards) {
        if (upsertKnowledgeCard(book, card, now())) changed = true
      }
      const reconciled = attachHistory(book, readQuizHistory(quizStateDir), bank.byId, now())
      if (changed || reconciled) persist()
      return { cards: cards.length, changed: changed || reconciled }
    },

    /** Attach questions answered before this Host started (or by another file). */
    reconcileHistory() {
      const history = readQuizHistory(quizStateDir)
      const changed = attachHistory(book, history, bank.byId, now())
      if (changed) persist()
      return { history: history.length, changed }
    },
  }
}
