## 当前快照（2026-11-27 · C9.7 侧栏做题面板）
C9.7（#105）：代码练习卡「在侧栏面板中打开」→ 右栏 `sokonanoda-exercise`。
Host 两条 loopback 路由（题目元数据 / 判卷）只从仓库题目集解析（白名单，不含
solution），判卷复用 C9.4 `compose.py` + `scripts/soko grade --json`；仅
`passed:true` 通过，诊断/sorry 失败，服务异常显示不可用；无右栏服务时按钮不渲染；
面板不写 `mark-code`。判据：host 33 / client 45，Before/After 见
`reports/2026-11-27-c97-exercise-panel/`。

## 当前快照（2026-11-27 · DSH 源码预览行号 / 跳行 / 懒加载）

#116：`.sokonanoda` 右侧预览正式对齐普通代码预览。行号改为按 owner 分页的
`page.offset + index` 计算（不再重排切片、不再用整份 `content.text` 重切），滚到末尾
再回顶部不重置；每行挂原生 `data-textpreview-line` 锚点，owner 的
`scrollToLine`/`markLine` 能直接跳到真实行并闪烁，目标页未加载时先加载再定位；补
Lean 高亮、复制按钮（成功/失败态）、空文件态与三位数 gutter 宽度；`sidebarRight`
服务晚到时也补注入，知识卡「出处」不再因加载顺序变成死链。判据：插件 client 36 /
host 29；真 DSH Web + 真内核 e2e（打开 unit10-cantor、滚动/回顶行号稳定、点出处定位）。

## 当前快照（2026-11-27 · DSH 源码预览光标 + goal 面板）

#115：`.sokonanoda` 右侧预览加只读光标：`↑/↓`/点击移动活动行并高亮；下方面板显示声明名、
status、`step/total`、假设、`⊢ goal`（`goal_runs`/`ty_runs` 上色，不重新分词），区分
「不在声明内 / 源码错误 / 超时 / 后端不可用 / 已无目标」。Host 新增 owner-only
`GET /sokonanoda-state`：只读、路径限工作区/仓库（拒 `..`、绝对路径、符号链接逃逸）、
参数数组调 `scripts/soko query state`、3 s 超时杀进程组；client 120 ms 去抖 + 单飞 + seq。
判据：插件 host 29 / client 28；真 DSH Web + 真内核 e2e；docs/status lint。
