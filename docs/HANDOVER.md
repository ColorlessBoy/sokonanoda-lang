# 交接文档（HANDOVER）

> 目的：一页看清「现在在哪、还剩什么、怎么继续」。权威仍在
> `REQUIREMENTS.md`（要求总账）、`STATUS.md`（逐轮日志）、`ROADMAP.md`（里程碑）；
> 本文是**汇总与索引**，随轮次更新。
>
> 快照：**v0.61.0**（2026-09-19 第一百〇八轮：用户要求「设计文档里的东西都做了吧」⇒ 把各篇设计的「未做/第二刀/残留边界」里不违反硬规则的全部实现：记法第三刀（binder 记法/重载/scoped/集合字面量/一元实参位）、namespace 扩展（子句/open…in/export/遮蔽 warning）、层级算术 + Eq 多态 + cast/Eq.ndrec、编辑器词表同轮 + 课程多清单聚合 + 成本台账；保留的只有内核冻结三项；`scripts/soko gate` exit 0：1163 passed、课程 36·329·99·0、台账全绿；上一个已发布版本是 0.60.0）。
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

## 2. 本会话完成的工作（第五十三～八十四轮的已发布项；第九十九～一百〇六轮 = 0.59.0 这一批）

> 第八十五～九十八轮（`query` 通道、I16 项目层、0.58.0 批次 1–4）逐轮记录在
> **`STATUS.md`** 与下文 §3；本表只列这两段。

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
| 99 | 0.59.0 | **`course` 认 `import`**（WO-007 / G-06：聚合与本单元 `grade` 同判、共用闭包摘要键） | `docs/gaps/WO-007-course-import.md` |
| 100 | 0.59.0 | **查询通道不再假绿**（WO-003 / G-10 + G-17；`query check` 退出码 0→1，**有意的契约变更**） | `docs/design/agent-query-channel.md` |
| 101 | 0.59.0 | **开练习的签名纳入类型检查**（WO-004 / G-01：坏签名 = diagnostic + Failed + 不发 `exercise.open`） | `docs/gaps/WO-004-open-exercise-signature.md` |
| 102 | 0.59.0 | **构造子进入类型命名空间**（WO-005 / G-02：规范名 `Ind.ctor` + 裸名闭包级别名） | `docs/design/ctor-namespace.md` |
| 103 | 0.59.0 | **派生 recursor 的宇宙参数镜像内核**（WO-006 / G-03）；课程侧 `lib/Exists` 升级真归纳 | `docs/design/prop-large-elim-mirror.md` |
| 104 | 0.59.0 | **L1 prelude**（L-01/L-02：逻辑与等式骨架 30 名字 + 族粒度让位；P1/P2/P3） | `docs/design/prelude-l1-proposal.md`（含 as-built） |
| 105 | 0.59.0 | **用户自定义记法第一刀**（WO-011 / G-04：`infix`/`infixl`/`infixr`/`notation`；文件内作用域 + 自动补前导类型参数；课程零改动） | `docs/design/notation-subset.md` |
| 106 | 0.59.0 | **0.59.0 收尾**：版本 bump（`Cargo.toml`/`package.json`/`Cargo.lock`）+ 课程清单 `requires 0.59` + **课程门禁接 `gate` 与 CI** + **站点卷 I 页面** + STATUS/HANDOVER/课程 README/teaching-project/REQUIREMENTS 同步 | `docs/design/course-gate-in-ci.md`、`STATUS.md` 第一百〇六轮 |

> 更早轮次见 `STATUS.md`（最近 3 轮）+ `docs/STATUS-ARCHIVE.md`（第 1–103 轮原文，
> 另收 0.56.2 线的第九十一轮续）。

## 3. 剩余 TODO（按建议顺序）

> **状态（2026-09-19，0.59.0 收尾）**：§3 A/A′/A″/B/C 的**可执行项已全部完成**
> （0.40.0–0.47.0）；§3 E 的两个 front/parser 缺口**已改挂** ROADMAP **I15 → H6-C**
> 并**已修**（0.56.0）；§3 G 的语言线项（G-01/G-02/G-03/G-04/G-06/G-10/G-17 +
> L-01/L-02 + 课程门禁 + 站点卷 I 页）**已全部落地并关账**（0.59.0）。
> **只剩**：§3 B/C 的"v1 不做"边界（`as`/or 模式、多 scrutinee、`if/then/else`、
> 宇宙多态参数、互/嵌套递归）、§3 D 的远期 L2/L3、§3 G 的 P7 backlog（`watch` 项目模式、
> `[deps]`、`namespace`），以及 §4 的已文档化技术债（多数需内核/pp 变更，违反 kernel 冻结）。

