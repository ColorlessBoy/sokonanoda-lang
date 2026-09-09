# course/ —— 课程层（Roadmap I7 / M4）

> **定位（用户原则，2026-09-07）：课程层只是给大模型提供路线图；具体执行层
> 需要大模型适配各个用户、灵活调整。** 这些文件是 agent 的素材库与可解性守护
> （CI 验证解答钥匙），**不是**用户直接消费的固定课程；用户面对的是 agent
> 按其反馈历史动态维护的画布（如根 `playground.sokonanoda`）。适配规则见
> `docs/teaching-session.md` §0。

把 playground 第一课拆成五个可独立编译的单元画布，并配上解答钥匙与 CI 守卫。

## 布局

> 单元顺序遵循**逻辑先行**哲学（用户原则，REQUIREMENTS §6）：单元①直接
> 证明命题；Sort 等"函数类型的类型是什么"这个问题自然出现时（单元④）
> 才揭晓。题池不变，编排跟着直觉走。

| 路径 | 面向 | 说明 |
|---|---|---|
| `unitN-*.sokonanoda` | 学习者 | 教学画布（中文）：`--` 讲解 + 已写好的演示 + 带 `sorry` 的练习。带洞是合法状态（`exercise.open`），逐声明容错。 |
| `en/unitN-*.sokonanoda` | 学习者 | 教学画布（英文镜像）：**与中文画布代码逐字节一致，仅 `--` 注释语言不同**；事件计数完全相同（CI 守卫）。 |
| `course.json` | agent/工具 | 有序课程清单：`{"file", "title", "title_en", "unit"}`，unit = 1..6。 |
| `solutions/unitN-*-solution.sokonanoda` | **agent 专用** | 解答钥匙（中文）：与对应画布一一对应，所有 `sorry` 已填入经完整内核验证的答案。**勿直接发给学习者**。 |
| `en/solutions/unitN-*-solution.sokonanoda` | **agent 专用** | 解答钥匙（英文镜像）：代码与中文钥匙一致，仅注释为英文。 |

## 双语布局（2026-09-09，用户要求）

中文为**权威源**（`course/` 顶层 + `solutions/`），英文镜像放 `course/en/`
（顶层画布 + `solutions/`），命名与中文文件一一对应。英文注释是**按语义
重构**的自然英文（用户原则，2026-09-09）：按英语语感重组句子与段落，不做
中文逐行镜像；但知识点、提示阶梯条数与顺序与中文同构。改课程时必须**双语
同步**：只改语言注释不改代码、只改代码不改注释都不行——`crates/cli/tests/
course.rs` 的镜像守卫会比较中英两版的事件计数（decl.checked / exercise.open
/ expr.reduced / diagnostic），任何一侧漂移都会显红。设计见
`docs/design-course-bilingual.md`。

## 约定

* 六个单元文件风格与根 `playground.sokonanoda` 一致：`--` 讲解（中文在顶层、英文在 `en/`）、演示已填、练习留 `sorry`。
* 每个单元至少一条 `#reduce` 自测（`#` 命令在课程文件里合法），保证 `expr.reduced` 事件可被 golden 测试观测。
* 每个单元文件各自带所需 axiom/inductive 块，独立编译（unit5 的显式 `inductive Nat` 块会取代该文件内的 prelude Nat）。
* 依赖洞的 `#reduce`（如 unit5 的 `#reduce add two two`）在画布里注释着，解出后放开；solution 文件里保持放开并带核对值。
* 事件词汇见 `docs/protocol.md`；教学决策表见 `docs/teaching-session.md`。

## CI 守卫

`crates/cli/tests/course.rs`：

1. `course/*.sokonanoda` 全部可编译（exit 0，stderr 无 `error[`）；
2. 每单元的 `decl.checked` / `exercise.open` / `expr.reduced` 事件数与 golden 表精确一致；
3. `solutions/*.sokonanoda` 全部 0 诊断、0 个 `exercise.open`（课程可解性证明）；
4. `course.json` 恰好按序列出这 5 个文件、unit = 1..5。
