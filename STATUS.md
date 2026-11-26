## 当前快照（2026-11-26 · C9.2 首批）
C9.2（#100）先修门禁再扩题：`coverage.py` 改用**可服务题**口径（遵守
`selection.exclude`，与 engine 同口径）；全库 `source` 检查进 `validate.py` 与 CI；
`verify_batch` 卡选项长度与 `why` 句数。
补 3 道门禁题（54 KP 全绿）+ B11/B12（I.8/I.9 各 18 题、4 绿 0 红）。
题库 266（可服务 257），KP 绿 54→62、红 107→99，综合题 39，知识卡 16；
text_audit 欠账 216/98/71 待按章清洗。
下一批：B5（I.4 首波）或 B7（I.5，32 题可清零）；见 CONTENT-PLAN §0.2。

## C9 题目集数据集 P0（2026-11-24 · issue #96）
- 结构：`model.py` 唯一解析器（课程清单 v1/v2、画布、解答）；`build.py` / `check.py` / `compose.py` 共用。记录带稳定 ID `<course>/[<chapter>/]<unit-stem>/<name>`、`prompt`/`hints`/`template`/`solution`/`source` 映射；匿名 `example` 用 `example-N`。
- 读数：quick-start 66 / 11 单元；set-theory 910 / 112；analysis 1032 / 173；190 道旧练习无 hint 照实记录；`build.py --json` 给出每门课计数与字节数。
- 边界：不改课程源与 kernel/front 判定；选择题层（P2）与插件接线（P3）未做，见 #96 阶段表。

## M6：QA / 文档 / 站点 / 发布（2026-11-21 · issue #22）
M1–M5 完成、分析课 171/171 单元；全量课程门禁 457 目标 / 4512 checked / 0 rejected；站点 `check-site.py` 10/10（`--browser` 12/12）；Arena 门禁经 PR #92 修复后在 `8d4167f5` PASS。书页 v3 与双语范围待用户拍板。

## 发版 0.99.0（2026-11-23 · M6 收口）
`scripts/bump.py 0.99.0` 写四处版本 + 3 个课程清单 + 站点数据；发布内容 = M1–M6 加 LKA 门禁修复；书页 v3/双语仍留 #22 后续。

## FTC 补证（2026-11-24 · issue #21 后续）
通用原函数版在 `ratRep` 层不成立（√2 两侧 0/1 反例：F′=0 但 F(b)−F(a)=1≠∫f=0）。用户拍板 B：加 `Exists.choose` 与倒数/商库，做面积函数这一半。已证 `lib/IntegralFTC.integral_hasDerivAt`（52 条 checked，含无除法差界与斜率极限）；单元172（9/9）与单元173（4/4）进课程。边界：通用原函数版仍不成立；见 PR #107。
