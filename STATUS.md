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

## C7b 切分引擎与 capstone（2026-11-25 · #97）
`lib/IntegralCatValue`（56 条 checked：切分、前缀后缀、矩形和、单调性、可积性相加）；
单元174（9 题）与单元175（5 题）解答全 checked、0 open。边界：完整区间可加性还缺
端点替换引擎；通用 Newton–Leibniz 与 Rolle/中值定理在 `ratRep` 层不成立。
