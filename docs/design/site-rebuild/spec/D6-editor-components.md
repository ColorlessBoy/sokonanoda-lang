# D6 — 编辑器表面组件（`site/assets/editor.css`）

> **页面作者照着这一份抄就够了。** 每个组件一节：类清单 + 一段**完整可粘贴**的 HTML。
> 片段里的每个字符串都来自真跑过的内核输出（命令写在 `.illus-cap` 里，可复现）；
> 没有一处是"看起来像"的编造值。
>
> 配套文件：`site/assets/editor.css`（唯一实现）、`site/assets/tokens.css`（令牌，唯一源）、
> `site/assets/fonts.css`（`--font-mono` 是唯一覆盖 `⊢` 的面）。
> 规则手册：`D1-design-rules.md`。验收页：`.cache/editor-surface.html`。

---

## 0. 依赖与硬规则

| 组件 | 需要 `tokens.css` | 需要 `fonts.css` | 需要 `base.css` / `site.css` |
|---|---|---|---|
| 全部九节 | ✅ 必须 | ✅ 必须（`⊢` 只有 `Soko Mono` 覆盖） | ❌ **不需要** |

`editor.css` **不依赖 `base.css` / `site.css`**：它自带字体栈、行高、等宽三件套
（关连字 / `tab-size: 2` / `tabular-nums`），也不引用任何页面级类名。
页面只要按 `tokens.css → fonts.css → editor.css` 的顺序链进来即可。

**四条纪律（违反即 bug，评审按 bug 处理）：**

1. 颜色只走令牌，`editor.css` 里 0 处裸 hex；
2. 字号只用 `--fs-*`，圆角只用 `--r-0` / `--r-1` / `--r-full`，间距只用 `--sp-*`；
3. **没有入场动画**，只有状态反馈（`[data-copy]` 按钮），并尊重 `prefers-reduced-motion`；
4. **这些面板是插图，不是界面**：没有 `cursor: pointer`、没有假焦点环、没有暗示可点的 hover。
   唯一的 `<button>` 是 `.panel-copy`，它必须挂 `data-copy-self`（`site.js` 真的接了剪贴板）。

**每个面板都必须包在 `<figure class="illus">` 里**，并且必须写 `.illus-tag`（这是什么 + "示意"）
与 `.illus-cap`（数据出处）。这不是装饰——它是"这是插图"的唯一视觉证据。写法见 §1 的完整片段。

---

## 1. 插图外壳 + `.editor`（VS Code 窗口）

### 类清单

| 类 | 作用 |
|---|---|
| `.illus` | `<figure>` 外壳；`display: flow-root` 包住面板 margin |
| `.illus-tag` | 面板名 + "示意"，放在面板**上方**，必写 |
| `.illus-cap` | `<figcaption>` 出处（产生这份数据的命令），必写 |
| `.editor` | 窗口本体：`--code-bg` 底 + `--rule` 边 + `--r-0`，等宽 |
| `.editor-bar` | 标题栏：文件名 + `.editor-mode` |
| `.editor-file` | 文件名，`--ink` 加粗，超长省略号 |
| `.editor-mode` | **自证"我不是界面"**的一行小字（如"只读示意"），靠右 |
| `.editor-tabs` | 标签条，`overflow-x: auto` |
| `.tab` / `.tab.is-active` | 标签；当前页用 `--code-bg` 底 + `--code-ink`，**不用强调色** |
| `.editor-body` | grid：`auto minmax(0,1fr)`（活动栏 + 代码区） |
| `.editor-rail` | 活动栏，宽 `--sp-6`（32px），**无 emoji** |
| `.rail-item` / `.rail-item.is-active` | 活动栏项，内联 SVG；当前项 `--ink` + 左侧墨条 |
| `.editor-code` | 代码滚动区，`overflow-x: auto`，`tabindex="0"` |
| `.code-lines` | `<ol>`，`min-width: max-content`（行号才能 sticky） |
| `.code-line` | 一行：`<span class="ln">` + `<span class="lc">` |
| `.ln` | 行号栏，`--sp-7` 宽、右对齐、`tabular-nums`、`user-select: none`、sticky |
| `.lc` | 代码文本，`white-space: pre`（永不折行） |
| `.code-line.has-hole` | 该行有 `sorry` → **行号**转朱（不额外画色条） |
| `.code-line.is-current` | "光标所在行"的静态表达，底色 `--surface-overlay` |
| `.editor-status` | 状态栏；`.st-item` 通用，`.is-checked` 绿、`.is-open` 朱、`.st-right` 靠右 |

### 完整片段

```html
<figure class="illus">
  <p class="illus-tag">编辑器表面示意 · 只读</p>
  <div class="editor">
    <div class="editor-bar">
      <span class="editor-file">playground.sokonanoda</span>
      <span class="editor-mode">只读示意</span>
    </div>
    <div class="editor-tabs">
      <span class="tab is-active">playground.sokonanoda</span>
      <span class="tab">SetTheory.lean.sokonanoda</span>
    </div>
    <div class="editor-body">
      <div class="editor-rail">
        <span class="rail-item" title="练习"><svg viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M2 4h2M2 8h2M2 12h2M6 4h8M6 8h8M6 12h8"/></svg></span>
        <span class="rail-item" title="课程"><svg viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"><path d="M2 3h5v10H2zM9 3h5v10H9z"/></svg></span>
        <span class="rail-item" title="项目"><svg viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"><path d="M2 4h4l1.5 2H14v7H2z"/></svg></span>
        <span class="rail-item is-active" title="目标面板 (Infoview)"><svg viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M3 3v10M6.5 8H14"/></svg></span>
      </div>
      <div class="editor-code" tabindex="0" role="group" aria-label="代码示意，可横向滚动">
        <ol class="code-lines">
          <li class="code-line"><span class="ln">277</span><span class="lc"><span class="tok-kw">theorem</span> <span class="tok-ty">forall_and</span> <span class="tok-punc">(</span><span class="tok-local">P</span> <span class="tok-punc">:</span> <span class="tok-ty">Person</span> <span class="tok-punc">-</span><span class="tok-punc">&gt;</span> <span class="tok-ty">Prop</span><span class="tok-punc">)</span> <span class="tok-punc">(</span><span class="tok-local">Q</span> <span class="tok-punc">:</span> <span class="tok-ty">Person</span> <span class="tok-punc">-</span><span class="tok-punc">&gt;</span> <span class="tok-ty">Prop</span><span class="tok-punc">)</span> <span class="tok-punc">(</span><span class="tok-local">h</span> <span class="tok-punc">:</span> <span class="tok-kw">forall</span> <span class="tok-punc">(</span><span class="tok-local">x</span> <span class="tok-punc">:</span> <span class="tok-ty">Person</span><span class="tok-punc">)</span><span class="tok-punc">,</span> <span class="tok-ty">And</span> <span class="tok-punc">(</span><span class="tok-local">P</span> <span class="tok-local">x</span><span class="tok-punc">)</span> <span class="tok-punc">(</span><span class="tok-local">Q</span> <span class="tok-local">x</span><span class="tok-punc">)</span><span class="tok-punc">)</span> <span class="tok-punc">:</span> <span class="tok-ty">And</span> <span class="tok-punc">(</span><span class="tok-kw">forall</span> <span class="tok-punc">(</span><span class="tok-local">x</span> <span class="tok-punc">:</span> <span class="tok-ty">Person</span><span class="tok-punc">)</span><span class="tok-punc">,</span> <span class="tok-local">P</span> <span class="tok-local">x</span><span class="tok-punc">)</span> <span class="tok-punc">(</span><span class="tok-kw">forall</span> <span class="tok-punc">(</span><span class="tok-local">x</span> <span class="tok-punc">:</span> <span class="tok-ty">Person</span><span class="tok-punc">)</span><span class="tok-punc">,</span> <span class="tok-local">Q</span> <span class="tok-local">x</span><span class="tok-punc">)</span> <span class="tok-punc">:</span><span class="tok-punc">=</span> <span class="tok-kw">by</span></span></li>
          <li class="code-line"><span class="ln">278</span><span class="lc">  <span class="tok-kw">apply</span> <span class="tok-ty">And.intro</span></span></li>
          <li class="code-line"><span class="ln">279</span><span class="lc">  <span class="tok-kw">intro</span> <span class="tok-local">x</span></span></li>
          <li class="code-line"><span class="ln">280</span><span class="lc">  <span class="tok-kw">exact</span> <span class="tok-ty">And.left</span> <span class="tok-punc">(</span><span class="tok-local">P</span> <span class="tok-local">x</span><span class="tok-punc">)</span> <span class="tok-punc">(</span><span class="tok-local">Q</span> <span class="tok-local">x</span><span class="tok-punc">)</span> <span class="tok-punc">(</span><span class="tok-local">h</span> <span class="tok-local">x</span><span class="tok-punc">)</span></span></li>
          <li class="code-line"><span class="ln">281</span><span class="lc">  <span class="tok-kw">intro</span> <span class="tok-local">x</span></span></li>
          <li class="code-line"><span class="ln">282</span><span class="lc">  <span class="tok-kw">exact</span> <span class="tok-ty">And.right</span> <span class="tok-punc">(</span><span class="tok-local">P</span> <span class="tok-local">x</span><span class="tok-punc">)</span> <span class="tok-punc">(</span><span class="tok-local">Q</span> <span class="tok-local">x</span><span class="tok-punc">)</span> <span class="tok-punc">(</span><span class="tok-local">h</span> <span class="tok-local">x</span><span class="tok-punc">)</span></span></li>
        </ol>
      </div>
    </div>
    <div class="editor-status">
      <span class="st-item">sokonanoda</span>
      <span class="st-item">playground.sokonanoda</span>
      <span class="st-item is-checked">30 checked</span>
      <span class="st-item is-open">4 open</span>
      <span class="st-item st-right">Ln 291, Col 3</span>
    </div>
  </div>
  <figcaption class="illus-cap">源码 <code>playground.sokonanoda:277-282</code>；计数来自
  <code>scripts/soko query check --file playground.sokonanoda</code>（decl_checked 30 / exercise_open 4）。</figcaption>
</figure>
```

