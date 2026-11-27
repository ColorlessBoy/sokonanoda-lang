# 课堂笔记：自动沉淀知识点 / 题目 / 图解的右侧栏闭环

状态：**已实现**（2026-11-27，issue #126）。设计目标只有一个——**聊天是上课，笔记是复习副本**：
看过的知识卡、做过的题、卡片图解自动沉淀，闲聊不进笔记。

## 1. 表面

```text
快路径回复（quiz service）
  → Host: notes.recordReply(payload)
      → 解析 learn_card(kind=knowledge) → upsert notes/<course>.json
      → 读 quiz/*.json 的全部 history → 按标签匹配到已看过的知识卡
  → GET /sokonanoda-notes          JSON 账本（?meta=1 只回 updatedAt/count）
  → GET /sokonanoda-notes-view     自包含 HTML（右侧栏 iframe）
  → GET /sokonanoda-notes.md       同一账本的 Markdown（复制/导出）
  → GET /sokonanoda-notes-asset?path=<whitelist>.png   图解，白名单 PNG
  → 右栏 page tab kind=sokonanoda-notes（iframe 轮询 updatedAt）
```

启动时也会跑一次 `notes.reconcileHistory()`，补齐插件加载前答过的题。
**只有本地 quiz 回复会写笔记**；模型回复、闲聊、工具结果都不经过 `recordReply`。

## 2. 数据模型

`$DSH_HOME/state/sokonanoda-dsh/notes/<course>.json`（默认课程 `set-theory`）：

```json
{
  "schema": "soko.class-notes/1",
  "version": 1,
  "course": "set-theory",
  "updatedAt": "2026-11-27T15:45:45.000Z",
  "notes": [{
    "id": "injective",
    "chapter": "I.2",
    "title": "单射 · Injective",
    "kicker": "卷 I · I.2 函数",
    "oneLiner": "单射 = 不同输入不会撞到同一个输出。",
    "keyPoints": ["…"],
    "definition": { "lang": "lean", "code": "def Function.Injective …" },
    "questions": [{
      "questionId": "I.2.inj-1", "prompt": "…", "options": ["…"],
      "answer": "A", "chosen": "A", "why": "…", "correct": true, "at": "…"
    }],
    "mistakes": [{ "questionId": "I.2.inj-1", "prompt": "…", "why": "…", "at": "…" }],
    "figures": [{ "id": "injective", "file": "injective.png", "src": "/sokonanoda-notes-asset?path=injective.png" }],
    "sources": [{ "label": "lib/Fun.sokonanoda:78" }],
    "tags": ["function", "injective"],
    "firstSeenAt": "…",
    "lastSeenAt": "…"
  }]
}
```

写入是**原子**的（临时文件 + rename），reader 永远看到完整账本。

## 3. 知识卡 → 笔记

- 输入是 quiz service 的原始 chat-completions payload，取
  `choices[0].message.tool_calls[*]` 里 `function.name === "learn_card"` 且
  `arguments.kind === "knowledge"` 的 `payload`。
- 同一张卡再次出现时刷新标题/要点/图解/`lastSeenAt`，不产生重复笔记；
  `firstSeenAt` 与已挂上的题目保留。
- `keyPoints` = `oneLiner` + 各 section 的 `list.items` / `paragraph.text`，去重、截断 12 条；
  表格不塞进要点（避免把复习页变成表格墙）。
- 图解**只**来自 `lib/class-notes-figures.mjs` 的显式映射；没有映射就不显示图。

## 4. 题目 → 笔记

- 读 `quiz/` 下**所有** `*.json` 的 `history`（`default.json` + 指纹文件），
  保留每一次作答；跨文件重复行按 `qid|at|choice|correct` 去重。
- `questions` = 每题**最新**一次作答；`mistakes` = 该题**任意一次**错误作答。
  因此「先错后对」的题在 `questions` 里是 ✓，同时仍留在「易错」——复习账本记录
  发生过什么，不是只记最后一次分数。
- 归属规则与 `engine.py explain_for` 一致：知识卡的 `tags` 与题目的 `tags`
  至少有 **2 个共享标签**，或至少 **1 个非通用标签**
  （`function / logic / types / glossary / order` 视为通用）。匹配不到就不进笔记。
- 只匹配**已经看过的卡**（账本里已有的 note）；引擎展示过但还没进笔记的卡不会
  凭空获得题目。
- 每题最多保留 12 条、易错最多 8 条，按 `at` 倒序。

## 5. 路由与信任边界

四条都是 `auth: 'required'` + **owner-only**（非 owner 一律 403），只读 GET/HEAD，
其它方法 405，`cache-control: no-store`：

| 路由 | 作用 | 防护 |
|---|---|---|
| `/sokonanoda-notes` | 账本 JSON；`?meta=1` 只回 `{ok, updatedAt, count}` | 只读 |
| `/sokonanoda-notes-view` | 自包含 HTML（iframe 用） | 文本全部 HTML-escape |
| `/sokonanoda-notes.md` | Markdown 导出 | — |
| `/sokonanoda-notes-asset?path=` | 图解 | 只接受**白名单 basename + `.png`**；`..` / `/` / 非白名单 400/403/404；不允许任意路径 |

数据在用户目录，不在仓库；`SOKO_DSH_DATA` 可整体搬走。页面不引用外部资源。

## 6. 右侧栏 page tab

- `ctx.sidebarRightTabs.register({ id: 'sokonanoda-dsh/class-notes', kind: 'sokonanoda-notes',
  guide: [...] })`；body 在 `sidebar.right.pane.tab` 槽，chip title 在
  `sidebar.right.pane.tab.title` 槽。
- body 是一个 same-origin iframe，指向 `/sokonanoda-notes-view`；尺寸用内联样式
  （`flex:1 1 auto`），不依赖 `dslc-*` 外部 CSS 注入。
- **自动出现**：每个页面加载启动一个 1.5s poller（只读 `?meta=1`）。第一次
  `updatedAt` 只作为基线；之后只要账本变化且已有笔记，就 `openTab` 一次，
  并停止自动打开。用户手动关掉后，本页不再抢焦点；刷新页面后重新判断。
  已经打开 notes tab 时只靠 iframe 自己刷新，不抢焦点。
- iframe 内页面每秒 poll `updatedAt`：变化时记住滚动位置并 `location.reload()`；
  同时把父窗口的 `--dsw-alias-*` token 与解析后的 font-family 镜像到 iframe，
  暗色/浅色与字体跟随 DSH，而不是跟随 OS。

## 7. 测试

```bash
node dsh/plugins/sokonanoda-dsh/test/run.mjs
```

- Host：卡片提取 / 去重 / 更新、`readQuizHistory` 保留错题、最新作答与易错分离、
  HTML/Markdown 渲染、四条路由的 owner/405/越权、图片白名单与 `..`、启动对账、
  快路径知识卡真实落盘。
- Client：page type + guide 入口、iframe src/title/CSS、purpose 之外不渲染；
  原有 3 个 fence 层契约不变。

## 8. 已知限制

- 图解只映射了少数卡片 id（`injective`、`set-ext`）；补图需要加 PNG 与一行映射。
- 题目归属是 **tag 启发式**，不是 AI 自由总结；匹配不到就跳过，不硬塞。
- 笔记按课程单文件（`<course>.json`）；多课程/多会话还没有独立账本。
- 没有编辑、用户截图、导出接口（Markdown 路由可复制）。
- 自动出现每页只做一次；如果用户首次看到的新笔记发生在 poller 基线之前，
  本页不会自动弹出（guide 入口仍在）。
