## M6：QA / 文档 / 站点 / 发布（2026-11-21 · issue #22）
M1–M5 已全部完成，M5 的 21 个单元在 2026-11-19 落盘，分析课 171/171 单元（卷 II 72、卷 III 99），M6 只做 QA、文档、站点与发布，不新增单元。全量课程门禁 `python3 courses/analysis/tools/check.py --json` rc=0：457 个目标、4512 checked、0 rejected、0 solutions_open、0 lib_open；`python3 scripts/notation-lint.py --root courses/analysis` rc=0，扫 470 个文件、15 处显式豁免。文档与站点入口已对齐 171/171，站点 `check-site.py` 10/10、加 `--browser` 12/12；零 cargo 学习者路径实测通过（`setup` → `doctor` → `grade` → `query hints` → 解答），分析课整门进度 `course --json` 报 171 单元、0 failed；`docs-lint`、`status-lint`、`docs-expiry-check --check` 均 rc=0。发版节点曾被 Lean Kernel Arena 门禁挡住（`b60796a9`：两条递归规则伪造被接受 + `perf/ite-taken-branch` 超时）；2026-11-23 内核修复 PR #92 合入后，`arena-gate.py` 在 `8d4167f5` 上 PASS（230 cases：accepted 135 · rejected 90 · declined 2 · timeouts 3，known slow 3），发版预检全过。书页媒介与双语范围待用户拍板，尚未开工。

## 发版 0.99.0（2026-11-23 · M6 收口）
`python3 scripts/bump.py 0.99.0` 写四处版本 + 3 个课程清单的 `requires` + `site/data`（写版本前强制跑 arena 门禁，PASS 后才落字）。发版内容 = M1–M6：分析课 171/171 单元（卷 II 72、卷 III 99）发布、QA/文档/站点/发布四包，加上本轮的 LKA 门禁修复（消去规则的证明无关捷径被关掉、`0 << 2^64` 归约修好、`perf/ite-taken-branch` 按 perf-only 登记 known-slow）。书页 v3（多页）与双语仍未合入 `main`，留在 issue #22 后续。

## FTC 补证（2026-11-24 · issue #21 后续）
FTC 的通用形式（任意原函数的 Newton–Leibniz）在当前 `ratRep` 代表元层不成立：取 F 在 √2 两侧为 0/1，则每个有理点都有 F′=0，但 F(b)−F(a)=1 而 ∫f=0；因此「完整证明」必须改口径（加选择公理/倒数库，或只做积分函数的第一部分并写明边界）。本批只落区间拼接引擎的取值引理 `lib/IntegralCat`（13 条 checked、0 open），`catPartition` 的严格上升与上下和搬运未完成；决定见 PR。
