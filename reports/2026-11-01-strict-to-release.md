# 全局计划：**strict 彻底推上线**（2026-11-01 定，主会话只做拆解/审/验收，开发交子代理）

> 上游：`reports/2026-10-29-strict-replan.md` §4（阶段 P0…P4 与 P2 的 13 刀）、
> `reports/2026-11-01-p212-s4b-census.md`（回落清单）、`reports/2026-11-01-craft-notes.md`（怎么干）。
> 本文回答一个上一棒没答的问题：**「上线」到底要交什么** —— 不只是代码判据，还有发版面。

## 0. 目的（先记住这个，再看判据）

**让 strict 成为可信、够快、与 Lean 4 一致的唯一判定路径**，而不是一套靠"读回文本猜类型"的影子实现
（`REQUIREMENTS §9.39` + `replan` 开篇）。三条导出：

1. **strict = 绑定策略，不是第二套 type oracle**（隐式位 = typed metavariable，位置实参只绑显式）；
2. **elaboration 禁止 print→parse** —— 它已造出**已发布**的正确性 bug（`\0soko_m58` 漏名），并让 strict 慢 **11×**；
3. **`strict ≤ loose` 是硬不变量**（计数判据，不用墙钟）⇒ 每批都要用 `scripts/strict-score.py` 复量。

⇒ **任何一批的"完成"都不许只看"回落变少了"**：必须同时看"还判得对吗、成本还 ≤ loose 吗、
文本通道的调用是不是在减少（P3 的终点是 0）"。

## 0bis. 目标态（用户可见，这就是"成功"的定义）

| # | 上线后用户看到什么 | 谁定 |
|---|---|---|
| 1 | **strict 是默认档**（打开编辑器/跑 CLI 不用设任何环境变量），`--dialect loose` 仍可显式选 | **用户拍板**（默认档由用户定，`REQUIREMENTS §9`） |
| 2 | strict 档解不出时给**人话**：点名是哪个 binder、给可照抄的出路（`@g <A> 0`），**永不**出现 `\0soko_m…` / `?1073741823` | 设计稿 Q3（S2 已实现，S3 接线） |
| 3 | strict **不慢于** loose（计数判据，不是墙钟） | replan §3 |
| 4 | `git grep SOKO_STRICT_KERNEL_APP` = **0**；`audit-strict-paths` strict 专属项 = **0** | P3 判据 |
| 5 | 发版闭环：bump → CI 绿 → auto-tag → release 资产齐 → 站点写的是**已发布版本** | `docs/RELEASE.md` §3/§5 |

## 1. 从现在到上线的整条链（**六批**，每批一个可 push 的批次边界）

| 批 | 内容 | 判据（可执行） | 谁做 |
|---|---|---|---|
| **1（在跑）** | S3 并入（`SOKO_STRICT_NO_FALLBACK` + 走 S2 报错出口）+ S4b 根因报告 | 开关关 714 行 0 差异 · c1 报错可读且 span 非 0:0 · `Fallback` 四例外与 S2 对齐 · S4b 有 Q1–Q5 · **`strict-score.py` 计分板绿（lib 17/17 两档 + `work_le_loose`）** | 子代理写，主会话审 + 跑计分板 |
| 2 | **S4b 实现**：`Set.Le`（`A ≼ B`）一族，`old_ok` **185→64** | 只降不升 · rc/诊断不变 · 动内核才跑两对语料 · 复现件修前红 | 子代理，主会话定分叉 |
| 3 | **S4c**（余 9 次：`unsolved` 4 / `argmismatch` 2+2 / `expected` 2）+ **S5**（CLI + LSP hover 判据 **+ 修 `error_hint.rs:93` 的死出路**，见 §2bis） | 四种例外逐条有测试；c1/c5/R1 文字测试 | 子代理 |
| 4 | **S6 翻默认**（`effective = dialect.strict()`、删 `env_on`、重写 `kernel_app_keys.rs`）+ **P2-13**（砍共享代码里的 strict 分支） | P2 总判据四件（lib 17/17 · 整门课 strict 全过 · 计数 ≤ loose · loose 逐字节） | 子代理写，主会话跑两对全语料 |
| 5 | **P3 清理** + **发版面**（见 §2） | `audit-strict-paths` 0 · grep 开关 0 · docs-lint 绿 · skills/编辑器/文档同轮同步 | 主会话 + 子代理 |
| 6 | **P4 发版**（bump → push main → CI → auto-tag → release 核对） | `release-preflight.sh` 全绿 + `gh release list` 资产齐 | **主会话，且 push main 前问用户** |

## 2. 发版面（P4 之前必须同轮做完的"用户可见"清单）

