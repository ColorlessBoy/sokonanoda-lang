# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-19（第一百〇六轮：**0.59.0 收尾**——语言线五刀（签名受检 /
> 构造子命名空间 / 派生 recursor 判据 / L1 prelude / 用户自定义记法）收成一个版本，
> 课程门禁接进 `scripts/soko gate` 与 CI，卷 I 上站点；版本 **0.59.0**，
> 发布由 push main → auto-tag 全自动；缺口台账门禁（`gap.py selftest` + `check`）
> 同轮接进 gate 与 CI，24 条缺口 **18 条 `fixed_in = 0.59.0`、`check` 全绿**；
> WO-010（诊断坐标）与 P4（课程跟随 prelude）同日落地）
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

## 本轮进度（2026-09-19，第一百〇八轮：0.61.0 —— 设计文档里「未做」的全部收口）

> 用户要求：**「设计文档里的东西都做了吧。」** 本轮把 `docs/design/` 各篇「未做 / 第二刀 / 残留边界」
> 里**不违反硬规则**的项目全部实现；只有两条内核冻结项（累积性、Prop 大消去）与一条内核 pp 项
> （源码级 print-back）保留为**有理由的边界**。

1. **记法第三刀**（`docs/design/notation-subset.md` §12 → as-built + §13）：**binder 记法**
   （`binder_notation`，`∃ x, p` / `∀ x, p`；命令拼写为什么不是 `notation-binder`：`-` 不是本语言
   标识符字符，实测被切碎）、**记法重载**（同符号同形状按期望类型选候选；歧义/无候选两个专用码 +
   人话 hint；重声明 import 来的符号仍是错误）、**`scoped` / `open scoped`**、**集合字面量 `{a}`/`{a,b}`**
   （新语法，`{}` 与 binder 定界符消歧）、**一元记法在实参位免括号**。课程单元⑧ 加一条 `example` 演示
   （练习数量与题意不动）。print-back 保留为「内核 pp 冻结 ⇒ 销不掉」（§13.1）。
2. **`namespace`/`open` 扩展**（`docs/design/namespace-open.md` §7 → as-built）：**子句**
   （`only`/`hiding`/`renaming` 互斥、先过滤后改名）、**`open Foo in <cmd>`**（限叶子命令，
   合成前缀补源码原文 `open` 头，做过变异验证）、**`export`**（文件内同 open + 跨 `import` 重放导出表；
   `open` 不跨）、**`open scoped`** 接线、**遮蔽 warning**（非 error）。
   `section`/`variable` 评估为**做不动**并给实测（`#check id Nat` ⇒ `Nat -> Nat`、`def u : Nat := id Nat`
   内核拒绝 ⇒ 本语言无隐式参数插入，auto-bound 落不出 Lean 体验）；`namespace` 跨文件传播澄清为"无需做"。
3. **层级算术与 Eq 多态**（`docs/design/eq-type-level-rewriting.md` §4）：宇宙层级支持**数字后缀加法**
   （`u+1`；`u+v`/`max` 给专用诊断——内核 `Level::Max` 无公开构造入口），`Eq.mp`/`Eq.mpr` 从 Type 0
   实例升级成**宇宙多态**（签名对齐 Lean core），装上 **`cast`** 与 **`Eq.ndrec`**（真 Lean 子集兼容；
   `cast` 原来是 front 单测里的自定义公理名，已改名）。`PRELUDE_NAMES` 45 → 47，B8 族让位照旧；
   L-03 复现件扩到 **10 checked / 0 diagnostic**。
4. **编辑器词表与课程工具**：`front::semantic::KEYWORDS` 与 `editor/vscode` TM 语法**同轮**加入
   `abbrev`/`prefix`/`postfix`/`binder_notation`/`scoped`（守护
   `tm_grammar_keywords_follow_the_single_source` 绿）；CLI `course` 支持**多清单聚合**
   （`course <path>… [--all]`，多份才加 `course.unit.manifest` 与 `summary.manifests`，单清单一个键不多）；
   课程门禁新增 `--ledger`（**默认关**，避免 CI 写仓库）产出成本台账 `docs/courses/ledger.jsonl`
   （已真跑一条：36/329/99/0 · 20024ms · v0.60.0）。
5. **保留的边界（有实测理由，非"没时间"）**：累积性与 Prop 大消去（内核冻结，硬规则 1）、
   源码级 print-back（类型文本由内核 pp 产出，记法不进内核）、`section`/`variable`（无隐式参数插入）、
   `u+v`/`max`（内核无公开构造入口）。