### 0. ✅ **0.56.2**（patch，2026-09-17）——`redundant-sorry` 诊断 （已并入 0.58.0，见 §3 G/本轮第九十八轮）

- 内容：学习者把答案写全、只多留一行 `sorry` 时，不再说"还没证出来"，改报
  `redundant-sorry`（CLI/`--json`/LSP 三处同一条 warning，span 收窄到那个 `token`；
  语义不变，仍 `exercise.open`）；洞级 `redundant` 标记在 `query goals`/`holes`
  与 `soko/goals` 两视图一致。设计与验收：`docs/design/redundant-sorry.md`。
- **已完成**：两处版本号 = `0.56.2`（`Cargo.lock` 已由 `cargo check` 跟上）、
  `editor/vscode/CHANGELOG.md` 的 `## [0.56.2]` 条目、`target/` 重建（CLI 自述
  `sokonanoda 0.56.2`，启动器解析回 `repo-build`）、扩展纯 Node 单测 35 条全绿
  （`npm run test:unit`）、`cargo test --workspace --locked` + `scripts/soko gate` 全绿。
  **VSIX 打包与 `code --install-extension` 没在本地跑**（release 构建 + 装进你的
  VS Code 属于有副作用的动作）：CI 的 release 会出全部 9 个 VSIX；要本地冒烟就
  `cd editor/vscode && npm run package:host && code --install-extension sokonanoda.vsix --force`。
- **已发布**：push `dd1902d` → `ci` 绿（含 auto-tag）→ `release` **11 job 全 success**
  → Release **26 资产**（lsp ×8 / cli ×8 / vsix ×9 / `SHA256SUMS`）+ Marketplace
  收录 `0.56.2`（索引延迟 ≈5 分钟）。**发布产物实测**：下载 darwin-arm64 CLI →
  `shasum -c` OK → `--version` = 0.56.2 → 对"多留一行 sorry"的文件 `--json` 出
  `warning[redundant-sorry]`。这次 **没有**撞上那 4 次复发的 Azure gallery 超时。
- 版本级别依据（`docs/vscode-dev-guide.md` §2）：**没多出新能力，只是反馈更正确
  ⇒ patch**。

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
  另有两个人**人工**运维命令 `/sokonanoda-update`（刷新缓存）与
  `/sokonanoda-doctor`（只读诊断）——DSH 命令名文法不允许 `/`，所以 opencode 的
  `/sokonanoda/update` 在 DSH 侧拼作 `/sokonanoda-update`（第九十一轮）；
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
- **课程聚合认 `import`（WO-007 / G-06，语言线本轮）**：`sokonanoda course <course.json>`
  对**有 `import` 的单元**走同一份项目闭包（与 `grade`/`query check`/`build` 共用
  `ProjectPlan::digest` 摘要键）：计数只取入口模块、`failed` = 闭包内所有模块的
  `events.errors` 之和（⇒ `failed == 0` ⇔ `grade <该单元>` exit 0）；模块根 = 单元最近的
  `sokonanoda.toml`（嵌套子项目优先），没有则回退 `course.json` 所在目录；**无 `import`
  的单元仍逐字节走单文件** ⇒ `course/course.json` 的两处 GOLDEN 不动。回归
  `crates/cli/tests/course_project.rs`（8 例：夹具转绿 / 与 grade·query 同判 / 负例不假绿 /
  模块根回退与嵌套优先 / cwd 无关 / 与 `build` 共用缓存）；复现
  `docs/gaps/repro/G06-course-import.sh` 修后 = exit 1（行为已变，`gap.py close` 的前置）。
  卷 I 实测：`node scripts/soko course "$PWD/courses/set-theory/course.json" --json`
  = 12 单元全 compiled、`checked:63 / open:93 / failed:0`。
