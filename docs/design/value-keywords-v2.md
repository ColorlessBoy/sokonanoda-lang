# 值位关键字 v2：改名 funintro/funapply + 输入期补全 + 关键字组合

> 状态：设计定稿（2026-09-13，第四十三轮）。用户三需求，本文是唯一权威口径；
> 实施按 ROADMAP §10 I13 的 S1–S4 顺序，由 subagent 逐个执行、每步全量门禁。

## 0. 用户需求原文（触发）

1. lambda 尾 `apply And.intro` 的 hover 按钮已正确，但**输入过程中**没有补全替换提示（要求模拟真人测试）。
2. `intro (apply And.intro)` 应当工作：`intro` 到内核那边就是 `fun (a : Prop) => fun (b : Prop) => fun (x : And a b) =>`，`(apply And.intro)` 在内核那边就是 `And.intro b a` 啥的——纯前端实现。
3. 值位 `intro`/`apply` 与 tactic 版本太易混，改名 **`funintro`** / **`funapply`**；按钮文案「展开为 apply 骨架」→「替换源代码 funapply」、「展开为 fun 骨架」→「替换源代码 funintro」。

## 1. 调研结论（事实依据）

### 1.1 输入期补全缺失的根因（探针实测，`probe_typing_completions_in_lambda_tail`）

逐键输入 `apply And.intro`（lambda 尾）的每个中间态请求补全：

| 输入中间态 | 声明状态 | 展开替换项 |
|---|---|---|
| `a` … `apply` | Failed（半截实参 → `elab-unknown-identifier`） | **缺席** |
| `apply ` | Failed（`elab-apply-needs-a-term`） | **缺席** |
| `apply And.intro`（整串完成） | Open（两个前提洞，σ 实例化正确） | 出现（骨架 `And.intro b a sorry sorry`） |

根因：`lsp/lib.rs::keyword_at` 要求 `d.status == Open`，而**输入中间态必然 Failed**。骨架数据本身完全正确（σ 实例化、双洞、span 全对）——纯门禁问题。

### 1.2 组合的可行性（调研：research-compose）

- parse 层：`parse_atom` 的 `(` 分支走 `parse_expr`，**括号内不识别关键字**——`(apply X)` 今天解析成 `App(Ident "apply", …)`。最小改法：在 `parse_atom` 给关键字加分支，使关键字成为**原子位表达式**；同时把 `parse_value`/`parse_lambda_tail_keyword` 四段逐字重复的解析收敛成共用 `parse_intro`/`parse_apply`。
- lowering 层：`apply.rs::lower_at`（私有）的输入恰好是「孤立 Apply 节点 + 给定目标类型 + 局部假设 context」——提为 `pub(crate)` 即可直接复用做「对最终 goal 类型降低一个 Apply」。缺的只是把 `src/span_start/options` 传进 `lower_intro_val`，并让 `peel_all_pi` 返回最终 goal 类型与层 binder（作 context）。
- 判定路径：组合产物带洞 → `open_goal` 走 ctor_spine_case → Open + 合成洞；`suggest.rs` 的 `synthetic` 分派（`intro_skeleton.is_some() || apply_skeleton.is_some()`）对组合态**自动成立**，不碰 `judge_hole_fill` 的 sorry 守卫。
- 错误面：`elab-apply-not-applicable` / `elab-apply-needs-a-term` / `elab-intro-not-a-function` 已覆盖主要失败模式；嵌套 `funintro(funintro …)` 让内层自然报 `elab-intro-not-a-function`（剥完无函数可剥），无需新码。

### 1.3 改名爆炸半径（调研：research-rename）

约 30 文件 / ~60 测试名。关键事实：课程 golden 是**事件计数**而非文本锚点——unit6 的 5 个 theorem（zh+en）同步改名后计数不变、golden 不动；`site/*.html` 零命中；solutions 零改动。

## 2. 决策单（已定，实施不再讨论）

