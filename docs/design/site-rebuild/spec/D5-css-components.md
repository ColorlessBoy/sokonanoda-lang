# D5 — 页面级组件（`site/assets/base.css` + `site.css`）

> 页面作者照着这一份抄。编辑器面板（`.editor` / `.goal` / `.events` / …）在
> **`D6-editor-components.md`**，不在这里。
>
> 唯一实现：`site/assets/base.css`（reset / 方格纸 / 排版 / 布局原语）、
> `site/assets/site.css`（页头页脚 + 共享组件）。
> 令牌唯一源：`site/assets/tokens.css`。样板页：`site/styleguide.html`。

## 0. 硬规则（违反即 bug）

| 规则 | 检查方式 |
|---|---|
| 颜色只走令牌，**不写裸 hex** | `grep -nE '#[0-9a-fA-F]{3,8}' site/*.html` 无命中 |
| 字号只用 `--fs-*`（加 `--fs-hero`） | 无手写 `font-size: 15px` |
| 圆角只用 `--r-0`（结构）/ `--r-1`（控件）/ `--r-full`（状态点） | 无第四种 |
| 间距只用 `--sp-*` | 无手写 `padding: 18px` |
| 区块级垂直间距是 **24 的整数倍** | 格线才对得上 |
| 无内联 `style=` | `check-site.py` 断言 |
| 无 `<img>` / 位图 | 同上 |
| 无入场动画 | 只有状态反馈 |

**被点名禁止的 AI 长相**：居中 hero、标题上方的 eyebrow 标签、链接尾巴 `→`、
emoji 当图标、所有容器同一个圆角、三张一样的卡片、紫蓝渐变。

---

## 1. 骨架

```html
<body data-page="PAGE" data-root="./">
<!--#nav-->
<!--/#nav-->
<main id="main">
  <div class="wrap">
    <nav class="breadcrumbs" aria-label="面包屑">…</nav>
    <header class="page-head">
      <p class="sec-num">§1</p>
      <h1>标题</h1>
      <p class="lead">引言</p>
    </header>
    …
  </div>
</main>
<!--#footer-->
<!--/#footer-->
<script src="assets/site.js"></script>
</body>
```

| 类 | 作用 |
|---|---|
| `.wrap` | **唯一**水平容器；不透明底（挡住方格），左右内边距 24/48 |
| `.measure` | 正文栏，`40rem` ≈ 37.6 汉字 ≈ 78 拉丁字符 |
| `.wide` | 宽版块（表格、面板、卡片网格） |
| `.stack` | 子元素之间 24px 纵向流 |
| `.row` | 横排 + 换行 + 16px gap（按钮组、标签组） |
| `.page-head` | 页头区，上下 96/48 |
| `.lead` | 引言，19px 次要墨色 |
| `.rule` | 一条 `hr` 分隔（只在真的换话题时用） |

---

## 2. 教科书构件（"教科书感"由它们承担，不由衬线承担）

| 类 | 用途 |
|---|---|
| `.sec-num` | 小节编号（`§2.3`），放在标题**里面**开头 |
| `.marginalia` | 页边注。配 `.has-marginalia` 父容器：宽屏进边栏，窄屏折回正文流 |
| `.statement` + `--definition` / `--theorem` / `--lemma` / `--example` | 定理/定义/例块；变体只改左边线颜色 |
| `.statement__label` | 块内标签行 |
| `.proof` | 证明块，`::after` 自动画墓碑符（不用字符，避免缺字） |
| `.display` | 展示式公式，居中、上下发丝线 |

```html
<div class="statement statement--theorem">
  <span class="statement__label">定理 1.2（空集是任何集合的子集）</span>
  <p>对任意集合 <code>A</code>，有 <code>∅ ⊆ A</code>。</p>
</div>
<div class="proof"><p>取任意 <code>x</code>，设 <code>x ∈ ∅</code>……</p></div>
```

---

## 3. 推理横线（全站唯一记忆点）

每页 2–3 次，位置固定。`⊢` 必须由 `--font-symbols` 渲染（没有系统字体覆盖它）。

