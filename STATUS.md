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

## 当前快照（2026-11-27 · C9.7 侧栏做题面板）
C9.7（#105）：代码练习卡「在侧栏面板中打开」→ 右栏 `sokonanoda-exercise`。
Host 两条 loopback 路由（题目元数据 / 判卷）只从仓库题目集解析（白名单，不含
solution），判卷复用 C9.4 `compose.py` + `scripts/soko grade --json`；仅
`passed:true` 通过，诊断/sorry 失败，服务异常显示不可用；无右栏服务时按钮不渲染；
面板不写 `mark-code`。判据：host 33 / client 45，Before/After 见
`reports/2026-11-27-c97-exercise-panel/`。
