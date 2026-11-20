## M6 P4：教师路径核对与文档修正（2026-11-20 · issue #22）
- 零 cargo 路径实测通过。`setup` rc=0，`doctor --json` 报 `ready:true`（v0.98.4）；`grade playground.sokonanoda --json` rc=0（23 checked、4 open、1 条 `reserved-declaration-name` warning，画布自带的 `Prop` 占位）；`query hints --line 376 --col 3` 三条，旧行 323 空数组。
- 分析课同一条路。`unit10-int-mul-identity.sokonanoda` rc=0（11 checked、4 open），`query holes` 四个洞，第 65 行三条提示；解答 `unit10-solution.sokonanoda` rc=0（15 checked、0 open）；技能举例的 `unit01-int-quotient.sokonanoda` 同样 rc=0（5 checked、4 open）；`course .../analysis/course.json --json` 报 171 单元、0 failed。
- 只改 `skills/sokonanoda-teacher/SKILL.md`：补学习者路径（setup → 画布 → grade → hints → solution）、分析课 course 命令与解答目录、`project` MCP 工具，hints 示例行 323 → 376；解答文件名按实测改成「以 `-solution.sokonanoda` 结尾」（171 个里只有 13 个恰好是 `unitNN-solution.sokonanoda`）。
- 门禁复跑全绿，`check.py --json` 报 457 目标、4512 checked、0 rejected、0 solutions_open、0 lib_open，`notation-lint` 扫 470 文件、15 处豁免，`cargo test -p sokonanoda-cli --test skill` 5/5，`docs-lint` 与 `status-lint` 均 rc=0。
- 边界：只核对路径与改文档；未动课程内容、kernel/front；书页与双语仍未开工。
## M6 P2：文档对齐 171/171 与 M6 状态（2026-11-20 · issue #22）
M1–M5 已全部完成，分析课 171/171 单元。M5 的 21 个单元在 2026-11-19 落盘，PR #62 于 2026-11-20 合并。根 README 增加分析课入口与课程目录位置，ROADMAP 记下 C8 的验收位置，ROUTE 的抬头从 M0 设计稿改为当前进度，分析课 README 去掉「M1 仍未收口」的旧句并补上 III.8 与 III.9 的入口。M6 目前只做 QA 与发布，书页媒介与双语范围等用户拍板，尚未开工。
复跑 `check.py --json` 与 `notation-lint --root courses/analysis` 均 rc=0，`docs-lint` 也绿。

## M6 P1：全量课程门禁实测（2026-11-20 · issue #22）
- `python3 courses/analysis/tools/check.py --json` rc=0：457 个目标（171 单元 / 2 卷 / 16 章）、4512 checked、1019 个待作答练习（画布，属正常）、0 rejected、0 solutions_open、0 lib_open；判卷二进制 v0.98.4（repo-build）。`check.py --selftest` PASS。
- `python3 scripts/notation-lint.py --root courses/analysis` rc=0：470 个文件零旧写法，15 处 `soko:notation-ok` 显式豁免。
- 计时：同一命令暖缓存复跑 63 秒；冷跑首轮同样 0 判负。
- 边界：只跑门禁与记数，不改课程内容；书页、站点与双语等 M6 其余工作待用户拍板媒介后开工。

