## 当前快照（2026-11-28 · 快思考调度 v2：单元连续 3 题 + 三题探针）

#138：学习引擎**只改调度与展示**（内核判定与题库不动）——同一单元先连做满 3 题才放开
（`unit_probe_target=3`，`pick` 兜底加 `unit_continuity_bonus`）；一个 KP 至少被探 3 次
（`probe_min_questions=3`）才可能结案；作答 1–2 次的 KP 记「探针中」，不再算已解/薄弱
（`map.min_probes=3`）。插件 progress 卡显示「已解 · 探针中 · 待做 · 薄弱」。判据：
`engine.py selftest`（新增 kp-min-probes / probe-min-questions / probe-unit-continuity /
unit-continuity）、真实 state 回放 3 题同单元、插件 host/client 回归、`validate.py`、
课程门禁 G1–G7；Before/After 见 `reports/2026-11-28-quiz-sched-v2/`。

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
