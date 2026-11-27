## 当前快照（2026-11-27 · C9.4 代码题真内核判卷）
C9.4（#102）：插件新增「代码题」快路径 + `engine.py code-next`（从题目集数据集按
确定顺序发具名代码题，默认 set-theory；懒加载，不改 `codeEvery` 瀑布）。代码练习卡
显示题干/提示/带 `sorry` 模板；用户贴围栏代码后只取证明体、贴回课程模板的可信签名
⇒ `compose.py` 组装完整文件 ⇒ `scripts/soko grade --json`；目标 `decl.checked` 且
无 `exercise.open` 且 0 `diagnostic` 才算通过。通过调 `mark-code` 记进度，失败显示
首条内核诊断并允许同题重试，「跳过这道代码题」调 `skip-code`。compose/soko 超时或
缺失一律 503 回落 `next()`；用户代码不进日志/仓库。插件测试 host 24 / client 22 绿。

## 当前快照（2026-11-26 · C9.3 卷 II.1 分析课）
C9.3（#101）：同一份题库承载卷 II.1 分析课（单元①–⑲）——新增 34 KP / 143 题
（L1 23 / L2 77 / L3 43，综合 31）、19 lesson、3 知识卡、13 术语；II.1 34 绿 / 0 红，
全库 KP 96 绿 / 99 红、题库 409 道（可服务 400）、综合 70。跨课程 source
（`courses/analysis/…`）解析补进 `source_check.py`；validate / selftest / simulate /
notation-lint 绿，coverage 的 99 红是 C9.2 的 I.4–I.13 预期欠账。下一批见
`knowledge/CONTENT-PLAN.md`。

## 当前快照（2026-11-26 · C9.2 首批）
C9.2（#100）先修门禁再扩题：`coverage.py` 改用**可服务题**口径（遵守
`selection.exclude`，与 engine 同口径）；全库 `source` 检查进 `validate.py` 与 CI；
`verify_batch` 卡选项长度与 `why` 句数。
补 3 道门禁题（54 KP 全绿）+ B11/B12（I.8/I.9 各 18 题、4 绿 0 红）。
题库 266（可服务 257），KP 绿 54→62、红 107→99，综合题 39，知识卡 16；
text_audit 欠账 216/98/71 待按章清洗。
下一批：B5（I.4 首波）或 B7（I.5，32 题可清零）；见 CONTENT-PLAN §0.2。
