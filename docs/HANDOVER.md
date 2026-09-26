# 交接文档（HANDOVER）

> 目的：一页看清「现在在哪、还剩什么、怎么继续」。权威仍在
> `REQUIREMENTS.md`（要求总账）、`STATUS.md`（逐轮日志）、`ROADMAP.md`（里程碑）；
> 本文是**汇总与索引**，随轮次更新。
>
> 快照：**v0.64.1**（2026-09-21 第一百二十三轮：**G-34** —— 记法消解的
> `judge_infer` 与 G-31 是同一个病（缓存键含整段前缀 ⇒ 未命中就整段前缀重跑一趟
> pass）；T-K22 落地（局部变量先取书写类型），`unit12-solution` **11.4s → 7.5s**，
> 内核零改动。**根治仍在 T-K20′**，且那套设施必须同时覆盖 `judge_pairs` 与
> `judge_infer`。见 §1.5。上一个已发布版本是 0.64.0）
> （语言线五刀 + 课程门禁 + 站点页）**。**这一版装了什么（全部用户可见）**：
> * **签名受检**（G-01 / WO-004）：值位是 `sorry` 时签名也过内核的类型/Prop 判定；
>   坏签名 = 一条 diagnostic + 声明 `Failed` + **不发** `exercise.open`。判卷只认
>   `decl.checked`/`diagnostic`——`exercise.open` 计数对签名腐烂**永远是盲的**。
> * **构造子命名空间**（G-02 / WO-005）：规范名 `Ind.mk`，裸名降为**闭包级解析别名**
>   （撞名报 `elab-ambiguous-ctor-alias`）；源文件自带 `inductive Nat` 的归约形态变化
>   已逐条实测重钉（课程零改动）。
> * **Prop + Type 参数 + 单构造子归纳**（G-03 / WO-006）：派生 recursor 的宇宙参数
>   **逐字镜像内核**（不再 panic）；课程侧 `courses/set-theory/lib/Exists.sokonanoda`
>   已从公理三件套升级成**真归纳**（名字与签名逐字不变）。
> * **L1 prelude**（L-01/L-02）：Full 模式自带逻辑与等式骨架 **30 个名字**
>   （`PRELUDE_NAMES` 12 → 42），**族粒度让位** ⇒ 入门课"自建骨架"的教学零改动。
> * **用户自定义记法第一刀**（G-04 / WO-011）：`infix:N`/`infixl:N`/`infixr:N`/`notation`；
>   数学符号独立 token、elab 内源到源重写（自动补前导类型参数）；**文件内作用域**、
>   **零事件**、点名形式永久可用。**第二刀未做**：`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ`、跨 `import`
>   的记法、binder 记法、记法重载。
> * **`course` 认 `import`**（G-06 / WO-007）与 **`query check` 同口径**（G-10 + G-17 /
>   WO-003）：解析不了不再假绿（`failed[]` + **exit 1**）——**有意的契约变更**。
> * **课程门禁**（G1–G5，`courses/set-theory/tools/check.py`）已接进 **`scripts/soko gate`
>   与 CI**（`test` job 的 step + report artifact）；**缺口台账门禁**（`scripts/gap.py
>   selftest` + `check`，`repro_expect` 支持"修好 = 判红"）同轮接入，24 条缺口
>   18 条关账、`check` 全绿；**站点有卷 I 页面**
>   `site/set-theory.html`（数据由 `scripts/gen-site-data.py` 生成，计数由门禁实测）。
> 前几轮：第一百〇五轮 = G-04 记法第一刀；第一百〇四轮 = L-01/L-02（L1 prelude）；
> 第一百〇三轮 = G-03（派生 recursor 的宇宙参数）；第一百〇二轮 = G-02（构造子命名空间）；
> 第一百〇一轮 = G-01（开练习的签名纳入类型检查）；第一百轮 = G-10 + G-17；
> 第九十九轮 = G-06（`course` 认 `import`）。
> 仓库根入口 `AGENTS.md`。
> **`redundant-sorry` 已并入 0.58.0**（原 0.56.2 线，2026-09-17 已单独发布过
> `v0.56.2`）：值位"多接了一行 `sorry`"由 kernel 终审（删掉该实参后整条声明必须能
> 过），warning 带 hint、洞级 `redundant` 标记、LSP 不再叠 "not yet solved"；
> 设计 = `docs/design/redundant-sorry.md`。**合并时修掉一个真 bug**：内核终审的
> warning 在项目模式下会丢归因（`split_report` 原先只重算语法级），现在
> `CompileOutput.warning_cmds` 与 `warnings` 平行、按命令下标归因。
> **DeepSeek Harness 适配已落地**：`docs/design/deepseek-harness.md`（H0–H4 全绿，
> 用法见 `dsh/README.md`）；仅 H5（Infoview/诊断通道/插件包）留 backlog。
> **内核真相查询通道**已落地：`docs/design/agent-query-channel.md`（I15，`query` + MCP）。
> **多文件 `import` 与项目管理**已落地（I16，0.57.0）：`import Foo.Bar` + 可选
> `sokonanoda.toml`、闭包编译（内核零改动）、闭包哈希缓存、CLI `--root`/`--no-project`、
> LSP 多文档 + 跨文件跳转/引用/改名 + 改依赖自动刷新下游、单元⑪ +
> `course/unit11-project/`。
> **项目状态视图**已落地（0.58.0）：`query project` / MCP `project` /
> LSP `soko/project` / VS Code「项目」树（根、清单来源、闭包模块 + 状态、诊断）；
> 设计 = `docs/design/project-view.md`。**余项**见 §3 G 与 `docs/TESTING.md` §5.7
> （只剩 P7 项）。