6. **验收**：`scripts/soko gate` exit 0（`cargo test --workspace --locked` **1163 passed / 0 failed**；
   课程门禁 **36 目标 · 329 checked · 99 open · 0 判负**；缺口台账门禁全绿）；版本 0.61.0（两处）+
   课程 `requires = "0.61"`；内核零改动。
7. **发布结果（2026-09-19 实测）**：push main（`f3902b3`）→ CI **一次全绿（9m18s）** → auto-tag
   打 **`v0.61.0`** → `release` **全 success** → GitHub Release **26 资产** + Marketplace 收录
   **0.61.0**。**发布产物实测**（下载 `sokonanoda-cli-aarch64-apple-darwin.tar.gz`）：`shasum -c`
   **OK** → `--version` = `sokonanoda 0.61.0` → 一段用新语法的文件判卷：`namespace N` +
   `abbrev T : Type := Prop -> Prop` + `def f : T := …` ⇒ 全局名 **`N.f`** 与 `T`/`f` 全部
   `decl.checked`（G-05 + G-08 在发布产物里可用）；`cast` 已可解析（该 smoke 文件里两条诊断来自
   它自己的宇宙层级写法，不是缺名字）。
   注：本轮 bump 后第一次 gate 曾 exit 3——`target/debug` 还是 0.60.0 而版本钉已到 0.61.0，
   启动器按 G-16 的守卫**拒绝**了缓存里的 0.55.0（守卫工作正常）；`cargo build -p sokonanoda-cli`
   重建后 gate 复绿。

## 本轮进度（2026-09-19，第一百〇七轮：0.60.0 —— 编辑器 build/rebuild + 五个缺口收口）

> 用户要求：**「vscode 还是没有 sokonanoda: build 或者 sokonanoda: rebuild 的命令。你这个剩下的没做的也要做。」**
> 本轮把编辑器缺的命令补上，并把台账里剩下的 **G-05 / G-07 / G-08 / L-03 / L-06 与记法第二刀**全部收口——
> 结果是台账 **24 条 = 22 条 `fixed` + 2 条 `workaround`，`open` 归零**。

1. **VS Code `build` / `rebuild`（用户直接报的缺口）**：CLI 一直有 `sokonanoda build [--clean] [<file>|<dir>]`
   （预热/清理共享编译缓存），扩展从没接出来。新增 `sokonanoda: build`（`alt+b`：编当前 `.sokonanoda`
   文件——CLI 顺着 `import` 编整个闭包——否则编第一个工作区文件夹）与 `sokonanoda: rebuild`
   （`alt+shift+b`：先 `build --clean` 清缓存再重编）；两者把 CLI 的 JSON Lines（`build.file` /
   `build.clean` / `build.summary`）灌进 **sokonanoda build** 输出面板，回一行
   `N 个文件 · 编译 X · 命中 Y · 失败 Z（ms）`（带「显示输出」按钮），跑完刷新练习/项目/课程三棵树；
   项目视图标题栏也有入口。**三层测试**：静态契约
   `crates/cli/tests/extension.rs::build_and_rebuild_commands_warm_the_compile_cache`、stub 宿主、
   真 VS Code 冒烟（e2e 14 → **15 条**）。版本随 feature bump **0.60.0**（`Cargo.toml` +
   `editor/vscode/package.json` + CHANGELOG + 课程 `requires` 0.59 → 0.60）。
2. **G-05 `namespace` / `open`**（设计 `docs/design/namespace-open.md`）：`namespace A … end A` 让声明名
   自动带前缀（`def mem` ⇒ 全局名 `A.mem`；带点则拼接），`open A` 让短名可用；引用解析收敛到
   `crates/front/src/compile/scope.rs` 的候选序列（命名空间链由内到外 → 精确名 → `open` 前缀按序），
   三条命令**零事件、不进声明表**，专用 parse 码 `parse-namespace-{mismatch,unclosed,shape}`。
   课程跟随：`courses/set-theory/lib/Set.sokonanoda` 的 22 条声明去掉源级 `Set.` 前缀
   （**全局名一个没变、外部引用零改动**，门禁计数逐字不变）。实测边界写进设计 §4/§6：
   未闭合 `namespace` 会打断判卷合成路径（新增 `parser::parse_fragment`）、`apply` 的
   `unify_spine` 是文本对齐（用 `judge::judge_render_type` 先过内核 pp，未用命名空间的文件零开销）。