| # | 决策 | 理由 |
|---|---|---|
| D1 | 值位关键字改名 `funintro`/`funapply`，**不保留旧名别名**，干净切换 | 留别名要在 parser 三处入口 + KEYWORDS + 补全去重加双名分支，长期成本高；课程/画布同一批改 |
| D2 | **by 块内 tactic `intro`/`apply` 完全不变**（这正是改名动机） | — |
| D3 | `semantic::KEYWORDS` **保留** `intro`/`apply`（by tactic 的语义高亮是位置无关的，移除会丢 tactic 高亮；有测试 `intro_classifies_as_keyword_in_value_and_tactic_positions` 双位置断言），**新增** `funintro`/`funapply` | 消费方：semantic 分类、LSP 裸关键字补全、hover 抑制链路 |
| D4 | 编辑器命令 ID **保留** `sokonanoda.expandIntro`/`expandApply`（内部不可见），只改用户可见文案 | ID 耦合 4 处，改了零用户收益 |
| D5 | 错误码 `elab-intro-not-a-function` / `elab-apply-needs-a-term` / `elab-apply-not-applicable` **保留机器码**，hint 文案改新名 | 码被 protocol/测试/skills 引用；改名是 breaking protocol 收益低。protocol.md 注明「码名为历史名」 |
| D6 | `intro_skeleton`/`apply_skeleton` 字段名**保留**（内部 API）；组合骨架填 `intro_skeleton`、`apply_skeleton` 留空 | 跨 front/lsp 的 pub 字段，改动波及大且用户不可见 |
| D7 | 按钮文案：funintro → 「替换源代码 funintro」；funapply → 「替换源代码 funapply」 | 用户指定 |
| D8 | actions.rs 的 quick-fix 标题「intro N 个 binder…」→「引入 N 个 binder…」（它用 intro 作动词，同样歧义） | 顺手消歧义 |
| D9 | AST 节点名 `Expr::Intro`/`Expr::Apply`、模块名 `intro.rs`/`apply.rs` **不改**（内部） | 无对外序列化 |

## 3. 特性设计

### 3.1 输入期补全（I13-S2）

**目标**：逐键输入时补全弹窗里始终有可选项；关键字敲完即可见「替换源代码」项；实参敲完、声明变 Open 后出现完整骨架替换项。

**设计**：`keyword_at` 改为两态：

1. **骨架态**（现状增强）：声明 Open 且骨架可算 → 现行为（textEdit 覆盖 `funapply <term>` 整段，newText = 完整骨架）。
2. **键入态**（新增）：声明非 Open（或骨架不可算），但光标处的词是 `funintro`/`funapply` 的**前缀或全词**，且该词处于值位/lambda 尾上下文 → 仍出项：label「funapply（替换源代码）」，textEdit 覆盖已敲的前缀、newText = 关键字全词（接受即补全单词），documentation 说明「实参敲完后出现完整骨架替换」。preselect 保持。

实现要点：
- 探测不依赖 DeclState 的骨架字段，但**仍需声明上下文**（光标在某声明的值区域内）——复用 `keyword_at` 的声明定位，把「status == Open」门槛替换为「骨架可算 → 骨架态；否则 → 键入态」；
- **前缀词提取**：从光标向前扫描 `[A-Za-z0-9_]`，得到部分词；`funintro`/`funapply` 前缀匹配（含全词）才算命中——这**推翻**了 v1 的「整词门控」（`typed_*_only_expand_on_the_whole_word` 断言前缀不出项），本设计明确反转为「前缀出键入态项、整词+实参完成出骨架态项」，原两条整词门控测试改写为新语义；
- 失败态声明的定位：Failed 声明的 `decl.span` 仍在（诊断来源就是它），`keyword_at` 的声明定位逻辑不需要 status；只有骨架字段读取受影响。

**真人输入测试**（`testutil::char_steps` + `type_step`，零 sleep）：
- 逐键输入 `funapply And.intro`（lambda 尾）：每个前缀态断言「替换源代码」项在、preselect、textEdit 覆盖前缀；整串完成断言骨架态项的 newText = `And.intro b a sorry sorry`；
- 逐键输入 `funintro`（裸）：键入态项全程在；
- 完成后删除实参重敲（选中重打）回归。

### 3.2 关键字组合（I13-S3）

**语法**：`funintro` / `funapply` 成为**原子位表达式**（`parse_atom` 识别），因此：
- `(funapply And.intro)` 解析为 `Expr::Apply`；
- `funintro (funapply And.intro)` 解析为 `Expr::Intro { answer: Some(Apply) }`；
- `parse_value` / lambda 尾的特判保留（裸关键字 span 从关键字 token 起），但实现收敛到共用 `parse_intro`/`parse_apply`。

**语义**（自底向上降低，内核零感知）：
- `funintro`（含答案 E）：剥掉目标类型剩余的全部 Pi 层 → lambda 链；末端放 `lower_keywords_in(E, final_goal_ty, context)`：
  - E 是 `Expr::Apply` → 调 `apply.rs::lower_at`（提为 pub(crate)）对 final_goal 降低 → 部分应用 + 前提洞；
  - E 是 `Expr::Intro` → 递归 `lower_intro_val`（对 final_goal；若 final_goal 非函数 → 既有 `elab-intro-not-a-function`，天然覆盖嵌套 funintro）；
  - E 是其它表达式 → 原样（现状：内核终审）。
