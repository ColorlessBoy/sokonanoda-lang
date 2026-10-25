# 卷 I《集合论》

> 「从集合论到分析」教学项目的第一卷。读者应已学完入门课（`course/`：逻辑连接词、
> `by`、归纳、量词）；本卷从「集合 = 谓词」开始，不预设集合论。
>
> 教材主线是 Enderton 的 *Elements of Set Theory* 与 Halmos 的 *Naive Set Theory*。
> 课程按两书的章节组织，但不逐题照搬：证明按本语言的能力重写，超出当前内核的地方
> 在单元开头或 hint 中标注。

## 这门课

本卷共 13 章、112 个单元，另有一页记法对照。每个单元是一份可以判卷的画布：先给几条
演示，再给一组留 `sorry` 的练习；配套解答把每个洞都填上，并随课程门禁一起校验。

课程把集合论当作证明的训练场，而不是公理清单。前四章建立集合语言——隶属、子集、
空集、并交差、补集、幂集、外延性，然后是有序对、关系、函数、像与原像、等势与
Cantor。第五至七章进入序关系、序数与选择公理；第八至十章处理基数算术、集族与
广义积、关系闭包与商；最后三章用代数视角重述集合与关系：集合代数、幂集与函数空间、
关系代数。

默认的工作逻辑是构造性的。排中律与经典推论作为一条独立的线，从单元 104 起引入；
凡用到经典假设的命题，单元里会明确标出。课程语法是 Lean 4 的一个子集，判卷由本仓库
的内核完成，不需要安装官方 Lean 工具链。

## 课程地图

| 章 | 标题 | 先修 | 单元 | 内容 | 教材 |
|---|---|---|---|---|---|
| I.1 | 集合、子集与集合运算 | — | 15（1–4 · 58–66 · 104–105） | 隶属、子集、空集、并交差、补集、幂集、外延性；经典逻辑入门 | Enderton §2.3 · Halmos §1–5 |
| I.2 | 序对、关系与函数 | I.1 | 4（5–7 · 75） | 有序对、笛卡尔积、关系、函数 | Enderton §3.1–3.4 · Halmos §6–10 |
| I.3 | 像、原像与基数 | I.2 | 16（8–10 · 73–74 · 92–93 · 96–103 · 112） | 像与原像、等势、势的比较、Cantor 定理、Cantor–Bernstein | Enderton §6.1/6.6 · Halmos §22–23 |
| I.4 | 论域、悖论与综合 | I.1 · I.3 | 12（11–12 · 55–57 · 87–91 · 94–95） | 论域、Russell 悖论、反例与不可证、综合读证明、单射与满射 | Enderton §1.2–1.5 · Halmos §1/§8 |
| I.5 | 序关系与良序 | I.2 | 4（13–14 · 53 · 67） | 偏序、全序、严格序、良序与良基、序嵌入 | Enderton §3.7 + §7.1–7.2 · Halmos §14/17 |
| I.6 | 序数 | I.5 | 13（15–16 · 42 · 80–83 · 106–111） | 传递集、序数、后继、∈-归纳、三歧、良基递归、ℕ 的算术 | Enderton §4.3 + §7.3–7.6 · Halmos §11/18–20 |
| I.7 | 选择公理与 ZF 公理体系 | I.2 | 7（17–18 · 48–49 · 69 · 71–72） | 选择公理、选择函数、ZF 公理、正则性、覆盖与广义并、伴随刻画 | Enderton §2.1 + §6.5 + §7.3 · Halmos §2/15 |
| I.8 | 基数与基数算术 | I.3 | 4（19–20 · 54 · 113） | 基数的比较与算术、等势代数、连续统假设 | Enderton §6.3–6.4 · Halmos §22/24 |
| I.9 | 集族、广义积与 n 元关系 | I.2 · I.5 | 3（21–23） | 集族、广义并交、索引积、柯里化 | Enderton §2.2 + §3.3 + §3.5 · Halmos §9 |
| I.10 | 关系闭包与等价关系 | I.2 | 12（24–27 · 43 · 76–79 · 84–86） | 关系闭包、等价关系、划分、商类型与良定义性 | Enderton §3.5 · Halmos §7/10/18 + 内核 Quot |
| I.11 | 集合代数 | I.1 | 7（28–32 · 46 · 50） | 分配律与 De Morgan、上下确界、格与对偶、单调与序同构、逐点序 | Halmos §5–6/14 · Enderton §1.8/2.2/7.1 |
| I.12 | 幂集与函数空间 | I.1 | 7（33–35 · 44 · 51 · 68 · 70） | 幂集代数、特征函数、函数合成、单满双射、逆函数、函数空间 | Halmos §5–6/8 · Enderton §1.8/2.2 |
| I.13 | 关系的代数 | I.2 | 8（36–41 · 45 · 52） | 关系代数、复合与逆、单位元、传递闭包、良基、函数的图 | Halmos §7/10/14 · Enderton §3.3/3.6/7.1 |

单元编号是写作顺序，不是章内序号，所以章内会跳号（例如 I.1 含 1–4、58–66、
104–105）。编号 1–113 中缺 47，因此单元总数是 112。单元清单的机器可读权威是
`course.json`；章级交付状态与边界见 `ENGINEERING.md`。

