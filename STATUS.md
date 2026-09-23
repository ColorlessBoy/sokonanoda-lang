# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-21（第一百二十九轮：**逐 surface 的判别性** —— 线 C 收口并发版；
> 折叠开关 `SOKO_NO_NOTATION_FOLD=1` 实测 **3 红 3 绿**（与设计逐格一致）；
> 课程门禁 36 目标 · 328 checked · 99 open · 0 判负**逐项不变**；版本 **0.65.0**）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`；**harness 适配 = `docs/design/deepseek-harness.md`**；
> **agent 查询通道 = `docs/design/agent-query-channel.md`**（ROADMAP I15）。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-23，第一百三十三轮：**发版闭环修通 + 迭代提速 30×**）

> 用户两条新要求都落到了实处：**BUMP 必须闭环（确认线上发版生效）** 与
> **gate 太慢严重阻碍迭代**。

1. **查出"发版停了三版"的根因并修通闭环**。`origin/main` 已经推到 0.65.1，
   而线上最新发布**还停在 v0.63.0** —— 中间三次推送的 CI 全红（e2e 的
   known-red 用例），而 `ci.yml` 的 **auto-tag 只在 CI 绿时发版** ⇒ 一个 tag
   都没打。**bump 在本地"完成"了、线上一步没动**，正是用户那条要求要防的事。
   * 修法：把线 D 的导航链落地（见下）⇒ e2e 从 **23/2** 变 **25/25**；
   * **闭环实测**：CI 绿 → auto-tag `v0.65.1` → release workflow success →
     **`gh release list` 第一行 = v0.65.1（Latest）**、**26 个资产**、
     `Cargo.toml` 版本与之**相等** ✓。
2. **线 D 导航链（T-D10..T-D13）**：`NotationDecl` 加 `span`/`module`
   （记法跨 `import` 传播，只有 target 不够）· 入口记法表进 `ProjectReport`
   （以前算完就丢）· `QueryDoc::notation_at` + `module_path` ·
   `goto_definition` 在声明表之前先试记法分支。
   **G-23 关账**（用户第 6 条反馈"记法不能跳转 / hover 无原始类型"整条修好）。
3. **迭代提速（用户报"gate 太慢"）——先量再改**：
   | 阶段 | 改前 | 改后 |
   |---|---|---|
   | 课程门禁 `check.py` | **164s** | **0s**（持久编译缓存） |
   | `cargo test --workspace` | ~250s | `--fast` 只跑改动过的 crate |
   | 缺口台账 `gap.py check` | 94s | `--fast` 跳过（提交前跑） |
   | **完整 gate** | ~15 分钟 | **5.25 分钟** |
   | **`gate --fast`** | — | **30 秒** |
4. **CI 的三条假红全是测试自身的时延假设**（不是产品回归，修它们时一行产品代码
   没改）：跨文件刷新的两条只等"被改的那份"、而下游是**异步**重发的 ⇒ 慢 runner
   上落到排水窗口外（改用 `did_change_at_drained_expecting` 等两份）；
   性能哨兵是**绝对秒数** ⇒ 同一用例本机 8.8s / CI 62s（150+ 用例并行抢 CPU）
   ⇒ 改成**机器无关的相对判据** `unit12/unit01 < 12`（本机 4.5×、CI 4.1×）。
   全部记进 `docs/CI-FAILURES.md`。
5. **文档**：`REQUIREMENTS.md` §9（新要求 + 背景）· `AGENTS.md` 命令区两档 gate ·
   `docs/vscode-dev-guide.md`「迭代速度」一节（含两档纪律与持久缓存的安全性）·
   `skills/sokonanoda-dev` 门禁一节。
6. **下一环**：**T-D03**（hover 的原始类型只对能解析出 target 的符号显示）
   ——它带 ⬆ BUMP(patch) 点，做完发 0.65.2 并**按新要求闭环确认**。

## 本轮进度（2026-09-21，第一百三十二轮：**闭包增量设计 + 线 D 开工**）

> 按用户拍板把 **T-K20 提前**到线 D 之前（依据：unit12 冷编译 9.8s），本轮做完
> 它的设计 + spike，随后开工线 D（记法跳转 + hover）。

1. **T-K20 闭包增量设计 + spike**（`docs/design/closure-incremental.md` +
   `scripts/spike-closure-incremental.py`）：现状（一个 Arena + 一个 builder 跑
   **整个闭包**）· 两个障碍（**O7** prelude 按闭包决定 ⇒ 会**静默改变判卷**；
   **O8** 七样每轮状态必须一起提升，含 `ns` 的单元边界 `reset`）· 报告归因
   （`split_report` 走命令下标、不用 span ⇒ 已编模块的报告也要留住）· 候选
   （K2-b 窄版先落地 / K2-a 结构正解）· 风险（高）。
   **spike 实测**：`unit01 → unit08 → unit12` 依次打开
   **14859ms → 8886ms（−40.2%）**，三个前缀全部 downward-closed ✓
   （与计划记的目标 15.06s → ≈8.5s 一致）。
   **守卫落地**：`compile::prelude_shape` **就是 `run_pass` 的安装判据**（同一个
   函数，不会漂）；计划要求的"脚本化验证"= `crates/front/tests/prelude_shape.rs`
   ——课程里每一个 `.sokonanoda` 的闭包形状必须一模一样且不撞 prelude 名字。
2. **T-D01 复现脚本**：`bash docs/gaps/repro/G23-notation-navigation.sh` → **0**
   （修前为红）✓。
3. **T-D02 hover 增加"原始类型"行**：计划说这是"全计划最便宜的一刀"，实测撞到
   **闭包**问题（与 T-C30/T-C31 同族）——`symbol_at` 只看本文件 + 内建 ⇒
   `import` 来的记法（`∈`/`⊆`）目标永远是 `None` ⇒ 连"展开成什么"都没有。
   补 `symbol_at_with_sources`（闭包前缀回退）+ `judge_type_of_constant`。
   实测 hover 与计划给的期望**逐字一致**：`Set.mem : forall (α : Type 0),
   α -> Set α -> Prop` ✓。那一行**故意不折记法**（它叫"**原始**类型"）。
   请求路径成本 `hover_ms = 0ms`（预算 50ms）✓。
4. **踩到的坑（已修 + 已记）**：G-23 的复现件把"**修了一半**"判成 exit **2**
   ——而 `docs/gaps/README.md` 的约定是 `2 = 环境/形状异常` ⇒ 台账（`open`）
   判成"行为已变" ⇒ **门禁红**。改成归 0（缺口仍在），消息里仍说清哪一半还差。
   **教训**：复现件的退出码是**契约**，不能拿 2 表达"部分完成"。
5. **判据**：`gate` **PASS** · 课程计数**逐项不变**（36 目标 · 328 checked ·
   99 open · **0 判负**）· `gap.py check` 全绿 · 新测试全绿。
   **未 bump**：线 D 的 bump 点在 T-D03/T-D17/T-D41。
6. **下一环**：**T-D03**（线 D 收尾：patch bump）。

## 本轮进度（2026-09-21，第一百三十一轮：**线 C 收口 + 0.65.1 + 排期提前 T-K20**）

> 用户最初六条反馈里的**第 5 条**（"infoview 里的 goal 展现没有用 notation 的方式"）
> 到此**整条闭环**：四个生产者 + 着色 + 真宿主 e2e + 检查点全过。

1. **T-C32 着色在 Infoview 里可见**：渲染侧本来就通（`infoview.js` 把 run 画成
   `tok-<kind>`），缺的是**测试**——补两条 webview 断言（目标行 + 声明卡片，
   记法符号各是一个 `tok-keyword` span）。扩展 `_pushState` 是
   `Object.assign({type:"state"}, state)` **全字段透传** ✓。
2. **T-C50 真宿主 e2e**：用例 #6 `goal text uses the file's notation` 本来就在
   （T-015..T-017 写的），本轮确认**转绿**（`--grep` → 1 passed；全量
   **23 passed / 2 failed**，剩的两条 #7/#8 是线 D）。