3. **G-07 课程清单 v2**（设计 `docs/design/course-manifest-v2.md`）：`course.json` 升成
   `soko.course/2`（1 卷 / 4 章 / 12 单元，每章 `prereqs`/`tags`/`quota.exercises`），
   **v1 扁平数组继续被接受**（入门课就是活体回归）；CLI 的 `course.unit` 只加
   `volume`/`chapter`/`tags`、`course.summary` 只加 `volumes`/`chapters`（v1 事件一个键都不多）；
   课程门禁新增 **G6 = 清单自洽**（卷/章 id 唯一、unit 恰好一章、`prereqs` 不悬空；
   **配额差额只报告不判红**，维持"只判形状、不锁计数"）；扩展课程树按卷→章→单元分组
   （v1 平铺逐字保留）、站点按卷/章渲染、`docs/protocol.md` 契约 additive。
4. **L-03 Type 层重写**（设计 `docs/design/eq-type-level-rewriting.md`）：实测内核**给** Eq 形状的大消去，
   堵路的是 prelude 的 `Eq` 是公理（无派生 recursor）+ 两条语法边界 ⇒ L1 新增 **B8 族**
   `axiom Eq.rec {u, v}`（与 Lean core 逐字同形）+ `def Eq.mp`/`Eq.mpr`，`PRELUDE_NAMES` 42 → 45，
   族让位照旧。复现件 `docs/gaps/repro/L03-eq-type-level.sokonanoda` 干净判卷（5 checked，
   含 `Vec.cast` 用 `@Eq.rec.{1, 1}` 在 `Sort 1` motive 上搬运）。残留边界（`Eq.mp` 是 Type 0 实例，
   多态版要层级算术 `u+1`）写进设计。
5. **L-06 累积性边界**（设计 `docs/design/prop-cumulativity-boundary.md`）：累积性是**内核性质**，
   冻结内核下不改（硬规则 1）；本轮把 `def T : Type := True` 的裸类型不匹配翻译成专用码
   **`kernel-prop-not-cumulative`** + 人话 hint（"本语言没有累积性；官方 Lean 4 有"），
   课程三条绕法（`Set.Equiv … : Prop` 数据化、unit07 数据版满射、unit10 `Exists.choose`）写进设计。
   台账改判 `workaround`（诊断质量那一半算 fixed）。
6. **G-08 `abbrev`**（设计 `docs/design/abbrev.md`）：实测 `def` 与 Lean `abbrev` 在本语言**无可观察差异**
   （类型位透明、`#reduce` 展开、hover 保留别名名、项层 `rfl` 成立；唯一差异 reducibility hint 在
   只有一个透明度层级的本语言里不可观察）⇒ 实现成**与 `def` 同语义的关键字**（真 Lean 子集兼容），
   AST/elab/内核零改动，白名单（parser 命令表 + CLI help + `docs/architecture.md`）同步。
7. **记法第二刀**（`docs/design/notation-subset.md` §11 as-built）：`prefix:N` / `postfix:N` 两条命令；
   卷 I 五个符号 `𝒫` / `ᶜ` / `''` / `⁻¹'` / `×ˢ` 落 `lib/Set.sokonanoda`（记法版与点名版**判卷计数逐一相等**）；
   **跨 `import` 传播**（闭包加载器改成"先收 import 边、先访问依赖、再解析自己"，依赖导出的记法表当继承表，
   重声明继承符号是 parse 错误）；顺带修掉 4 个既有缺陷（记法前导参数下溢 panic、`by.rs` 的
   `atom_text` 括号清单漏记法、`judge_infer` 读目标签名剥错 binder、`by` 块目标里的记法判不动）。
   未做（设计 §12 逐条给理由）：binder 记法、记法重载、`scoped`、集合字面量、print-back。
8. **台账收口**：24 条 = **22 `fixed` + 2 `workaround`（L-04 / L-06）、`open` 0 条**；
   `python3 scripts/gap.py check` 全绿。**验收**：`scripts/soko gate` exit 0
   （fmt/clippy/`cargo test --workspace --locked` **1107 passed**、课程门禁 **36 目标 · 329 checked ·
   99 open · 0 判负**、台账门禁全绿）；版本 0.60.0（两处）+ 课程 `requires = "0.60"`。
