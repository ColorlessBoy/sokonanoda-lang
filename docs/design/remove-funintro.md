# 设计：移除值位关键字 `funintro`（2026-09-14）

> 触发：用户评估「`funintro` 跟 `funapply` 一样，实现起来稀里糊涂的，不如直接
> 删了」。`funapply` 已在 0.22.0 因性能/复杂度移除；`funintro` 是仅存的值位
> 关键字，本次一并删除，回到「值位只有普通表达式 + `by` 块」的干净语义。

## 1. 现状与问题

`funintro`（正式名 `Expr::Intro`，原名值位 `intro`，0.17.0 引入）是一个
**发生在值位的关键字**：`theorem t : A -> B := funintro` 由前端把声明类型
剩余 binder 一次展开成 `fun (…) => … => sorry` 骨架，编辑器再提供

- 补全项（`filter_text = "funintro"`，替换 token 为骨架）——含「同一行末尾 /
  下一行 / trailing space / 声明 binder 已剥」等多种定位分支；
- hover 展开按钮（`sokonanoda.expandIntro`）；
- code action「替换源代码 funintro」；
- inlay（洞在关键字 token 上）。

问题：

1. **语义冗余**：它到内核就是 `fun` 链 + `sorry`，与直接写 `fun`/用 `by intro`
   完全等价；判定、目标、错误路径都要为它单开分支。
2. **交互复杂**：LSP 为一个关键字维护大量定位/补全/替换分支与回归测试
   （`crates/lsp/src/lib.rs` 117 处、扩展 49 处），维护成本与出错面大。
3. **教学噪声**：课程 `unit6` 需要额外讲一个「快捷输入」概念，且它的
   「自动展开」掩盖了 `fun`/`by` 本身的语义。

## 2. 方案：整体删除

删除语言关键字 `funintro`（值位与原子位）、`Expr::Intro`、前端 `lower_intro_val`
与 `intro_skeleton` 通道，以及编辑器的补全/hover 按钮/code action/inlay。
值位回到两种形态：**普通表达式** 与 **`by <tactic 序列>`**。

### 2.1 前端

- `ast.rs`：删 `Expr::Intro { answer, span }`；
- `parser.rs`：删值位关键字分支、原子位 `(funintro …)` 分支、lambda 尾
  `parse_lambda_tail_keyword`；`KEYWORDS` 不再含 `funintro`；
- `semantic.rs`：删关键字分类与相关测试；
- 删 `crates/front/src/compile/intro.rs`；`check.rs` 的 `lower_value` 不再返回
  骨架（`LoweredValue` 收窄为 `(Expr, Vec<ByStep>)`）；
- `report.rs`：删 `DeclState.intro_skeleton`；
- `error.rs`：删仅服务于 funintro 的错误分类（若已无引用）；
- `spine.rs` / `proof.rs` / `suggest.rs` / `elab.rs` / `goals.rs` / `by.rs`：
  删 `Expr::Intro` 匹配臂与注释。

### 2.2 保留什么

- tactic `intro`（`by` 块内的一次一层）**不动**——它是 `by` 引擎的一部分；
- 命名箭头 / `fun` / `forall` 语法不动；
- `by` 引擎、goal 视图（含本轮多目标显示）不动。

### 2.3 LSP

- 删 `VALUE_KEYWORD`、值位关键字 hover/code action/补全展开的全部代码与测试；
- 半截表达式 hover 里「或用 `funintro`」文案改为「或用 `by`/`fun`」；
- inlay 的 funintro 分支删除。

### 2.4 编辑器（VS Code）

- 删 `sokonanoda.expandIntro` 命令注册与 `package.json` 声明；
- 删相关集成测试；README 描述去掉该特性。

### 2.5 课程与文档

- `course/unit6-by-tactics.sokonanoda`（+ en 镜像）：删除「补充：值位 funintro」
  段与练习 6（`by_ex6 := funintro`），用等价 `by intro …; exact …` 或显式 `fun`
  收尾；`solutions/` 钥匙同步；golden 计数更新；
- docs（`protocol.md`/`README.md`/`TESTING.md`/`architecture.md`/`design/*`）、
  根 `README.md`、`site/`（含 `gen-site-demos.py` 产出的演示）、`skills/`：
  删除或改写引用；历史归档 `STATUS-ARCHIVE.md` 保留原文不动（历史记录）。

### 2.6 明确不做

- 不引入替代关键字（如 `fun!`）：值位只保留表达式与 `by`；
- 不迁移旧文件：`funintro` 从此是解析错误（与 `funapply` 相同的处理）。

## 3. 测试

- front：删除 funintro 解析/展开/判定测试；保留并确认 `by` + `fun` 路径；
- LSP：删除 funintro 补全/hover/code action/inlay 测试；保留 `by`/goal 测试；
- CLI e2e：删除 funintro 用例；course golden 按新 unit6 计数更新；
- extension：删除 funintro 集成测试；静态契约删对应断言。

## 4. 验收

- 全仓 `rg funintro` 仅剩历史归档/本设计文档（无活代码/活课程引用）；
- `sokonanoda gate` PASS；
- `course` golden、`course_status` 汇总与新的 unit6 一致；
- 版本：本轮并入 **0.27.0**（与多目标显示同版），CHANGELOG `Removed` 记录；
- `REQUIREMENTS.md §9`、`STATUS.md` 同步。

## 5. as-built（2026-09-14）

- 前端：删 `Expr::Intro`、`parse_intro` / `parse_lambda_tail_keyword`、原子位
  `funintro` 分支、`KEYWORDS` 的 `funintro`、`compile/intro.rs`、
  `DeclState.intro_skeleton`、`lower_value` 的骨架分支、`ErrorKind::ElabIntroNotAFunction`；
  `spine.rs`/`proof.rs`/`elab.rs`/`goals.rs`/`semantic.rs` 匹配臂与注释同步。
- LSP：删 `VALUE_KEYWORD` / `keyword_*` / `expandIntro` hover 与补全 / inlay
  funintro 分支及全部对应测试；半截表达式 hover 文案改为「或用 `by`/`fun`」。
- 编辑器：删 `sokonanoda.expandIntro` 命令与处理器、`markdown.isTrusted`
  白名单、package.json 命令声明；集成测试与 README 同步。
- 课程：`unit6`（zh+en）「补充 funintro」段与练习 6 改写为综合 `by` 练习；
  `solutions/` 钥匙同步；golden 事件计数不变（练习数不变，仅作答形态变化）。
- 文档/site/技能：`protocol.md` 删值位关键字小节、`architecture.md`、`README`、
  `docs/README.md`、`TESTING.md`、`ROADMAP I13` 标注废弃；`term-intro.md` /
  `value-keywords-v2.md` 加废弃横幅；`site/` 两个页面 hero 换成 goal 演示；
  `gen-site-demos.py` 删 completion 演示；teacher 技能表同步。
- 历史归档 `STATUS-ARCHIVE.md`、`CHANGELOG.md`、`REQUIREMENTS §9` 保留原文。
- 版本：并入 **0.27.0**（与多目标显示同版），CHANGELOG `Removed`。
