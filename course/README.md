# course/ —— 课程层（Roadmap I7 / M4）

> **定位（用户原则，2026-09-07）：课程层只是给大模型提供路线图；具体执行层
> 需要大模型适配各个用户、灵活调整。** 这些文件是 agent 的素材库与可解性守护
> （CI 验证解答钥匙），**不是**用户直接消费的固定课程；用户面对的是 agent
> 按其反馈历史动态维护的画布（如根 `playground.sokonanoda`）。适配规则见
> `docs/teaching-session.md` §0。

把 playground 第一课拆成十一个可独立编译的单元画布，并配上解答钥匙与 CI 守卫。

## 布局

> 单元顺序遵循**逻辑先行**哲学（用户原则，REQUIREMENTS §6）：单元①直接
> 证明命题；Sort 等"函数类型的类型是什么"这个问题自然出现时（单元⑤）
> 才揭晓。`by` 写法提前到单元④当"反馈加速器"（P2 已落地，见
> `docs/design/course-syllabus.md` §5.1）；归纳拆成单元⑥（Ⅰ：显式
> `inductive Nat`/手写 `Nat.rec`/递归 `match` 自动 IH）与单元⑦（Ⅱ：
> 参数化 `Option`/依赖 `match`=归纳/嵌套通配模式/带索引 `Vec`）。单元⑧
> （量词）是单元①的逻辑补充，可紧跟单元①教学。单元⑨（关系与联结词，
> P3 新增）把单元①的 `Or` 升级为真 `inductive`（自动派生 `Or.rec`）、用
> `def` 给出 `Iff`、并把 `Le`/`Even` 立为归纳关系（`Le.rec`/`Even.rec` 也
> 由前端自动派生）；单元⑩
> （读证明与综合，P3 新增）不教新语法，练自解释三问、formal↔informal
> 互译、评阅错证明与期末小项目。单元⑪（模块与项目，I16 新增）把编译单元
> 从「一个文件」升级为「入口文件 + import 闭包」：教 `import Foo.Bar` 置顶
> 规则、模块名↔路径（`-` 不是模块名字符）、`sokonanoda.toml` 项目根与零配置
> 退路、闭包级的重名/成环/依赖阻断/prelude 规则，并附一个可运行的
> `unit11-project/` 两文件项目。清单按教学顺序排列。
> 所有 `-- soko:hint` 只给触发条件与该用的引理/构造子名，**从不含完整答案**。

| 路径 | 面向 | 说明 |
|---|---|---|
| `unitN-*.sokonanoda` | 学习者 | 教学画布（中文）：`--` 讲解 + 已写好的演示 + 带 `sorry` 的练习。带洞是合法状态（`exercise.open`），逐声明容错。 |
| `en/unitN-*.sokonanoda` | 学习者 | 教学画布（英文镜像）：**与中文画布代码逐字节一致，仅 `--` 注释语言不同**；事件计数完全相同（CI 守卫）。 |
| `course.json` | agent/工具 | 有序课程清单：`{"file", "title", "title_en", "unit"}`，unit = 1..11。 |
| `solutions/unitN-*-solution.sokonanoda` | **agent 专用** | 解答钥匙（中文）：与对应画布一一对应，所有 `sorry` 已填入经完整内核验证的答案。**勿直接发给学习者**。 |
| `en/solutions/unitN-*-solution.sokonanoda` | **agent 专用** | 解答钥匙（英文镜像）：代码与中文钥匙一致，仅注释为英文。 |
| `unit11-project/` | 学习者 | 单元⑪的**可运行多文件项目**：`sokonanoda.toml` + 库模块 `Logic.sokonanoda` + 入口 `Canvas.sokonanoda` + 跨文件练习 `Exercises.sokonanoda`（答案在它的 `solutions/`）。它不在 `course.json` 里，也不进 golden 表——判卷命令写在单元⑪的 11.6 节。 |
| `shared/` | 维护者 + agent | **课程共享库子项目**（`sokonanoda.toml` + `Nat` 规范模块 + 自检入口 `Demo.sokonanoda`）。它不参与教学主线，作用是：① 让"课程内容 import 化"有个可运行样例（`scripts/soko grade course/shared/Demo.sokonanoda`）；② 给逐字重复的声明块一个**唯一规范副本**，由 `crates/cli/tests/course_shared.rs` 双向守住 **8** 份拷贝不漂移（2026-09-21 实测）。**`And`/`Or` 的规范模块已于 2026-09-21（C2.5）随自建骨架一起删除**——入门课现在直接用 prelude 的真归纳，没有副本可守。 |

## 双语布局（2026-09-09，用户要求）

