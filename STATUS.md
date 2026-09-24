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

## 本轮进度（2026-09-24，第一百三十九/四十轮：**线 D 收口 + 0.65.3 发版中**）

1. **T-D16**（勾上，无产品代码）：判据两条实跑——G-23 复现件 **exit 1**（已修）·
   `tests::navigation` **11 条绿**（含本轮补的记法跳转用例）。它是 T-D10 + T-D15
   合起来交付的，这轮把判据钉住。
2. **T-D17 记法 hover 的精确范围**：`range` 从 `None` 换成
   `notation_input::symbol_span_at(...)`（与 `symbol_at` **共用** `symbol_token_at`
   ⇒ "认得出来"与"给出范围"永不漂移）。判据把光标停在 `⁻¹'` 的**中间**（最容易
   歪的位置），断言正好覆盖 3 个字符。LSP **158 通过**；真宿主 e2e
   `--grep "notation symbol"` **2 passed / 0 failed**。
3. **T-D41 文档同步 + ⬆ BUMP 0.65.3**：`notation-subset.md` 新增 §4.1
   「编辑器支持（as-built）」（五条能力各配判据）· `TESTING.md` 守护表新增
   「记法编辑器导航」一行 · `editor/vscode/README.md` 升级为"输入 + 可导航" ·
   `CHANGELOG.md` 0.65.3。判据：`--test skill` **4 passed** · 完整 `gate` **PASS**。
4. **修掉四条 CI 假红**（全部是判据自身的余量/时序，**不是产品回归**）：
   * gap 台账：复现件**硬编码本机路径** + `gap.py` **把任何非零退出都当"已修"**
     ⇒ 环境异常（exit 2）被静默读成"修好了"（真缺口 G-37 被判成已修）。
     两处都修：路径从 `__dirname` 推；`judge()` 对 `code == 2` 直接判红 +
     `selftest` 钉两条（现在 16 条判据）。
   * 缩放判据：CI 实测 **12.4×** 超阈值 12（**O(n²) 是 64×**，12.4 显然不是）
     ⇒ 阈值 **12 → 20**（判别力不减），注释写明"放宽的是噪声余量、不是判据形状"。
   * e2e 项目树：`projectRoot()` 只等**标签**、没等**描述** ⇒ 慢 runner 上
     "2 模块"还没填。改成等"行完整"。**这次按纪律取了 artifact 里的用例名与
     断言行**（上一轮"24/1 但没取到名字"是不合格的处置）。
5. **⚠ 环境阻塞（需要用户处理）**：服务重启后，本机 **DSH 沙箱后端起不来**
   （`sandbox-exec: Operation not permitted`），且 `target/` 里的删除被拦
   （cargo 无法重新链接 build script ⇒ **本地 cargo 构建/测试跑不动**）。
   ⇒ 本轮的后半段**只能用 CI 当验证通道**（改动本身是常量与等待条件，风险低，
   且正是为 CI 红而改）。**恢复办法**：修好沙箱后端（或让 `target/` 可写可删）
   后跑一次 `scripts/soko gate` 复核。
6. **发版状态**：0.65.3 已推 main，CI 在跑；**发版尚未触发**（要等 CI 绿）。
   线上最新仍是 v0.65.2。下一轮第一件事就是**核对 `gh release list` 是否出现
   0.65.3**（REQUIREMENTS §9 的闭环要求）。

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