9. **发布结果（2026-09-19 实测）**：push main（`a948f65`）→ CI **一次全绿（8m20s）** →
   auto-tag 打 **`v0.60.0`** → `release` **全 success** → GitHub Release **26 资产**
   （lsp ×8 / cli ×8 / vsix ×9 / `SHA256SUMS`）+ Marketplace 收录 **0.60.0**。
   **发布产物实测**（下载 `sokonanoda-cli-aarch64-apple-darwin.tar.gz`）：`shasum -c` **OK**
   → `--version` = `sokonanoda 0.60.0` → 一段同时用新语法的文件干净判卷（exit 0）：
   `namespace A` + `def f` ⇒ 全局名 **`A.f`**（G-05）、`abbrev T : Type := Prop -> Prop`（G-08）、
   `def uses : T := A.f` ⇒ 3 条 `decl.checked`。官网 `data/site.json` = `version 0.60.0` +
   `set_theory` 36 目标 · 329 checked · 99 open · 0 判负。

## 本轮进度（2026-09-19，第一百〇六轮：0.59.0 收尾（语言线五刀 + 课程门禁 + 站点页））

> 本轮**只做版本与文档**：把第九十九～一百〇五轮攒下的用户可见改动收成 **0.59.0**，
> 把课程门禁接进 `scripts/soko gate` 与 CI，把卷 I 搬上站点。硬规则 1（内核冻结快照）
> 与硬规则 6（用户/agent 路径零工具链）全程未动：`crates/kernel/` 与 `crates/` 下任何
> 源码**本单零改动**（语言线代码在前几轮已落地，工作树里原样保留）；判定仍只走内核与退出码。

1. **版本 bump（0.58.0 → 0.59.0）**：`Cargo.toml` 的 `[workspace.package] version`
   与 `editor/vscode/package.json` 的 `version`（两处必须一致，契约测试
   `crates/cli/tests/extension.rs::cargo_and_extension_versions_match` 守着）；
   `cargo metadata --format-version 1` 让 `Cargo.lock` 跟上（3 个包：cli/front/lsp）。
   **课程侧版本钉同步**：`courses/set-theory/sokonanoda.toml` 的 `requires` 0.58 → 0.59
   （WO-003 / WO-007 的课程侧收尾项；记法对照页本身就要 ≥0.59.0）。
   实测：`target/debug/sokonanoda --version` = `sokonanoda 0.59.0`。
2. **这一版装了什么（全部用户可见，逐条对应轮次与设计）**：
   * **签名受检**（WO-004 / G-01，第一百〇一轮）：值位是 `sorry` 时签名也要 elaborate
     并过内核的「是不是类型 / `theorem` 的是不是 Prop」；坏签名 = 一条 diagnostic
     （span 取签名自身）+ 声明 `Failed` + **不发** `exercise.open`——判卷只认
     `decl.checked` 与 `diagnostic`，`exercise.open` 计数对签名腐烂永远是盲的；
   * **构造子命名空间**（WO-005 / G-02，第一百〇二轮，`docs/design/ctor-namespace.md`）：
     规范名 `Ind.ctor`，裸名降为解析别名（闭包内唯一；撞名报 `elab-ambiguous-ctor-alias`）；
     源文件自带 `inductive Nat` 时归约形态变化已逐条实测重钉；
   * **Prop + Type 参数 + 单构造子的归纳**（WO-006 / G-03，第一百〇三轮，
     `docs/design/prop-large-elim-mirror.md`）：派生 recursor 的宇宙参数逐字镜像内核
     （不再 panic）；**课程侧出口本轮收掉**：`courses/set-theory/lib/Exists.sokonanoda`
     从公理三件套升级成**真归纳**（`Exists.intro` 是构造子、`Exists.elim` 由自动派生的
     `Exists.rec` 定义，名字与签名逐字不变 ⇒ units/ 的点名调用零改动）；
   * **L1 prelude**（L-01/L-02，第一百〇四轮，`docs/design/prelude-l1-proposal.md`）：
     Full 模式自带 Lean core 的逻辑与等式骨架 **30 个名字**（`PRELUDE_NAMES` 12 → 42），
     **族粒度让位** ⇒ 入门课"自建骨架"的教学一个字不用改；
   * **用户自定义记法第一刀**（WO-011 / G-04，第一百〇五轮，
     `docs/design/notation-subset.md`）：`infix:N`/`infixl:N`/`infixr:N`/`notation` +
     数学符号独立 token + elab 内源到源重写（自动补前导类型参数）；**文件内作用域**、
     **零事件**、点名形式永久可用且两种写法判卷一致；`𝒫`/`''`/`⁻¹'`/`×ˢ` 留第二刀；
   * **`course` 认 `import`**（WO-007 / G-06，第九十九轮）与 **`query check` 同口径**
     （WO-003 / G-10 + G-17，第一百轮）：前者让聚合与单文件 `grade` 同判（`failed == 0`
     ⇔ `grade` exit 0），后者把"这份文本解析不了"从假绿翻成 `failed[]` + **exit 1**
     ——**这是有意的契约变更**（同口径后 `query check` 可作 `grade` 的交叉复核）。