### 说明

- **活动栏的四个图标对应真扩展的四个视图**（练习 / 课程 / 项目 / 目标面板，`editor/vscode/package.json:143-165`）。
  内联 SVG、`stroke="currentColor"`、16×16，一套风格；**不用 emoji**（D1 A5）。
- **没有假窗口 chrome**：没有红黄绿点、没有假 URL 栏、没有假菜单栏（D1 A4）。
- 状态栏的绿/朱是**真的**：`30 checked` 是内核真通过的计数，`4 open` 是真的还没解的洞。
  换数字时必须重新跑命令，不许手写。
- `tabindex="0"` 在 `.editor-code` 上：它真的能横向滚动，键盘用户要能滚。
  对应的 `:focus-visible` 是真焦点环，不是装饰。
- 390px 不破页：`.editor-body` 是 grid，代码区自己横滚，**页面本身永不横滚**。

---

## 2. 语法着色 `.tok-*`（9 个类）

### 类清单

| 类 | 颜色令牌 | 语义 |
|---|---|---|
| `.tok-kw` | `--code-kw` | 语言关键字与 tactic（`theorem` / `by` / `intro` / `exact` / `apply` / `forall` …） |
| `.tok-ty` | `--code-ty` | **类型与常量**：sort、inductive、axiom、ctor、def、theorem |
| `.tok-str` | `--code-str` | 字符串字面量（本仓真实源码里没有，实际只出现在事件流的 JSON 里） |
| `.tok-num` | `--code-num` | 数字，附 `tabular-nums slashed-zero` |
| `.tok-cmt` | `--code-cmt` | 注释（`-- …`） |
| `.tok-hole` | `--code-hole` | `sorry` —— 朱批，语义就是"等着判定" |
| `.tok-punc` | `--code-punc` | 标点（`:=` `->` `(` `)` `,` `:`） |
| `.tok-local` | `--ink-2` | **局部假设**：`binder` 与 `unknown_ident`（学习者自己起的名字） |
| `.tok-err` | `--verm-ink` | 内核真的没认出来的标识符；**朱 + 波浪线**（颜色不是唯一通道） |

### `goal_runs` / `ty_runs` 的 `kind` → 类映射（**权威表**）

`kind` 的词表由 `front::semantic::SemanticKind::as_str()` 拥有（16 个，见
`docs/protocol.md` §`soko/stateAt`）。真编辑器（`editor/vscode/media/infoview.js:55`）
发的是逐 kind 的 `tok-<kind>`；本站只有 9 个类，所以**必须**按下面这张表收敛。
收敛依据是 `SemanticKind::tm_scope()` 的真实分组，不是随手合并：

| `kind` | 真编辑器的 scope 分组 | 本站类 | 为什么 |
|---|---|---|---|
| `keyword` | `keyword.other` | `.tok-kw` | 直接对应 |
| `sort` | `storage.type` | `.tok-ty` | sort 是类型 |
| `inductive_name` / `inductive_use` | `storage.type` | `.tok-ty` | 真分组就是 type |
| `axiom_name` / `axiom_use` | `entity.name.type` | `.tok-ty` | 真分组是 type |
| `ctor_name` / `ctor_use` | `entity.name.type` | `.tok-ty` | 真编辑器另给一色；本站 8 色预算下并入"常量" |
| `def_name` / `def_use` | `entity.name.function` | `.tok-ty` | D1 明令**不给函数调用单独配色**（占代码量 75%，配了等于没配） |
| `theorem_name` / `theorem_use` | `entity.name.function` | `.tok-ty` | 同上 |
| `number` | `constant.numeric` | `.tok-num` | 直接对应 |
| `hole` | `markup.inserted` | `.tok-hole` | 直接对应（朱批） |
| `binder` | `variable.parameter` | `.tok-local` | 局部假设 |
| `unknown_ident` | `variable.other` | `.tok-local` | 真编辑器给的是 **variable 色，不是错误色**——目标状态里的 `x` 是局部变量，不是错误 |
| **无 `kind`** | —— | **不加类** | 协议规定：无 kind 的 run 是连接符（空白/标点），**不上色** |

> ⚠️ **`.tok-err` 不在映射表里，这是有意的。** 它标的是**源码里被内核拒绝的字符**
> （诊断 `span` 覆盖的那几个字符），不是 `goal_runs` 的分类结果。
> 把 `unknown_ident` 画成 `.tok-err` 会让目标面板里每个局部变量都变成红色波浪线——
> 那是错的，真编辑器也不那么做。

### 完整片段

