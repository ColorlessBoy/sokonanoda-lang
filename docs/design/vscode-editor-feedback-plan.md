# E1 计划（VS Code 项目模式六条反馈）—— **收口索引**

> **状态：124/124 全部收口** ✓（E2 计划 §4 已继承其结论；见 `docs/design/e2-plan.md`）。
> **本文件只留"现行结论 + 未决 + 清单"** ✓（用户 2026-09-26 文档瘦身 ✗）；
> **逐字原文**（4375 行：每条根因的实测命令与输出、四条线的完整设计、风险与收尾义务）
> ⇒ `docs/archive/vscode-editor-feedback-plan-full-2026-09-26.md.gz`
> （`gunzip -c … | less` ✓）。**归档 ≠ 销毁** ✓。
>
> **E2 继承的四条结论**（**不要再走一遍** ✗）：**T-K11** 实测零收益（代码留作惰性开关）·
> **T-K12c / T-K13** 接线存档（`decl_idx` 墙 —— E2 的阶段 D 就是那次重构，重新分步）·
> **T-K30** 重新定级（需新 API ⇒ E2 阶段 C）· **T-K31**（`TcCache` 4MB 复用池）**实测无收益**
> ⇒ 不做（记账即可）。内核三原语（`with_env` / `snapshot` / 六个 interner 与 `Dag` 的 `Clone`）
> 是 E2 阶段 D 的**现成积木** ✓。

---

## 1. 六条反馈 → 根因（一句话版）

| # | 用户反馈 | 根因（实测，见 §2） | 所在层 |
|---|---|---|---|
| 1 | 编译很慢 | 项目文件在 LSP 上**每次动作都从零编译整个 import 闭包**：release 实测 unit01 **1.8s** / unit08 **4.6s** / unit12 **8.5s** / unit12 解答 **36.1s**；编译期间**整把文档锁被占住**，hover/goals/stateAt 全部排队 | LSP + front |
| 2 | 没有实现编译后的文件加速 VS Code | **LSP 对含 `import` 的文档既不读也不写任何缓存**（`crates/lsp/src/lib.rs:152-156`，`grep digest crates/lsp` = 0）；项目缓存只活在 CLI crate（`crates/cli/src/project_cache.rs`），且形状是「一闭包一条、只存入口模块」（`crates/front/src/query/mod.rs:215` 拿到它就把 `project` 置 `None`）——LSP 需要的**逐模块报告**根本不在里面 | LSP + CLI |
| 3 | 新打开一个文件就有临时编译 | ① 每份 `Doc` 各编一份闭包，文档之间零共享（打开 `lib/Set` 编 {Logic,Set}，打开 unit08 又编 {Logic,Exists,Set,Image,unit08}）；② 项目模式**先白编一遍入口单文件**再编闭包，结果被丢弃（`crates/front/src/query/mod.rs:117` vs `:135`/`:144-151`）；③ `Doc::set_text` **没有"文本没变就返回"的短路** ⇒ 保存 / 编辑器外改动会重编同一文本；④ 项目缓存**永不写**：只写"完全干净"的项目（`is_clean` 要求每个模块 `warnings.is_empty()`），而 `sorry` 是 WARNING | LSP + front |
| 4 | Infoview「声明」栏经常失效，unit 教学文件没有 | `QueryDoc::goals` 被 `parsable()` 挡住（`crates/front/src/query/mod.rs:481` → `:403-408`）；项目入口单文件 parse **必然**失败（记法来自 `import`）⇒ `Err(NotParsable)` ⇒ LSP `.unwrap_or_default()`（`crates/lsp/src/lib.rs:495`）⇒ `decls: []`。**同一条 G-20 补丁 `check()` 与 `Doc::set_text` 都打了，只有 `goals` 漏了** ⇒ 目标栏好、声明栏空（正是用户看到的不对称）。同族：`holes`/`nextHole`（`alt+n` 跳洞）全死 | front + LSP |
| 5 | goal 展现没有用 notation | **goal 文本有四个生产者**，不是一个：根状态与声明列表走**内核 pp**（必然点名）、无 `by` 的开练习走 `render_expr`（**保留**记法）、`by` 步进走 `render_expr` 但一旦经 `canonical_goal*`/`apply`/`cases` 就换成 pp 文本再 parse 回来（丢）。实测同一份数据 `goal = "(a ∈ A) -> a ∈ B"` 而 `ty = "… Set.mem α a A …"` | front |
| 6 | 记法不能跳转、hover 没有原始类型 | 记法使用处在 elab 里**硬编码** `resolution: None`（`crates/front/src/compile/elab.rs:2914`），而 `definition_at` **就是** `hover_type_at(…).resolution`（`crates/lsp/src/render.rs:306-314`），没有第二条路 ⇒ `definition` = `null`；hover 的 `target` 只认**本文件**声明的记法（`notation_input.rs:261-267`）⇒ import 来的 `∈` 连"展开成"都没有；而 `Set.mem` 的签名其实**已经算过又被丢掉**（`elab.rs:1228` 的 `judge::judge_type_of_constant`，**该函数是 `pub`**） | front + LSP |

