## 当前快照（2026-11-26 · C9.2 首批）
C9.2（#100）先修门禁再扩题：`coverage.py` 改用**可服务题**口径（遵守
`selection.exclude`，与 engine 同口径）；全库 `source` 检查进 `validate.py` 与 CI；
`verify_batch` 卡选项长度与 `why` 句数。
补 3 道门禁题（54 KP 全绿）+ B11/B12（I.8/I.9 各 18 题、4 绿 0 红）。
题库 266（可服务 257），KP 绿 54→62、红 107→99，综合题 39，知识卡 16；
text_audit 欠账 216/98/71 待按章清洗。
下一批：B5（I.4 首波）或 B7（I.5，32 题可清零）；见 CONTENT-PLAN §0.2。

## C7b 切分引擎与 capstone（2026-11-25 · #97）
- 引擎：`lib/IntegralCatValue`（56 条 checked）——`cutHead`/`cutTail`（在 c 处插入切点）、`cutHead_isPartition`/`cutTail_isPartition`、台阶搬运、`cutStepSum`（矩形和沿切开精确相加）、`prefixPart`/`suffixPart`、`sub_add_gap`、`hasIntegral_le_of_nonneg_right`（非负 ⇒ 面积值不下降）、`integrable_cat`（可积性沿切点相加）。
- 单元：174（9 题，解答 9 checked / 0 open）、175（5 题，解答 5 checked / 0 open；卷 III capstone「面积函数的画像」：可导 · 值唯一 · 不降 + 常值曲线可算读数）。
- 边界：完整区间可加性（任意分割的上下和都被 `I1+I2` 夹住）还缺「切点只差一个代表元时替换切点」的端点替换引擎；Newton–Leibniz 通用版与 Rolle/中值定理在有理代表元层不成立（√2 两侧 0/1；`(x²−2)²−3(x−1)` 在 [1,2] 上端点相等而导数无有理零点）——这是数学边界，不是漏做。不新增公理，未改 kernel/front。

## C9.1 题库规格与知识点注册（2026-11-25 · issue #99）
`knowledge/SCHEMA.md` 固化 choice/KP schema 与批次 checklist；I.4–I.13 新增 107 KP（+2 既有 = 109，课程共 161），14 批 B5–B18、bank 目标 493；`validate.py` 新增 keyPoint/prereq 无环/lessons 同步判据。validate OK；coverage 预期红 107（C9.2 补）；selftest/simulate/notation-lint 绿。