```html
<figure class="illus">
  <p class="illus-tag">语法着色示意 · 真词表</p>
  <div class="editor">
    <div class="editor-bar">
      <span class="editor-file">playground.sokonanoda</span>
      <span class="editor-mode">只读示意</span>
    </div>
    <div class="editor-body">
      <div class="editor-code" tabindex="0" role="group" aria-label="代码示意，可横向滚动">
        <ol class="code-lines">
          <li class="code-line"><span class="ln">1</span><span class="lc"><span class="tok-kw">theorem</span> <span class="tok-local">bad2</span> <span class="tok-punc">:</span> <span class="tok-ty">And</span> <span class="tok-ty">True</span> <span class="tok-ty">True</span> <span class="tok-punc">:</span><span class="tok-punc">=</span> <span class="tok-ty">And.intro</span> <span class="tok-err">Tru</span></span></li>
        </ol>
      </div>
    </div>
  </div>
  <figcaption class="illus-cap">这一行是内核真的拒过的输入：
  <code>printf 'theorem bad2 : And True True := And.intro Tru\n' | scripts/soko - --json</code>
  回答 <code>elab-unknown-identifier</code>，<code>span</code> 是 1:43-1:46 —— 那三个字符就是 <code>.tok-err</code>。
  代码区的 <code>.tok-kw</code> 只标 <code>front::semantic::KEYWORDS</code> ∪ 词法关键字 <code>forall</code>
  （<code>crates/front/src/token.rs:432</code>），<code>.tok-ty</code> 只标本文件的真声明名
  （<code>scripts/soko query goals</code> 的 name 集 ∪ <code>PRELUDE_NAMES</code>）。</figcaption>
</figure>
```

> **诚实标注**：代码区的 tok 标记是**按真词表算出来的**，不是内核逐 token 给的；
> 目标面板（§3）不一样——那边的每个 `goal_runs` / `ty_runs` 都是内核直接给的，
> 页面**永远不许重新分词**（`docs/protocol.md` §`soko/stateAt` 的硬要求）。

---

## 3. 目标面板 `.goal`（Infoview）

### 类清单

| 类 | 作用 |
|---|---|
| `.goal` | 面板本体：`--surface` 底 + `--rule` 边 + `--r-0`，等宽 |
| `.goal-head` | 声明行：`.goal-decl` / `.goal-kind` / `.goal-line` / `.goal-status` |
| `.goal-status` | 状态词；`.is-checked` 绿、`.is-open` / `.is-failed` 朱 |
| `.goal-stack` | `<ol>`，多目标的栈 |
| `.goal-item` | 一个目标；第二个起有 `--rule` 分隔线 |
| `.goal-label` | `目标` 或 `目标 k/n`（真面板的原串） |
| `.hyps` / `.hyp` | 假设列表 / 一行假设（`--sp-4` 悬挂缩进） |
| `.hyp-name` / `.hyp-sep` / `.hyp-ty` | `名字` / `:` / `类型`；类型是 `<code>`，`pre-wrap` 折行 |
| `.goal-rule` / `.goal-turn` / `.goal-rule-bar` | **推理横线**：`⊢` + 1px `--rule-strong` 横线（全站记忆点的正位） |
| `.goal-ty` | 待证目标，在横线**之下**，`pre-wrap` 折行 |
| `.goal-progress` | `by 进度 k/n`（真面板的原串） |
| `.goal-solved` | `已无目标` |
| `.goal-empty` | `光标不在任何声明内。` |

### 3.1 多目标（真数据：`and_forall`，内核一次给两个子目标）

```html
<figure class="illus">
  <p class="illus-tag">目标面板示意 · 多目标</p>
  <div class="goal">
    <div class="goal-head">
      <span class="goal-decl">and_forall</span>
      <span class="goal-kind">theorem</span>
      <span class="goal-line">L291</span>
      <span class="goal-status is-checked">checked</span>
    </div>
    <ol class="goal-stack">
      <li class="goal-item">
        <p class="goal-label">目标 1/2</p>
        <ul class="hyps">
          <li class="hyp"><span class="hyp-name">P</span><span class="hyp-sep">:</span><code class="hyp-ty"><span class="tok-ty">Person</span> -&gt; <span class="tok-ty">Prop</span></code></li>
          <li class="hyp"><span class="hyp-name">Q</span><span class="hyp-sep">:</span><code class="hyp-ty"><span class="tok-ty">Person</span> -&gt; <span class="tok-ty">Prop</span></code></li>
          <li class="hyp"><span class="hyp-name">h</span><span class="hyp-sep">:</span><code class="hyp-ty"><span class="tok-ty">And</span> ((<span class="tok-local">x</span> : <span class="tok-ty">Person</span>) -&gt; <span class="tok-local">P</span> <span class="tok-local">x</span>) ((<span class="tok-local">x</span> : <span class="tok-ty">Person</span>) -&gt; <span class="tok-local">Q</span> <span class="tok-local">x</span>)</code></li>
          <li class="hyp"><span class="hyp-name">x</span><span class="hyp-sep">:</span><code class="hyp-ty"><span class="tok-ty">Person</span></code></li>
        </ul>
        <p class="goal-rule"><span class="goal-turn">⊢</span><span class="goal-rule-bar"></span></p>
        <pre class="goal-ty"><span class="tok-local">P</span> <span class="tok-local">x</span></pre>
      </li>
      <li class="goal-item">
        <p class="goal-label">目标 2/2</p>
        <ul class="hyps">
          <li class="hyp"><span class="hyp-name">P</span><span class="hyp-sep">:</span><code class="hyp-ty"><span class="tok-ty">Person</span> -&gt; <span class="tok-ty">Prop</span></code></li>
          <li class="hyp"><span class="hyp-name">Q</span><span class="hyp-sep">:</span><code class="hyp-ty"><span class="tok-ty">Person</span> -&gt; <span class="tok-ty">Prop</span></code></li>
          <li class="hyp"><span class="hyp-name">h</span><span class="hyp-sep">:</span><code class="hyp-ty"><span class="tok-ty">And</span> ((<span class="tok-local">x</span> : <span class="tok-ty">Person</span>) -&gt; <span class="tok-local">P</span> <span class="tok-local">x</span>) ((<span class="tok-local">x</span> : <span class="tok-ty">Person</span>) -&gt; <span class="tok-local">Q</span> <span class="tok-local">x</span>)</code></li>
          <li class="hyp"><span class="hyp-name">x</span><span class="hyp-sep">:</span><code class="hyp-ty"><span class="tok-ty">Person</span></code></li>
        </ul>
        <p class="goal-rule"><span class="goal-turn">⊢</span><span class="goal-rule-bar"></span></p>
        <pre class="goal-ty"><span class="tok-local">Q</span> <span class="tok-local">x</span></pre>
      </li>
    </ol>
    <p class="goal-progress">by 进度 2/4</p>
  </div>
  <figcaption class="illus-cap">逐字来自
  <code>scripts/soko query state --file playground.sokonanoda --line 291 --col 3</code>：
  <code>goals</code> 两个、<code>step 1</code> / <code>total 4</code>。假设与目标的每个 <code>.tok-*</code>
  都对应一个真的 <code>ty_runs</code> / <code>goal_runs</code> run，映射见 §2。</figcaption>
</figure>
```

### 3.2 单目标（真数据：`forall_exists` 的根状态）