- **查询通道的 parse 假绿清零（WO-003 / G-10 + G-17，语言线本轮）**：`front::query`
  的 `parse_error` 原先**只写不读**——`check` 把"这份文本解析不了"答成全零 +
  `failed: []`（退出码 0），`goals`/`holes` 答空数组 + `ok:true`，而同一份文本走
  `grade` 是对的。现在：`check` 把 parse 诊断合成进 `failed[]`（`counts` 保持全 0、
  `ok:true`、退出码 1）；`goals`/`holes`/`next_hole` 改返回 `Result<_, QueryError>`，
  解析失败 = `not-parsable` + `ok:false` + 退出码 1（"正常的没有"仍是空数组/`None`）。
  LSP 侧不改 wire：parse 诊断继续走 `publishDiagnostics`，`soko/goals` 答空
  （`.unwrap_or_default()`），`soko/nextHole` 答 `null`。测试三层：front 单测 3 条
  （含"缓存只存 clean"的不变量）+ CLI e2e 4 条（`--text`/`--file` 两条输入通道、
  G-17、坏依赖项目回归、真课程单元⑤ 正例/反例与 `grade` 对拍）；复现
  `docs/gaps/repro/G10-query-check-parse-error.sh` 与新增的
  `docs/gaps/repro/G17-query-goals-holes-parse-error.sh` 修后都 = exit 1。
  `--json` 事件流与两处课程 GOLDEN **一字未动**（改动全在 `front::query` 的摘要视图
  与 CLI 信封）。**注意 `query check` 的退出码 0→1 是有意的契约变更**（minor）。
  as-built 说明同步在 `docs/protocol.md` 与 `docs/design/agent-query-channel.md`。
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
- **合并轮修掉的跨模块归因 bug（第九十八轮，0.58.0）**：`redundant-sorry` 是
  pass 2 现算的 warning，`split_report` 原先只按单元重算语法级 warning ⇒ **项目
  入口的"多写了一行 `sorry`"会被静默丢掉**。现在 `CompileOutput.warning_cmds` 与
  `warnings` 严格平行（`push_warning(cmd, w)`），按**命令下标**归因；语法级 warning
  由 `kernel_phase` 钉在所属单元的区间上。回归
  `compile::tests::warnings_are_attributed_to_the_unit_that_produced_them`。
  改任何"每命令输出通道"（events/errors/**warnings**/hovers）时，都要检查
  `split_report` 与 `session::build_suffix_snapshots` 两个消费者。
- 三道"静默错误"门仍在（`front/tests/perf.rs`、`cli/tests/watch.rs`、judge/suggest
  静默无建议）：改项目层时别让它们变成假绿。
- **开练习的签名纳入类型检查（WO-004 / G-01，语言线本轮）**：`def`/`theorem`/`example`
  值位是 `sorry` 时，签名过去**从来没过类型检查**（`walk.rs` 三处 `elab_expr(…).ok()`
  吞错），于是 `theorem t : 3 := sorry` 报 `exercise.open` + 0 诊断 + exit 0——与
  "还没做"无法区分，158 条课程练习的签名腐烂对判卷不可见。现在签名先 elaborate
  （`Err` 走既有失败通道），再由内核终审「是不是类型」（探针 axiom 过
  `try_check_declar_at`）与「`theorem` 的是不是 Prop」（`TypeChecker::is_proposition`）；
  不过 ⇒ 一条 diagnostic（span 取签名自身，G-15）+ 声明 `failed` + **不发**
  `exercise.open`。**判据纪律**：`exercise.open` 计数对签名腐烂永远是盲的——判卷只认
  `decl.checked` / `diagnostic`。全仓 105 个 `.sokonanoda` 对拍**新增诊断 0 条**、
  两处课程 GOLDEN 未动、课程门禁仍 315 checked · 96 open · 0 判负；
  **内核零改动**（探针走既有公开 API）。复现
  `docs/gaps/repro/G01-open-exercise-signature.sokonanoda`（修后 = exit 1）+
  `docs/gaps/repro/G01-course-signature-mutations.sokonanoda`（课程级变异体）。
  同轮把 14 处**夹具**里本来就不合法的签名（`theorem t : Prop := …`、
  `theorem bad : Prop -> Prop := sorry` 等）改成合法写法——判据一个字没放宽。