3. **课程门禁接进本地与 CI**（唯一真相 `courses/set-theory/tools/check.py`；设计
   `docs/design/course-gate-in-ci.md`，as-built §9）：判据 **G1–G5 与规模无关**
   （`grade` 退出码 0 / 目标存在 / 解答 0 open 且 checked>0 / 解答覆盖画布每个具名练习 /
   lib+Demo 0 open）；`scripts/soko gate` 里 python3 探不到 ⇒ **exit 3**（无法判定 ≠ 绿），
   cargo 门禁绿了才跑课程门禁并把**解析到的**二进制经 `SOKONANODA_BIN` 透传；
   `ci.yml` 的 `test` job 新增 `--selftest` + `--annotations --report --summary` step
   （用当轮 `target/debug` 二进制，**不新建 job** ⇒ 课程红自动挡住 `auto-tag` 的发布）
   + `course-gate-report` artifact。
4. **站点卷 I 页面**：`site/set-theory.html`（零构建 HTML）上线；数据块由
   `scripts/gen-site-data.py` 生成——版本读 `Cargo.toml`、轮次标题读本文件、
   **计数由课程门禁实测**（`counts_source: "gate"`）——三样都不许手写；
   `scripts/check-site.py` 绿。
5. **文档同轮**：本文件轮转（第一百〇三轮移入 `docs/STATUS-ARCHIVE.md`，归档头部范围
   1–102 → **1–103**）；`docs/HANDOVER.md` 快照 → 0.59.0 + §2/§3 的 as-built 汇总；
   `courses/set-theory/README.md` 现状表**据实重算** + 补记法对照页 / Exists 升级；
   `docs/design/teaching-project.md` 附录 A 的 fixed/0.59.0 状态与 P1/P-C 收尾；
   `REQUIREMENTS.md` §9 追加本条。
6. **课程门禁实测（0.59.0 二进制）**：**36 个目标 · 355 checked · 99 open · 0 判负**
   （canvas 96 / solutions 0 / lib 0），`--selftest` exit 0。比上一条记录多 2 个目标
   （记法对照页 + 它的解答，07:5x 落地）——课程在长，所以门禁**只判形状、不锁计数**。
7. **验收**：`grep -n "^version" Cargo.toml` = `0.59.0`；
   `grep -n "\"version\"" editor/vscode/package.json` 首行 = `0.59.0`；
   `cargo test -p sokonanoda-cli --test extension` 33 passed（含版本契约）；
   `python3 courses/set-theory/tools/check.py` exit 0、`--selftest` exit 0；
   `git diff --stat crates/` 本单零改动（工作树里第九十九～一百〇五轮的语言线改动未动）。
   `scripts/soko gate` **全绿 exit 0**（含课程门禁与新增的台账门禁）；
   **未 commit**（仓库约定：由主线统一落 commit）。
