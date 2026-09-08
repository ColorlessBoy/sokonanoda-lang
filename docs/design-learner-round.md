# 设计：学习者反馈轮（2026-09-07，第一堂真实课的验收反馈）

> 来源：第一次真实教学会话中学习者的直接反馈（验收 = 学习者本人）。
> 每条：问题 → 方案 → 验收。

## F1 去掉 sorry，sorry 成为唯一占位符

**问题**：`sorry` 与 `sorry` 双轨（初版设计的历史残留）。Lean 4 只用 `sorry`；
双轨让学习者多记一个符号，且 `sorry` 无处不在造成文案噪音。

**方案**：
- lexer：`sorry` 不再产出 Hole token——遇到 `?` 报**教学错误**：
  「??? 已移除：未完成的证明请写 sorry（与官方 Lean 一致）」；
- `sorry`（表达式位置的标识符）→ Hole，既有机制全复用（open 状态/
  部分作答/构造子多洞/跳洞/hover）；
- 全内容清扫：playground、course/、examples/、测试、文档——sorry → sorry；
- golden/protocol 契约同步（事件不变，span 可能因长度变化平移）。

**验收**：`grep -rn 'sorry' playground course examples` 零命中（讲解文字除外，
如"??? 已移除"的错误提示本身）；全部测试绿。

## F2 hover 显示「表达式 : 类型」（箭头优先级可视化）

**问题**：hover 在 `->` 上只显示类型（如 `Prop`），学习者想要
「对应的表达式以及它的类型」——先结合的是哪一段、它的类型是什么。

**方案**：hover 类型行改为 `{源码切片} : {类型}`——hover 行的 span 恰好
就是子表达式的源码范围，切片即得。例：
`axiom Or.inr : (a : Prop) -> (b : Prop) -> b -> Or a b`
- 第一个 `->` → `(a : Prop) -> (b : Prop) -> b -> Or a b : Prop`
- 第二个 `->` → `(b : Prop) -> b -> Or a b : Prop`
- 第三个 `->` → `b -> Or a b : Prop`

**验收**：LSP 探针逐 `->` 验证三行输出。

## F3 声明名 hover 显示完整签名

**问题**：hover 在声明名/axiom 名上显示「axiom And.intro — 已通过内核
检查」，不如显示完整类型签名。

**方案**：`DeclState` 增加 `ty_text`（内核渲染的声明类型）：
- 已检查声明：`declar.info().ty` 经 pp 渲染；
- 开放练习：elaborate 类型后的渲染（hover 管线已有）；
- 声明 hover 重写为：第一行签名（`axiom And.intro : forall (a : Prop), …`）、
  第二行状态。

**验收**：hover `And.intro` 名字 → 显示完整签名。

## F4 点分名字导航核实

**问题**：`And.intro` 里点 `And` 与 `intro` 分别有下划线、都跳到
`And.intro` 行；学习者期望 `And` 跳到 `And`、`intro` 跳到 `And.intro`。

**方案**：核实（a) tokenizer 将 `And.intro` 作为**单个**标识符（点为
续字符）——下划线应为整名一个 span，若出现两段则为 bug；(b) 语义上
点分名是**原子名**（与官方 Lean 一致：And.intro 是一个常量，整体跳转）。
`And` 部分的"跳到 And"不是 Lean 的行为，不做；但要在探针里确认 span
原子性，若 tokenizer 确实分裂则修。

## F5 解析错误结论（无 bug）

学习者的多 binder lambda（`fun (a : Prop) (b : Prop) (ha : a) (hb : b) => …`）
**语法支持且内核通过**；当时报的 177:1 parse 错误是判卷时文件处于编辑
中间态。教训：判卷前确认文件保存稳定（watch 流天然解决）。
固化：多 binder lambda 加测试防回归。

## 文风与措辞（延续 zh-style）

- hover 措辞新增纪律：「输入/输出」只配函数；类型只说"……的类型"。
- 文案禁比喻堆叠：术语先行（已入 zh-style §四.1/4b）。

## 实施顺序

F5 结论 → F1（最大机械面）→ F2/F3（LSP）→ F4（探针）→ 文档同步 → 全绿。