## 学习路径

| 阶段 | 章 | 主题 | 建议 |
|---|---|---|---|
| 基础 | I.1–I.4 | 集合语言、关系与函数、像与基数、论域与反例 | 逐单元做，每题都动手 |
| 序 | I.5–I.7 | 序关系、序数、选择公理与 ZF | 抽象度上升，先看清定义再证 |
| 基数与商 | I.8–I.10 | 基数算术、集族与广义积、闭包与商 | 证明开始变长，主动拆辅助引理 |
| 代数 | I.11–I.13 | 集合代数、幂集与函数空间、关系代数 | 旗舰内容密集，适合综合复习 |

## 怎么学一个单元

1. **打开画布**：`units/<章 id>/unitNN-<slug>.sokonanoda`。先读开头「本单元只允许
   用什么」；每道练习上方有三段 `-- soko:hint`：思路、目标形态、关键件。hint 不含答案。
2. **自己填 `sorry`**。在 VS Code 里装本仓库的扩展，可以直接看到当前目标与假设；
   在命令行里，`scripts/soko query state` 能查某一行的状态。
3. **卡住时先看 hint，再看解答**：`units/solutions/<章 id>/unitNN-solution.sokonanoda`。
   解答是项风格，刻意写得短；拿它的关键件，再用 tactic 写法自己重做一遍。
4. **自检**。画布上的 `sorry` 是合法状态，所以 `grade` 不会因为没做完而判负；
   要看还剩几题，读 `query check` 的 `exercise_open`：

```bash
# 画布：counts.exercise_open = 还留着的 sorry 数
scripts/soko query check --file "$PWD/courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda"

# 解答：exercise_open 应为 0，decl_checked 应大于 0
scripts/soko query check --file "$PWD/courses/set-theory/units/solutions/I.1/unit01-solution.sokonanoda"

# 单文件诊断：退出码 0 = 没有解析/判定错误
scripts/soko grade "$PWD/courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda"
```

整门课自检用 `python3 courses/set-theory/tools/check.py`（较慢，日常做单元用上面的
单文件命令即可）。如果 PATH 上已经有 `sokonanoda`，把 `scripts/soko` 换成它。

## 记法与标准库

- **记法对照页**：`units/notation-cheatsheet.sokonanoda` 把同一个命题的记法版与
  点名版并排写一遍，附十对演示和三道练习。它是「第二遍」的教学装置，不属任何单元。
- **课程标准库**：`lib/` 是 L2 层，放集合论的词汇与定义展开引理（`Set`、`Rel`、
  `Fun`、`Order`、`Ordinal`、`ZF`、`Choice`、`Cardinal`、`Equiv`、`Image` 等）。
  有数学内容的陈述留在单元的 L3 练习里；分层规则见 `docs/design/course-stdlib.md`。
- **记法规则**：课程正文一律写 Lean 4 记法（`a ∈ A`、`A ⊆ B`、`f '' A`……）。
  点名的等价写法只在记法对照页、以及确实需要点名的边界处出现。写作规则与已知边界
  见 `AGENTS.md`。

## 目录

| 路径 | 内容 |
|---|---|
| `lib/` | L2 课程标准库：集合、关系、函数、序、序数、选择、基数等模块，外加自检入口 `Demo` |
| `units/<章 id>/` | 单元画布：演示 + 练习（练习是带 `sorry` 的声明） |
| `units/solutions/<章 id>/` | 同题解答，项风格；画布不 import 它 |
| `units/notation-cheatsheet.sokonanoda` | 记法对照页（不属任何单元，但同样有画布与解答） |
| `course.json` | 单元清单 v2（卷 → 章 → 单元，含先修、标签、配额） |
| `gaps/` | 本卷撞到的语言缺口复现件 |
| `tools/` | 课程门禁、清单单测、画布/解答对账、库闭包检查 |
| `OPEN-ITEMS.md` | 课程侧未结项 |
| `ENGINEERING.md` | 工程状态：判据、读数、章级交付、CI 接线 |

## 参考教材

- Herbert B. Enderton, *Elements of Set Theory*, Academic Press, 1977.
- Paul R. Halmos, *Naive Set Theory*, Van Nostrand, 1960.

章表里的节次只是导航，不表示逐节覆盖；每章的靶子与「必证 / 必破」见
`docs/design/set-theory-syllabus.md`。

## 工程与状态

教材面到此为止。课程同时是语言内核的主要语料，工程事实不写在这个文件里：

- `ENGINEERING.md`：判据 G1–G7、怎么跑课程门禁、最近有记录的读数、章级交付状态、
  CI 接线。
- `OPEN-ITEMS.md`：课程侧未结项（固定格式，可直接接手）。
- `gaps/` 与 `docs/gaps/ledger.jsonl`：本卷撞到的语言缺口与最小复现。
- `docs/design/set-theory-syllabus.md`：大纲与章级靶子；`docs/design/course-stdlib.md`：
  L1/L2/L3 的分界。

旧 README 里的逐轮门禁流水与逐单元 as-built（2026-09-19 至 2026-10-08）不再维护；
原文用 `git log --all -- courses/set-theory/README.md` 取。