## 1. 30 秒接手

```bash
sokonanoda setup        # 用户/agent：版本锁定下载 CLI+LSP（零 cargo，幂等）
sokonanoda doctor       # 0=就绪 3=未就绪
sokonanoda gate         # 贡献者门禁：fmt + clippy + test + playground 锚点（需要 cargo）
                        #   + 课程门禁（卷 I，python3）+ 缺口台账门禁（python3）；探不到 python3 ⇒ exit 3
python3 scripts/gap.py check      # 单跑台账契约：每条缺口的复现必须与 status 一致（含 repro_expect 覆盖）
cargo test --workspace --locked   # 全量（4 个 lib + 12 个集成测试文件）
```

- **用户/agent 路径零工具链**：执行只用 `sokonanoda` 子命令 / Release 二进制 / VSIX；
  **禁止**要求安装 Rust/cargo（REQUIREMENTS §2 第 9 条）。
- **贡献者**才需要 cargo；CI 与本地命令一致。
- 判定永远走 kernel，**禁止文本比对**（REQUIREMENTS §2 第 4 条）。

## 4. 已知限制 / 技术债

- **0.58.0 新增（批次 4，已完成）**：项目状态视图 `query project` / 对应 MCP 工具
  `project` / LSP `soko/project` / VS Code「项目」树 + 状态栏 tooltip；
  `ModuleReport::status`（`compiled`/`load-failed`/`blocked`）是**新增字段**，
  改项目层报告时别丢它（`ModuleStatus` 的 code 就是协议）。设计 =
  `docs/design/project-view.md`（§9 写明不做依赖图/写操作/模块级缓存）。
- **剩余结构债（按建议顺序，2026-09-18 盘点）**：`run_pass` 已拆完（批次 3），
  仓库里仍超 ~500 行惯例的大文件按优先级排：
  ① `front/src/compile/tests.rs` 4828（**测试**模块，按 `walk`/`kernel_phase`/
  `units` 分文件最省事）；② `front/src/compile/elab.rs` 2854（elaborator 本体，
  可按 `build_def`/`build_theorem`/`install_inductive_block`/`ElabScope` 切）；
  ③ `front/src/parser.rs` 2065；④ `lsp/src/lib.rs` 1554；⑤ `editor/vscode/extension.js`
  1493。切割一律"只动位置不动语义 + 事件计数契约 + 二进制对拍"，一次一刀。