```html
<figure class="illus">
  <p class="illus-tag">目标面板示意 · 单目标</p>
  <div class="goal">
    <div class="goal-head">
      <span class="goal-decl">forall_exists</span>
      <span class="goal-kind">theorem</span>
      <span class="goal-line">L347</span>
      <span class="goal-status is-open">open</span>
    </div>
    <ol class="goal-stack">
      <li class="goal-item">
        <p class="goal-label">目标</p>
        <ul class="hyps">
          <li class="hyp"><span class="hyp-name">P</span><span class="hyp-sep">:</span><code class="hyp-ty"><span class="tok-ty">Person</span> -&gt; <span class="tok-ty">Prop</span></code></li>
          <li class="hyp"><span class="hyp-name">h</span><span class="hyp-sep">:</span><code class="hyp-ty">(<span class="tok-local">x</span> : <span class="tok-ty">Person</span>) -&gt; <span class="tok-local">P</span> <span class="tok-local">x</span></code></li>
        </ul>
        <p class="goal-rule"><span class="goal-turn">⊢</span><span class="goal-rule-bar"></span></p>
        <pre class="goal-ty"><span class="tok-ty">Exists</span> <span class="tok-ty">Person</span> <span class="tok-local">P</span></pre>
      </li>
    </ol>
  </div>
  <figcaption class="illus-cap"><code>scripts/soko query state --file playground.sokonanoda --line 347 --col 3</code>
  （<code>step -1</code>：光标在第一条 tactic 之前，拿到的是根状态）。单目标时标签就是「目标」，没有 <code>k/n</code>。</figcaption>
</figure>
```

### 3.3 已无目标

```html
<figure class="illus">
  <p class="illus-tag">目标面板示意 · 证明闭合</p>
  <div class="goal is-solved">
    <div class="goal-head">
      <span class="goal-decl">forall_and</span>
      <span class="goal-kind">theorem</span>
      <span class="goal-line">L277</span>
      <span class="goal-status is-checked">checked</span>
    </div>
    <p class="goal-solved">已无目标</p>
  </div>
  <figcaption class="illus-cap"><code>scripts/soko query goals --file playground.sokonanoda</code> 里
  <code>forall_and</code> 的 <code>goals</code> 是 <code>[]</code>。</figcaption>
</figure>
```

### 3.4 光标不在声明内

```html
<figure class="illus">
  <p class="illus-tag">目标面板示意 · 空态</p>
  <div class="goal is-empty">
    <div class="goal-head">
      <span class="goal-decl">—</span>
      <span class="goal-kind">—</span>
    </div>
    <p class="goal-empty">光标不在任何声明内。</p>
  </div>
  <figcaption class="illus-cap"><code>scripts/soko query state --file playground.sokonanoda --line 285 --col 36</code>
  真的回答 <code>ok:false</code> + <code>error.code: "outside-declarations"</code>。</figcaption>
</figure>
```

### 说明

- **`.goal-rule` 是记忆点的正位**（D1 §记忆点）：假设在上，一条 1px `--rule-strong`
  横线在下，待证在横线之下。`⊢` 与横线在同一个 flex 行里，所以"规则线"和"转牌"
  不可能分家。
- 类型文本用 `pre-wrap` 折行（真面板也是 `white-space: pre-wrap`），**不横向滚动**——
  目标类型经常比 390px 宽，横滚会让"假设 ↔ 目标"的对照读不了。代码区相反，那边不折行。
- `checked` / `open` / `failed` 是**内核给的真状态词**（`query goals` 的 `status`），
  所以这里才允许绿/朱：绿 = 真的过了，朱 = 真的没过或还没解。
- `⊢` 必须落在 `--font-mono` 里（JuliaMono 是唯一覆盖 U+22A2 的面）；
  `.goal` 已经设了 `font-family: var(--font-mono)`，**不要**在面板内部覆盖字族。
- **与真面板的两处差异，都是刻意的**：
  1. 真 Infoview 的空态串是 `已无目标 ✓`，本站写 `已无目标`——D1 的 B1 反 emoji 扫描把
     U+2713 计入 `[\x{2600}-\x{27BF}]`，加了它就过不了那条门禁；
  2. 真面板的目标头是 `<button>`（能跳转），本站是 `<p class="goal-label">`——
     插图里没有跳转这回事，做成按钮就是撒谎。

---

## 4. 行尾期望类型 `.inline-goal`（静态 inlay hint）

### 类清单

| 类 | 作用 |
|---|---|
| `.inline-goal` | 贴在 `sorry` **结束处**的 `: <期望类型>`；`--fs-xs` 等宽、`--ink-3`、`--surface-overlay` 底、`--r-1` 细边 |

**为什么它不会顶开行高**：行内元素的垂直 `padding` / `border` **不参与行盒高度计算**
（CSS 规范），所以 `.inline-goal` 只给 `padding-inline`，`padding-block` 保持 0。
实测：`.inline-goal` 盒高 17px，所在行盒 23px —— 行距零影响。

### 完整片段

```html
<figure class="illus">
  <p class="illus-tag">编辑器表面示意 · 开放练习</p>
  <div class="editor">
    <div class="editor-bar">
      <span class="editor-file">playground.sokonanoda</span>
      <span class="editor-mode">只读示意</span>
    </div>
    <div class="editor-body">
      <div class="editor-code" tabindex="0" role="group" aria-label="代码示意，可横向滚动">
        <ol class="code-lines">
          <li class="code-line"><span class="ln">346</span><span class="lc"><span class="tok-kw">theorem</span> <span class="tok-ty">forall_exists</span> <span class="tok-punc">(</span><span class="tok-local">P</span> <span class="tok-punc">:</span> <span class="tok-ty">Person</span> <span class="tok-punc">-</span><span class="tok-punc">&gt;</span> <span class="tok-ty">Prop</span><span class="tok-punc">)</span> <span class="tok-punc">(</span><span class="tok-local">h</span> <span class="tok-punc">:</span> <span class="tok-kw">forall</span> <span class="tok-punc">(</span><span class="tok-local">x</span> <span class="tok-punc">:</span> <span class="tok-ty">Person</span><span class="tok-punc">)</span><span class="tok-punc">,</span> <span class="tok-local">P</span> <span class="tok-local">x</span><span class="tok-punc">)</span> <span class="tok-punc">:</span> <span class="tok-ty">Exists</span> <span class="tok-ty">Person</span> <span class="tok-local">P</span> <span class="tok-punc">:</span><span class="tok-punc">=</span></span></li>
          <li class="code-line has-hole"><span class="ln">347</span><span class="lc">  <span class="tok-hole">sorry</span><span class="inline-goal">: Exists Person P</span></span></li>
        </ol>
      </div>
    </div>
  </div>
  <figcaption class="illus-cap">形态就是真 inlay hint：<code>crates/lsp/src/inlay.rs:83</code> 的
  <code>": &lt;期望类型&gt;"</code>，位置在洞的结束处（<code>end_position(hole)</code>），
  <code>padding_left: true</code> 对应这里的 <code>margin-inline-start: var(--sp-2)</code>。
  类型取自 <code>scripts/soko query state --file playground.sokonanoda --line 347 --col 3</code>
  的 <code>goal</code> 字段（<code>Exists Person P</code>）。</figcaption>
</figure>
```

### 说明

- 外层那行要加 `class="code-line has-hole"`：`.has-hole` 把**行号**转成朱色，
  和行内的 `sorry` 形成两个不冲突的通道（不是加色条，见 §1）。
- 单洞声明的期望类型 = `query goals` 的 `goal`；多洞/多子目标时必须**按位置顺序**
  对齐 `sub_goals`，不能用 span 反查（`crates/lsp/src/inlay.rs:64-71` 有实测过的坑）。

---

## 5. 波浪线 `.squiggle` 与诊断块 `.diag-pop`

### 类清单

