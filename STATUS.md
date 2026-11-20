## M6 P7：文档过期处置（2026-11-21 · issue #22）
- `python3 scripts/docs-expiry-check.py --check` 原报 3 份过期并阻塞提交。逐份复核后都保留并续期，理由写进 `scripts/docs-expiry.json`：`docs/notes/HANDOFF-kernel.md` 到 2027-02-21（仍是 `scripts/check-lean4-alignment.py` 的 Lean 4 对照第二落点，被 kernel/front 注释与 G-30 复现件引用），`docs/ux-findings-2026-10-12.md` 到 2027-02-21（UX 队列索引，ONBOARDING §0.2 引用），`docs/notes/PLAN-align-lean4.md` 到 2027-05-21（9 行墓碑，两份活文档仍按此路径引用）。
- `docs/notes/HANDOFF-kernel.md` 抬头补复核行：正文停在 2026-10-04，排队的 G-29/30/31/62/91/92/93 均已在台账 `fixed`，当前无在办内核项，下次内核改动时覆盖重写。
- 复跑 `python3 courses/analysis/tools/check.py --json` rc=0：457 目标、4512 checked、0 rejected、0 solutions_open、0 lib_open；`python3 scripts/check-site.py` 10/10；`python3 scripts/notation-lint.py --root courses/analysis` rc=0；`docs-expiry-check.py --check`、`docs-lint`、`status-lint` 均 rc=0。
- 边界：只动文档登记、一处交接单抬头与 STATUS；未动课程内容、kernel/front、站点页面；书页与双语未开工。

## M6 P2：文档对齐 171/171 与 M6 状态（2026-11-20 · issue #22）
M1–M5 已全部完成，分析课 171/171 单元。M5 的 21 个单元在 2026-11-19 落盘，PR #62 于 2026-11-20 合并。根 README 增加分析课入口与课程目录位置，ROADMAP 记下 C8 的验收位置，ROUTE 的抬头从 M0 设计稿改为当前进度，分析课 README 去掉「M1 仍未收口」的旧句并补上 III.8 与 III.9 的入口。M6 目前只做 QA 与发布，书页媒介与双语范围等用户拍板，尚未开工。
复跑 `python3 courses/analysis/tools/check.py --json` 得 457 个目标、4512 checked、0 rejected、0 solutions_open、0 lib_open；`python3 scripts/notation-lint.py --root courses/analysis` 扫 470 个文件 rc=0；`python3 scripts/docs-lint.py` rc=0。

## M6 P1：全量课程门禁实测（2026-11-20 · issue #22）
- `python3 courses/analysis/tools/check.py --json` rc=0：457 个目标（171 单元 / 2 卷 / 16 章）、4512 checked、1019 个待作答练习（画布，属正常）、0 rejected、0 solutions_open、0 lib_open；判卷二进制 v0.98.4（repo-build）。`check.py --selftest` PASS。
- `python3 scripts/notation-lint.py --root courses/analysis` rc=0：470 个文件零旧写法，15 处 `soko:notation-ok` 显式豁免。
- 计时：同一命令暖缓存复跑 63 秒；冷跑首轮同样 0 判负。
- 边界：只跑门禁与记数，不改课程内容；书页、站点与双语等 M6 其余工作待用户拍板媒介后开工。