- **开练习的类型子表达式没有 hover 行**（本轮盘点发现，**刻意保留现状**）：
  `def`/`theorem`/`example` 的开练习路径把 `elab_expr` 的 hovers 收进一次性
  `Vec::new()`（`def` 曾额外推一个 `nodes: Vec::new()` 的空 `CmdHover`——纯空操作，
  本轮删除），所以"练习签名里的 `And`/`Nat` 悬停只有源码切片、没有类型行"。
  真要补：把 `hovers` 收进 `CmdHover` 即可（`env_at` 有效），但那是行为变更，
  需配 LSP 回归 + 扩展现有 `hover` 测试。
- **spine meta 方案 A** 仍有缺口：更深嵌套、`def` 包裹结果类型的 whnf 展开
  （需内核/pp 暴露 whnf，违反冻结）→ 仍走 B′；见 `docs/design/spine-meta-a.md`。
- **参数化归纳 v1**：带索引、宇宙多态参数、互/嵌套递归不做。
- **`match` 依赖 motive**：`judge_infer` 往返限制已修（0.39.1，见 §3 A）；其余同 B。
- **prelude `Nat` 经 `Nat.rec` 归约**：结果可能显示为不合并一元链（与 numeral
  def-eq），已文档化并钉测试；`#reduce 1 + 1 => 2`、`three => 3` 正常。
- **perf 套件是哨兵**：外部基准（Lean Kernel Arena）已立项但 opt-in
  （`scripts/perf-arena.sh`）；见 `docs/PERF.md`。
- **`TESTING.md §5` 盲区**：编辑器 codeLens/quick-fix 已补进程内 rpc；VS Code
  Electron 集成走 `editor/vscode/src/test/extension.test.js`。
- **（2026-09-18 已闭环，留档）项目入口没有 quick-fix / 子洞探针**：三层根因都补齐
  ——判据前缀（`QueryDoc::judge_prefix`）、`suggest_with`/`probe_sub_goal_types_with`、
  闭包级 `GoalTemplates`；`didChangeWatchedFiles` 同批完成。见 `docs/TESTING.md` §7b。
- **（2026-09-18 已闭环，留档）`query` 不走项目闭包缓存**：现在 `check`/`build`/`query`
  共用 `crates/cli/src/project_cache.rs` 的同一份摘要键，且 `QueryDoc::check()` 不再
  二次编译（复用 `set_text` 存下的 `CompileOutput`）。3×12 项目实测：冷 49→25ms、
  热 37→3.4ms。`--text` 中间态仍不缓存。数字见 `docs/PERF.md`。
- **（批次 3 进行中，2026-09-18 第二刀完成）`run_pass` 尾部已出**：
  第一刀把 `SourceUnit` / `unit_ranges` / `split_report` / `compile_all_units`
  移到 `crates/front/src/compile/units.rs`（108 行）；第二刀把 `check.rs` 改成
  目录模块，`run_pass` **尾部**（`EnvBuilder::finish` 之后的内核阶段 + 签名/cutoff
  + 报告装配，≈360 行）原样搬进 **`check/kernel_phase.rs`**（`Walked` 结构体接原
  局部变量，`finish_pass(walked) -> PassResult`）。
  第三刀（**批次 3 完成**）：命令走查主循环（≈750 行）进 **`check/walk.rs`** ——
  `Walk` 持可变累加器（builder / known_universes / inductives / out / ops /
  cmd_hovers / decl_states / example_idx / built_inductives），`CmdCtx` 持每命令
  派生的只读上下文（idx / templates / `Cow` 前缀 / trusted / env_before / options /
  skip），每个 `Command` 变体一个方法，arm 的 `continue` 改 `return`。
  现状：`check/mod.rs` **791** + `walk.rs` **951** + `kernel_phase.rs` **413**
  （原 1918 行单文件、`run_pass` ≈1174 行）。
  **坑（写在这里省下一次 debug）**：`CmdCtx` 的 `&'x T` 字段直接拷进 `ElabCtx` 会让
  `ElabCtx<'arena, 'b>` 的 `'b` 被统一到 `'x`，于是 `'arena: 'x` 变成方法签名上
  无法证明的义务（rustc 会提示 "add explicit lifetime `'arena` to the type of `c`"）；
  两个解法都用上了：`&'a UnivMap<'a>` 那种"两个寿命写成同一个"的字段要拆开（或者
  干脆就地建空表，`HashMap::new()` 不分配），其余引用过一次恒等函数 `local()`
  重借成局部寿命。**不要**用 `&*c.field`——`clippy::borrow_deref_ref` 是 deny。
  验收：`cargo test --workspace --locked` 862 passed；**二进制对拍**（改动前后两个
  CLI 跑全部 58 个 `.sokonanoda` + `--root`/`--no-project`/stdin/`query check|goals|
  holes`）输出逐字节相同。
