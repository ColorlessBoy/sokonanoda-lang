## 当前快照（2026-11-27 · C9.7 侧栏做题面板）
C9.7（#105）：代码练习卡「在侧栏面板中打开」→ 右栏 `sokonanoda-exercise`。
Host 两条 loopback 路由（题目元数据 / 判卷）只从仓库题目集解析（白名单，不含
solution），判卷复用 C9.4 `compose.py` + `scripts/soko grade --json`；仅
`passed:true` 通过，诊断/sorry 失败，服务异常显示不可用；无右栏服务时按钮不渲染；
面板不写 `mark-code`。判据：host 33 / client 45，Before/After 见
`reports/2026-11-27-c97-exercise-panel/`。

## 当前快照（2026-11-27 · 课堂笔记右侧栏闭环）
#126：快路径知识卡 / 答题历史自动沉淀成右栏「课堂笔记」——知识点、✓/× 题目、易错与白名单图解；闲聊不入账。账本 `notes/<course>.json`（原子写）由四条 owner-only 只读路由服务；page tab + guide，iframe 轮询 `updatedAt`，首次变化自动打开一次。判据：插件 host 40 / client 46；真 DSH Web Before/After（`reports/2026-11-27-class-notes/`）。

## 当前快照（2026-11-27 · DSH 源码预览行号 / 跳行 / 懒加载）

#116：`.sokonanoda` 右侧预览正式对齐普通代码预览。行号改为按 owner 分页的
`page.offset + index` 计算（不再重排切片、不再用整份 `content.text` 重切），滚到末尾
再回顶部不重置；每行挂原生 `data-textpreview-line` 锚点，owner 的
`scrollToLine`/`markLine` 能直接跳到真实行并闪烁，目标页未加载时先加载再定位；补
Lean 高亮、复制按钮（成功/失败态）、空文件态与三位数 gutter 宽度；`sidebarRight`
服务晚到时也补注入，知识卡「出处」不再因加载顺序变成死链。判据：插件 client 36 /
host 29；真 DSH Web + 真内核 e2e（打开 unit10-cantor、滚动/回顶行号稳定、点出处定位）。