| 类 | 作用 |
|---|---|
| `.squiggle` | 朱色波浪下划线，`text-decoration` 实现（**不是背景图**） |
| `.squiggle.is-warn` | 波浪线改 `--warn-ink`（warning 事件不是诊断） |
| `.diag-pop` | 诊断块：`--surface` 底 + `--rule` 边 + `--shadow-pop`（全站唯一阴影） |
| `.diag-head` | 头行：`.diag-code` / `.diag-stage` / `.diag-at` |
| `.diag-code` | 错误码 chip（`--verm-wash` 底 + `--verm-ink` 字 + `--r-1`） |
| `.diag-stage` | 规范文本行的 stage 字段：`error[kernel]` |
| `.diag-at` | 位置 `行:列`，靠右，`tabular-nums` |
| `.diag-msg` | 内核原话 |
| `.diag-hint` | 教学提示；左侧 2px 朱条 + **必须**有 `.diag-hint-tag` 文字标签（D1 C 档） |
| `.diag-hint-tag` | 提示的文字标签，如 `怎么改` |
| `.diag-pop.is-warn` | warning 变体：chip 与色条改走 `--warn-*` |

### 5.1 error（真数据：`elab-unknown-identifier`）

```html
<figure class="illus">
  <p class="illus-tag">诊断块示意 · error</p>
  <div class="editor">
    <div class="editor-code">
      <ol class="code-lines">
        <li class="code-line"><span class="ln">1</span><span class="lc"><span class="tok-kw">theorem</span> <span class="tok-local">bad2</span> <span class="tok-punc">:</span> <span class="tok-ty">And</span> <span class="tok-ty">True</span> <span class="tok-ty">True</span> <span class="tok-punc">:</span><span class="tok-punc">=</span> <span class="tok-ty">And.intro</span> <span class="tok-err">Tru</span></span></li>
      </ol>
    </div>
  </div>
  <div class="diag-pop">
    <p class="diag-head">
      <span class="diag-code">elab-unknown-identifier</span>
      <span class="diag-stage">error[elab]</span>
      <span class="diag-at">1:43</span>
    </p>
    <p class="diag-msg">unknown identifier `Tru`</p>
    <p class="diag-hint"><span class="diag-hint-tag">怎么改</span>这个名字还没有被定义。检查拼写，或确认它出现在你前面的某个声明里（练习要在解决之后才能被后面的代码引用）。若它在该命名空间里，检查前缀或加 open。</p>
  </div>
  <figcaption class="illus-cap">逐字来自
  <code>printf 'theorem bad2 : And True True := And.intro Tru\n' | scripts/soko - --json</code>：
  <code>stage</code>=<code>elab</code>、<code>code</code>=<code>elab-unknown-identifier</code>、
  <code>span</code>=1:43-1:46。<code>.tok-err</code> 的波浪线位置就是那个 span。</figcaption>
</figure>
```

> `.squiggle` 用在哪：当你要在**散文或表格里**引用一段出错文本时，用
> `<span class="squiggle">Tru</span>`；在代码区里则直接用 `.tok-err`（它已经含波浪线）。

### 5.2 warning（真数据：`redundant-sorry`）

```html
<figure class="illus">
  <p class="illus-tag">诊断块示意 · warning</p>
  <div class="diag-pop is-warn">
    <p class="diag-head">
      <span class="diag-code">redundant-sorry</span>
      <span class="diag-stage">warning[redundant-sorry]</span>
      <span class="diag-at">333:3</span>
    </p>
    <p class="diag-msg">这一行的 sorry 是多余的：前面的项已经完成了证明，sorry 不能再接在这里。</p>
    <p class="diag-hint"><span class="diag-hint-tag">怎么改</span>删掉这一行 sorry，这条声明就会通过内核检查；若还想继续写，请把它换成真正缺少的那部分。</p>
  </div>
  <figcaption class="illus-cap">来自 <code>scripts/soko query check --file playground.sokonanoda</code>
  的 <code>warnings[1]</code>（<code>redundant-sorry</code>，span 333:3-333:8）。
  warning <strong>永不影响退出码</strong>，所以它拿 <code>--warn-*</code>，不拿朱。</figcaption>
</figure>
```

### 说明

- 波浪线用 `text-decoration: underline wavy`，**不是背景图**：背景图不跟随字号、
  换主题要重做一张、打印会掉，而且会把下划线画成色块。
- 颜色不是唯一通道：波浪的**形状**本身就是第二通道（WCAG 1.4.1），红绿色盲也能看见。
- `.diag-pop` 的 `overflow-wrap: anywhere` 不是可选项：内核消息里会整段引用类型
  （如 `` `Pi (b : Sort(0)), Pi (ha : True.[]), …` ``），那是一整串不含空格的 ASCII，
  不给断行权就会把 390px 的页面撑破。

---

## 6. 事件流 `.events`

### 类清单

| 类 | 作用 |
|---|---|
| `.events` | 滚动容器：`--code-bg` 底 + `--rule` 边 + `--r-0`，`overflow-x: auto` |
| `.events-list` | `<ol>`，`min-width: max-content` |
| `.event` | 一行事件：`flex` |
| `.event-seq` | 序号栏，`--sp-6` 宽、右对齐、`tabular-nums`、`user-select: none`、sticky |
| `.event-type` | 事件类型；`.is-checked` 绿 / `.is-open`·`.is-error` 朱 / `.is-warn` 棕 / `.is-info` 中性 |
| `.event-json` | JSON 原文，`white-space: pre`，`flex: 1 0 auto` |
| `.event-src` | 注解列（源码行号），`--sp-8` 宽、右对齐、`user-select: none` |
| `.events.is-annotated` | 打开注解列；不加这个类时 `.event-src` 被 `display: none` |

**事件类型 → 变体的映射（按判定语义，不按好看）：**

| `type` | 变体 | 理由 |
|---|---|---|
| `decl.checked` / `example.checked` | `.is-checked`（绿） | 内核真的过了 |
| `exercise.open` | `.is-open`（朱） | 真的还没解 |
| `diagnostic` | `.is-error`（朱） | 真的判错 |
| `warning` | `.is-warn`（棕） | "请注意"，不是诊断 |
| `expr.typed` / `expr.reduced` / `decl.printed` | `.is-info`（中性） | 只是输出，没有判定 |

### 6.1 基本形态

