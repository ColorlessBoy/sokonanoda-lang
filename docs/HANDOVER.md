# 交接文档（HANDOVER）

> 目的：一页看清「现在在哪、还剩什么、怎么继续」。权威仍在
> `REQUIREMENTS.md`（要求总账）、`STATUS.md`（逐轮日志）、`ROADMAP.md`（里程碑）；
> 本文是**汇总与索引**，随轮次更新。
>
> 快照：**v0.58.0**（2026-09-18），最近一轮 **第九十七轮**。仓库根入口 `AGENTS.md`。
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
cargo test --workspace --locked   # 全量（4 个 lib + 12 个集成测试文件）
```

- **用户/agent 路径零工具链**：执行只用 `sokonanoda` 子命令 / Release 二进制 / VSIX；
  **禁止**要求安装 Rust/cargo（REQUIREMENTS §2 第 9 条）。
- **贡献者**才需要 cargo；CI 与本地命令一致。
- 判定永远走 kernel，**禁止文本比对**（REQUIREMENTS §2 第 4 条）。

## 2. 本会话完成的工作（第五十三～八十四轮，全部已发布）

> 第八十五轮之后（`query` 通道、I16 项目层、批次 1–4）逐轮记录在 **`STATUS.md`**
> 与下文 §3；本表保留早期轮次不重复。

| 轮 | 版本 | 内容 | 设计 / 证据 |
|---|---|---|---|
| 53 | — | TODO 清账：穷尽守卫修复、codeLens/quick-fix 测试、内核 2 个 ignored fixture 重建解禁、`install.sh`+`.devcontainer`、4 份大项设计 | 见各 `docs/design/*` |
| 54 | 0.28.0 | 值位 `let`（Phase 1） | `docs/design/elaborator-let-match.md` |
| 55 | 0.29.0 | 编译器服务事件流：`file.didChange` + `service.hello` + `--doc/--workspace` | `docs/design/compiler-service-events.md` |
| 56 | 0.30.0 | VS Code webview Infoview 目标面板 | `docs/design/webview-infoview.md` |
| 57 | 0.31.0 | 扩展**强制内置 LSP**（`serverOverride` opt-in）+ `sokonanoda: doctor` 自检 | `docs/design/extension-server-policy.md` |
| 58 | 0.32.0 | spine meta 方案 A（请求期内核探针补子洞期望类型） | `docs/design/spine-meta-a.md` |
| 59 | 0.32.1 | I8 early-cutoff 依赖精确化 + arena 基准立项 | `docs/design/early-cutoff.md` |
| 60 | 0.33.0 | `match` v1（非递归源内归纳） | `docs/design/match.md` |
| 61 | 0.33.1 | tactic hover 呈现升级（代码围栏 + 高亮 + 排版） | `docs/design/tactic-hover.md` §5 |
| 62 | 0.34.0 | 无注解 `let`（内核推断绑定类型） | `docs/design/elaborator-let-match.md` §13 |
| 63 | 0.35.0 | `match` 递归归纳（自动插入 IH） | `docs/design/match.md` §10 |
| 64 | 0.35.1 | 发布加固：`SHA256SUMS` + SLSA provenance | `docs/RELEASE.md` §6 |
| 65 | 0.36.0 | `match` 支持 prelude `Nat`（内置 Nat 改真实可信归纳） | `docs/design/match.md` §10 |
| 66 | 0.37.0 | watch stdin 客户端命令（subscribe/unsubscribe/ping） | `docs/design/compiler-service-events.md` §9 |
| 67 | 0.38.0 | 参数化归纳声明（非带索引）+ `match` | `docs/design/parameterized-inductives.md` |
| 68 | 0.39.0 | `match` 依赖 motive（归纳法形状可用） | `docs/design/match-dependent-motive.md` |
| 69 | 0.39.1 | `judge_infer` 类型往返健壮性（Arrow domain 补括号） | `docs/design/match-dependent-motive.md` §8 |
| 70 | 0.40.0 | 统一 goal 呈现（`front::semantic` runs）+ Infoview 落右侧 + engine ^1.106 + 市场简介护栏 | `docs/design/goal-rendering.md` |
| 71 | 0.41.0 | prelude `Bool`（非递归真实可信归纳） | `docs/design/match.md` §10 Phase 5 |
| 72 | 0.42.0 | `match` 模式编译器 v1（嵌套/字面量/通配/守卫） | `docs/design/match-patterns.md` |
| 73 | 0.43.0 | 呈现面高亮统一（`sokonanoda` 围栏全量） | `docs/design/goal-rendering.md` §7 |
| 74 | 0.44.0 | Infoview 声明类型提示 + 点击跳转 | `docs/protocol.md` / `webview-infoview.md` |
| 75 | 0.45.0 | 应用位置 binder 类型推断（I6 最后一项） | `docs/design/elaborator-let-match.md` as-built |
| 76 | 0.46.0 | `match` 作为 tactic（`by` 块内）+ judge 前缀修复 | `docs/design/by-tactics.md` §2 |
| 77 | 0.47.0 | 带索引归纳（`Vec A n` 声明 + 派生 recursor + match） | `docs/design/indexed-inductives.md` |
| 78 | 0.48.0 | 编译结果缓存（olean 式）+ Infoview 类型换行/`⊢`/点击跳转 | `docs/design/compile-cache.md` |
| 79 | 0.49.0 | 共享缓存 + `sokonanoda build` + Infoview 稳定/反馈 + 高亮单一起源 | `docs/design/{compile-cache,highlighting,webview-infoview}.md` |
| 80 | 0.50.0 | Infoview 自研调色板（主题 token 解析回退） | `docs/design/highlighting.md` §3b |
| 81 | 0.51.0 | `by` 块换行分隔 tactic（`;` 或换行） | `docs/design/by-tactics.md` §11 |
| 82 | 0.52.0 | 课程大纲重构 P1（内容修补 + 测试加固 + 设计锁定） | `docs/design/course-syllabus.md` |
| 83 | 0.53.0 | 课程大纲重构 P2（`by` 提前到 #4、归纳拆 Ⅰ/Ⅱ、8 单元 + 门面同步） | `docs/design/course-syllabus.md` §6 |
| 84 | 0.54.0 | 课程大纲重构 P3（#9 关系与联结词、#10 读证明与综合 → 锁定 10 单元） | `docs/design/course-syllabus.md` §0/§6 |

> 更早轮次见 `STATUS.md`（最近 3 轮）+ `docs/STATUS-ARCHIVE.md`（第 1–94 轮原文）。

## 3. 剩余 TODO（按建议顺序）

> **状态（2026-09-17）**：§3 A/A′/A″/B/C 的**可执行项已全部完成**（0.40.0–0.47.0）；
> §3 E 的两个 front/parser 缺口**已改挂** ROADMAP **I15 → H6-C**
> （`docs/design/agent-query-channel.md`）：它们决定查询通道给出的"真相"是否完整，
> 不再是孤立的 front 待办（本轮只改挂与文档，未实现）。
> §3 D「远期 L2/L3」与 §4 的已文档化技术债（多数需内核/pp 变更，违反 kernel
> 冻结）仍在。

### A′. ~~统一 goal 呈现 + Infoview 落右侧~~ ✅ 已完成（0.40.0）
- 单一分类源 `front::semantic`（`tag_runs`/`tag_expr`/`declaration_kinds` +
  `SemanticKind::{ALL, as_str}`）；`soko/stateAt`/`soko/goals` 下发
  `goal_runs`/`ty_runs`（旧字符串字段保留）；Infoview 渲染 `tok-<kind>`
  （主题变量单一映射）；视图移入右侧 `secondarySidebar` 容器，engine
  `^1.106.0`；删握手 + 「暂时不可用」，静默回退树组；TM 语法与
  `front::semantic` 由测试锁死。见 `docs/design/goal-rendering.md`。

### A. 已完成
- **`judge_infer` 往返健壮性** ✅（0.39.1）：`proof::render_expr` 的 Arrow domain 位
  补括号；依赖 `match`/suggest/hover/level 均受益；证据 `render_expr_round_trips`
  + `match_dependent_motive_with_function_typed_binder_round_trips_safely`。

### A″. ~~呈现面高亮统一~~ ✅ 已完成（0.43.0）
> 用户 2026-09-15 追问后补做。
- 统一手段：LSP `code_block`/`CODE_LANG`、扩展 `codeMarkdown`——凡渲染
  `.sokonanoda` 文本一律 ` ```sokonanoda ` 围栏（着色只来自 `front::semantic`
  的 TM 语法/语义 token）。
- 覆盖：表达式/签名 hover（原 ` ```text `）、声明 hover（签名 + 目标态代码块）、
  补全 documentation 签名、练习树 tooltip（`appendCodeblock(…, "sokonanoda")`）。
- 刻意保持纯文本（VS Code 不渲染 markdown/无法着色）：诊断消息、inlay hint、
  TreeItem.description、CodeAction 标题；见 `docs/design/goal-rendering.md §7`。
- 契约：`code_fences_always_use_the_sokonanoda_language`、
  `rendered_language_text_uses_the_sokonanoda_fence`。

### B. `match` Phase 2 余项
- ~~**带索引归纳**~~ ✅ 已完成（0.47.0）：`num_indices>0` 声明 + index-aware
  派生 recursor + `match`（常量结果类型）；结果类型依赖索引不在 v1。见
  `docs/design/indexed-inductives.md`。
- ~~**`match` tactic**（`by` 块内用 `match`）~~ ✅ 已完成（0.46.0）：`by` 白名单加
  `match`（臂体是项，等价 `exact (match …)`）；顺带修 `judge_terms` 前缀（使
  `by exact match …` 可用）。臂体内再写一串 tactic 为后续可选扩展。
- ~~**嵌套/守卫/字面量模式**~~ ✅ 已完成（0.42.0）：有序 arm + 列式模式编译；
  通配 `_`、嵌套 `some (succ k)`、Nat 字面量（脱糖 `succ^k zero`）、Bool 守卫
  `if`；`docs/design/match-patterns.md`。剩余：`as`/or 模式、多 scrutinee、
  `if/then/else` 表达式。

### C. I6 剩余
- ~~**prelude `Bool`**~~ ✅ 已完成（0.41.0）：`install_bool_prelude`（非递归真实可信归纳，
  `Bool.true`/`Bool.false` + `Bool.rec`）；`match` 可用；文件自带 `inductive Bool`
  时让位（`explicit_bool` 闸 + `PreludeShape` 四元组）。见 `docs/design/match.md` §10 Phase 5。
- ~~**binder 类型推断（非依赖情形）**~~ ✅ 已完成（0.45.0）：有期望望远镜时可推断
  （既有）；无期望的**应用位置**从实参类型推断（`annotate_application_lambda`，
  支持柯里化），实参不足仍报 `elab-untyped-binder`。见
  `docs/design/elaborator-let-match.md` as-built。
- ~~`Nat.succ`/`Nat.add` 边界裸名 `#reduce` 测试~~ ✅ 已完成（0.42.0）：
  `bare_prelude_nat_names_stay_terminating`（裸 `#reduce Nat.add` 终止为常量）。

### E. 课程层发现（P3 新增单元 #9，2026-09-16）→ **✅ 已修（I15 / H6-C，0.56.0，2026-09-17）**

> 两条都**不再是孤立的 front 待办**：新设计 `docs/design/agent-query-channel.md`
> 要把"内核真相查询层"（`front::query` + CLI `query` + MCP）抽出来，而这两个缺口
> 直接决定查询通道给出的真相是否完整、以及 agent（主要作者）写出的合法 Lean 子集
> 会不会被拒。**处置：并入 H6-C，与查询通道同轮修复并验收**（A6）。
> **下面两条的描述已在 2026-09-17 用发布版二进制实测校正**——早先的
> "索引递归 `Prop`""IH 形状不符"两个判断都是错的。

- **`derive_recursor` 拒绝「带索引 + 字段写在结果箭头链里」的归纳**（真 bug，**✅ 已修**：
  `elab.rs` 改用 `spine_of_codomain`，测试见 `compile::tests::indexed_inductive_with_arrow_style_field_derives_recursor`
  与 `cli_arrow_style_indexed_field_derives_and_reduces`）：
  实测 `P : Nat -> Prop` + `ctor b (n : Nat) : P n -> P (Nat.succ n)` 被内核拒
  （`assert_nonnested_recursors_def_eq`，`kernel/src/inductive.rs:1706`）；
  **同一形状改具名字段 `(n : Nat) (h : P n)` 即通过**；索引 `Type`（`W : Nat -> Type`）
  一样失败、非索引（`Or` 带箭头字段）正常 → **触发条件是"有索引可丢"，与 `Prop` 无关**。
  根因在 `derive_recursor` 用只认 Ident/App 的 `src_spine` 读 ctor 结果的索引实参
  （`crates/front/src/compile/elab.rs:2613`；`src_spine` 对 `Arrow`/`Forall` 返回 `None`，
  `:1733`），箭头写法下 `ctor.result` 就是箭头链 → `ctor_indices = []` → minor 结论
  **丢掉索引实参**。修法：改用已会剥箭头的 `spine_of_codomain(&ctor.result)`
  （`:2378-2387`），**一处一行**；`small_elim`/`is_prop_block_ty`（`:2499`/`:2429`）
  与 IH 路径**都不动**（IH 用的 `spine_of_codomain` 本来就是对的）。
- **顺带发现（同函数，同轮修）**：`elab.rs:471` 把 `is_k: false` **写死**，导致
  **单构造子 `Prop`**（`inductive True : Prop` / `ctor trivial : True`）派生出的 recursor
  被内核拒：`recursor declares the wrong k-reduction flag (left: false, right: true)`
  （`kernel/src/inductive.rs:661-662`，`init_k_target` `:1268-1276`）。
- **`inductive` 参数/ctor 字段不吃多名字 binder 组**（解析器缺口，**✅ 已修**：
  `parser.rs` 新增 `parse_inductive_binders`，参数与 ctor 字段都改走 `push_binders`；
  测试见 `parser::tests::inductive_block_parses_multi_name_parameter_group` 等 3 条
  与 `cli_inductive_accepts_multi_name_binder_groups`）：
  `parse_inductive_block`（`crates/front/src/parser.rs:363`）与 `parse_ctor`（`:402`）
  调**单名** `parse_binder`，而同文件的组感知机制 `push_binders`（`:1033-1048`，
  `parse_arrow:659`/`parse_lambda:991`/`parse_forall:1013`/`parse_decl_binders:151` 都在用）
  早就支持 → 这就是"Pi 位能写、inductive 不能"的原因。修法：两处循环体换成
  `self.push_binders(&mut …)?`（**AST/elab/kernel 都不用改**）。
  **同类缺口**（一次修完）：`parse_ctor` 字段同修；`parse_let`（`:480-518`）的
  `Expr::Let{binder}` 是单个，`let a b : T := v` 连语法都不存在 → 需 AST/脱糖决策；
  tactic `intro`（`:236-247`）只吃一个名字（Lean 的 `intro a b` 同样失败）→ 独立特性。

**H6-C as-built（2026-09-17，0.56.0）**：两条都修了，课程 unit9/unit10 随之简化
（`Le`/`Even` 去掉手写 `rec`/`iota`、`Or (A : Prop) (B : Prop)` → `(A B : Prop)`，
EN 与 CN 代码逐字节一致、golden 事件计数不变）。**顺带修掉的第三个 bug**：修
`is_k` 时第一版把它近似成"字段数 == 参数数"，被 CLI e2e 抓住——内核判据是
`pi_telescope_size(ctor.ty) == local_params.len()` ⟺ 构造子**没有自己的字段**
（`Both (A B : Prop)` + `mk (a : A) (b : B)` 是反例；`Q : Nat -> Prop` + `q : Q 0`
是反向反例，它是 K 目标）。细节与"镜像内核谓词"的方法见
`docs/design/agent-query-channel.md` H6-C as-built 与 `docs/LESSONS.md`。

### F. DeepSeek Harness 适配（第八十六–八十七轮，2026-09-17：✅ 已落地，0.55.0）
- 设计与计划：**`docs/design/deepseek-harness.md`**（差距 G1–G10、DSH 侧事实
  §1.2 共 25 条带源码行号、阶段 H0–H4 ✅ + backlog H5、决策 D-1…D-6、验收 A1–A6、
  as-built §9）；用法：**`dsh/README.md`**；调研底稿：`docs/notes/dsh-project-assets.md`。
- 现在能用的：DSH 打开本仓库 → 技能与 `/sokonanoda-*` 命令自动可用；
  `scripts/soko doctor --json` 报就绪；`scripts/soko grade …` 判卷；
  `dsh web --patch ./dsh/cordis.patch.yml` 后 `lsp` 工具可 hover/跳定义。
- **三条最容易踩的 DSH 事实**：① 技能名本身即斜杠命令，但必须在
  `<repo>/.agents/skills`；② DSH 的 LSP **只有** definition/references/
  implementation/hover，**服务端诊断被忽略**、`soko/*` 无消费者，判卷一律走
  CLI `--json`；③ 项目/家目录 `.env` 都**不能设 PATH**，也没有项目级钩子 →
  一切走 `scripts/soko`。
- H5 backlog：B1 Infoview 客户端插件、B2 诊断通道（DSH 演进或 MCP）、
  B3 `SessionStart` provisioning、B4 把启动器+拦截+命令做成 npm 插件包。

### G. 多文件 `import` 与项目管理（I16，**0.57.0 已落地**）
- 设计 + 调研 + **as-built**：**`docs/design/imports-and-projects.md`**（§5.1 有三处与
  设计的偏差、交付物清单、P5 的实现选择与剩余 P7 项）；调研底稿：
  `docs/notes/multifile-prior-art.md`、
  `docs/notes/project-roots-and-incremental-caches.md`。
- 一句话：编译单元从「一个文件」升级为「项目闭包」——`import Foo.Bar`（Lean 置顶语法）、
  模块名↔路径（Lean 同款，`-` 非法）、模块根 = `--root` > 最近 `sokonanoda.toml`
  （上溯止于 `.git`/HOME）> 入口文件目录（**无清单也能 import**）；跨模块声明在同一
  arena/`EnvBuilder` 里按拓扑序入表（**内核零改动**）。
- **不变式**：无 `import` 的文件行为**逐字节不变**（缓存键/事件流/golden 计数不动），
  由 `crates/cli/tests/imports.rs::import_free_files_are_byte_identical_to_the_single_file_path` 守住。
- **接手前必须知道的三条**：① 诊断**按命令下标**归属文件（`CompileOutput.error_cmds`），
  按 span 会串文件；② 项目模式只对"解析出 import"的文件启用（A1 是硬不变量）；
  ③ `crates/front/src/project/` 是唯一闭包实现，改它先读 `docs/architecture.md` §4.5。
- **性能例行化（0.57.0）**：`scripts/perf-ledger.sh` 跑全部 perf 套件并把分阶段
  记录追加进 `docs/perf/ledger.jsonl`（跨版本对比"哪一环退化"）；基线见
  `docs/PERF.md`「项目层与编辑器宿主」。扩展侧新增宿主层行为测试
  `editor/vscode/test-extension-host.js`（stub host，7 例）。
- **课程共享库（0.57.0）**：`course/shared/` 是"教学内容 import 化"的安全形态
  （规范模块 `And`/`Or`/`Nat` + 自检入口 `Demo.sokonanoda`），画布保持自给自足；
  `crates/cli/tests/course_shared.rs` 双向守护 24/12/8 份副本。整包 import 化明确不做。
- **判据前缀（0.57.0 修的坑）**：项目模式下 `match`/`by` 的前缀必须含依赖声明
  （`walk.rs` 里的 `closure_prefixes` → `CmdCtx::prefix_src`），否则入口看不见导入的
  名字——这条在 `docs/architecture.md` §4.5 有专段，改判据相关代码前先读。项目入口的
  quick-fix 已修（批次 1），§7b 已闭环。
- **LSP 能力（0.58.0 完整）**：多文档、跨文件 `definition`/`references`/`rename`、
  改依赖自动刷新下游（未落盘编辑经内存覆盖可见）、诊断只在变化时重发、
  **`soko/project` 项目状态视图**（根/清单来源/闭包模块表/每模块状态；VS Code
  项目树 + 状态栏 tooltip；CLI `query project` 与 MCP `project` 同源）。
  留 P7 backlog 的只有：`watch` 项目模式、`[deps]`、`namespace`。
  编辑器外的改动（`git checkout`/脚本）自 2026-09-18 起会自动刷新已打开文档
  （`workspace/didChangeWatchedFiles`：只重编译闭包里含该路径的那些，缓冲优先）。
  多文档测试必须用 `testutil::notify_with_drain`（原因见 `docs/TESTING.md` §5.7）。
- 三道"静默错误"门仍在（`front/tests/perf.rs`、`cli/tests/watch.rs`、judge/suggest
  静默无建议）：改项目层时别让它们变成假绿。

### D. 远期（L2/L3）
- 协作/多用户、远程；compiler service 的跨文件转播 / `setContent`（v1 未做）。
- DSH 侧 Infoview/诊断通道（`docs/design/deepseek-harness.md` §5 H5 的 B1/B2）。

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

- 要求总账：`REQUIREMENTS.md`（§2 硬规则、§9 追加日志）
- 进度：`STATUS.md` / 归档 `docs/STATUS-ARCHIVE.md`
- 里程碑/待办：`ROADMAP.md §10`
- 架构/gotchas：`docs/architecture.md`（§6 内核改动清单、§8 gotchas）
- 协议：`docs/protocol.md`；测试地图：`docs/TESTING.md`
- 发布：`docs/RELEASE.md`；CI 台账：`docs/CI-FAILURES.md`
- VS Code 规范：`docs/vscode-dev-guide.md`
- 设计：`docs/design/`（新增见 `docs/README.md` 列表）
- 技能：`skills/sokonanoda-{dev,teacher,ci}`