8. **缺口台账收口（主线，同日）**：把「缺口即测试」从**人肉纪律**变成**门禁**——
   * `scripts/soko gate` 第四步 = `python3 scripts/gap.py selftest` + `check`；
     `ci.yml` 的 `test` job 同款 step `Gap ledger is consistent (docs/gaps)`
     （`SOKONANODA_BIN` 指当轮 `target/debug`，~3 s，**不新建 job**）。红了 =
     语言变了而台账没跟上（或修好忘了关账）；
   * **台账新增 `repro_expect`**（`clean`/`rejected`/`exit0`/`nonzero`）覆盖
     "由 status 推导期望"的默认：**有些缺口的「修好」恰恰是判红**——G-01 就是
     （复现件钉的是「签名写错必须被拒」），已写 `"repro_expect":"rejected"`；
     取值与复现类型不匹配会直接判不一致（写错的字段不会被默认推导悄悄盖过）；
     `gap.py selftest` 14 条判据钉住判定规则本身（含 4 种取值 + 2 种非法形态）；
   * **G-09 关账**：包装层早已把 `assertion failed:`/`unwrap()` 归 `kernel-internal`
     + 「这不是你的代码问题」提示（测试钉住），唯一已知可达触发路径随 G-03 关闭 ⇒
     改判 `fixed` 并**撤下 `repro`**（与 G-03 共用、已转绿），两半结论写进 `notes`；
   * 结果：`python3 scripts/gap.py check` **exit 0 全绿**，`gap.py list` =
     24 条里 18 条 `fixed_in=0.59.0`、未关账 6 条（L-04 `workaround` + 5 条
     `painful`/`nice`：L-03/G-05/G-07/G-08/L-06）；`.gitignore` 补
     `__pycache__/`（python 工具已是仓库一部分）；`docs/gaps/README.md`、
     `docs/design/teaching-project.md`、`skills/sokonanoda-{dev,ci}` 同轮同步。
   * **第一次真跑就抓到一个真 bug（本台账门禁自己的）**：`gate` 把解析到的二进制经
     `SOKONANODA_BIN` 透传给课程门禁的同时也透传给了台账门禁，而 G-11/G-16 的复现
     **测的就是启动器自己的解析链**——夹具里那个「陈旧缓存必须被拒绝」的现场被显式覆盖
     绕过，两条复现假报「缺口仍在」、gate 因此 exit 1。修法：`gap.py` 新增 `clean_env()`
     剔除 `SOKONANODA_BIN`/`SOKONANODA_LSP_BIN`（`selftest` 钉住）、`gate` 对台账那一跑
     **不透传**、复现脚本自身再加一行 `unset` 兜底。教训见 `docs/LESSONS.md`
     （「门禁注入的环境变量会短路复现夹具」）。
9. **WO-010 / G-15（诊断坐标自描述）**：`query check` 的 `failed[]`/`warnings[]` **新增**
   1 基 `start_line`/`start_col`/`end_line`/`end_col`——**只加不删**（`start`/`end` 仍是
   字节 offset、坐标空间 = 入口文件；schema 号、事件种类、双 GOLDEN 都不动）。
   台账原记的「内核 span 漂到别的声明」是**量具缺陷**（复现脚本把字节 offset 当字符下标），
   真缺口是坐标不自带单位与坐标空间。守护三层：front 两条（span 的字节切片逐字等于出错命令，
   收紧原来那条只断言 `line >= 1` 的假守护）+ CLI e2e 一条（`failed[]` 行列 ≡ `grade --json`
   的 span、依赖只以入口 `import-dependency-failed` 出现）+ 复现重写（修前 exit 0 / 修后
   exit 1，双二进制对照实测）。同轮同步 `docs/protocol.md`、`docs/TESTING.md`、
   `courses/set-theory/AGENTS.md` 的判卷纪律、`dsh/mcp/server.js` 的工具描述。
10. **P4 课程跟随 prelude**：`courses/set-theory/lib/Logic.sokonanoda` 那 26 条声明
   **退化成只有注释的空壳**（prelude 已自带同名 30 个，34 处 `import lib.Logic` 一字未改），
   课程侧 **65 处项位裸名** `inl`/`inr` 改点号名 `Or.inl`/`Or.inr`（G-02 定形后裸项名已不存在；
   裸**模式**仍被接受，为一致性一起改）。课程门禁 **36 目标 · 329 checked · 99 open ·
   0 判负**——`checked` 少掉的 26 条正是删掉的重复脚手架，`open` 一条不变（= 没删练习、
   没加 `sorry` 的机械证据）。L-01/L-02 台账 `notes` 补记 P4 已跟随。
11. **发布结果（2026-09-19 实测）**：push `main`（`347bd83`）→ 第一次 CI **红**在 LSP 项目
   哨兵（下一条），修好后重推（`3c145e9`）→ CI **7/7 job 全绿**（lint / test / e2e ubuntu×2 /
   e2e macos-latest / e2e-ledger / **auto-tag**）→ auto-tag 打 **`v0.59.0`** 并 dispatch
   `release` → release **11 job 全 success** → GitHub Release **26 资产**
   （lsp ×8 / cli ×8 / vsix ×9 / `SHA256SUMS`）+ Marketplace 收录 **0.59.0**
   （2026-09-19T01:30:07Z 索引；9 个平台 VSIX 全部 publish 成功）。
   **发布产物实测**（下载 `sokonanoda-cli-aarch64-apple-darwin.tar.gz`）：`shasum -c` **OK**
   → `--version` = `sokonanoda 0.59.0` →
   **G-01**：`theorem t9 : 3 := sorry` ⇒ `diagnostic` `kernel-expected-sort` + exit 1
   （签名受检真的在包里）；**G-04**：`infix:50 " ∈ "` 定义后 `x ∈ A` 判卷通过；
   **G-15**：`query check --compact` 的 `failed[0]` 带 `start_line/start_col/end_line/end_col`
   （1:14→1:15）且 `start/end` 仍是字节 offset；**L-01/L-02**：无 `import` 直接用
   `And.intro` / `Or.elim` / `Not.intro` / `Iff.refl` / `Iff.symm` / `Iff.trans` / `absurd` /
   `Eq.symm` 全部 `decl.checked`。`e2e-ledger` 自动把三条腿（Linux×2 + Darwin×1，各 **14/14**、
   `dirty=false`、server `0.59.0 == 扩展 v0.59.0 (bundled)`）回提交进 `docs/e2e/ledger.jsonl`；
   官网实测：进度页第一百〇六轮、`data/site.json` = `version 0.59.0` + `set_theory` 36 目标 ·
   329 checked · 99 open · 0 判负。性能基线见 `docs/perf/ledger.jsonl`（下一条）。