```html
<figure class="illus">
  <p class="illus-tag">事件流示意 · scripts/soko grade --json</p>
  <div class="events">
    <ol class="events-list">
      <li class="event"><span class="event-seq">1</span><span class="event-type is-checked">decl.checked</span><code class="event-json"><span class="tok-punc">{</span><span class="tok-ty">"human"</span><span class="tok-punc">:</span><span class="tok-str">"checked declaration Prop"</span><span class="tok-punc">,</span><span class="tok-ty">"name"</span><span class="tok-punc">:</span><span class="tok-str">"Prop"</span><span class="tok-punc">,</span><span class="tok-ty">"type"</span><span class="tok-punc">:</span><span class="tok-str">"decl.checked"</span><span class="tok-punc">}</span></code></li>
      <li class="event"><span class="event-seq">2</span><span class="event-type is-checked">example.checked</span><code class="event-json"><span class="tok-punc">{</span><span class="tok-ty">"human"</span><span class="tok-punc">:</span><span class="tok-str">"checked example"</span><span class="tok-punc">,</span><span class="tok-ty">"type"</span><span class="tok-punc">:</span><span class="tok-str">"example.checked"</span><span class="tok-punc">}</span></code></li>
      <li class="event"><span class="event-seq">3</span><span class="event-type is-checked">decl.checked</span><code class="event-json"><span class="tok-punc">{</span><span class="tok-ty">"human"</span><span class="tok-punc">:</span><span class="tok-str">"checked declaration prop_id"</span><span class="tok-punc">,</span><span class="tok-ty">"name"</span><span class="tok-punc">:</span><span class="tok-str">"prop_id"</span><span class="tok-punc">,</span><span class="tok-ty">"type"</span><span class="tok-punc">:</span><span class="tok-str">"decl.checked"</span><span class="tok-punc">}</span></code></li>
      <li class="event"><span class="event-seq">4</span><span class="event-type is-open">exercise.open</span><code class="event-json"><span class="tok-punc">{</span><span class="tok-ty">"human"</span><span class="tok-punc">:</span><span class="tok-str">"exercise open (fill the sorry)"</span><span class="tok-punc">,</span><span class="tok-ty">"name"</span><span class="tok-punc">:</span><span class="tok-str">"exists_intro_rule"</span><span class="tok-punc">,</span><span class="tok-ty">"type"</span><span class="tok-punc">:</span><span class="tok-str">"exercise.open"</span><span class="tok-punc">}</span></code></li>
      <li class="event"><span class="event-seq">5</span><span class="event-type is-open">exercise.open</span><code class="event-json"><span class="tok-punc">{</span><span class="tok-ty">"human"</span><span class="tok-punc">:</span><span class="tok-str">"exercise open (fill the sorry)"</span><span class="tok-punc">,</span><span class="tok-ty">"name"</span><span class="tok-punc">:</span><span class="tok-str">"exists_elim_rule"</span><span class="tok-punc">,</span><span class="tok-ty">"type"</span><span class="tok-punc">:</span><span class="tok-str">"exercise.open"</span><span class="tok-punc">}</span></code></li>
    </ol>
  </div>
  <figcaption class="illus-cap">真 stdout 的节选，逐字来自
  <code>scripts/soko grade playground.sokonanoda --json</code>（完整 38 行）。
  序号栏与注解列都不进剪贴板，复制整块得到的就是逐字 JSON Lines。</figcaption>
</figure>
```

### 6.2 注解变体（每行标出它指的是源码哪一行）

给 `.events` 加 `is-annotated`，并在每行末尾补 `.event-src`：

```html
<figure class="illus">
  <p class="illus-tag">事件流示意 · 标注源码行</p>
  <div class="events is-annotated">
    <ol class="events-list">
      <li class="event"><span class="event-seq">1</span><span class="event-type is-checked">decl.checked</span><code class="event-json"><span class="tok-punc">{</span><span class="tok-ty">"human"</span><span class="tok-punc">:</span><span class="tok-str">"checked declaration Prop"</span><span class="tok-punc">,</span><span class="tok-ty">"name"</span><span class="tok-punc">:</span><span class="tok-str">"Prop"</span><span class="tok-punc">,</span><span class="tok-ty">"type"</span><span class="tok-punc">:</span><span class="tok-str">"decl.checked"</span><span class="tok-punc">}</span></code><span class="event-src">L84</span></li>
      <li class="event"><span class="event-seq">2</span><span class="event-type is-checked">decl.checked</span><code class="event-json"><span class="tok-punc">{</span><span class="tok-ty">"human"</span><span class="tok-punc">:</span><span class="tok-str">"checked declaration prop_id"</span><span class="tok-punc">,</span><span class="tok-ty">"name"</span><span class="tok-punc">:</span><span class="tok-str">"prop_id"</span><span class="tok-punc">,</span><span class="tok-ty">"type"</span><span class="tok-punc">:</span><span class="tok-str">"decl.checked"</span><span class="tok-punc">}</span></code><span class="event-src">L175</span></li>
      <li class="event"><span class="event-seq">3</span><span class="event-type is-open">exercise.open</span><code class="event-json"><span class="tok-punc">{</span><span class="tok-ty">"human"</span><span class="tok-punc">:</span><span class="tok-str">"exercise open (fill the sorry)"</span><span class="tok-punc">,</span><span class="tok-ty">"name"</span><span class="tok-punc">:</span><span class="tok-str">"exists_intro_rule"</span><span class="tok-punc">,</span><span class="tok-ty">"type"</span><span class="tok-punc">:</span><span class="tok-str">"exercise.open"</span><span class="tok-punc">}</span></code><span class="event-src">L331</span></li>
      <li class="event"><span class="event-seq">4</span><span class="event-type is-open">exercise.open</span><code class="event-json"><span class="tok-punc">{</span><span class="tok-ty">"human"</span><span class="tok-punc">:</span><span class="tok-str">"exercise open (fill the sorry)"</span><span class="tok-punc">,</span><span class="tok-ty">"name"</span><span class="tok-punc">:</span><span class="tok-str">"exists_elim_rule"</span><span class="tok-punc">,</span><span class="tok-ty">"type"</span><span class="tok-punc">:</span><span class="tok-str">"exercise.open"</span><span class="tok-punc">}</span></code><span class="event-src">L339</span></li>
    </ol>
  </div>
  <figcaption class="illus-cap">行号来源：<code>decl.checked</code> / <code>exercise.open</code>
  用 <code>scripts/soko query goals</code> 的字节 offset 换算，带 <code>span</code> 的事件直接用
  <code>span.start.line</code>——没有一处是估的。</figcaption>
</figure>
```

### 说明

- **注解列对齐是结构保证的**：`.events-list` 取 `min-width: max-content`（所有行等宽 =
  最宽那行），`.event-json` 是 `flex: 1 0 auto` —— 它把短行的余量吃掉，
  于是 `.event-src` 永远贴右边缘。不需要 grid，也不需要 `display: contents`。
- 一行一个事件，`white-space: pre`，**永不折行**；长 JSON 自己横滚，页面不横滚。
- 序号与注解都 `user-select: none`：复制出来的必须和 stdout 逐字一致。

---

## 7. 终端 `.terminal`

### 类清单

| 类 | 作用 |
|---|---|
| `.terminal` | 转录本体；**排版式边框**：只有上下两条 `--rule-strong` 细线，无圆角盒子、无侧边 |
| `.term-label` | 文字标签（工具名），上下各一条细线 |
| `.term-body` | `<pre tabindex="0">`；`overflow: auto` + `max-height: calc(var(--sp-5) * 16)`（384px） |
| `.term-cmd` | 你要输入的命令，`--ink` |
| `.term-prompt` | `$ ` 前缀，`--ink-3` + `user-select: none`（不进剪贴板） |
| `.term-out` | 你会看到的输出，`--ink-2`，无前缀 |

### 完整片段

```html
<figure class="illus">
  <p class="illus-tag">命令行示意 · scripts/soko</p>
  <div class="terminal">
    <p class="term-label">scripts/soko</p>
    <pre class="term-body" tabindex="0"><span class="term-cmd"><span class="term-prompt">$ </span>scripts/soko version</span>
<span class="term-out">  repo:   v0.61.0 (from Cargo.toml) (darwin-arm64, aarch64-apple-darwin)</span>
<span class="term-out">  cache:  /Users/penglingwei/.local/share/sokonanoda/bin</span>
<span class="term-cmd"><span class="term-prompt">$ </span>scripts/soko doctor</span>
<span class="term-out">  version:  v0.61.0 (from Cargo.toml)</span>
<span class="term-out">  platform: darwin-arm64 (rust: aarch64-apple-darwin)</span>
<span class="term-out">  ready:    yes</span></pre>
  </div>
  <figcaption class="illus-cap">真输出（<code>scripts/soko version</code> / <code>scripts/soko doctor</code>，
  本机 0.61.0）。<code>$ </code> 前缀 <code>user-select: none</code>，复制出来只有命令与输出。</figcaption>
</figure>
```

### 长输出（纵向滚动）