这一块上一棒没动过，但**不做的后果是发出去的版本和文档/编辑器不一致**（AGENTS「收尾义务」）：

1. **`REQUIREMENTS §9` 追加一条**：strict 默认档 + 两档差异（设计稿 Q3 的"有意不同"：我们的 unsolved 在应用末尾就报、
   Lean 留到声明末尾）+ `SOKO_STRICT_AT` 仍是逃生门。
2. **skills 三件**：`sokonanoda-dev`（`skills/sokonanoda-dev/SKILL.md:73` 还写着 `SOKO_STRICT_AT=1` 与"默认 Loose"）、
   `sokonanoda-teacher`（`:429` 提到 `--dialect strict`）、`sokonanoda-ci`；`.agents/skills/` 入口不动（`crates/cli/tests/dsh.rs` 守）。
3. **VS Code**：设置项 `sokonanoda.dialect.strictImplicitArgs` 已存在（`editor/vscode/package.json:307`）⇒ 要看它是否还描述
   "实验/需要环境变量"；README / CHANGELOG / package.json 同轮改；设置项文案算用户可见改动。
4. **接手文档**：`AGENTS.md` 硬规则 7（strict = 绑定策略）要补"**默认已开**"；`docs/ONBOARDING.md` 的开档说明；
   `docs/architecture.md` §6 内核台账补 S0b/S4a 两条（陈旧 `Value` 的第三、四个补丁点）。
5. **站点**：`scripts/gen-site-data.py` 自动出数据（**不手写版本号**）；`python3 scripts/check-site.py` 10 项 +
   `--browser` 两项。
6. **台账**：`docs/e2e/ledger.jsonl` 按批次记一条；`docs/perf/ledger.jsonl` 若内核有性能改动则记。
7. **发版机制**：`python3 scripts/bump.py <x.y.z>`（写 Cargo.toml + package.json + Cargo.lock + 清单 requires + site 数据）
   → `scripts/release-preflight.sh` → push main（**问用户**）→ `ci.yml` auto-tag → release → `gh release list` 核对。

## 2bis. 上线前必须修的用户可见文案（**S3 子代理在读数报告里抓到的真缺陷**）

`crates/front/src/compile/error_hint.rs:93`（`ElabImplicitArgumentUnsolved` 的 hint）仍写着
「出路 ② 给整个表达式或相关实参加类型标注」—— 而本子集**没有** `(e : T)` 这种语法（`REQUIREMENTS §9` 的 a1 教训：
「给用户的出路必须是可执行的」）。S2 的 `unsolved_message` 已经删掉这条，但 **hint 表是另一个来源**，没同步。
⇒ 归入 **S5**：把这条 hint 改成与 S2 message 一致的、能照抄的出路（`@g <A> 0`），并加一条断言「hint 里不许出现
`(e : T)` 这类本子集没有的语法」。翻默认（S6）之后这条会被更频繁地看到 ⇒ **上线前必修**。

## 2ter. **全语料不变量实测**（2026-11-01 14:47，主会话量；这是本轮最重要的一张表）

口径：`git ls-files '*.sokonanoda'`（357 个）× {loose, strict}，各跑 `grade --json` + `SOKO_JUDGE_STATS=1`，
闸门开（`SOKO_STRICT_KERNEL_APP=1`），每跑独立冷缓存。二进制 = S4b 并入后的 `target/release/sokonanoda`。

| 量 | loose | strict | 比值 |
|---|---:|---:|---:|
| `JUDGE_INFER.calls` 合计 | 21,033 | 21,367 | **1.02×** |
| `JUDGE_INFER_SPLIT.misses` 合计 | 3,083 | 3,116 | **1.01×** |
| **逐文件 `strict ≤ loose`** | — | **347 / 355 成立**（8 个不成立；**S6 之后 = 348/355、7 个**） | — |

**8 个不成立的文件**（S6 之后 7 个：`unit12-solution` 变好）（全部是 `fallback_old_ok > 0` 的文件 ⇒ **违反量 100% 由回落解释**）：

| 文件 | calls loose→strict | misses loose→strict |
|---|---|---|
| `unit19-solution` | 64 → **284** | 4 → 14 |
| `unit19-cardinal-le` | 64 → **157** | 4 → 12 |
| `unit12-solution` | 56 → **100** | 9 → 12 |
| `unit06-solution` | 129 → 139 | 7 → 12 |
| `G48-…nullary-sugar` | 2 → 12 | 2 → 5 |
| `unit02-subsets-empty` | 35 → 42 | 3 → 6 |
| `G19-zero-ary-notation-applied` | 8 → 12 | 2 → 5 |
| `G30-min-eq-subst-half` | 4 → 8 | 2 → 6 |

