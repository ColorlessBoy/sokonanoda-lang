## M6 P9：发版状态与文档死链（2026-11-21 · issue #22）
M6 的非书页 QA 工作已完成：全量课程门禁 457 目标 / 4512 checked / 0 rejected / 0 solutions_open / 0 lib_open，`notation-lint` 470 文件 rc=0，站点 10/10，`docs-lint` 与 `release-preflight --fast` 均 rc=0。教师路径、文档过期处置与复现件诊断的修正分别在草稿 PR #70、#71、#72，待合并；根 README 的三处死链改成文档地图入口，ROADMAP、分析课 README 与发版手册记下 M6 当前状态。
发版节点仍被 Lean Kernel Arena 门禁挡住：`python3 scripts/arena-gate.py` 在构建 `b60796a9` 上报 3 条失败——`bugs/rec-rule-proof-irrel` 与 `bugs/rec-rule-proof-irrel-nested` 该拒却接受，`perf/ite-taken-branch` 超时。内核修复不在 M6 的 QA 范围，版本号在修复前不写。

## M6 P2：文档对齐 171/171 与 M6 状态（2026-11-20 · issue #22）
M1–M5 已全部完成，分析课 171/171 单元。M5 的 21 个单元在 2026-11-19 落盘，PR #62 于 2026-11-20 合并。根 README 增加分析课入口与课程目录位置，ROADMAP 记下 C8 的验收位置，ROUTE 的抬头从 M0 设计稿改为当前进度，分析课 README 去掉「M1 仍未收口」的旧句并补上 III.8 与 III.9 的入口。M6 目前只做 QA 与发布，书页媒介与双语范围等用户拍板，尚未开工。
复跑 `python3 courses/analysis/tools/check.py --json` 得 457 个目标、4512 checked、0 rejected、0 solutions_open、0 lib_open；`python3 scripts/notation-lint.py --root courses/analysis` 扫 470 个文件 rc=0；`python3 scripts/docs-lint.py` rc=0。

## M6 P1：全量课程门禁实测（2026-11-20 · issue #22）
- `python3 courses/analysis/tools/check.py --json` rc=0：457 个目标（171 单元 / 2 卷 / 16 章）、4512 checked、1019 个待作答练习（画布，属正常）、0 rejected、0 solutions_open、0 lib_open；判卷二进制 v0.98.4（repo-build）。`check.py --selftest` PASS。
- `python3 scripts/notation-lint.py --root courses/analysis` rc=0：470 个文件零旧写法，15 处 `soko:notation-ok` 显式豁免。
- 计时：同一命令暖缓存复跑 63 秒；冷跑首轮同样 0 判负。
- 边界：只跑门禁与记数，不改课程内容；书页、站点与双语等 M6 其余工作待用户拍板媒介后开工。