```html
<div class="turnstile">
  <ul class="turnstile__given">
    <li><span class="tok-local">P</span> : <span class="tok-ty">Prop</span></li>
    <li><span class="tok-local">h</span> : <span class="tok-local">P</span></li>
  </ul>
  <p class="turnstile__rule"><span class="turnstile__turn">⊢</span></p>
  <p class="turnstile__goal"><span class="tok-local">P</span></p>
</div>
```

---

## 4. 按钮与链接

| 类 | 何时用 |
|---|---|
| `.btn.btn--primary` | **一屏最多一个**；只给"真的能立刻做成的那件事" |
| `.btn.btn--secondary` | 次要动作 |
| `.btn.btn--quiet` | 文字型（"读文档"这类） |

两个 CTA 之间要表达"二选一"时**明写 `OR`**（访客确实只有两类）。

---

## 5. 代码与数据

| 类 | 用途 |
|---|---|
| `.code` | 代码图：可选 `.code__label` 语言标签 + 复制按钮 + `<pre><code>` |
| `.cmd` | 终端块；**必须**配 `.cmd__note` 写明"`$` 是提示符，不用输入" |
| `.json` | 页面级 JSON 块（面板级用 D6 的 `.events`） |
| `.diag` / `.diag__code` / `.diag__stage` / `.diag__msg` / `.diag__hint` | 一条诊断 |
| `.kbd` | 按键 |

复制按钮的契约（`site.js`）：
`<button type="button" data-copy="#some-id" data-copy-idle="复制" data-copy-done="已复制">复制</button>`

**不要写 `hidden`**：`site.css` 用 `html.js` 门控显示，`site.js` 会往 `<html>` 加 `.js`。
无 JS 时按钮不出现（一个按不动的按钮比没有按钮更糟），有 JS 时才显示。

---

## 6. 表格

`.table-scroll` 是**必需**包装（窄屏横滚，且不撑破页面），里面放 `.table`。
数值列加 `.num`（等宽数字 + 右对齐）。`<caption>` 必写。

```html
<div class="table-scroll">
  <table class="table">
    <caption>这张表在比什么</caption>
    <thead><tr><th scope="col">口径</th><th scope="col" class="num">checked</th></tr></thead>
    <tbody><tr><th scope="row">卷 I</th><td class="num">329</td></tr></tbody>
  </table>
</div>
```

---

## 7. 区块（结构上必须各不相同）

| 类 | 形状 |
|---|---|
| `.unit-card` | 课程单元：编号 / 标题 / 元信息，像目录不像卡片 |
| `.feature` | 左主张 + **右真实样例**；窄屏上下堆叠 |
| `.callout--verified` | 绿：背后有真实内核判定 |
| `.callout--limit` | 朱：诚实的限制 / 未做 / 已知问题 |
| `.callout--note` | 黄：只是请注意 |

**审计规则**：看到强调色就问"这里背后有什么真实判定"。一个元素只带一个语义色。

---

## 8. 数据展示

| 类 | 用途 |
|---|---|
| `.stats` / `.stats__n` / `.stats__n--verified` / `.stats__label` | 大数字；`--verified` 才用绿 |
| `.chip` + `--checked` / `--open` / `--failed` / `--version` | 状态徽标 |
| `.timeline` | 竖线 + 日期 + 标题 + 正文 |
| `.ledger` | 密排台账表（斑马纹） |
| `.filter` + `[data-filter]` | 筛选框；**无 JS 时不显示**（`site.js` 负责显形） |
| `.meta-row` | 版本/平台/许可这类元信息横排 |

---

## 9. 折叠与标签页（都无 JS 成立）

**折叠是走查页的核心机制**（Alectryon 同款）：

```html
<div class="fold">
  <input class="fold__toggle visually-hidden" type="checkbox" id="f1">
  <label class="fold__head" for="f1">theorem and_forall : …</label>
  <div class="fold__body">…真实目标状态…</div>
</div>
```

展开指示是 CSS 画的三角（不用 `▸` 字符）；`:focus-visible` 画在 label 上。

标签页用 `<input type="radio" class="tabs__input">` + `.tabs__list` / `.tabs__tab` / `.tabs__panel`。

---

## 10. 状态与零散件

`.squiggle`（诊断波浪线）、`.is-empty`（空态）、`.small` / `.muted` / `.faint`、
`.visually-hidden`、`.skip-link`（由页头提供）、`.breadcrumbs`、`.pager`、`.toc`。