**⇒ 两条结论**：
1. **现在的门是瞎的**：`strict-score.py` 的 `work_le_loose` 只量 3 个恰好都在门内的文件 ⇒ 它绿 ≠ 不变量成立。
   必须把它换成/加上**全语料逐文件**比对（本表就是那个判据的第一次读数；门要能咬住这 8 个）。
2. **不变量违反 = 回落税**：8/8 都是 `old_ok > 0` 的文件（内核先做一遍、老路再做一遍）。⇒ 消回落（S4c）与
   翻默认（S6）会把它推下去；但**写明的例外**（`input contains a meta` / `shape` / `ho_motive`，合计 ~55 次）
   仍会留下回落 ⇒ 上线时**要么**接受"例外文件不满足 `strict ≤ loose`"（写进 §9.39 的已知偏差），
   **要么**把例外也修掉（`input contains a meta` 要动内核 = P1 限制）。**这是发版前必须拍板的一条（D4）**。

## 2quater. 已知限制（**接受、写进台账、不阻塞上线**）

1bis. ✅ **记法路由的"出路"已可照抄**（**S4d** 2026-11-01 修好；S5 实测 R1 曾是 `` `@Ne <α> ` ``：**没有操作数、末尾一个空格**）。
   根因在 `kernel_app_fallback::facts_notation` 拼 `whole_text` 时只放了规范名 ⇒ `unsolved_message` 的 `strip_prefix`
   拿到**空** `rest`。现在 `whole_text` = 规范名 + **操作数**（`render_msg` 渲染、**逐位加括号** —— `''` 是 `infixr:80`，
   不加括号会被读成 `(@Ne <α> X) '' Y`，实测 `kernel-rejected`）⇒ 出路
   `@Ne <α> ((fun (_ : Set Nat) => ∅) '' ({@Set.univ Nat})) ((fun (_ : Set Nat) => ∅) '' ({∅}))`。
   ⚠ 粘回去仍**不判绿**，但那**不是**出路文本的事：`''` 在 `@Ne` 里没有期望类型 ⇒ `elab-notation-argument-unsolved`
   （**逐字的源码文本**、`<α>` 填对也照报，strict/loose **两档同样**）⇒ 记法前导类型参数的推断边界（S4b/S4c 同族）。
1. **LSP hover 在"开放练习的签名"里拿不到类型**（2026-11-01 S5 子代理实测）：`crates/front/src/compile/check/walk.rs`
   的开放练习臂在 `open_signature` 回 `Err` 时**只 push 错误、不 push hover 行**（hover 收在局部 `Vec` 里、随 `Err` 一起丢）
   ⇒ 用户 hover c1 签名里的 `g`/`0` 只拿到声明卡片「未通过，见诊断」。
   **值位失败不受影响**（`def bad : Prop := g 0` 的 `0` 已能拿到 `0 : Nat`）。
   修它要先拆 `walk.rs`（**2887 行 = docs-lint ⑨ 的基线，一行都不许涨**）⇒ 独立一刀，放 P3 之后。
2. **`error_hint.rs:93` 的出路 ②**（"给整个表达式或相关实参加类型标注"）—— 本子集没有 `(e : T)` 语法 ⇒ **S5 删掉并加断言**。
3. **例外类回落不满足 `strict ≤ loose`** —— 见 §2ter 与 D4（已决：接受 + 写进 §9.39）。

## 2quinquies. **S4c 之后**：剩下的 8 次非例外回落，全部根因在内核侧（2026-11-01 15:45，measured）

`old_ok` 117 → **115**（`ok_notation_argmismatch` 2→0 是**真前端 bug**：`peel_pi` 把结果箭头也数成一层 ⇒
`missing` 多一 ⇒ 操作数错位；已按老路的**结构前提**修好，无新猜法）。

剩下 8 次（`ok_unsolved` 4 · `ok_argmismatch` 2 · `ok_expected` 2）**根因都不在前端**（S4c 用探针定性，内核一行没碰）：

| 类 | 根因 | 要修得动什么 |
|---|---|---|
| `ok_unsolved` 4 | 元素类型闭环（`(fun _ : Set Nat => ∅) '' {∅}`），诊断自己说"`α` 由实参和期望类型**都定不下来**" | 老路的同形兄弟借用 ⇒ 新猜法；或内核收元变量 |
| `ok_argmismatch` 2 | 推迟位用**内核项**域补上后错位（`deferred=1`）⇒ 要内核那台打印器 | 内核打印器 |
| `ok_expected` 2 | **部分应用**（`args=1 prefix=4`，`finalize` 拿**残余 Π** 与期望比） | 内核建模部分应用，或前端替缺失实参造元变量（新猜法） |