**一句话**：1/2/3 是**同一个缓存故事**；4 是**一个判据漏打补丁**；
5 是**渲染层按生产者分裂**；6 是**记法表没有对编辑器暴露**。

---


---

## 10. 明确不做的事（避免再次"改得很大却漏得多"）

> 2026-09-21 内核解冻后，原先列在这里的"跨模块增量"、"`by` 前缀重判根治"、
> "`build <目录>` 编一次全项目"**已转正**，进 §5.7 线 K（批次 5）。
> 下面这些仍然不做：

1. **诊断消息里的类型改用记法**：本轮只改 Infoview 的 goal / 类型行（§11 问题 6）。
2. **`position_to_offset` 的 UTF-16 修正**（T-D31）：立台账，单独立项
   （会动所有位置映射与测试夹具）。
3. **重构 `extension.js`（1906 行）/ `crates/lsp/src/lib.rs`（1700 行）**：
   结构债已登记（`docs/HANDOVER.md` §4），但"修 bug 顺带重构"正是上次出纰漏的模式；
   要拆单独开批次。
4. **换掉 `stumpalo` 或重写 arena 层**：线 K 只做"可检查点/可复用"这一刀，
   不换基础设施（换基础设施的风险与收益不成比例）。

---


---

## 11. 未决问题（需要用户拍板）

1. ~~**批次制 vs 每环节一个版本**~~ —— **2026-09-21 用户拍板：到一定程度就 bump，
   不等批次**（§0.2 的定案）。bump 触发：① 用户可感知的能力落地；② 距上次 bump
   ≥ 8 个环节；③ 用户要拿去测。**13 个 bump 点已标进 §13**
   （`python3 scripts/plan.py bumps`），批次 0 零用户可见改动 ⇒ 不发版。
2. **"慢"主要发生在哪个动作**：打开新文件 / 每次按键 / 保存 / 切文件，对应不同根因
   （A1+A6 / A1+A5 / A7 / A8）。T-009 与 T-007 会给数字，但用户的体感更权威。
3. **优先级**：推荐 `阶段0 → 线B → 线A → 线C → 线D → 线K`。若"慢"比"面板空"更痛，
   把线 A 提到线 B 之前（但那样第一批就碰缓存键与条目形状，风险更高）。
4. **线 K 的先后**：内核解冻后，`by` 前缀缓存与跨模块增量是**收益最大的两刀**
   （36.1s 那个数字的大头）。建议**紧接线 A**（批次 5），不要拖到最后——
   否则线 A 做完你仍会觉得"换一个文件又要等"。
5. **缓存键的 `build_stamp`（T-009）**：若结论是"CLI 预热确实到不了 LSP"，
   是否接受改键（会让现有缓存全部失效一次）？
