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

## M4 单元142–143：极限的唯一性与局部性（2026-11-15 · issue #20 / PR #47）
- `unit142-limit-unique`：8 题。两条极限各给 ε/2 与一个半径，取小半径后在 `a + δ/2` 上接起来，`close_trans` 合成 `close L M ε`，`ratRel_of_close_all` 收成 L ~ M。另证换值、常函数与恒等函数的唯一性，以及常函数零不以 1 为极限。解答 8 checked、0 open。
- `unit143-limit-local`：1 定义 + 11 题。`AgreeNear f g a` 说有一个正半径，邻域里（除 a 外）两函数逐点相等；两个半径取小，接近性沿 `f x ~ g x` 搬运，得到极限不变与反方向；局部等于常数 c 的极限只能是 c。解答 12 checked、0 open。
- 引擎：新建 `lib/LimUnique`（8）与 `lib/LimLocal`（12）；`lib/Demo` 补 import 与四条自检。
- 判据：`check.py --json` rc=0（377 目标 · 3646 checked · 796 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 20/27）；`notation-lint` rc=0（390 文件，15 处既有豁免）；触及文件文案 0（BLOCK/WARN/INFO 全 0）；`style-lint --selftest` rc=0；`docs-lint` rc=0。
- 边界：函数停在有理代表元层；这里证的是极限值的唯一性，斜率函数逐点相同还要消去非零因子，留到后续单元。不新增公理，也不碰 kernel/front，不改 M1/M2/M3。