⇒ **S6 翻默认前必须决定这三类怎么办**（另开一刀 / 归例外）。**另有一条既存缺口被 S4c 的例外"照出来"**：
`unit19-solution` 在开关开时 `0/0 → 1/2`，落在 `63:45`（`Set.notMem_empty (f x) (hn x hx)` = **一个脊上分两步应用**，
老路路线③ 有、内核路径没有）+ `66:52` 级联 ⇒ 也要在 S6 前处理或归例外。

## 2sexies. **D1 已决：不翻默认档**（2026-11-01 18:20，S7 实测后）

S7（翻 `Dialect::default()`）在收口时暴露三条**既有缺口**，任何一条都不该被"默认档"放大给所有用户：

| # | 缺口 | 实测 | 影响面 |
|---|---|---|---|
| 1 | **`check.py` 的 `targets=258` 递归收 `units/solutions/*`**，含 2 个不在 `course.json` 的 orphan ⇒ 默认档下**课程门禁常红**（G1+G4） | 2 个文件被判负（`unit12-solution` 的 `Ne` 隐式位 · `unit19-solution` 的脊上两步） | 门禁口径 |
| 2 | **`eqsubst` 的 old-style 前缀重建在 strict 被拒**（`apply Eq.subst _ hx` / `apply Eq.subst (Eq.symm h)`），loose 绿 | front 侧既有边界（`eqsubst.rs` 三处 `Expr::App { explicit_spine: false }`） | 课程语料 0 命中，但**用户可见**（unit02 练习 6 的第一问就是这么写的） |
| 3 | **G-48 形状 `¬ (∅ ≈ {b})` 默认档报 `unknown identifier  soko_m2`** —— **内部元变量 binder 漏到用户面前** | strict 红、loose 绿 | **§9.39 要消灭的那一类**（已发布过 `\0soko_m58`） |

⇒ **决定**：S7 **不并入**。strict 以"**不再需要环境变量的显式档**"上线（S6 + 发版面已完成）；
翻默认留到这三条各自修完之后。三条的最小复现件与根因定位进
`reports/2026-11-01-strict-gaps-before-default-flip.md`。

⇒ 对完成定义的影响：**判据 1（strict 是默认档）本版不发**；改为"strict 可选、无需 env"（已达成）。
这是**有意的范围决定**，不是失败：把三条已知缺陷推给所有用户不是"上线"。

## 2septies. 翻默认前的**最终验收**（2026-11-01 21:10，`d5d3b9c5`）

| 项 | 读数 |
|---|---|
| 整门课 `check.py` **loose** | 258 目标 · 2191 checked · 914 open · **0 判负** |
| 整门课 `check.py` **strict** | 258 目标 · **2191 checked** · 914 open · **0 判负** ✓ |
| 714 行语料 **loose 半** | **0 变化**（逐字节） |
| 714 行语料 **strict 半** | 只 2 条，都是**拒绝 → 接受**：`unit12-solution` `2c11fd83`→`0d1a8f45`（`923fd738` 源级改写）· `unit19-solution` `bfce0819`→`47260191`（`a027992f` 出口规则收窄） |
| `notation-lint` | OK（332 文件零旧写法；797 处显式豁免） |
| 冻结四文件 | `elab 8841` · `implicit 1218` · `judge 4748` · `meta 1848` 一行未动 |
| `crates/kernel/**` | **0 行** |

⇒ **翻默认档的前置全部清空**，S7 已派出。

## 2octies. **代码侧已完成**（2026-11-02 03:00）；只剩 P4 发版（D3 待用户点头）

| # | 完成定义 | 状态 | 证据 |
|---|---|---|---|
| 1 | strict 是默认档，无需环境变量 | **✅** | `ee20473a`/`5001aa55`：默认 `id2 Nat 3` rc=1、`--dialect loose` rc=0；两处 env 在代码里 0 处 |
| 2 | `grep SOKO_STRICT_KERNEL_APP`=0 + strict 专属文本通道=0 | **✅** | `40443665`（30 点逐条归类；唯一不对称点 `elab.rs:4853` 是 **loose 专属**） |
| 3 | lib 17/17 · 整门课 strict 全过 · loose 逐字节 | **✅** | 计分板（不带 env）四门绿：`lib_strict/lib_loose 17/17`、`course_strict 112/112`；`check.py` 默认档 **258/2191/0 判负**；loose 语料 **0 行** |
| 4 | `strict ≤ loose` | **⚠ 7/355**（D4 已决接受） | 差额全来自写明的例外回落；全语料 calls 1.023×、misses 1.009× |
| 5 | 两对 714 行语料 | **✅** | 翻默认前后两档逐字节不变；内核 0 行（本批无内核改动） |
| 6 | 报错是人话 | **✅** | c1 `2:14` · 出路可照抄 · hover 保住 · `error_hint` 死出路已删 · **G3 后内部名不再泄漏**（`0:0`→使用处） |
| 7 | 发版闭环 | **⬜ D3 待点头** | `scripts/release-preflight.sh` 已跑到 ④（LKA 门禁，长跑中）；①②③ 已过 |
| 8 | 文档同轮同步 | **✅** | `26400a5c`（14 处）+ `5001aa55`（8 处，S7 翻默认）· `docs-lint` 绿 · 冻结文件零增长 |

