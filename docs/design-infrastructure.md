# 基础设施全面完善：差距分析 + 方案脑暴

> 状态：**设计草案（brainstorm），不是已定稿的 PRD**。
> 目的：把"基于 sokonanoda 封装一个现代形式化证明教学语言"拆成可执行的工作流，
> 每个决策点给出 2–3 个选项、取舍与推荐，最后列"待你确认"清单。
> 配套：`docs/architecture.md`（现状事实）、`docs/research.md`（外部参照）。

---

## 1. 目标与约束（一句话复述）

**目标**：在 `sokonanoda-lang` 里做一门"现代形式化证明编程语言 + 教学平台"——完整内核、受限且真实的教学语法、练习引擎、可给人也给 agent 的结构化反馈，最终长成协作式教学（L2/L3）。
**约束**：kernel 完整、无官方工具依赖、语法是真实 Lean 4 子集、白名单即课程、L0 先行、TDD/重复测试、反馈即功能。

## 2. 现状盘点（2026-09-06 快照）

已具备：

- M0：kernel 完整迁移 + 内存 API（`EnvBuilder`/`try_check_declar`/`infer_closed_type`/`reduce_closed`）+ arena 与 memory_api 测试。
- M1：`.sokonanoda` 前端（lexer/parser/elaborator/CLI/REPL）、span 诊断、ASCII `->`、`Sort n`/`Sort u`/`{u}`/`id.{u}`/`@id.{u}`、命名箭头、`#print`。
- 语料：fol-basics、py-fol-core（Eq/propext/定理批）、py-nat（显式 inductive + iota）、lesson-01/02。
- `#prove` 草案（intro/exact/apply/assumption + partial lambda 回显，`done` 走 kernel）。
- 显式 `inductive ... ctor ... rec ... iota ... end` 块 + deep reduce。
- 反馈：文本事件行 + **新增 `--json`（JSON Lines 事件）+ 错误 stage/code + 语料 CI**（本轮完成）。

缺口（按 ROADMAP M2–M4 与"现代化语言"标准看）：

1. **练习引擎只有骨架**：`example : T := ???` 能产 `exercise.open`、填满后 kernel 判定通过/拒绝；但**没有"练习定义"**（名字、目标、期望 `#check` 类型/`#reduce` 值），错误只分到 stage，没有教学提示模板，没有 `exercise.solved/failed` 语义。
2. **prelude 太小且"占位自引用"**：只有 Nat.zero/succ/add；Bool、Eq、rfl 语义、"减/乘/比较"等名字都缺；占位自引用体需要被正式对齐（见 architecture §5.4 的风险）。
3. **elaborator 全显式**：无 binder 类型推断、无 `let`、无 `match`、无依赖消除的"自然写法"；课程第 5 单元（归纳类型 + match）因此还上不了。
4. **课程内容基本没有**：只有 4 个示例文件，不是 M4 的 5 单元 × 3–8 练习。
5. **协议未闭环**：`docs/protocol.md` 的事件名已有文本实现，但 exercise 的 solved/failed、诊断分类、incremental 片段事件未实现。
6. **测试基础设施**：无 golden 事件比对（语料测试只查 exit code）；kernel 错误仍是 panic 字符串。
7. **文档**：本轮补齐 architecture/research/design 三件套 + README/ROADMAP 刷新（见下）。

---

## 3. 五个关键决策点（brainstorm）

### D1 练习/答案区格式

- **A（现状）**：`example : T := ???`，元数据放 `--` 注释。优点：改动零；缺点：agent/UI 无法机器识别"这是第几题、期望什么"。
- **B（推荐，轻量指令）**：加 `#exercise "名称"` 作为块头，答案区仍是 `example : T := ???`；`#exercise` 只是元数据声明，不改变 kernel 语义，允许被解析成 `Command::Exercise`。优点：向后兼容、机器可识别、可挂期望；代价：parser 加一个命令。
- **C（重型 DSL）**：`#exercise` 内嵌期望块（期望 `#check` 类型、期望化简值、提示）。优点：表达力最强；代价：格式先行设计，容易返工，违背"M1 先定义格式"的渐进路线。

**推荐 B，C 的期望字段推迟到 D2 的第二步**，避免在格式未验证前锁死语法。

### D2 练习判定语义（分两步）

- **B1（现在）**：判定 = kernel 通过（`ExerciseOpen` → 填洞 → `ExampleChecked`）。
- **B2（推荐下一步）**：练习定义带**期望目标类型**（与 `example` 的类型做 kernel conv 比对），失败分类：
  `unfinished`（还有洞）/ `type-mismatch`（kernel 拒绝或类型不符）/ `reduction-mismatch`（若练习要求 `#reduce` 到某值）。
- **B3（远期）**：多洞练习、按序填洞、goal-state 提示（对标 lean4game Hint）。

**推荐 B1→B2→B3**；B2 的判定全部走 kernel：目标类型用 `infer_closed_type` + `def_eq`，化简值用 `reduce_closed` + 结构相等。

### D3 错误分类与教学提示

- **A（现状）**：`Diagnostic{parse}` / `CompileError{elab|kernel}`，消息是字符串。
- **B（推荐）**：把 stage 之下再细分稳定的 `ErrorKind` 枚举（unknown-identifier、universe-arity、hole-not-allowed、kernel-rejected、kernel-internal…），每个 kind 一个教学提示模板（可放在 front 的 `hints.rs`），并保留原始 message 与 span。
- **C（远期）**：kernel 侧把 panic 断言换成显式 `KernelError` 树（含 conv 差异的两端表达式），教学前端可生成"左边类型 X、右边 Y"级反馈。