- **构造子进入类型命名空间（WO-005 / G-02，语言线本轮）**：`ctor mk` 的**规范名**是
  `Ind.mk`（源名已含点则原样 ⇒ prelude 的 `Nat.zero`/`Bool.true` 零改动）；裸名保留为
  **解析别名**（闭包内唯一时解析，重复报新码 `elab-ambiguous-ctor-alias`，真实声明优先）
  ——这是**教学子集的扩展、不是 Lean 语义**（Lean 里裸 `mk` 不可解析），日落与 37 个文件的
  机械改名一起开 WO-005b。派生 recursor 的 minor/iota 规则名、`top_level_def_spans`、
  `import-name-collision` 都跟着走规范名；`match` 分支与显式 `iota` 仍按**源名**匹配。
  **归约形态变化（实测）**：源文件自带 `inductive Nat` 时 `#reduce add two two` 从纯
  `succ` 链变成混合表示 `Nat.succ (Nat.succ (Nat.succ 1))`——19 处文本断言按实测逐条重钉，
  未做机械替换。**课程零改动**。设计 `docs/design/ctor-namespace.md`。
- **Prop + Type 参数 + 单构造子的归纳（WO-006 / G-03，语言线本轮）**：`derive_recursor`
  的 `small_elim = is_prop_block_ty && 多构造子` 是**源码近似**，内核真值还要看
  `large_elim_test_aux`（单构造子）⇒ `inductive Bar (A : Type) : Prop` + `ctor mk (a : A)`
  （= `Exists` 的形状）被内核断言拒（`left: 1 / right: 0`）。修法：把"派生 recursor"推迟到
  构造子类型 elaborate 之后、判据**逐字镜像内核**，"字段类型是不是 Prop 值"交给**真内核**
  （`judge_infer` 问排序）——不写第二套近似、不做"试探 + 回退"。**用户可见契约**：这种块
  派生的 recursor 带 **0 个**宇宙参数（`Bar.rec` 不接受宇宙参数），而字段就是结果索引的
  仍是 1 个。**顺带修掉一个 oracle bug**：`judge_infer` 取的是事件流里**第一条**
  `TypeChecked`，前缀里只要已有 `#check` 就拿回旧答案（同样影响 `match` 的 motive 层级）；
  改为按 `event_cmds` 取**最后一条命令**的事件。课程侧出口已收：
  `courses/set-theory/lib/Exists.sokonanoda` 升级为真归纳（名字/签名逐字不变，units 零改动；
  "大消去"仍不可用且**是正确的行为**——motive 只能落 `Prop`，台账 L-06）。
  设计 `docs/design/prop-large-elim-mirror.md`。
- **用户自定义记法第一刀（WO-011 / G-04，语言线本轮）**：`infix:N`/`infixl:N`/`infixr:N`/
  零元 `notation` 四条命令；数学符号成为**独立 token**（码点类 `U+2200–22FF` /
  `U+2A00–2AFF` / `\`，最大咬合），记法在 elab 内**源到源**重写成 `App` 形状并**自己补
  前导类型参数**（只做裸变量匹配，不引入元变量）。**文件内作用域**（不跨 `import`）、
  **不是声明**（零事件、不进声明表/goal 视图；`SemanticKind::ALL` 与 `tm_scope` 逐字节不变）、
  符号文本全标识符字符报 `notation-shape`、未声明符号报 `notation-unknown-symbol`、
  解不出类型参数报 `elab-notation-argument-unsolved`。**兼容护城河**：点名形式永久可用，
  两种写法判卷一致（省 `α` 的 `Set.mem a A` 改前改后同样被拒）。**课程零改动**（画布仍点名；
  `units/notation-cheatsheet.sokonanoda` 是"点名 ↔ 记法"对照页，也进同一套判据）。
  **第二刀未做**：`𝒫`/`ᶜ`（Unicode 字母不是符号）、`''`/`⁻¹'`/`×ˢ`、跨 `import` 的记法、
  binder 记法、记法重载。设计 `docs/design/notation-subset.md`；e2e `crates/cli/tests/notation.rs`。