3. **T-C40 断言与 golden 更新**：不按计划给的行号审（行号早被挪走了），改成审
   `git diff 7874dd4..HEAD` 里测试文件的**每一条 golden 改动**——全程只重钉
   **5 处**，全是 `And` → `∧`，每处都先跑测试读实际输出再改；内核 pp
   **一个字节没改**（红线）；空断言扫描无命中。`cargo test --workspace --locked`
   → **exit 0**（39 suite，0 failed）。
4. **T-C41 文档 + CHANGELOG + ⬆ BUMP patch → 0.65.1**：`goal-rendering.md` §8
   as-built（四个生产者的最终行为 + "判定没动"的证据）、`notation-subset.md`
   补"渲染"一节（N1–N7 一条不变，只记显示侧的边界表）、CHANGELOG、
   REQUIREMENTS §9（第 5 条交付）、README、teacher 技能（**照面板念目标**）、
   `vscode-dev-guide.md` 两条坑。
5. **CP-C 检查点全过**：`verify-editor-issues.sh` → **已修 6 · 缺口仍在 1 ·
   环境异常 0**（第 5 条 **已修** ✓；剩的第 6 条 G-23 记法导航属线 D）·
   四生产者判别性全绿 · 课程计数**逐项不变**（36 目标 · 328 checked · 99 open ·
   **0 判负**）· `cargo test --workspace` 全绿 · `perf-compare --since c74c0046`
   **exit 0** · e2e #6 转绿。
6. **踩到的坑（已记）**：bump 之后**必须重建**——`scripts/soko` 要求仓库构建的
   版本与版本钉**匹配**，否则 exit 3，`verify-editor-issues.sh` 会把五条全报成
   「环境异常」（假红）。
7. **排期提前（用户拍板）**：线 C 的 4 条收完后**插 T-K20/T-K20′**（G-31 + G-34
   的根治设施），清单已把 `T-K20` 挪到线 D 之前（`plan.py check` 只校验集合、
   不校验顺序 ⇒ 合法）。依据：unit12 **冷编译 9.8s（release）**，其中一部分是
   judge 每批合成文档 + 整前缀重跑（实测 126k 次调用）。
8. **下一环**：**T-K20**（`docs/design/closure-incremental.md` + spike）。