**推荐 B 现在做，C 单独立项**（kernel 改动风险高，不应阻塞教学层）。

### D4 增量与服务边界

- **A（现状）**：整文件/整 buffer 重编译；REPL 靠声明累积模拟增量。
- **B（推荐）**：L1 最小 service：监听文件、整文件重编译、广播与 `--json` 相同的事件；把"文件 + 事件游标"作为协议单元，editor/agent 各自消费。
- **C（远期）**：片段级增量（只重编改动片段，保持已检查定义）——以 decl 依赖图为基础。

**推荐 A→B（服务只做"整文件 + 事件流"），C 等 B 验证协议后再做**。这与 ROADMAP 的 L0→L1 分层一致，避免在 L0 阶段做重。

### D5 内容组织与课程管线

- **A（现状）**：examples/*.sokonanoda 随手放。
- **B（推荐）**：目录化 + 清单：
  ```text
  course/
    lesson-01-functions.sokonanoda   # 每文件 = 一单元（概念卡 + 示例 + 3–8 练习）
    lesson-02-naturals.sokonanoda
    ...
    course.json                      # 顺序、依赖、每单元练习数（供 UI/agent 用）
  ```
  每个文件既是人读教材，又是机器输入；新增课程 = 新增文件 + golden。
- **C（远期）**：worlds/levels 双级（对标 lean4game），world 依赖从示例解法推导。

**推荐 B**；C 等编辑器层（L2）再引入。

---

## 4. 推荐工作流（按依赖排序，标注谁先做）

| # | 工作流 | 内容 | 验收 |
|---|---|---|---|
| I0 | **地基（本轮已做一部分）** | docs 三件套 + `--json` 事件 + 错误 stage/code + 语料 CI + examples 语料测试 | `cargo test --workspace` 绿；`--json` 输出协议字段 |
| I1 | **练习引擎 v1（M2）** | `#exercise` 块头（D1-B）+ 期望目标类型 + `exercise.solved/failed` 事件 + 错误分类 kind + 提示模板 v1（D2-B2/D3-B） | 恒等函数练习：填对 → solved；填 `fun n => n+1` → type-mismatch + 模板提示 |
| I2 | **prelude 对齐与扩充** | Bool/Eq/`rfl` 所需结构、nat 运算符名字特判清单化；把"占位自引用体"改成显式受信任声明并写清理由 | `#check Eq.refl ...` 可用；新增 prelude 内容全部有测试 |
| I3 | **elaborator 推进** | binder 类型推断（先非依赖情形）→ `let` → 单构造子 `match`/递归（M4 课程第 5 单元前必须） | 每项 TDD：front 单测 + CLI e2e + 课程用例 |
| I4 | **第一门课（M4）** | 5 单元 × 3–8 练习，按 course/ 目录组织 + golden 事件 | CI 跑全部课程文件并比对 golden |
| I5 | **L1 最小服务** | 文件监听 + 整文件重编译 + 事件广播（复用 `--json` 序列化） | 三步模拟（agent 出题/用户作答/改题）事件流正确 |
| I6 | **kernel 显式错误（长期）** | panic → `KernelError`，conv 差异带两端项 | conv 失败可给出结构化差异 |
| I7 | **L2/L3（不做）** | VS Code 扩展与讲课 agent | —— |

本轮交付覆盖 I0 的大部分；I1–I5 已按 D1–D5 拆好、可直接开工。

---

## 5. 风险与对策（更新版）

| 风险 | 对策 |
|---|---|
| 练习格式设计返工 | D1 先做轻量 `#exercise` 头，期望字段分步加 |
| 教学前端阻塞在 kernel 错误改造 | D3：前端先用自己的 kind + 提示模板，kernel 改造单独立项 |
| 增量/服务在 L0 做过头 | D4：先整文件事件流，片段增量等协议验证 |
| prelude 占位体被误展开/不一致 | I2 明确"受信任声明清单"并加测试（含裸名 `#reduce`） |
| 课程语法超出白名单 | 每个语法点 = 课程 + 测试 + golden，CI 强制 examples 有效 |
| 事件只对人友好 | `--json` 与协议名一一对应，新增事件必须带 machine payload |
| "第二套 Lean" | 白名单 + 必须官方子集；新语法先写能过官方 Lean 的样例 |

---

## 6. 待确认清单（给用户的决策点）

1. 练习格式按 **D1-B**（`#exercise "名"` 头 + `example : T := ???`）推进吗？
2. 判定语义按 **D2：先期望目标类型（B2），化简值断言（B3）随后** 推进吗？
3. 错误分类按 **D3-B**（front 级稳定 kind + 提示模板），kernel panic→显式错误（D3-C）是否要单独立项？
4. 内容先做 **D5-B**（course/ 目录 + course.json），还是保持 examples/ 平铺？
5. 本轮已实现的地基（--json、stage/code、CI、docs）是否符合预期？哪一项要回退/改法？

（以上在默认模式下我按推荐方案推进了 I0；I1 起等你确认后再动手，避免在格式/语义未定前返工。）
