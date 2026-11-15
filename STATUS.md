## M4 单元142–143：极限的唯一性与局部性（2026-11-15 · issue #20 / PR #47）
- `unit142-limit-unique`：8 题。两条极限各给 ε/2 与一个半径，取小半径后在 `a + δ/2` 上接起来，`close_trans` 合成 `close L M ε`，`ratRel_of_close_all` 收成 L ~ M。另证换值、常函数与恒等函数的唯一性，以及常函数零不以 1 为极限。解答 8 checked、0 open。
- `unit143-limit-local`：1 定义 + 11 题。`AgreeNear f g a` 说有一个正半径，邻域里（除 a 外）两函数逐点相等；两个半径取小，接近性沿 `f x ~ g x` 搬运，得到极限不变与反方向；局部等于常数 c 的极限只能是 c。解答 12 checked、0 open。
- 引擎：新建 `lib/LimUnique`（8）与 `lib/LimLocal`（12）；`lib/Demo` 补 import 与四条自检。
- 判据：`check.py --json` rc=0（377 目标 · 3646 checked · 796 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 20/27）；`notation-lint` rc=0（390 文件，15 处既有豁免）；触及文件文案 0（BLOCK/WARN/INFO 全 0）；`style-lint --selftest` rc=0；`docs-lint` rc=0。
- 边界：函数停在有理代表元层；这里证的是极限值的唯一性，斜率函数逐点相同还要消去非零因子，留到后续单元。不新增公理，也不碰 kernel/front，不改 M1/M2/M3。

## M4 单元141：导数的代数：垂直平移与取负（2026-11-15 · issue #20 / PR #47）
- `unit141-deriv-shift-neg`：1 条辅助给出 + 16 题。沿 y 轴抬高不改变斜率：`(f x + c) - (f a + c)` 里的 c 抵消；关于 x 轴翻折让斜率整体变号：`(-f x) - (-f a) = -(f x - f a)`。代表元层备三条桥（减去同一个加数、乘负数、取负过减法），随后同一个斜率函数照旧交出（取负时整体取负），极限用常函数与取负律。正文 17 条声明（1 给出、16 题），解答 17 checked、0 open。
- 引擎：新建 `lib/DerivShift`（17），`lib/Demo` 补 import 与三条自检。
- 判据：`check.py --json` rc=0（371 目标 · 3597 checked · 777 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 18/27）；`notation-lint` rc=0（385 文件，15 处既有豁免）；触及文件文案 0（BLOCK/WARN/INFO 全 0）；`style-lint --selftest` rc=0；`docs-lint` rc=0。
- 边界：函数停在有理代表元层；和、差与常数倍要让乘法对加法分配，留到后续单元。不新增公理，也不碰 kernel/front，不改 M1/M2/M3。

## M4 单元140：导数的定义（2026-11-15 · issue #20 / PR #47）
- `unit140-derivative-definition`：3 定义 + 11 题。`IsSlope f a q` 要求在 a 之外逐点满足 `q x · (x - a) ~ f x - f a`，`HasDerivAt f a L` 要求某个斜率函数的函数极限是 L。ℚ 的倒数还没有构造出来，差商因此写成乘积等式；x 与 a 不同时它唯一决定 q x，补上倒数后与商逐字相同。常函数与恒等函数分别给出导数零与导数一。正文 14 条声明（3 定义给出、11 题），解答 14 checked、0 open。
- 引擎：新建 `lib/RatRing`（17）与 `lib/Deriv`（14），`lib/Demo` 补 import 与三条自检。
- 判据：`check.py --json` rc=0（368 目标 · 3556 checked · 761 open · 0 rejected · 0 solutions_open · 0 lib_open，M4 17/27）；`notation-lint` rc=0（381 文件，15 处既有豁免）；文案 0；`style-lint --selftest` rc=0；`docs-lint` rc=0。
- 边界：函数停在有理代表元层；导数的唯一性要「非零因子可消去」，留到后续单元。不新增公理，也不碰 kernel/front，不改 M1/M2/M3。
