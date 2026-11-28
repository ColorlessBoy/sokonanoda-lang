## 当前快照（2026-11-28 · 知识卡匹配改 kp/IDF 评分 · #142）

#142：`explain_for` 改 **kp 优先 → IDF 稀有标签加权 → 稳定破平**（kps 命中数 →
标签数 → 卡片 id，不依赖 `cards.json` 顺序）；`cards.json` 补 `set-pi-lemmas` /
`cardinal-le-mono` 两张真卡（正文取 live 草稿、标签只留描述性的）并为 21 张既有卡补 `kps`；`validate.py` 判卡片
`kps` 引用、`engine.py selftest` 新增 4 条 explain-* 判据并接进 CI `gates-course`。
Before/After 见 `reports/2026-11-28-card-scoring/`。

## 当前快照（2026-11-28 · 知识卡 sections 完整渲染 · #139）

#139：`sokonanoda-dsh` 知识卡按 engine `sections` 顺序渲染**全部**段——`list` 全部
items（不再 `slice(0, 2)`）、`paragraph` 的 `text`、`table` 的 `columns`/`rows`
（与课程海报渲染器同形）都进卡；标题 / 一句话 / `definition.code` / `source` 位置
不变，长卡片换行 + 聊天滚动，不静默截断；纯文本快路径（`cards=false`）同步去掉
sections/items 截断。判据：插件 host 41 / client 47，真 DSH Web Before/After 见
`reports/2026-11-28-knowledge-card-sections/`。

## 当前快照（2026-11-28 · dsh-images 画册缩放 / 预加载 / 移动端）

#136：`dsh-images` 在 v1 翻页之上补齐交互——点页/`⤢` 开全屏 lightbox（双指/滚轮
100–400%、拖拽、双击 250%、`Esc`、焦点回到 stage）、`Home`/`End`/`PageUp`/`PageDown`、
首屏只 eager 第一页 + ±1 预加载、坏图保留路径可重试、24px 圆点/底部安全区/边缘提示、
aria-live 页码、深色 token、`prefers-reduced-motion`；Host health 报 `clientAgeMs`
供 HMR 未生效时提示刷新；图片/health 路由改为 `auth: required`。7 页样例进
host 9 / client 16 回归并加 CI job。Before/After 见 `reports/2026-11-28-dsh-images/`。