同一个结构，把整段长输出放进 `<pre class="term-body">` 即可；`max-height` 会封顶，
滚动条长在块内部：

```html
<figure class="illus">
  <p class="illus-tag">命令行示意 · 完整事件流（长输出）</p>
  <div class="terminal">
    <p class="term-label">scripts/soko grade playground.sokonanoda --json</p>
    <pre class="term-body" tabindex="0"><span class="term-cmd"><span class="term-prompt">$ </span>scripts/soko grade playground.sokonanoda --json</span>
<span class="term-out">{"human":"checked declaration Prop","name":"Prop","type":"decl.checked"}</span>
<span class="term-out">{"human":"checked example","type":"example.checked"}</span>
<span class="term-out">{"human":"exercise open (fill the sorry)","name":"exists_intro_rule","type":"exercise.open"}</span>
<span class="term-out">{"code":"redundant-sorry","hint":"删掉这一行 sorry，这条声明就会通过内核检查；若还想继续写，请把它换成真正缺少的那部分。","human":"warning[redundant-sorry]: 这一行的 sorry 是多余的：前面的项已经完成了证明，sorry 不能再接在这里。","message":"这一行的 sorry 是多余的：前面的项已经完成了证明，sorry 不能再接在这里。","span":{"end":{"column":8,"line":333,"offset":20823},"start":{"column":3,"line":333,"offset":20818}},"type":"warning"}</span></pre>
  </div>
  <figcaption class="illus-cap">完整 38 行真事件见 <code>.cache/raw-grade.jsonl</code>，
  这里只列到第 4 行做形态示意。块高被 <code>max-height</code> 封在 384px，
  滚动条长在块内部；<code>tabindex="0"</code> 让键盘也能滚。</figcaption>
</figure>
```

### 说明

- **`<pre>` 里的换行就是行分隔符**：第一个 `<span>` 必须紧跟在 `<pre …>` 后面写，
  **最后一个 `</span>` 必须紧贴 `</pre>`**——否则会多出一个空行。
  两个 `<span>` 之间换行是对的（那正是行与行之间的分隔）。
- 命令与输出靠两个通道分开：`$ ` 前缀（形状）+ 墨色深浅（`--ink` vs `--ink-2`）。
- `tabindex="0"` 是必需的（D1 §终端块）：没有它键盘用户滚不动这个块。
- **窄屏把单行控制在 ~60 字符内**（D1）：这不是 CSS 能管的，是选材问题——
  优先选输出短的命令。路径很长的行让它横滚也行，别手动截断输出文本。
- **不许手改输出文本**。路径是本机实测值；若页面不想暴露绝对路径，换一条
  `--json` 命令并只展示需要的字段，而不是把 `/Users/…` 改成 `~`。

---

## 8. 补全列表 `.completion`

### 类清单

| 类 | 作用 |
|---|---|
| `.completion` | `<ul>` 列表：`--surface` 底 + `--rule` 边 + `--shadow-pop`（浮层）+ `--r-0`；`max-width: 100%` |
| `.comp-item` | 一行；`.is-selected` 加 `--surface-sunken` 底 + 左侧 2px 墨条 |
| `.comp-kind` | 种类列，`--sp-8` 定宽 ⇒ 名字左边缘对齐 |
| `.comp-name` | 标识符，`--ink` |
| `.comp-sig` | 签名列，`flex: 1 1 auto` + `min-width: 0` + 省略号（空间不够先截断它） |
| `.comp-detail` | 真 `detail` 串，`flex: none`，靠右 |

**kind 短标签 → 真 `CompletionItemKind`：** `var`=VARIABLE · `kw`=KEYWORD ·
`struct`=STRUCT · `fn`=FUNCTION · `const`=CONSTANT。
`detail` 串来自 `crates/lsp/src/lib.rs:1372-1426`：`本域 binder` / `宇宙` /
`prelude` / `{kind} · {status}`。

### 完整片段

```html
<figure class="illus">
  <p class="illus-tag">补全列表示意</p>
  <ul class="completion">
    <li class="comp-item is-selected"><span class="comp-kind">var</span><span class="comp-name">x</span><span class="comp-sig">Person</span><span class="comp-detail">本域 binder</span></li>
    <li class="comp-item"><span class="comp-kind">kw</span><span class="comp-name">intro</span><span class="comp-sig"></span><span class="comp-detail"></span></li>
    <li class="comp-item"><span class="comp-kind">struct</span><span class="comp-name">Prop</span><span class="comp-sig"></span><span class="comp-detail">宇宙</span></li>
    <li class="comp-item"><span class="comp-kind">const</span><span class="comp-name">True.intro</span><span class="comp-sig"><span class="tok-ty">True</span></span><span class="comp-detail">axiom · solved</span></li>
    <li class="comp-item"><span class="comp-kind">fn</span><span class="comp-name">forall_and</span><span class="comp-sig">forall (P Q : Person -&gt; Prop), (forall (x : Person), And (P x) (Q x)) -&gt; …</span><span class="comp-detail">theorem · solved</span></li>
    <li class="comp-item"><span class="comp-kind">const</span><span class="comp-name">Person</span><span class="comp-sig"><span class="tok-ty">Type</span> <span class="tok-num">0</span></span><span class="comp-detail">axiom · solved</span></li>
  </ul>
  <figcaption class="illus-cap">四列都是真的：kind 是 <code>CompletionItemKind</code>，
  detail 是 <code>crates/lsp/src/lib.rs:1372-1426</code> 里那几串，签名是
  <code>scripts/soko query goals --file playground.sokonanoda</code> 的真 <code>ty_runs</code>
  （<code>Person</code> 那行的 <code>Type 0</code> 就是 <code>sort</code> + <code>number</code> 两个 run）。</figcaption>
</figure>
```

### 说明

- **不给它 `role="listbox"`**：那是"你可以选"的承诺，而这块是插图。
  用 `<ul>` + 选中行 `aria-current="true"` 表达"当前项"，读屏不会以为能按方向键。
  若页面真的做了可交互的补全列表，那是另一个组件，不要复用这个类名。
- 选中态不是唯一通道：除了底色，还有左侧 2px 墨条。
- 真编辑器把 `solved ✓` 印成带勾的；这里按 D1 B1（U+2713 落在反 emoji 扫描区间）
  写作 `solved`，是**有意的、已记录的**差异。

---

## 9. 期望 vs 实际 `.diff`

### 类清单

| 类 | 作用 |
|---|---|
| `.diff` | 两块之间的 grid，`gap: var(--sp-3)` |
| `.diff-side` | 一块；`.is-add` 期望（绿） / `.is-del` 实际（红）；左侧 3px 色条 |
| `.diff-mark` | `+` / `−` 字形列，被一条发丝线隔开（**通道 1 + 4**） |
| `.diff-main` | 右侧内容区 |
| `.diff-head` | 文字标签（`期望（签名要求）` / `实际（内核看到的）`） |
| `.diff-body` | `<pre>`，类型原文，`pre-wrap` 折行 |

### 完整片段

```html
<figure class="illus">
  <p class="illus-tag">差异示意 · kernel-rejected</p>
  <div class="diff">
    <div class="diff-side is-add">
      <p class="diff-mark">+</p>
      <div class="diff-main">
        <p class="diff-head">期望（签名要求）</p>
        <pre class="diff-body">Pi ( : True.[]), False.[]</pre>
      </div>
    </div>
    <div class="diff-side is-del">
      <p class="diff-mark">−</p>
      <div class="diff-main">
        <p class="diff-head">实际（内核看到的）</p>
        <pre class="diff-body">Pi (h : True.[]), True.[]</pre>
      </div>
    </div>
  </div>
  <figcaption class="illus-cap">两个类型都是从 <code>diagnostic.message</code> 里原样切出来的：
  <code>printf 'theorem t1 : True -&gt; False := fun (h : True) =&gt; h\n' | scripts/soko - --json</code>
  回答 <code>类型不匹配：期望 `Pi ( : True.[]), False.[]`，实际是 `Pi (h : True.[]), True.[]`</code>。</figcaption>
</figure>
```