6. **记法渲染的范围**：`printback-feasibility.md` §4 已经圈定 5 个落点
   （`ty`/`goal`/`reduce`/`expr_hover`/签名与补全），并明确**不落**
   `CheckEvent::TypeChecked/Reduced`。是否接受这个圈定？**诊断消息里的类型**
   （走 `debug_print`，不是 pp，安全）要不要一起改？
7. **记法导航的语义**（T-D20..T-D23）：内建目标（`∧`→`And`，无源码声明）怎么办？
   跨文件记法的"定义"是库的 `infix` 行还是入口的 `import` 行？重载返回 1 个还是 N 个？
8. **语料对拍要不要进 CI**（T-K02）：Lean Kernel Arena 现在**只有 1 个用例被收集**、
   且 CI 里没有 `LEAN_KERNEL_ARENA` ⇒ `cargo test --workspace` 里**静默空过**。
   修好收集逻辑后，是接进 CI（要下载/克隆外部语料、`init`/`std` 是 309MB/526MB），
   还是在文档里明说"本地可选、CI 不做"？

### 11b. 调研留下的实验清单（**做完才知道上界**）

调研给了 9 个必须用实验回答的问题，前 4 个直接决定线 K 的收益与风险：

1. **T-K03**：36.1s 里 parse / elab / **内核检查** / pp 各占多少？
   （决定 T-K11 的收益上界；探针可抄 `by-tactics.md` §13）
2. **T-K11 验收实验**：加 `SOKO_JUDGE_ENV_REUSE=0/1` 开关后，两态在全语料上
   `--json` 是否逐字节相同？
3. `unit12-solution` 实际发生多少次 `judge_pairs_uncached`？
   （调研静态数出 ≈20：9 个声明值位 + 闭包 11 个 `:= by`；用 `judge.rs:481-489`
   的 `pass_count()` 或 §13 的 prefix_bytes 探针实测）
4. **T-K12 验收实验**：`judge_infer`/`judge_type_of` 换成 `with_env` 路径后，
   文本是否与合成 `#check` **逐字相同**？（`EnvLimit` 从 `Empty` 变成
   `ByIndex(k)`；pp 内部自用 `PpUnlimited`，理论上无关，但**必须实测**）
5. `ExportFile` 加 `&ArenaRef` 是否真的让 `check_all_declars_par` 编译失败？
   （预期失败；确认后写进设计文档的"被否方案"）
6. `courses/set-theory` 每个入口闭包的 **prelude 形状是否完全相同**？
   （T-K20 复用守卫的前提；可脚本化验证）
7. LSP 里"每次动作都从零编闭包"是否仍成立？4.59s 里**入口 vs 各 lib 各占多少**？
   （per-module 计时）
8. 内核 Infix 记法的操作数顺序（`lhs=args[len-1]`）是上游约定还是 bug？
   —— 虽然**本计划不走内核 pp**，但若将来要填 `notations` 表，
   **必须先写一条 `And a b` → `a ∧ b` 的内核测试**。
9. `TcCache::new` 每次 `with_ctx` 清 4MB 在真实课程上占多少？（T-K31；
   独立于本任务的低风险提速点）

---


---

## 13. 执行清单（**线性顺序，逐条勾**；124/124 ✓）

> 每条一行的**收口索引** ✓；完整规格（判据 / 实测 / 交付记录）见归档 §13。

