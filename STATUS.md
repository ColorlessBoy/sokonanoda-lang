## 当前快照（2026-11-26 · C9.3 B1–B3）
C9.3（#101）：同一份题库新增 II.1 单元①–⑨ 的 22 KP / 91 题（L1 16 / L2 48 / L3 27，
综合 22）、9 lesson、3 卡片、13 术语；II.1 22 绿 / 0 红。validate OK；
selftest/simulate 绿。全库 107 红是 C9.2 的 I.4–I.13 预期红。计划见
`courses/set-theory/knowledge/CONTENT-PLAN.md` §2.2。

## 当前快照（2026-11-25 · C7b 卷 III capstone）
分析课 **175/175** 单元（新增单元174 区间切分引擎、单元175 卷 III capstone，`lib/IntegralCatValue` 56 条 checked）；切分、前缀/后缀、非负 ⇒ 面积值不下降、可积性相加进库。边界：完整区间可加性还缺「替换切点」端点引擎；通用原函数版 Newton–Leibniz 与 Rolle/中值定理在 `ratRep` 层**不成立**（反例见 `courses/analysis/README.md` 单元175 条目），是数学边界而非待办。判据：`check.py --json` 0 rejected / 0 solutions_open / 0 lib_open（175 单元），`notation-lint`/`docs-lint` rc 0。

## C7b 切分引擎与 capstone（2026-11-25 · #97）
`lib/IntegralCatValue`（56 条 checked：切分、前缀后缀、矩形和、单调性、可积性相加）；
单元174（9 题）与单元175（5 题）解答全 checked、0 open。边界：完整区间可加性还缺
端点替换引擎；通用 Newton–Leibniz 与 Rolle/中值定理在 `ratRep` 层不成立。

## C9.1 题库规格与知识点注册（2026-11-25 · issue #99）
`knowledge/SCHEMA.md` 固化 choice/KP schema 与批次 checklist；I.4–I.13 新增 107 KP（+2 既有 = 109，课程共 161），14 批 B5–B18、bank 目标 493；`validate.py` 新增 keyPoint/prereq 无环/lessons 同步判据。validate OK；coverage 预期红 107（C9.2 补）；selftest/simulate/notation-lint 绿。