- **（0.57.0 新增，结构债，部分清偿）`crates/front/src/compile/check/` 1940 行**：I16 把闭包
  编译加在这里（`compile_all_units` / `split_report` / `unit_ranges` /
  `top_level_def_spans_over`，+201 行），`run_pass` 一度是 553–1726 行的单个函数
  （≈1174 行，main 时已 ≈970 行）。**批次 3 已清**：尾部 → `check/kernel_phase.rs`、
  命令走查 → `check/walk.rs`，`run_pass` 现在只剩闭包装配 + 前缀合成 + 调用两段
  （见上一节）。教训：位置搬移要配"二进制对拍"（同一批输入的 stdout 逐字节比较），
  比 golden 单测覆盖面大得多；`cargo fmt` 会重排搬过去的代码，对拍前先归一化空白。触发点：任何再往 `run_pass` 里加分支的需求。
- **（0.57.0 已闭环）多文件 LSP 的跨文件失效与跨文件改名**：改依赖 ⇒ 含它的打开
  文档自动重编译重发；`references`/`rename` 都跨文件；未落盘的依赖编辑通过**内存
  覆盖**（`load_closure_with_overlay` / `QueryDoc::set_text_with_overlay`）进入闭包，
  也进闭包摘要。曾经"实现会挂"的结论是**测试写法**问题：服务端一次通知可能连发
  多条诊断，测试必须先排空再等通知（`testutil::notify_with_drain`）。细节与教训见
  `docs/TESTING.md` §5.7。`soko/project` 已在 0.58.0 落地（`docs/design/project-view.md`）；
  仍未做（P7）：`[deps]`、`namespace`/`open`。
- **（0.56.1 已清）`crates/lsp` 测试文件的拆分**：0.56.0 把测试模块移出 `lib.rs`
  时形成过 `tests.rs` 2567 行的债，0.56.1 已按"`tests/mod.rs`（共享夹具）+
  按特性分文件"拆完：`mod.rs` 399 行（31 个共享 const/fixture + `pub(crate) use`
  再导出）+ `hover` 392 / `lenses` 366 / `navigation` 280 / `state` 262 /
  `goals` 252 / `lifecycle` 242 / `hover_brackets` 167 / `tokens` 122 / `perf` 117，
  `by_sorry_range_tests.rs` 60 原地保留；`lib.rs` 仍 1105 行（`mod tests;` 自动
  解析到 `tests/mod.rs`）。**测试体、断言、测试名零改动**（95/95 函数名一致、
  220 条 assert 记账相等、规范化行流只差 plumbing 的 `use`/`mod` 行）。

## 5. 环境 / Gotchas（本会话实打实踩过）

