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

## 本轮进度（2026-09-19，第一百〇四轮（语言线）：L-01/L-02 落地 —— L1 prelude 装上逻辑与等式骨架）

> 台账 blocker：prelude 只有 `Nat`/`Bool`/`Eq` 三家，Lean core 的**逻辑与等式骨架**
> 全缺——课程侧只能靠 `courses/set-theory/lib/Logic.sokonanoda` 手写 26 条兜底
> （"暴力"的根源）。设计 = `docs/design/prelude-l1-proposal.md`（本轮的提案文档，
> 已有逐条签名 / 让位规则 / 三件套计划 / GOLDEN 预测数值）。

1. **P1 安装 + 让位**：`crates/front/src/compile/prelude.rs` 新增
   `PRELUDE_L1_SRC`（规范源文本，28 条声明）、`L1_FAMILIES`（B1–B7 族表 +
   依赖边）、`install_l1_prelude`（axiom 走 `build_axiom`、def 走 `build_def`、
   归纳块走 `install_inductive_block`——**与用户声明同一条 elaborator**）。
   让位粒度 = **族**（不是单名），族间按依赖闭包（B5→B2、B6→B3、B7→Eq）；
   触发集合 `taken` 从 `user_top_level_names` 换成 **`top_level_def_spans_over`
   的键集**（补上构造子/递归子，设计 §2.3-1）。
2. **两个 as-built 修正（提案未预见，都是实测出来的）**：
   * **顺序：先 Eq 后 L1**——B7 的定义体引用 `Eq.subst`/`Eq.refl`，必须等 Eq 进环境；
   * **重入闸 `L1_INSTALL_DEPTH`**——装 `And` 归纳块时
     `large_elim_test_mirror` → `field_sort_via_kernel` → `judge_infer` →
     **内层 `compile_fol_with`**，内层又装 L1 ⇒ 无限递归（实测
     `stack overflow, SIGABRT`）。计数 > 0 时 `install_l1_prelude` 直接返回。
   另修正提案 §3.2 一处**方向写反**的措辞（依赖边是 B6 用 `And.left`，
   所以声明 `And` 让位 B6，反之不成立；§2.2 的表是规范）。
3. **P2 白名单与豁免面**：`PRELUDE_NAMES` 补 30 条（12 → **42**）；
   拆出 `PRELUDE_NEVER_YIELDS`（只含 `Nat`/`Bool` 家族）给
   `check_name_collisions`——L1 名字按族合法让位，**不能**进豁免面，否则两个模块
   各自声明 `True` 就不再报友好的 `import-name-collision`（设计 §2.3-2）。
   `goals.rs` 的 `GoalTemplates` 按**同一条让位规则**吃 L1 源文本。
   parser 白名单**零改动**（L1 不引入新语法）。
4. **P3 课程用例 + 两处 GOLDEN 据实重算**：单元②（唯一不声明 L1 名字的早期单元）
   中英画布加 `eq_symm_demo`（用 prelude 的 `Eq.symm`，一行）＋ 两题 hint 改
   "两解对照"；两份解答同步。真二进制实测：`unit2 = (3,5,2)`、
   `course_status` 的 `unit2 = (3,5,0,2)`、summary `checked 85 → **86**`
   ——**与提案 §4.2 的预测值逐字相同**。另发现**第三处** GOLDEN
   （`cli.rs::cli_course_is_stable_with_a_warm_cache` 也钉 `checked = 85`）⇒ 同步。
   `course_shared.rs`（44 份副本一致性）**一字未改而全绿**——这就是"让位"生效的证据。
5. **课程侧兜底保留（硬要求）**：`courses/set-theory/lib/Logic.sokonanoda` 的
   28 条声明**一条没删**，文件头加了"prelude 现在自带哪些（30 个名字 + 让位规则 +
   两处定形差异）"，避免两处真相打架。门禁复跑 **315 checked · 96 open · 0 判负**，
   与 L1 落地前逐字相同（让位 ⇒ 课程语义不变）。**P4（74 处 `inl`/`inr` 项位改
   点号名、`lib/Logic` 退化成空壳）未做**。
6. **复现件（台账契约）**：新增 `docs/gaps/repro/L01-l1-prelude-logic-skeleton.sh`
   与 `L02-eq-core-lemmas.sh`，四段自断言（Full 可用 / Bare 干净 / 让位依赖闭包 /
   `Or` 是真归纳块）。修前两者 exit 0（缺口仍在），修后 **exit 1**（修后形状成立）；
   `L-01`/`L-02` 已 `gap.py close --version 0.59.0`。`L-03`（Type 层重写）**仍 open**
   （B7 只装同宇宙三引理，`Eq.subst` 的 motive 仍是 `α -> Prop`），notes 已注明。
7. **三层测试**：front 7 条（`l1_prelude_is_available_in_full_mode`、
   `l1_or_is_a_real_inductive_for_match`、`l1_family_yield_is_dependency_closed`、
   `l1_yield_needs_the_whole_family`、`l1_taken_includes_ctors_and_recursors`、
   `l1_yield_is_closure_wide`、`l1_ctor_templates_feed_sub_goal_types`、
   `prelude_names_match_installs`、`eq_symm_is_installed`、`bare_mode_has_no_l1`）+
   CLI 4 条（Full / `--bare` / 注释指令 / `query check` ↔ 事件流计数一致）+
   课程 GOLDEN 两处 + 复现件两个。
8. **验收**：`cargo build -p sokonanoda-cli -p sokonanoda-front -p sokonanoda-lsp` 过；
   `cargo test -p sokonanoda-front`（522）· `-p sokonanoda-cli`（全套）· `-p sokonanoda-lsp`（141）
   全绿；`cargo fmt --check` / `cargo clippy`（教学 crates）零警告；
   `python3 courses/set-theory/tools/check.py` 绿；`python3 scripts/gap.py check` 全绿。
   **未跑 `scripts/soko gate`、未 bump 版本、未 commit**（仓库约定：主线统一）。
9. **文档同轮**：`docs/architecture.md` 新增 §5.4.1（族表 + 让位规则 + 两个 as-built
   要点 + 白名单/豁免面）+ §4.1 白名单行注明"prelude 名字不属于语法白名单"；
   `docs/design/course-stdlib.md` §2 更正"And 走 axiom 族"→"真归纳块 + 点号构造子"
   并标注已实现，§3.2-A 的 G-02 行注明 L1 已绕过；
   `docs/design/prelude-l1-proposal.md` 加 as-built 节；
   `docs/design/teaching-project.md` P-C3 ✅ / L-01 行；
   `docs/TESTING.md` 新增 L1 行；`docs/HANDOVER.md` 版本表 +1 行；
   `skills/sokonanoda-teacher/SKILL.md` + `references/curriculum.md`（L1 词汇与让位规则）；
   `editor/vscode/CHANGELOG.md` 记一行（补全列表多 30 个名字 = 用户可见改动）。

