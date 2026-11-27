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

## 当前快照（2026-11-27 · C9.6 发布与文档包）
C9.6：把 C9 产物做成可安装、可发现的发布。插件 README 补干净安装、
题库/代码题入口与基数、代码题提交、排障表与隐私边界；`dsh/README`、根 README、
`site/llms.txt` 加入口；技能同步题库/代码题。bump 到 0.100.0，CHANGELOG 列 C9 范围
与已知限制。判据：docs-lint / status-lint / check-site / skill+dsh / 插件测试 /
preflight --fast
