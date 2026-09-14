# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-14（第四十八轮：量词课程——course 单元⑦ + 画布第二课，0.26.0）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-14，第四十八轮：量词课程——单元⑦ + 画布第二课）

> 用户提出：教程里逻辑内容偏少，缺 forall/exists 题目，参考 Metamath 出题；
> 随后要求同步加进 `course/` 并提交推送。判定全部走内核。

1. **画布第二课**（`playground.sokonanoda`）：∀=依赖函数类型（引入写 fun、
   消去写应用、命名箭头等价）；`Exists` 按单元①的老办法立成公理三件套
   （intro 交证人、elim 交给函数且结论不提证人）；`Person`/`someone` 论域
   + 两个已填演示 + `#reduce` 自测 + 7 道练习（练习 6–12，含 ★/★★）各挂
   三层 `soko:hint`；7 份钥匙全部经完整内核验证。
2. **course 单元⑦**（`unit7-quantifiers.sokonanoda`）：同题重编号 1–7，自带
   逻辑骨架（True/And）与 `Exists` 公理，中文画布 + `solutions/` 钥匙 +
   英文镜像（代码逐字节一致、事件计数一致），golden `(14, 7, 1)`；
   `course.json` unit=1..7、`course/README.md` 七个单元/1..7 同步。
3. **守卫与文案**：`course.rs` golden +「seven units」；`course_status.rs`
   golden + 汇总 `units=7 / checked=47 / open=34`；根 README、site
   `index/course/en` 三处静态文案、teacher `curriculum.md` 单元表、
   `docs/teaching-session.md`（§3 新增第二课钥匙表、§5 单元⑦）同步。
4. **版本** 0.25.0 → **0.26.0**（Cargo + VSIX 两处；CHANGELOG Added）。
5. **验收**：`cargo fmt --check` 0；`cargo clippy --workspace --all-targets`
   教学 crates 0 告警（kernel 保持 warning）；`cargo test --workspace --locked`
   **545 passed / 0 failed**（8 ignored）。

## 本轮进度（2026-09-14，第四十七轮：sorry 洞期望类型精确化）

> 用户报告：练习 5 `(And.right a (Not a) x) sorry` 的 hover 显示整个声明
> 类型，应显示洞的期望类型 `a`（用户以 `((…) sorry : a)` 说明）。

1. **根因**：goal 走查（func_spine_case）只覆盖声明望远镜内的实参；
   `And.right` 全量应用后结果 `Not a`，`sorry` 是它的函数实参——超量应用
   直接 `return None` → generic fallback 用整个声明类型当目标。
2. **修复**（goals.rs）：FuncTemplate 增加 `result_ty`（望远镜剥完的残余）
   与 `def_body`（仅 def）；超量应用时把结果类型按 def 体逐步展开
   （`Not a` ⇒ `a -> False`），继续按箭头匹配剩余实参 → 洞期望 = 箭头
   定义域 `a`。剩余目标 `False`、假设 a/x 一并展示。
3. **hover**：decl_at 的 Open 分支在光标落在洞上时优先显示
   「此处 sorry 的期望类型」+「剩余目标」。
4. **测试**：front `overapplied_spine_through_def_shows_hole_expected_type`
   （goal="False"、sub_goals[0].ty="a"）+ LSP
   `hover_on_sorry_in_overapplied_spine_shows_hole_expected_type`。用户
   案例按其原话钉成单元测试。
5. 验收：545 passed / 0 failed（+2）；clippy 0；真实 LSP 协议跑
   playground 确认 hover 输出正确。版本 0.24.0 → **0.25.0**（crates 改动
   必须随 commit bump 版本——LESSONS 铁律）。

## 本轮进度（2026-09-13，第四十六轮：性能测试例行化）

> 用户要求：性能测试例行化、覆盖全面+细致（编译器 + VS Code 插件特性）、
> 每版本可见、回归时能定位到哪个改动。

1. **阈值断言哨兵**（`crates/front/tests/perf.rs` 3 个 +
   `crates/lsp/src/lib.rs` 3 个，随 `cargo test --workspace` 例行执行）：
   编译器缩放比（400/50 块 ≤12×，O(n²)=64× 必红）、增量编辑每键 <50ms
   且 kernel_checks≤1、编辑首练习不随文件长度超线性；LSP didChange
   round-trip <50ms、completion/hover/goals 各 <10ms（50 块文件）。
2. **每版本留档**：CI "Performance report" 步骤提取 PERF 行 →
   `perf-report-v<version>-<sha>.txt` artifact（每次 push 都有）；
   本地同口径 `scripts/perf-report.sh`。对比相邻版本报告即可定位回退
   场景 → git log 找改动。
3. **阈值设计原则**：只抓算法级回归（线性理论值 ×1.5 余量），CI 噪声
   不误报；绝对延迟抓用户可感劣化。设计文档 `docs/PERF.md`（含基线）。
4. 扩展层无独立计算路径——所有特性经 LSP，故覆盖在 LSP 请求层
   （didChange/completion/hover/soko-goals）。
5. 验收：cargo test --workspace 543 passed / 0 failed（+6 perf）；clippy 0。
6. 版本 0.23.0 → **0.24.0**：纯基建无功能面变化，但用户要求每轮工作
   有独立版本号（性能报告按版本对比）；CHANGELOG 以 Development/Infrastructure
   节记录。
