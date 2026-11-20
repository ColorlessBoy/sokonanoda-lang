## M6 P2：文档对齐 171/171 与 M6 状态（2026-11-20 · issue #22）
M1–M5 已全部完成，分析课 171/171 单元。M5 的 21 个单元在 2026-11-19 落盘，PR #62 于 2026-11-20 合并。根 README 增加分析课入口与课程目录位置，ROADMAP 记下 C8 的验收位置，ROUTE 的抬头从 M0 设计稿改为当前进度，分析课 README 去掉「M1 仍未收口」的旧句并补上 III.8 与 III.9 的入口。M6 目前只做 QA 与发布，书页媒介与双语范围等用户拍板，尚未开工。
复跑 `python3 courses/analysis/tools/check.py --json` 得 457 个目标、4512 checked、0 rejected、0 solutions_open、0 lib_open；`python3 scripts/notation-lint.py --root courses/analysis` 扫 470 个文件 rc=0；`python3 scripts/docs-lint.py` rc=0。

## M6 P1：全量课程门禁实测（2026-11-20 · issue #22）
- `python3 courses/analysis/tools/check.py --json` rc=0：457 个目标（171 单元 / 2 卷 / 16 章）、4512 checked、1019 个待作答练习（画布，属正常）、0 rejected、0 solutions_open、0 lib_open；判卷二进制 v0.98.4（repo-build）。`check.py --selftest` PASS。
- `python3 scripts/notation-lint.py --root courses/analysis` rc=0：470 个文件零旧写法，15 处 `soko:notation-ok` 显式豁免。
- 计时：同一命令暖缓存复跑 63 秒；冷跑首轮同样 0 判负。
- 边界：只跑门禁与记数，不改课程内容；书页、站点与双语等 M6 其余工作待用户拍板媒介后开工。

## M4 单元144：非零因子消去（2026-11-16 · issue #20 / PR #47）
- `unit144-cancel-nonzero`：10 题。两条斜率函数在 a 外逐点满足 `(x-a)·q x ~ f x - f a`，接起来得 `(x-a)·q x ~ (x-a)·r x`；x ≠ a 时约掉 x-a。课程没有倒数，消去回到代表元：u ≠ 0 即分子 p ≠ q，大的写成小的加 k+1，用 ℕ 正因子消去律约掉；有理数层先对齐公共分母。由此得 `ratMul_cancel`、`isSlope_agree`、`slope_agree_near`、`hasDerivAt_unique`，恒等函数的导数只能是 1。正文 10 题，解答 29 checked、0 open。
- 引擎：新建 `lib/RatZero`（4）、`lib/IntCancel`（16）、`lib/RatCancel`（4）、`lib/DerivUnique`（6），`lib/Demo` 补 import 与四条自检。
- 判据：`check.py --json` rc=0（383 目标 · 3713 checked · 806 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 21/27）；`notation-lint` rc=0（396 文件，15 处豁免）；触及文件文案 0（BLOCK/WARN/INFO 全 0）；`style-lint --selftest` rc=0；`docs-lint` rc=0。
- 边界：函数停在有理代表元层；不构造倒数，不新增公理，也不碰 kernel/front，不改 M1/M2/M3。