12. **推 main 后第一次 CI 红，已修**（run 35411049219）：`test` job 红在 LSP 项目哨兵
   `perf_project_did_open_and_keystroke`（CI 实测按键 480ms > 300ms 预算；同机单跑 17ms、
   满负载并行 86ms，且 pre-batch 与当前二进制同夹具对拍 best 26ms vs 25ms ⇒ **无产品回归**）。
   这是 2026-09-18「串行 + best-of-N」口径的**漏网用例**（当时只改了单文件延迟，项目级漏了）：
   修法是按键延迟改来回编辑 best-of-3 + 三个 project 用例加 `PROJECT_PERF_LOCK` 互相串行，
   **阈值不动**；修后满负载并行连跑 3 次 = 17/20/19ms。`auto-tag` 被这次红正确挡住
   （0.59.0 没有带着假红发出去）。台账与预防：`docs/CI-FAILURES.md`（2026-09-19 条）+
   `docs/PERF.md` §采样口径（纪律升级为"所有性能哨兵默认串行 + best-of-N"）；
   **0.59.0 性能基线已留档**：`docs/perf/ledger.jsonl`（front `keystroke_recompile_closure`
   best 35.26ms vs 0.58.0 的 33.88ms、lsp 项目按键 14ms 与 0.58.0 一致 ⇒ 五刀无开销回归）。
13. **未做 / 下一轮**：记法第二刀（`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ`、跨 `import` 的记法、binder
   记法、重载）；**L-03**（`Eq.subst` 的 Type 层重写）；L-06（无累积性 + `Exists.elim`
   只能 Prop）；G-05/G-07/G-08 等 `painful` 项；课程侧小清扫（单元文件头里 9 处
   「逻辑（lib.Logic）」的来源标注改成「prelude 提供」）；发布本身全自动
   （push main → `ci.yml` auto-tag → `release.yml` 26 资产 + VSIX ×9 + SLSA provenance）。

## 本轮进度（2026-09-19，第一百〇五轮（语言线）：WO-011 / G-04 第一刀 —— 用户自定义记法 `∈`/`⊆`/`∅`）

> 台账 blocker：没有 `notation`/`infix`，集合论课程只能写前缀形式
> `Set.mem α a A` / `Set.subset α A B`，与纸笔数学（`a ∈ A`、`A ⊆ B`）迁移成本极高。
> 设计 = `docs/design/notation-subset.md`（N1–N7 + 优先级梯子 + 已知差异表 +
> 第二刀清单）。**本轮只做 Lean core 级第一刀**；`𝒫`/`''`/`⁻¹'`/`×ˢ` 留给第二刀。