## 2nonies. **P4 的两个拦路石**（2026-11-02 03:50 实测）

1. **D3：push main 要用户点头**（用户早先定的规矩）。
2. **LKA 门禁在本机跑不完**（`scripts/arena-gate.py`）：`release-preflight.sh` 的 ④ 在 02:11 起**冻住 85 分钟无输出**，
   且**没有编译活动**（`target/release/deps/*.rmeta` mtime 停在 02:11）⇒ 不是慢，是挂。
   诊断：`gh` 已登录（`ColorlessBoy`）、`api.github.com` 返回 200 ⇒ 不是网络基本连通性问题；
   用 `--offline`（本地语料）**同样挂住 >10 分钟** ⇒ 大概率是**语料跑起来本身**（`cargo` 侧或脚本自己的等待）。
   ⇒ `bump.py` 在写版本号**之前**就跑这个门禁、失败不写 ⇒ **P4 卡在这里**。
   出路（都留给下一轮）：① 用 `SOKO_ARENA_CORPUS=<dir>` 显式指定已知良好语料（**不是跳过**，是显式来源）；
   ② 查 `--offline` 为什么挂（`SOKO_ARENA_TIMEOUT` 默认 120s/例、`CARGO_TIMEOUT_S`）；③ 换一台能跑通它的机器做发版。

## 3. 用户决策点（只有三个，其余我全包）

| # | 决策 | 现在需要吗 |
|---|---|---|
| **D1** | **默认档**：strict 直接默认开，还是先给一个"设置项默认开、环境变量仍可关"的过渡版本 | 批 4 之前 |
| **D2** | **S4b 分叉**（若根因是内核 P1 限制）：(a) 内核补一阶近似 / (b) 课程改点名 `Set.Le α β A B` / (c) 记为已知差异进 §9.39 | 批 1 的 Q2 出来之后**立即**要 |
| **D3** | **push main / 发版**（批次收尾的推送不必问；merge 到 main 与打 tag 必须问） | 批 6 |
| **D4** | ~~例外文件不满足 `strict ≤ loose` 能不能发~~ **已决（2026-11-01，主会话按用户「你来把控」授权）**：走 **(a) 接受 + 写进 §9.39 的已知偏差**。理由：例外类（`input contains a meta` / `shape` / `ho_motive`）的回落是**接受面**的需要——删掉它们会让现在能过的课程文件被拒（`第 3 条` 直接失败），而修 `input contains a meta` 要拆掉 P1 的内核限制（1–2 批，且动内核）。⇒ 上线时 §9.39 写一句：「`strict ≤ loose` 在全语料 347/355 个文件上成立；其余 8 个的差额**全部**来自写明的例外类回落（合计 1.02×、单文件最差 4.4×）」 | **已决，批 4 前再核一次数字** |

## 4. 分工与"100% 成功"的机制

- **主会话只做四件事**：写实施单（每刀的"怎么做 + 判据 + 边界"）、审子代理产出、跑重判据（两对全语料 / 全课程普查 /
  发版预检）、发版与对用户的汇报。**不亲自写判定代码**（用户 2026-11-01 明确）。
- **子代理只做**：单一刀的实现或只读调研；prompt 里必须带硬纪律摘要 + 可执行判据 + "不许动"边界（见 craft-notes §1–8）。
- **每刀两笔提交**（代码 + `docs(strict): …已并入（sha）…读数`），计划文档永不落后（craft-notes §1）。
- **失败的处理**：中间失败允许，但**必须**归因到"拆解/理解"并**当场改拆法**（用户 2026-11-01：「中间的失败都是任务没拆解好、理解好」），
  不许把它记成"问题本身很难"。判据：同一步连红 3 次 ⇒ 换拆法（拆小、换层、或先补只读调研）而不是重试。
