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

## 当前快照（2026-11-27 · C9.4 代码题真内核判卷）
C9.4（#102）：插件新增「代码题」快路径 + `engine.py code-next`（从题目集数据集发具名
代码题，默认 set-theory；懒加载，不改 `codeEvery` 瀑布）。代码练习卡显示题干/提示/带
`sorry` 模板；用户贴围栏代码后取证明体、贴回课程模板的可信签名 ⇒ `compose.py` 组装
⇒ `scripts/soko grade --json`；目标 `decl.checked` 且无 `exercise.open` 且 0
`diagnostic` 才算通过。通过调 `mark-code`，失败显示首条内核诊断并允许同题重试，
「跳过」调 `skip-code`；compose/soko 缺失或超时 503 回落 `next()`，用户代码不进日志。
插件测试 host 24 / client 22 绿。