- **编辑器 LSP 与发布版本可能不一致**：
  - VS Code 扩展**默认强制用内置 LSP**（0.31.0 起）；`sokonanoda.serverPath` /
    `SOKONANODA_LSP_BIN` / 工作区构建**默认被忽略**，要覆盖需开
    `sokonanoda.serverOverride`（贡献者）。用 `sokonanoda: doctor` 看解析来源
    （`source=`）与版本；`restart server` 回执带 `source=`。
  - opencode 插件**优先用仓库构建** `target/{debug,release}/sokonanoda-lsp`；
    改 Rust 后需重启（`cargo build -p sokonanoda-lsp` 或 `sokonanoda gate` 会刷新
    debug 构建；注意 `cargo test`/`clippy` **不一定**重编该 bin）。
- **旧扩展版本**：VS Code 在**下次完整启动**时清理 `.obsolete` 旧版本；长时间不整退
  会堆积（只占磁盘，不影响解析——扩展始终用运行中那份的内置 bin）。
- **`sokonanoda version` 报的是下载缓存**，不是编辑器在跑的服务器；要看真实运行版本
  用 LSP 的 `soko/version`。
- **CI：marketplace-publish 间歇 Azure gallery 超时**（已多次记录）：`build`/
  `package-vsix`/`github-release` 会先成功（Release 25+1 资产齐全），仅发布步骤失败；
  探活后 `gh run rerun --failed` 即可。台账 `docs/CI-FAILURES.md`。
- **发布资产现为 26 个**（8 lsp + 8 cli + 9 vsix + `SHA256SUMS`），另附 SLSA
  provenance；校验见 `docs/RELEASE.md` §6。
- **本地 perf 哨兵在机器重载时会误报**（`incremental_edit_anywhere_is_fast` 受
  CPU 争用）；单跑通过即环境问题，重跑 `gate`。
- **Xcode 许可未接受会让 cargo 直接构建失败**（2026-09-17 本机实测，Xcode 27）：
  `xcrun --sdk macosx --show-sdk-path` 与 `ar` 都被系统拒绝（exit 69），链接必挂。
  绕过（不改系统设置）：`DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --workspace --locked`；
  根治：`sudo xcodebuild -license accept`。与仓库代码无关。
- **DSH 侧三条硬事实**（`docs/design/deepseek-harness.md` §1.2）：技能根只有
  `.dsh/skills` / `.agents/skills`；LSP 只读且**丢掉服务端诊断**；项目不能改
  工具调用的 PATH、也没有项目级钩子 → 一律用 `scripts/soko`。

## 6. 关键文档索引

- **`docs/design/lean-style-0.62.md`** — 全课程 Lean 4 化（**未发布** 0.62.0 批次）的
  用户可见特性清单 + 课程事实（当前计数、入门课删自建 `And`/`Or` 骨架的后果）+ 站点/文档 agent
  需要知道的三个坑（G-21 报错仍半修、记法在实参位的边界、记法对照页的双写法是故意的）。

- 要求总账：`REQUIREMENTS.md`（§2 硬规则、§9 追加日志）
- 进度：`STATUS.md` / 归档 `docs/STATUS-ARCHIVE.md`
- 里程碑/待办：`ROADMAP.md §10`
- 架构/gotchas：`docs/architecture.md`（§6 内核改动清单、§8 gotchas）
- 协议：`docs/protocol.md`；测试地图：`docs/TESTING.md`
- 发布：`docs/RELEASE.md`；CI 台账：`docs/CI-FAILURES.md`
- VS Code 规范：`docs/vscode-dev-guide.md`
- 设计：`docs/design/`（新增见 `docs/README.md` 列表）
- 技能：`skills/sokonanoda-{dev,teacher,ci}`


---

> **会话过程部分已归档** ✓（3 节：最新一轮快照 / 本会话完成的工作 / 剩余 TODO —— 均已过期，
> 现状以 `STATUS.md` + `docs/E2-HANDOVER.md` 为准 ✓）⇒ `docs/archive/handover-process-2026-09-26.md.gz`（`gunzip -c … | less` ✓）。