中文为**权威源**（`course/` 顶层 + `solutions/`），英文镜像放 `course/en/`
（顶层画布 + `solutions/`），命名与中文文件一一对应。英文注释是**按语义
重构**的自然英文（用户原则，2026-09-09）：按英语语感重组句子与段落，不做
中文逐行镜像；但知识点、提示阶梯条数与顺序与中文同构。改课程时必须**双语
同步**：只改语言注释不改代码、只改代码不改注释都不行——`crates/cli/tests/
course.rs` 的镜像守卫会比较中英两版的**画布与 solution** 事件计数
（decl.checked / exercise.open / expr.reduced / expr.typed / diagnostic），
任何一侧漂移都会显红。设计见
`docs/design/course-bilingual.md`。

## 为什么画布不 import（以及 `shared/` 存在的理由）

单元画布**故意各自自给自足**：学习者打开一个文件就能看到全部前置声明，不用追
模块；golden 表、中英镜像契约、课程树与网站进度也都按"一单元一文件"钉着。把画布
改成 `import` 会同时打破这四件事，换来的只有约 **6%** 的行数（2026-09-21 实测：
44 个教学文件共 4408 行，其中 288 行落在逐字重复的声明块里）——不划算。

> 这两个数会随课程增删而变，**别在别处再抄一份**（它们已经陈过一次）。重量办法：
> 用 `course_shared.rs` 的同一口径——去掉 `--` 注释行与空行后做**子串包含**判定，
> 数有多少文件含规范文本（那份表就在该测试里，改课程就同步它）；行数 `wc -l` 那
> 44 个文件即可。

但复制粘贴有另一半成本：**改一处漏一处没有任何测试能发现**（事件计数只看得见
语义变化，改个 binder 风格照样全绿）。所以：

* 逐字重复的块在 `shared/` 里各有一份**规范副本**（2026-09-21 C2.5 之后只剩显式
  `Nat` 块 **8** 份，含中英与解答钥匙）；`And`/`Or` 那两条线随自建骨架退役；
* `crates/cli/tests/course_shared.rs` 双向检查：规范文本变了而某份拷贝没跟上 ⇒ 红；
  某个文件抄了这段却没登记 ⇒ 也红（语料含 `unit11-project/`——2026-09-21 补，
  之前它不在扫描面里，是守卫空洞）；
* 画布里出现 `import` 同样红（挡住"顺手改成 import 库"的回归；`unit11-project/`
  是有意的例外，它**就是** import 教学样例）；
* `Demo.sokonanoda` 是这套共享库的自检入口（跨模块递归 + `#reduce`，另两条
  `And`/`Or` 演示改用 prelude 的真归纳），判卷：
  `scripts/soko grade course/shared/Demo.sokonanoda`。

## 约定

* 十一个单元文件风格与根 `playground.sokonanoda` 一致：`--` 讲解（中文在顶层、英文在 `en/`）、演示已填、练习留 `sorry`。
* 每个单元至少一条 `#reduce` 自测（`#` 命令在课程文件里合法），保证 `expr.reduced` 事件可被 golden 测试观测；单元⑩/⑪ 不教新语法、也不引入 `#` 命令，所以它们的 `expr.reduced` 是 0（golden 表如实记录）。
* 每个单元文件各自带所需声明、独立编译：`True`/`False`（以及单元⑧ 的 `Exists`）
  仍自带 `axiom`，**`And`/`Or`/`Not`/`Iff` 自 2026-09-21（C2.5）起一律用 prelude 的真归纳**
  （单元① 用 `True`/`False` 讲 `axiom` 是什么）；unit6/unit7 的显式 `inductive Nat`
  块会取代该文件内的 prelude Nat。
* 单元⑪ 的画布自身仍是单文件（课程单元就是一个文件）；它的**跨文件**练习放在 `unit11-project/Exercises.sokonanoda`，那里第一行就是 `import Logic`。
* 依赖洞的 `#reduce`（如 unit6 的 `#reduce add two two`）在画布里注释着，解出后放开；solution 文件里保持放开并带核对值。
* 事件词汇见 `docs/protocol.md`；教学决策表见 `docs/teaching-session.md`。

## CI 守卫

`crates/cli/tests/course.rs`：

1. `course/*.sokonanoda` 全部可编译（exit 0，stderr 无 `error[`）；
2. 每单元的 `decl.checked` / `exercise.open` / `expr.reduced` 事件数与 golden 表精确一致；
3. `solutions/*.sokonanoda` 全部 0 诊断、0 个 `exercise.open`（课程可解性证明），
   且画布练习名都能在 solution 里找到同名声明（`solution_covers_every_canvas_exercise`）；
4. `course.json` 恰好按序列出这 11 个文件、unit = 1..11；
5. `course/en/` 镜像与中文画布、`course/en/solutions/` 与中文钥匙的事件计数逐项相等。