1. **词法（三个 as-built 事实都实测确认）**：① 没有字符串 token ⇒ 新增
   `TokenKind::Str`（只服务记法声明里的符号文本；未闭合报 `unterminated-string`，
   span 在**开引号**）；② 数学符号**本来是标识符字符** ⇒ 新增 `TokenKind::Sym`
   （码点类 `U+2200–22FF` / `U+2A00–2AFF` / `\`，最大咬合），`is_ident_start`
   相应收窄；`∀` 的 `Forall` 分支**排在符号分支之前**，所以 `∀` 行为逐字节不变。
2. **parse**：四条命令 `infix:N` / `infixl:N` / `infixr:N` / `notation`；
   优先级插在 `parse_arrow`（最松）与 `parse_app`（最紧）之间的新梯子上
   （`parse_plus` 改为委托 `parse_operators(0)`，`+` 仍是 65 号内建、行为逐字节
   不变）。结合规则写死：`infix` = `p`/`p+1`、`infixl` = `p`/`p+1`（同级左结合）、
   `infixr` = `p+1`/`p`；**非结合同级链报错**。符号文本去空白后不能全是标识符
   字符（`notation-shape`）、未声明符号 `notation-unknown-symbol`、重复声明同一
   符号报错。**文件内作用域**：声明之后生效（不跨 `import`——第二刀）。
3. **elab（第三件 as-built：没人补前导 `Type` 参数）**：记法在 elab 内**源到源**
   重写成 `App` 形状，并**自己补前导类型参数**——只做**裸变量匹配**（不引入元变量
   /一般合一）：先从操作数类型解、再从期望类型解；解不出报
   `elab-notation-argument-unsolved`（`#check ∅` 就是这一条）。这一步是嵌套零元记法
   （`∅ ⊆ A`）能工作的关键：`∅` 的类型从外层记法给出的期望类型 `Set α` 反解。
4. **兼容护城河（实测）**：点名形式永久可用，且**省 `α` 的点名写法
   `Set.mem a A` 今天被内核拒绝、改后仍被拒绝**（同码 `kernel-rejected` 同 stage
   `kernel`）——两种写法判卷一致，不是"记法替代点名"。
5. **不新增语义面**：记法**不是声明**——不发任何事件、不进声明表、不进 goal 视图；
   `SemanticKind::ALL` 与 `tm_scope` 表**逐字节不变**（声明过的符号在语义层分类为
   `Keyword`，**未声明的符号不产生 run**，所以目标文本里的 `⊢` 仍是普通 run）。
6. **课程零改动（硬要求）**：`courses/set-theory/` **一个字节未动**，画布仍写点名
   形式；由 `the_shipped_course_still_uses_the_pointful_spelling` 钉住。
   课程门禁复跑 **315 checked · 96 open · 0 判负**，与记法落地前逐字相同。
7. **三层测试（TDD：先红后绿）**：front 词法 7 条 + parser 14 条 + elab 8 条 +
   semantic 3 条；CLI e2e `crates/cli/tests/notation.rs` **9 条**（五元计数一致 /
   无新事件种类 / 护城河 / 未声明符号的教学 hint / 复现件形状 / `#check ∅` 的
   unsolved 码 / stdin↔文件一致 / **真课程单元②的记法变体判卷一致** / 课程仍写点名
   形式）+ `protocol.rs` 一条（3 个 parse 码 + 2 个 elab 码的 stage）。
   第三层用**临时副本**（`/tmp` 复制 lib + 清单 + 改写签名的单元），课程树不动。
8. **复现件翻成"已修后形状"**：`docs/gaps/repro/G04-notation.sokonanoda` 补了使用行
   `def use (α : Type) (a : α) (A : α -> Prop) : Prop := a ∈ A`（`gap.py` 判据 =
   干净判卷 + 有 `decl.checked`）；`gap.py check` 里 G-04 已翻成「已判卷通过」。
   台账 G-04 行按 WO 要求重写 `today`/`expected_lean`/`blocks`（`blocks` **没有**
   清空：单元 6 的 `''`/`⁻¹'` 仍等第二刀）。
9. **验收**：`cargo build -p sokonanoda-cli -p sokonanoda-front -p sokonanoda-lsp` 过；
   `cargo test -p sokonanoda-front`（554 lib）· `-p sokonanoda-cli`（全套 20 个测试
   二进制）· `-p sokonanoda-lsp` 全绿；课程门禁绿；`gap.py check` 里 G-04 一致。
   **未跑 `scripts/soko gate`、未 bump 版本、未 commit**（仓库约定：主线统一）。
10. **文档同轮**：`docs/architecture.md` §4.1（新 token / 新命令 / 新 AST + 记法
    一节 N1–N7 摘要）、§5 白名单边界两条（记法是糖、是唯一例外面）；
    `docs/TESTING.md` 新增记法守护行；`docs/protocol.md` 5 个新码；
    `skills/sokonanoda-teacher`（语言能力速查 + 记法七条要点）·
    `sokonanoda-dev`（设计清单 + 白名单例外面）；`AGENTS.md` 硬规则 3 补记法例外；
    `editor/vscode/`（`tmLanguage.json` 数学符号规则 + CHANGELOG + README）；
    `docs/gaps/ledger.jsonl` G-04 行；本文件。
