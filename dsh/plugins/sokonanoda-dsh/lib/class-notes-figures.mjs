/**
 * Figures attached to class notes.
 *
 * A note only gets a figure through this explicit table: the asset route
 * serves exactly the files named here (`path` must be one of them), never an
 * arbitrary PNG. Adding a figure is a two-line change plus the PNG under
 * `dsh/plugins/sokonanoda-dsh/docs/figures/`.
 */

export const FIGURE_ROUTE = '/sokonanoda-notes-asset'

/** card id → figure descriptors. `file` is a basename, never a path. */
export const NOTE_FIGURES = Object.freeze({
  injective: Object.freeze([
    Object.freeze({
      id: 'injective',
      file: 'injective.png',
      title: '单射：不同输入不会撞到同一个输出',
      alt: '两个不同的 A 中元素沿映射 f 送到 B 中两个不同的元素',
    }),
  ]),
  'set-ext': Object.freeze([
    Object.freeze({
      id: 'set-ext',
      file: 'set-ext.png',
      title: '外延性：互相包含 ⇒ 相等',
      alt: 'A ⊆ B 与 B ⊆ A 同时成立时两个集合重合',
    }),
  ]),
})

/** Every PNG the asset route may serve. */
export function allowedFigureFiles() {
  const files = new Set()
  for (const figures of Object.values(NOTE_FIGURES)) {
    for (const figure of figures) files.add(figure.file)
  }
  return files
}

/** Ledger-ready figures for one note id, with a same-origin `src`. */
export function figuresForNote(noteId) {
  const figures = NOTE_FIGURES[noteId]
  if (!Array.isArray(figures)) return []
  return figures.map((figure) => ({
    id: figure.id,
    title: figure.title,
    alt: figure.alt,
    file: figure.file,
    src: `${FIGURE_ROUTE}?path=${encodeURIComponent(figure.file)}`,
  }))
}
