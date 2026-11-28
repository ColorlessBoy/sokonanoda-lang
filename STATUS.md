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

## 当前快照（2026-11-27 · 课堂笔记右侧栏闭环）
#126：快路径知识卡 / 答题历史自动沉淀成右栏「课堂笔记」——知识点、✓/× 题目、易错与白名单图解；闲聊不入账。账本 `notes/<course>.json`（原子写）由四条 owner-only 只读路由服务；page tab + guide，iframe 轮询 `updatedAt`，首次变化自动打开一次。判据：插件 host 40 / client 46；真 DSH Web Before/After（`reports/2026-11-27-class-notes/`）。