- `funapply t` 在原子位：即当前 lambda 尾/值位 apply 语义（类型经 `judge_infer` 推断）。
- 骨架：组合骨架 = funintro 部分全剥 + funapply 部分展开，整体填 `intro_skeleton`（D6），形如
  `fun (a : Prop) => fun (b : Prop) => fun (x : And a b) => And.intro b a sorry sorry`；
  洞 span = funapply 节点的 span（多个前提洞共享），`keyword_at` 现有 `find` 无碍。
- 等价性契约（与 intro/apply 同款）：`funintro (funapply And.intro)` vs 手写 `fun (a : Prop) => fun (b : Prop) => fun (x : And a b) => And.intro b a sorry sorry` ——同 status/goal/sub_goals 期望类型/洞数；填洞后内核判定一致。
- 错误：funapply 结论与最终目标不合 → `elab-apply-not-applicable`；嵌套 funintro → 内层 `elab-intro-not-a-function`；漏降低的关键字节点仍由 elab 护栏（`elab-hole-misplaced` internal）兜底。

### 3.3 改名（I13-S1）

按调研清单执行，决策按 §2。要点重申：
- parser 三处入口（value / lambda 尾 / **原子位——S3 加**）统一收敛为 `parse_intro`/`parse_apply`；
- KEYWORDS 保留旧两词 + 新增两词（D3）；LSP 的 label/filterText/sortText/`ValueKeyword::name()`、`keyword_copy`/`keyword_detail` 全改新名；
- hint 文案改新名（error.rs 三处 + intro.rs/apply.rs 内消息），机器码不变（D5）；
- course/unit6 zh+en 的 5 个 theorem 与「三种写法对照」文案、`playground.sokonanoda` 的 `apply And.intro` → `funapply And.intro`；golden 计数不受影响（调研已证）；
- ~45 个值位测试改名；by 块 tactic 测试（约 8 个含 intro/apply 字样）确认不动；
- 编辑器按钮文案按 D7；命令面板 title 按 package.json 现格式（category 提供前缀）。

## 4. 兼容与迁移

- **无别名期**（D1）：0.21.0 起值位只认 `funintro`/`funapply`；写旧名 `intro`（值位）会变成普通标识符 → `elab-unknown-identifier`（教学 hint 提示新名——**hint 增强**：unknown identifier 的消息若词面是 `intro`/`apply`，附一句「值位关键字已改名 funintro/funapply」）。
- 画布/课程/文档同批发版，官网 progress 由 gen-site-data 自动带新版本号。

## 5. 验收标准（I13 总验收）

1. 逐键输入（`char_steps` 模拟）在**每个中间态**都有可选补全项；整串完成出现骨架替换项（§3.1 的三条真人输入测试绿）。
2. `funintro (funapply And.intro)` 端到端：Open、goal=`And b a`、sub_goals 期望类型依次 `b`/`a`、组合骨架逐字符正确；等价性契约测试绿。
3. 全仓 `grep -w "intro\|apply"` 在**值位语义**无残留旧名（by tactic 除外）；课程/画布/文档/技能同步；course golden 不变。
4. 三层门禁全绿（front/lsp/cli + 扩展单测 + 集成测试）；clippy 0；`--json playground` exit 0。
5. 版本 0.20.0 → **0.21.0**（新语法 = minor），push main 后 auto-tag 自动发布（对自动发版链的又一次实战验证）。

## 6. 实施切分（ROADMAP I13）

| 步骤 | 内容 | 执行者 | 文件集 |
|---|---|---|---|
| S0 | 探针（已完成，本轮调研产出）+ 设计文档 | 主会话 | 本文件 |
| S1 | 改名 funintro/funapply + 解析收敛 + 文案/课程/画布/编辑器 + 测试全扫 | subagent-1 | parser/semantic/error/intro/apply/check/elab + lsp lib/inlay/actions + editor/vscode + course×2 + playground + 各 docs |
| S2 | 输入期补全两态门控 + 真人输入测试改写 | subagent-2 | lsp lib.rs + testutil + design 文档 §8 |
| S3 | 关键字组合（parse_atom + lower_at 复用 + 组合骨架）+ 组合矩阵测试 | subagent-3 | parser/front compile×3 + lsp 文案 |
| S4 | protocol/TESTING/architecture/README/site 技能收口 + 版本 0.21.0 + 发布 | 主会话 | docs + Cargo.toml/package.json |

依赖：S1 →（S2、S3 可串行，因共享 lsp lib.rs 与 parser）。每步完成跑全量门禁，红不交接。
