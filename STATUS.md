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

## 本轮进度（2026-09-23，第一百三十四轮：**线 D 的 hover 收口 + 发 0.65.2**）

1. **T-D03 hover 的"原始类型"三种形态齐了**：本文件声明（`⊗`）/ 语言内建（`∧`）/
   `import` 来的（`∈`）各一条测试。
   **"解析不出就不显示"钉在函数层**（`notation_input::target_resolution_tests`），
   不是 hover 层——因为**在能编译的文件里这条不可达**（认得出来的符号必有 target）。
   我试着加 LSP 级反向用例时构造不出"能编译 + 符号无 target"的文件，所以如实钉在
   函数层、**不硬凑假用例**；hover 那侧靠"那一行写在 `if let Some(target)` 里"
   结构性保证。
2. **⬆ BUMP patch → 0.65.2**（§0.2："用户可感知的能力落地"）。CHANGELOG 另记了
   本版包含的**记法跳转**（随 0.65.1 发布的 T-D10..T-D13）与**门禁提速**。
3. **发版闭环（用户新要求）**：推 main → CI → auto-tag → release →
   `gh release list` 核对。**上一版 v0.65.1 已确认上线**（26 资产、Latest、
   `Cargo.toml` 与之相等）；0.65.2 已推送，等 CI 与 release 产出后核对。
4. **bump 的已知代价实测**：bump 会让**编译缓存全失效**（缓存键含
   `CARGO_PKG_VERSION`）⇒ bump 后第一次完整 gate 从 5.25 分钟变成 **37.7 分钟**
   （课程门禁与测试套件都从头编一遍）。这是文档里记过的代价，不是回归；
   `gate --fast` 仍然 ~30s。
5. **下一环**：T-D14（parser 保留记法符号 token 的 span——AST 变更，为"表达式内
   跳转"铺路）。

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