- [x] `T-001` 六条反馈落缺口台账
- [x] `T-002` 复现脚本：声明栏（LSP 探针）
- [x] `T-003` 复现脚本：记法跳转 + hover 原始类型（LSP 探针）
- [x] `T-004` 复现脚本：项目缓存对带 `sorry` 的单元不生效（CLI）
- [x] `T-005` 复现脚本：LSP 每次打开都重编（两个独立 LSP 进程）
- [x] `T-006` 复现脚本：goal 文本丢记法（CLI）
- [x] `T-008` 一条人工验收命令
- [x] `T-009` 实验：缓存键的 `build_stamp` 到底会不会让 CLI 预热对 LSP 失效
- [x] `T-011` 环节循环脚本 `scripts/dev-loop.sh`
- [x] `T-012` 真 VS Code **单用例**跑法（L4 层的前提）
- [x] `T-013` 版本纪律修订：bump = **发布边界**，不是 commit 边界
- [x] `T-014` 把"肉眼看效果"的回路写进文档
- [x] `T-015` 项目夹具：一个最小的 import 项目
- [x] `T-016` 测试可见的 Infoview 载荷（`testApi` 扩面）
- [x] `T-017` `scripts/vscode-e2e.sh` 加三个开关（L4 层的前提）
- [x] `T-018` 六条反馈的 e2e 用例（矩阵）
- [x] `T-019` e2e 进 CI 与台账
- [x] `T-007` 编辑器路径性能台账（把 §2.1 变成可重复的哨兵）
- [x] `T-020` 真实课程 + 编辑器路径的 perf 套件（七条 case）
- [x] `T-021` `scripts/perf-check.sh --case <name>`（单场景快跑，**每环节用**）
- [x] `T-022` `scripts/perf-compare.py`（**回归比较器**，本节最值钱的一条）
- [x] `T-023` 性能进 DoD 与 CI
- [x] `T-010` 文档与要求入账
- [x] `T-B01` 实测表：哪些文件坏、哪些好
- [x] `T-B02` 实测：`nextHole` / `stateAt` / `hints` 在项目入口的现状
- [x] `T-B03` 收敛成单一判据 `QueryDoc::usable()`
- [x] `T-B04` 补 LSP 断言（夹具已经在了）
- [x] `T-B05` 复现转绿
- [x] `T-B06` `alt+n` 跳洞在项目文件里可用
- [x] `T-B07` CLI 侧的假绿也钉住
- [x] `T-B08` 客户端 E4：请求期间切文档 ⇒ 新文档必须补取数
- [x] `T-B09` 客户端 E5：`_pendingDecls` 合并把新文档那次取数吃掉
- [x] `T-B10` 客户端 E6：空数组是真值 ⇒ `ensureDeclarations()` 永久空转
- [x] `T-B11` 客户端 E8：指纹吞掉"只改了类型"的更新
- [x] `T-B12` Webview 空态三态化
- [x] `T-B13` 真宿主 e2e：声明栏真的有卡片（矩阵用例 #1）
- [x] `T-B15` 真宿主 e2e：`alt+n` 跳洞（矩阵用例 #2）
- [x] `T-B16` 顺带修复 G-28：`→` 右边直接跟 `∀` 不解析
- [x] `T-B14` 文档同步
- [x] `T-A20` 项目模式不再白编一遍入口单文件
- [x] `T-A21` `set_text` 文本未变即短路
- [x] `T-A22` 保存 / 编辑器外改动不再重编同一文本
- [x] `T-A24` `project_view_reason()` 不再每次 parse 整份文本
- [x] `T-A25` `build <目录>` 的 O(文件数 × 闭包) 如实记账
- [x] `T-A01` 项目缓存下沉到 `front`
- [x] `T-A02` 缓存键去掉"可执行文件 mtime"这个不稳定的量
- [x] `T-A03` 缓存条目 v2：按模块存（对齐设计 §4.8）
- [x] `T-A04` 冷/热 `--json` 逐字节一致（带 `sorry` 的项目）
- [x] `T-A05` `requires` 漂移不再静默关掉缓存 + 两条写缓存路径规则一致
- [x] `T-A08` `requires` 的单一来源 + 漂移门禁（**用户判定的根因**）
- [x] `T-A06` 依赖改动仍必 miss
- [x] `T-A07` `--text` 中间态的处置
- [x] `T-A10` LSP 读项目缓存（命中即回放）
- [x] `T-A11` LSP 写项目缓存
- [x] `T-A12` LSP 侧测试：两次独立进程打开，诊断逐字节一致
- [x] `T-A13` 跨文件能力在缓存命中后仍然工作
- [x] `T-A14` 实测数字
- [x] `T-A15` 命中缓存后 Session 快照的处置
- [x] `T-A23` 扇出：改一个依赖不重编所有打开文档
- [x] `T-A30` 编译不再独占 `Mutex<Docs>`
- [x] `T-A60` 缓存与扇出的 e2e 断言
- [x] `T-A50` 设计文档 as-built
- [x] `T-A51` 性能台账收口
- [x] `T-A52` （可选，用户拍板）打开工作区时预热缓存
- [x] `T-K32` `pretty_printer.rs` 零单测
- [x] `T-C01` 四个生产者 × 真实文件的实测表
- [x] `T-C02` 消费者审计（**不然会静默改坏判卷**）
- [x] `T-C03` 设计文档：**把已有的 `printback-feasibility.md` §4 升格为权威设计**
- [x] `T-C03b` `DisplayText` 护栏（**先于任何折叠代码**）
- [x] `T-C04` 记法表复用 `judge.rs` 的重建（提成公共函数）
- [x] `T-C10` 折叠函数第一刀：只做二元 infix 族
- [x] `T-C11` arity 的来源
- [x] `T-C12` `scoped` 的保真度
- [x] `T-C13` 重载（一个符号 → N 个目标）的处置
- [x] `T-C14` 折叠层的损失护栏
- [x] `T-C20` 生产者 1+3：根状态与声明列表的 `ty_text`
- [x] `T-C21` 生产者 2：无 `by` 的开练习（应当已经好，补守护）
- [x] `T-C22` 生产者 4：`by` 步进里被 pp 化的四处
- [x] `T-C23` binder ty（假设行的类型）
- [x] `T-C24` 逐 surface 的判别性测试
- [x] `T-C25` 边界：命中不了就回退
- [x] `T-C30` `semantic::tag_runs` 填 `Names::notations`
- [x] `T-C31` 目标文本里的**导入名**不再标 `unknown_ident`
- [x] `T-C32` 着色在 Infoview 里可见
- [x] `T-C50` 真宿主 e2e：goal 文本用记法（矩阵用例 #6）
- [x] `T-C40` 断言与 golden 更新（**计数中性**）
- [x] `T-C41` 文档 + CHANGELOG
- [x] `T-K20` 设计文档 `docs/design/closure-incremental.md` + spike
- [x] `T-D01` 复现脚本
- [x] `T-D02` hover 增加"原始类型"行
- [x] `T-D03` hover 的"原始类型"只对**能解析出 target** 的符号显示
- [x] `T-D30` 记法符号不再误解析到外层 binder（**独立正确性 bug**）
- [x] `T-D31` `position_to_offset` 的 UTF-16 语义（**独立缺口**）
- [x] `T-D10` `NotationDecl` re-export + 补 `span`/`module`
- [x] `T-D11` 闭包级记法表进 `ProjectReport`/`QueryDoc`
- [x] `T-D12` 解析 API：`notation_resolve(text, table, offset)`
- [x] `T-D13` hover 的"展开成"（import 来的记法）
- [x] `T-D14` parser 保留记法符号 token 的 span
- [x] `T-D15` 新增 `ResolvedTarget` 变体并绕开覆写
- [x] `T-D16` LSP `definition` 处理记法变体
- [x] `T-D17` hover 的 `range` 收窄到符号本身
- [x] `T-D20` 内建/ prelude 目标的定义跳转怎么办
- [x] `T-D21` 跨文件记法的"定义"是哪个
- [x] `T-D22` `scoped` 记法的导航是否尊重作用域
- [x] `T-D23` 重载：一个 `Location` 还是 N 个
- [x] `T-D24` `documentHighlight`/`references`/`rename` 覆盖记法符号
- [x] `T-D40` 三层测试（矩阵用例 #7/#8）
- [x] `T-D41` 文档同步
- [x] `T-D50` 记法声明的**目标名**是使用点（着色 + 跳转，一条修两个症状）
- [x] `T-D51` 折叠层扩到 prefix / postfix / binder / 零元（`forall` → `∀` 等一批符号）
- [x] `T-D52` `def` 的声明多一行"真正定义"（`:=` 之后的 body）
- [x] `T-K01` 全语料逐字节对拍工具
- [x] `T-K02` 内核改动的验收清单 + **修语料对拍的收集逻辑**
- [x] `T-K03` `36.1s` 花在哪：分阶段 profile
- [x] `T-K10` 设计文档 `docs/design/by-prefix-reuse.md` + 三个候选的定稿
- [x] `T-K11` **K1-a：纯 front 的 judge TrustPlan 复用（零内核改动，先做）**
- [x] `T-K12a` **K1-b 前半：内核 `EnvBuilder::with_env` + `install_all_preludes` 唯一实现**（✅ 2026-09-24；判据：往返回归单测 + 内核 60 条测试 ✓）
- [x] `T-K12c` **K1-b 后半：影子环境接进 judge**（⏸ **存档·不做**：机制级原因见下，5 轮调查结论）（⏸ **存档**：对照判据判定影子与内核阶段**不等价** ✗ —— 差在增量记账 `skip`/`trust`/pass1-pass2 ⇒ 需要**实质重构**；影子已关进 `SOKO_SHADOW_CHECK`，默认零成本 ✓；本条的 `⬆ bump minor` 随之顺延 ✓）
- [x] `T-K13` **K1-c（备选）：`EnvBuilder::snapshot()` 克隆式检查点**（内核侧 ✅ 已完成：`Clone` ×6 + `Dag: Clone` + `snapshot()` + 判据；**front 接线 ⏸ 存档**，归入「把 check-then-add 移进 walk」的内核级重构档）
- [x] `T-K22` **K1-d：记法消解的类型查询走局部书写类型（G-34；本刀是缓解，根治在 T-K20′）**
- [x] `T-K30` `build <dir>` 不再逐文件各编一份闭包（✗ **重新定级·存档**：三轮取证证明现有 API 下做不到 —— 缓存键是逐入口 `plan.digest` ⇒ 同模块各文件永不共享闭包编译；分组会改 `build.summary` 语义（入口视角 vs 全项目 `has_errors`）；`plan_project` 只加载**单入口闭包** ⇒ 需**新 front API**「把一个模块根的全部文件当一个 unit 集编一次」。基线已量：**146.07s / 35 文件**）
- [x] `T-K31` `TcCache::new` 每次 `with_ctx` 清 4MB（✗ **实测无收益 ⇒ 已回退**：池化后冷跑 12.29s/11.67s vs 改动前 11.96s/11.67s（噪声内）；机制是 `vec![0u8; 1<<22]` 走 mmap **惰性零页**、池化反而强制 memset。阴性结果在 `docs/perf/ledger.jsonl`）
- [x] `T-K40` 内核改动台账 + 文档（✅ 2026-09-24：`architecture.md` §6 补 4 行（`with_env`/`snapshot`/interner+`Dag` 的 `Clone`/T-K31 回退的阴性记录）；`docs/PERF.md` 新增「线 K 的实测账」对比表，**每行都带复现命令** ✓；`by-tactics.md` 的「根因未修」改为**机制级 as-built 但明确仍**未修**** ✓（K1-a 零收益、K1-b 撞 `decl_idx` 墙 ⇒ 归内核级重构档））
- [x] `T-K41` `STATUS.md` 的"根因未修"话术更新（✅ 2026-09-24：原行已被第 87 轮重写覆盖，**意图以更明确的形式补回** —— 快照里写明「**性能根因：未修** ✗，不许当成已修；属内核级重构档」，并指向靶心数字与三个验收工具）
