## 当前快照（2026-11-24 · C9 P0）
C9 题目集数据集 P0 落盘：`courses/exercise-dataset/` 从三门课程教材抽出 **1995 道代码练习**（294 单元、解答 1995/1995、4949 条 `soko:hint`，JSON 4.59 MB，生成物进 git 但由 `build.py` 唯一生成）；大计划 issue **#96**。判据：`build.py --check` 判源文件漂移，`check.py` 判结构/来源映射/解答，`check.py --selftest` 对 6 类坏数据必须判红；抽样 `compose.py` → 真内核判卷（模板有 `exercise.open`、解答 0 open、0 diagnostic）。

## C9 题目集数据集 P0（2026-11-24 · issue #96）
- 结构：`model.py` 唯一解析器（课程清单 v1/v2、画布、解答）；`build.py` / `check.py` / `compose.py` 共用。记录带稳定 ID `<course>/[<chapter>/]<unit-stem>/<name>`、`prompt`/`hints`/`template`/`solution`/`source` 映射；匿名 `example` 用 `example-N`。
- 读数：quick-start 66 / 11 单元；set-theory 910 / 112；analysis 1019 / 171；190 道旧练习无 hint 照实记录；`build.py --json` 给出每门课计数与字节数。
- 边界：不改课程源与 kernel/front 判定；选择题层（P2）与插件接线（P3）未做，见 #96 阶段表。

## M6：QA / 文档 / 站点 / 发布（2026-11-21 · issue #22）
M1–M5 完成、分析课 171/171 单元；全量课程门禁 457 目标 / 4512 checked / 0 rejected；站点 `check-site.py` 10/10（`--browser` 12/12）；Arena 门禁经 PR #92 修复后在 `8d4167f5` PASS。书页 v3 与双语范围待用户拍板。

## 发版 0.99.0（2026-11-23 · M6 收口）
`scripts/bump.py 0.99.0` 写四处版本 + 3 个课程清单 + 站点数据；发布内容 = M1–M6 加 LKA 门禁修复；书页 v3/双语仍留 #22 后续。
