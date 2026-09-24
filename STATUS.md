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

## 本轮进度（2026-09-23，第一百三十八轮：**用户第 7/8 条反馈的机制查明**）

> 用户要求：「背后的 bug 机制先搞明白，然后再统一修复，这个应该是一个共性问题。」
> ——两条都查到根因（**不是猜的**），并落成缺口台账 + 计划环节。

1. **机制 A：声明栏的 `forall` 是"一批符号"的问题，而且是三层叠加**
   （缺口 **G-38**，复现件 `docs/gaps/repro/G38-folding-only-infix.sh`）：
   * ① `display.rs::fold_spine` 只放行 `Infix|Infixl|Infixr`（注释里就写着
     "前缀/后缀与 binder 记法的折叠留给后续环节"）⇒ `𝒫`/`ᶜ`/`∀∃`/`∅` 全漏折；
   * ② **`∀`/`∃` 根本不在内建记法表**（`BUILTIN_NOTATIONS` 只有 `∧ ∨ ↔ ¬ = ≠`）
     ⇒ 光改过滤器也折不出来；
   * ③ 声明栏那个 `forall` 是**内核 pp 打的 telescope**，不是源码里的 `∀`
     ⇒ 折叠要认 `forall (x : T), body` 这个**形状**。
   实测 `ty` = `forall (α : Type 0) (a : α) (A : Set α), a ∈ A -> a ∈ A`
   （`∈` 折了、`forall` 没折）。
2. **机制 B：记法声明的目标名从来不是使用点**（缺口 **G-37**，真 LSP 复现件）
   ——五条声明的目标名 × {definition, hover, documentHighlight} = **15 个请求全为
   null** ⇒ **ctrl+点击不能跳转是共性问题**。而"只有三条没高亮"是**同一根因的
   第二种症状**：语义 token 类型号不同（在本文件里声明的 → **4 = FUNCTION**；
   不在作用域的 `Set.image`/`Set.preimage`/`Set.prod` → **5 = VARIABLE**
   = `UnknownIdent`），因为分类只能退回作用域查找。
3. **新增计划环节**（`plan.py check` OK，123 环节）：
   * **T-D50** 记法声明的目标名成为使用点（着色给"已知引用" + 跳转走闭包 +
     hover 说明）；特别注明：`Set.image` 等**确实不在作用域** ⇒ `resolution`
     诚实为 `None`，**但着色必须仍按"已知引用"**（否则退回今天的"没高亮"）；
   * **T-D51** 折叠扩到 prefix/postfix/binder/零元 + 补 `∀`/`∃` 表项
     （回读必须仍可解析；**判负/事件计数不得变化**）；
   * **T-D52**（用户第 8 条）`def` 的声明多一行"真正定义"——内核
     `Declar::Definition { info, val, hint }` **手里就有 value**，只差一个访问器；
     判据 + **性能必须先量**（报告每次编译都构建，多算一次 `pp_expr` 是新增成本，
     超预算就改惰性）都写进了条目。
4. **下一环**：回到 §13 线性清单（`T-D14` 之后的 **T-D16/T-D17/T-D41**），
   再插 T-D50/T-D51/T-D52。

## 本轮进度（2026-09-23，第一百三十七轮：**T-D14 AST 变更（`symbol_span`）**）

1. **`Expr::Notation` 增加 `symbol_span`**（只覆盖那个符号，不是整段节点）；
   `bump_operator` 改为**返回 `Token`**（以前 `bump()` 的返回值被丢掉）；
   `notation_node` 加参数。六处构造点全填（infix 族 / prefix / postfix / binder /
   零元）。**为什么要它**：节点 span 覆盖整段（`a ∈ A` 三个 token），而编辑器要问的
   是"光标是不是正好压在这个**符号**上"——`notation_at` 以前只能靠**词法重新扫
   文本**回答；现在 AST 侧直接有答案。
2. **判据**：`a_notation_nodes_symbol_span_covers_only_the_symbol`（`∈` 正好 3 字节
   且是节点 span 的真子区间）；并按本条"风险"提示给
   `notation_records_a_hover_row_covering_the_whole_notation` 补断言——"hover 行
   覆盖整段"与"`symbol_span` 只覆盖符号"**不矛盾**（两者回答不同问题）。
3. **AST 变更的连带面**：`by.rs` / `compile/elab.rs` / `compile/goals.rs` ×2 /
   `display.rs` ×2 / `spine.rs` ×4，编译器全部指出来、逐个改。
4. **⚠ 过程事故（第三次同类）**：一次 `str.replace` 把 `elab.rs` **写少了 4852 行**
   ——`git diff --stat` 立刻暴露 ⇒ 恢复重来，之后**每处改动都先断言匹配唯一、
   再核对行数增减**。教训写进 `skills/sokonanoda-dev` 新增的"批量文本替换的纪律"。
5. **下一环**：T-D15（`ResolvedTarget` 增加 `Notation` 变体并绕开覆写）。

## 本轮进度（2026-09-23，第一百三十六轮：**T-D40 三层测试补齐 + `gate --fast` 修缺陷**）

1. **T-D40 三层测试补齐**（矩阵用例 #7/#8）。e2e 两条早在 T-D02/T-D10..T-D13
   就落地了，但三层里**缺两层**，这轮补上：
   * **LSP**：`goto_definition_on_a_notation_symbol_lands_on_its_declaration`
     —— `definition` 在 `∈` 上跳到**声明它的模块**那一行；
   * **front**：`notation_folding_does_not_clobber_a_use_points_resolution`
     —— 线 C 的折叠只动**显示副本**，点名使用点的 `resolution` 仍在。
   * **判据实跑**：`vscode-e2e.sh --grep "notation symbol" --profile debug`
     ⇒ **2 passed / 0 failed**（46s），台账进 `docs/e2e/ledger.jsonl`。
   * **一处如实记录**：front 那条最初想断言"记法符号自己的 hover 行没有
     resolution"，实测**这个夹具里没有正好落在 `⊆` 上的 hover 行** ⇒ 那半条是
     **空断言**，删掉换成"前提守卫 + resolution 仍在"两条真能失败的前提。
     **不凑数。**
2. **发现并修掉 `gate --fast` 的一个真缺陷**：它只看**未提交**的改动
   （`git diff HEAD`）⇒ **提交之后**跑就打印"crates/ 下没有改动"、**静默跳过所有
   单测**——而 `--fast` 恰恰最常在提交后跑。改成取三段并集（`origin/main...HEAD`
   ∪ 工作区 ∪ 未跟踪）；无远端时退回 `HEAD~1`。**造了一个"已提交未推送的 crates
   改动"验证过**：修后确实跑 `cargo test -p sokonanoda-front --lib`。
3. **下一环**：T-D14（parser 保留记法符号 token 的 span——AST 变更，为"表达式内
   跳转"铺路）。

