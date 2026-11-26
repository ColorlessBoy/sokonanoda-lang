## 当前快照（2026-11-25 · C7b 卷 III capstone）
分析课 **175/175** 单元（新增单元174 区间切分引擎、单元175 卷 III capstone，`lib/IntegralCatValue` 56 条 checked）；切分、前缀/后缀、非负 ⇒ 面积值不下降、可积性相加进库。边界：完整区间可加性还缺「替换切点」端点引擎；通用原函数版 Newton–Leibniz 与 Rolle/中值定理在 `ratRep` 层**不成立**（反例见 `courses/analysis/README.md` 单元175 条目），是数学边界而非待办。判据：`check.py --json` 0 rejected / 0 solutions_open / 0 lib_open（175 单元），`notation-lint`/`docs-lint` rc 0。

## C7b 切分引擎与 capstone（2026-11-25 · #97）
- 引擎：`lib/IntegralCatValue`（56 条 checked）——`cutHead`/`cutTail`（在 c 处插入切点）、`cutHead_isPartition`/`cutTail_isPartition`、台阶搬运、`cutStepSum`（矩形和沿切开精确相加）、`prefixPart`/`suffixPart`、`sub_add_gap`、`hasIntegral_le_of_nonneg_right`（非负 ⇒ 面积值不下降）、`integrable_cat`（可积性沿切点相加）。
- 单元：174（9 题，解答 9 checked / 0 open）、175（5 题，解答 5 checked / 0 open；卷 III capstone「面积函数的画像」：可导 · 值唯一 · 不降 + 常值曲线可算读数）。
- 边界：完整区间可加性（任意分割的上下和都被 `I1+I2` 夹住）还缺「切点只差一个代表元时替换切点」的端点替换引擎；Newton–Leibniz 通用版与 Rolle/中值定理在有理代表元层不成立（√2 两侧 0/1；`(x²−2)²−3(x−1)` 在 [1,2] 上端点相等而导数无有理零点）——这是数学边界，不是漏做。不新增公理，未改 kernel/front。

## C9.1 题库规格与知识点注册（2026-11-25 · issue #99）
`knowledge/SCHEMA.md` 固化 choice/KP schema 与批次 checklist；I.4–I.13 新增 107 KP（+2 既有 = 109，课程共 161），14 批 B5–B18、bank 目标 493；`validate.py` 新增 keyPoint/prereq 无环/lessons 同步判据。validate OK；coverage 预期红 107（C9.2 补）；selftest/simulate/notation-lint 绿。
