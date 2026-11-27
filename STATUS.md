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

## 当前快照（2026-11-27 · C9.5 调度、复盘与学习地图）
C9.5（#103）：`engine.py` 校准阶梯改由题库推导（不再硬编码 I.1–I.3），`next` 加
`--chapter/--level/--kp` 钻取；`map` 增加 KP **已解/待做/薄弱**（结构计数，阈值进
`params.map`）、按章汇总与 **C9.4 代码题进度**，聊天与插件 progress 卡读同一份快照；
每题耗时进 history（`--elapsed-ms` 优先，否则发出→作答间隔；只汇总、不作判据）；
`state` 加 `metrics`（正确率/byLevel/耗时/KP 覆盖/代码），知识卡阈值改读
`params.wrong_card_threshold`。`review.py` 读同一份状态出薄弱 tag/KP、错题、代码进度、
内容缺口与下一批建议，报告落 gitignore 的 `reports/`。
判据：engine selftest 47、simulate 50、review selftest、validate、插件 host 24 /
client 22 全绿；coverage 99 红是 C9.2 预期欠账。