- **L1 prelude（L-01/L-02，0.59.0 / 第一百〇四轮）**：Full 模式自带 Lean core 的逻辑与
  等式骨架 **30 个名字**（`PRELUDE_NAMES` 12 → 42），分 B1–B7 七族，`And`/`Or` 是**真归纳块**
  （点号构造子，可 `match`）。**让位粒度 = 族**、族间按依赖闭包（B5→B2、B6→B3、B7→Eq），
  触发集合 = 闭包的顶层名字并集（含构造子/递归子）⇒ 入门课单元①④⑤⑧⑨⑩⑪ 的"自建骨架"
  教学**一个字不改**（`course_shared.rs` 44 份副本一致性测试未改而全绿）。两处 as-built 修正：
  **先 Eq 后 L1**（B7 的定义体引用 `Eq.subst`/`Eq.refl`）；**`L1_INSTALL_DEPTH` 重入闸**
  （否则装 `And` 归纳块时内层 `compile_fol_with` 再装 L1 ⇒ stack overflow）。
  `PRELUDE_NAMES`（材料）与 `PRELUDE_NEVER_YIELDS`（碰撞检查豁免面，只含 Nat/Bool）拆成两个
  常量；parser 白名单零改动。卷 I 的 `lib/Logic` 26 条**一条没删**（文件头注明 prelude 自带哪些）。
  设计 `docs/design/prelude-l1-proposal.md`（含 as-built）。
- **课程门禁接进 gate 与 CI（P-C6，0.59.0 / 第一百〇六轮）**：判据 **G1–G5**
  （每个目标 `grade` 退出码 0 / 目标存在 / 解答 0 open 且 checked>0 / 解答覆盖画布每个具名
  练习 / lib+Demo 0 open）——**与规模无关、不锁计数**；唯一真相是
  `courses/set-theory/tools/check.py`（python3，零 cargo）。`scripts/soko gate` = cargo 门禁
  绿了再跑课程门禁与缺口台账门禁（`scripts/gap.py selftest` + `check`；探不到 python3 ⇒
  **exit 3**，绝不静默跳过；课程那一跑把二进制经 `SOKONANODA_BIN` 透传解析结果，
  **台账那一跑刻意不透传**——G-11/G-16 测的就是启动器的解析链，覆盖会短路夹具）；`ci.yml` 的 `test` job 加 `--selftest` + `--annotations --report --summary`
  step（用当轮 `target/debug` 二进制、`timeout-minutes: 5`、`course-gate-report` artifact）
  ——**不新建 job**，课程红自动挡住 `auto-tag` 的发布。当轮实测
  **36 目标 · 355 checked · 99 open · 0 判负**。设计/as-built `docs/design/course-gate-in-ci.md`；
  入口 `courses/set-theory/README.md`；判卷三纪律（退出码 / 绝对路径 / span 只作参考）在
  `courses/set-theory/AGENTS.md`。
- **缺口台账门禁（0.59.0 / 第一百〇六轮，主线收尾）**：「缺口即测试」从人肉纪律变成门禁——
  `scripts/soko gate` 第四步 = `python3 scripts/gap.py selftest` + `check`；`ci.yml` 的
  `test` job 同款 step `Gap ledger is consistent (docs/gaps)`（~3 s、不新建 job）。
  台账新增 **`repro_expect`**（`clean`/`rejected`/`exit0`/`nonzero`）：期望默认由 `status`
  推出，但**有些缺口的「修好」恰恰是判红**——G-01 即为 `"rejected"`；取值与复现类型不匹配
  直接判不一致，`gap.py selftest` 14 条判据钉住判定规则。当轮 **G-09 关账**（包装层已有稳定码
  `kernel-internal` + 「这不是你的代码问题」提示；唯一已知可达触发路径随 G-03 关闭 ⇒
  撤下 `repro`、改判 `fixed`）。结果：`gap.py check` **exit 0 全绿**，24 条里 17 条
  `fixed_in = 0.59.0`（含 WO-010 / G-15），未关账 6 条（L-04 `workaround` + L-03/G-05/G-07/G-08/L-06）。
  协议 `docs/gaps/README.md`，设计 `docs/design/teaching-project.md` §6。
- **编辑器 `build` / `rebuild`（0.60.0 / 第一百〇七轮，用户直接报的缺口）**：CLI 的
  `sokonanoda build [--clean] [<file>|<dir>]`（预热/清理共享编译缓存）在编辑器里有了入口——
  `sokonanoda: build`（`alt+b`，编当前文件/工作区）与 `sokonanoda: rebuild`（`alt+shift+b`，
  先 `--clean` 再重编）；JSON Lines 进 **sokonanoda build** 输出面板 + 一行摘要 + 刷新三棵树。
  三层测试（静态契约 / stub 宿主 / 真 VS Code e2e 第 15 例），文档 README/CHANGELOG/AGENTS/skills
  同轮；版本 feature bump 0.60.0。