### 说明

- **D1 要求的四个通道同时在**：① 行首 `+` / `−` 字形；② 左侧 3px 色条；
  ③ 背景色；④ 被发丝线隔开的标记列（行号列的等价物）。
  红绿是红绿色盲的经典冲突对，所以 ①④ 这两个**非颜色**通道不是可选项。
- 用 U+2212（`−`）而不是 ASCII `-`：它在 `Soko Mono` 子集里
  （`scripts/gen-site-fonts.py` 的 `MEASURED` 含 `0x2212`），等宽对齐成立，且与 `+` 视觉等重。
- **顺序**：期望在上、实际在下。内核的消息是先说期望再说实际，面板跟着它走。
- 两块都是 `pre-wrap`：类型可以很长，390px 下折行而不是横滚（这里是**对照**，
  折行不影响可比性；代码区才必须横滚）。

---

## 10. 怎么诚实地给这些面板写说明

这一节是**强制**的：静态 HTML 画的产品界面最大的风险，是读者以为它能点。

### 每个面板必须有的三件

1. **`.illus-tag`**：面板名 + "示意"。例：`目标面板示意 · 多目标`。
2. **`.illus-cap`**：`<figcaption>` 里写**产生这份数据的完整命令**。例：
   `scripts/soko query state --file playground.sokonanoda --line 291 --col 3`。
   读者要能照着复现——这是"真数据"唯一可验证的证据。
3. **`.editor-mode`**（只有 `.editor` 需要）：窗口自己带一行 `只读示意`，
   因为编辑器窗口是最容易被误认成真界面的一个。

### 永远不要做

| 不要 | 为什么 |
|---|---|
| 给面板加 `cursor: pointer`、`:hover` 抬升、假 `:focus` 样式 | 那是"可以点"的承诺；面板里除了 `[data-copy]` 按钮没有能点的东西 |
| 加 `role="img"` + `aria-label` 把内容藏起来 | 面板里是真文本，读屏**应该**读得到；`<figure>` + `<figcaption>` 已经是正确的语义 |
| 加 `role="listbox"` / `role="tree"` / `role="textbox"` | 同上，那是可交互控件的角色 |
| 编造数字、编造内核消息、编造路径 | 本项目全部可信度都在"真命令、真输出"上；换数字必须重跑命令 |
| 用 emoji 当图标 | D1 A5；活动栏用内联 SVG，符号用 `--font-symbols` |
| 把 `.tok-*` 用在**非**内核给的类型文本上还声称是内核着色 | 目标面板必须用 `goal_runs` / `ty_runs`，不许重新分词 |

### `.panel-copy`（唯一允许有 hover 的东西）

只有**真的接了剪贴板**时才用：`site.js:108-125` 监听 `[data-copy]` / `[data-copy-self]`，
写入剪贴板并把文案换成"已复制"。

```html
<button type="button" class="panel-copy" data-copy-self data-copy-idle="复制输出" data-copy-done="已复制">复制输出</button>
```

`data-copy-self` 复制最近的 `<pre>` / `<code>` 的 `textContent`——所以行号、`$` 前缀、
序号栏都不会进剪贴板（它们都是 `user-select: none` 的独立元素）。
**没有这条接线就不要用这个类**：静态插图上出现一个点了没反应的按钮，比没有按钮更糟。

---

## 11. 验收记录（本轮实测）

| 项 | 结果 |
|---|---|
| 裸 hex | `grep -nE '#[0-9a-fA-F]{3,8}' site/assets/editor.css` → **0 命中** |
| 字号 | 全部 `var(--fs-*)`，0 处裸 `px/rem/em` 字号 |
| 圆角 | 全部 `var(--r-0)` / `var(--r-1)`，0 处裸值 |
| 间距 | `padding` / `margin` / `gap` 0 处裸 `px`；裸 px 只出现在 1px 发丝线、2px/3px 指示条、`text-decoration-thickness` |
| 固定高 | `^\s*height:` → 0 命中（长输出用 `max-height`） |
| `transition: all` | 0 命中 |
| emoji / dingbats | 0 命中（U+2600–27BF 等区间） |
| 横向溢出 @390px | **真 390px 视口**（iframe）下 `documentElement.scrollWidth == 390`，`PAGE_OVERFLOW=no`；只有 `.editor-code` / `.events` / `.term-body` 三个**设计上就该滚**的容器有内部横滚 |
| 行高不受 `.inline-goal` 影响 | 注记盒 17px < 行盒 23px |
| `⊢` 的字面 | `getComputedStyle(.goal-ty).fontFamily` 首位 = `"Soko Mono"`（JuliaMono，唯一覆盖 U+22A2 的面） |

> ⚠️ **`scripts/site-shot.py` 的 390px 不是真 390px。** 实测：headless Chrome 在 macOS 上
> 把窗口宽度钳在 **500px**，`--window-size=390` 产出的 390×1200 PNG 是**500px 布局的左侧裁切**
> （像素比对：左 390 列 100% 相同）。所以截图"看起来没溢出"不能证明 390px 没问题。
> 本轮的真 390px 验证用 iframe 做（iframe 有独立视口），见 `.cache/probe-390.html`。

---

## 12. 与 `base.css` / `site.css` 的交叉检查（同轮并行落地的两个文件）

`editor.css` **不依赖**它们，但页面会四个一起链（`site/styleguide.html:17-21` 的顺序是
`fonts → tokens → base → site → editor`，`editor.css` **最后**）。已逐个核对同名类：

| 同名类 | 对方在哪 | 结论 |
|---|---|---|
| `.editor` / `.goal` / `.events` / `.terminal` | `base.css:389`，在 `@media print`（开于 `:367`）里 | **不冲突**：只影响打印，且打印时把代码面强制成 `--code-bg` / `--code-ink` 正是想要的。唯一差别是打印会给 `.terminal` 补一圈四边框——纸面上比"只有上下细线"更好读，接受。 |
| `.squiggle` | `site.css:913` | **重复定义**。`site.css` 用的是 `var(--mark)`，而 **`tokens.css` 里没有 `--mark`**（只有 `--mark-wash`），所以那条 `text-decoration-color` 会失效退回 `currentColor`。`editor.css` 排在最后 ⇒ 本文件的 `var(--verm)` 生效，行为正确。本文件已把 `text-decoration-skip-ink: none` 也写进去，成为 `site.css` 那条的超集，**谁后加载行为都一致**。建议下一轮把 `site.css` 的 `.squiggle` 删掉（页面级波浪线也应走 `--verm`）。 |
| `.is-empty` | `site.css:921`（通用工具类，带 `padding: var(--sp-7) 0`） | **会串味**：空态面板是 `<div class="goal is-empty">`。已在 `.goal` 上显式写 `padding: 0`（同优先级 + 后加载 ⇒ 挡得住）。 |

**结论**：四处同名类里只有 `.squiggle` 是真正的重复定义，且当前加载顺序下结果是正确的；
两处（`.squiggle`、`.is-empty`）已在 `editor.css` 里做了防御性处理，**不需要改对方文件**。