- **五个缺口收口（0.60.0 / 第一百〇七轮）**：**G-05** `namespace`/`open`（parser 加前缀 +
  `compile/scope.rs` 解析顺序 + 三个专用 parse 码；课程 `lib/Set` 22 条声明去前缀、外部零改动）；
  **G-07** 课程清单 v2（卷/章/先修/标签/配额 + 门禁 G6 清单自洽 + CLI/扩展/站点/v1 兼容）；
  **G-08** `abbrev`（实测与 `def` 无可观察差异 ⇒ 同语义关键字）；**L-03** Type 层重写
  （prelude B8：`Eq.rec` + `Eq.mp`/`Eq.mpr`，`Vec.cast` 实测 checked）；**L-06** 累积性边界
  （内核性质不改；新错误码 `kernel-prop-not-cumulative` + 课程三条绕法）；**记法第二刀**
  （`prefix`/`postfix` + `𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ` + **跨 `import` 传播** + 顺带修 4 个既有缺陷）。
  台账 **24 条 = 22 fixed + 2 workaround（L-04/L-06）、`open` 归零**。
- **诊断坐标自描述（WO-010 / G-15，0.59.0）**：`query check` 的 `failed[]`/`warnings[]`
  **新增** 1 基 `start_line`/`start_col`/`end_line`/`end_col`（**只加不删**：`start`/`end` 仍是
  字节 offset、坐标空间 = **入口文件**；schema 号、事件种类、双 GOLDEN 都不动）。台账原记的
  「内核 span 漂到别的声明」是**量具缺陷**（复现脚本把字节 offset 当字符下标），真缺口是坐标
  不自带单位。守护三层：front 两条（span 的字节切片逐字等于出错命令——原来那条只断言
  `line >= 1`）、CLI e2e 一条（`failed[]` 行列 ≡ `grade --json` 的 span；依赖只以入口
  `import-dependency-failed` 出现）、复现重写（修前 exit 0 / 修后 exit 1，双二进制对照）。
  同轮 `docs/protocol.md`、`docs/TESTING.md`、`courses/set-theory/AGENTS.md`、`dsh/mcp/server.js`。
- **课程跟随 prelude（P4，0.59.0）**：`courses/set-theory/lib/Logic.sokonanoda` 的 26 条声明
  **退化成只有注释的空壳**（prelude 已自带同名 30 个；34 处 `import lib.Logic` 一字未改），
  课程侧 **65 处项位裸名** `inl`/`inr` → `Or.inl`/`Or.inr`。课程门禁 **36 目标 · 329 checked ·
  99 open · 0 判负**（差额 26 = 删掉的重复脚手架；`open` 不变 ⇒ 没删练习、没加 `sorry`）。
  设计与 as-built：`docs/design/prelude-l1-proposal.md` §6、`docs/design/course-stdlib.md` §3。
- **站点卷 I 页面（P5，0.59.0 / 第一百〇六轮）**：`site/set-theory.html`（零构建 HTML）
  从 `site/data/site.json` 的 `set_theory` 块渲染单元表 + 每单元计数；那份数据由
  `scripts/gen-site-data.py` 生成——版本读 `Cargo.toml`、轮次读 `STATUS.md`、
  **计数由课程门禁 `--json` 实测**（`counts_source: "gate"`），三样都不许手写；
  `python3 scripts/check-site.py` 绿（10 页、链接与版本干净）。
  > ⚠️ **2026-09-21 起这条作废**：官网简化成**一个页面**（是什么 / 怎么安装 /
  > 核心特点 / 未来的计划，第一百二十轮），`set-theory.html` 与 27 个页面一起删除，
  > 生成器也只剩下"版本号"一件事，而且**版本改读最新的已发布 tag**（不再读
  > `Cargo.toml`）。当前权威 = `docs/design/site-single-page.md`，
  > 验收 = `python3 scripts/check-site.py`（10 项，含真 Chrome 的 `--browser`）。

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
