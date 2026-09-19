# 卷 I《集合论》（course: set-theory）

> 教学项目「从集合论到分析」的第一卷，建在语言仓内（`courses/set-theory/`）。
> 总体计划：`docs/design/teaching-project.md`；大纲：`docs/design/set-theory-syllabus.md`；
> 分层判据（L1/L2/L3）：`docs/design/course-stdlib.md`；缺口台账：`docs/gaps/`。
>
> 与入门课（`course/`，11 单元）的关系：本卷**假设读者已学完入门课**（逻辑、`by`、
> 归纳、量词），直接从"集合 = 谓词"开始。

## 怎么判卷（一条命令）

```bash
python3 courses/set-theory/tools/check.py             # 判据 G1–G5 + 人读表 + 汇总
python3 courses/set-theory/tools/check.py --selftest  # 判据通道自检（故意坏文件必须被拒）
python3 courses/set-theory/tools/check.py --json      # 机器可读（含计数；CI 用 --report 落盘）
python3 courses/set-theory/tools/check.py --only "单元 5" --bisect   # 二分到第一个判红的声明
```

**判据只有五条，全部与课程规模无关**（设计 `docs/design/course-gate-in-ci.md` §3）：

| # | 判据 |
|---|---|
| G1 | 每个目标 `grade` 退出码 0（只认退出码，不认 `query check`——台账 G-10） |
| G2 | 目标存在：`course.json` 条目 / `lib/` / 每个画布的解答文件 |
| G3 | 解答：`exercise.open == 0` 且 `decl.checked > 0`（退出码对合法 open 返回 0，只靠 G1 抓不到没填的洞） |
| G4 | 解答覆盖画布每个具名练习（防"删题代替填洞"） |
| G5 | `lib` + Demo：`exercise.open == 0`（库里有 `sorry` 会让引用它的单元判卷失真） |

计数（checked / open / diagnostics）**只进报告与台账**，从不参与判红：课程还在长，
锁死 `checked == N` 会让门禁从质量闸退化成记账本。台账用
`--ledger [路径]` 追加一条 JSONL（默认 `docs/courses/ledger.jsonl`，只在该跑的时候手动跑）。

退出码：**0** 全绿 / **1** 有目标被判负 / **2** 前置缺失（判卷二进制与仓库版本不一致、
`--only` 没匹配到目标等——**无法判定 ≠ 绿**）。

门禁内部用 `scripts/soko grade <绝对路径>`——**有意一律绝对路径**：台账 G-12 **已在 0.59.0 修掉**
（模块根绝对化 + 空 parent 护栏；相对路径现在也判绿，实测
`node scripts/soko grade courses/set-theory/units/unit02-subsets-empty.sokonanoda` = exit 0），
门禁仍用绝对路径是因为 `--bisect` 的前缀文件必须落在目标同目录、且绝对路径让失败输出无歧义。
**判据只看 `grade` 的退出码**
（G-10 已修（≥0.59.0）：`query check` 现在同样带 parse 诊断并 exit 1，可作交叉复核）。
判负时输出三段式：权威段（标签 + 绝对路径 + 退出码）、参考段（诊断原样，**span 只作参考**）、
定位段（`--only "<标签>" --bisect`——按顶层声明边界二分，结论不依赖诊断 span，台账 G-15）。

本地与 CI 跑的是同一条命令：`scripts/soko gate`（贡献者门禁 = cargo 门禁 + 课程门禁；
探不到 python3 时**直接 exit 3**，不静默跳过）与 `.github/workflows/ci.yml` 的 `test` job
（`SOKONANODA_BIN` 指当轮 `target/debug/sokonanoda`，`timeout-minutes: 5`，失败打注解 +
step summary，`--report` 进 artifact）。

单个文件判卷：

```bash
node scripts/soko grade "$PWD/courses/set-theory/units/unit01-sets-membership.sokonanoda"
node scripts/soko query state --file "$PWD/courses/set-theory/units/unit01-sets-membership.sokonanoda" --line 40 --col 3
```

## 目录

| 路径 | 作用 |
|---|---|
| `lib/` | **课程标准库**（L2 层）：集合的词汇 + 定义展开引理；名字用 **Loogle 取证版**（`Set.notMem_empty`、`Set.mem_powerset_iff`、`Set.mem_sdiff`、`Set.mem_empty_iff_false`…）；`Exists` 已是真归纳（G-03/0.59.0） |
| `units/` | 单元画布（演示 + 练习，练习 = 带 `sorry` 的声明） |
| `units/notation-cheatsheet.sokonanoda` | **记法对照页**（大纲 §4 的"第二遍"，G-04）：同一个命题的**点名形式 ↔ 数学记法**（`∈`/`⊆`/`∅`/`∪`）并排演示 + 3 道记法练习。**不是单元**（不进 `course.json`），但**是画布**：门禁按同一套 G1/G3/G4 判它（`页面 …` 两行） |
| `units/solutions/` | 解答钥匙（agent 专用；画布不 import 它） |
| `gaps/` | 本卷撞到的**新**缺口（收编进 `docs/gaps/ledger.jsonl`） |
| `tools/check.py` | 课程门禁（判据 G1–G5，与规模无关；`--selftest` / `--bisect` / `--json`） |
| `course.json` | 单元清单（顺序 + 标题） |

## 现状（2026-09-19，12 单元全部落地）

**门禁实测（2026-09-19，0.59.0 二进制，P4 之后）：36 个目标 · 329 checked · 99 open · 0 判负**
（`python3 courses/set-theory/tools/check.py --json`：`canvas_open` 96 / `solutions_open` 0 /
`lib_open` 0；P4 前是 355 checked —— 差额 = `lib/Logic` 空壳化少掉的 26 条声明，**open 不变**）。
数字只作现状记录——门禁本身不锁计数（课程还在长），所以这份表随每次重算更新，
**不手写旧数字**。

| 单元 | 标题 | 练习 | 解答（checked） |
|---|---|---|---|
| 1 | 集合与隶属 | 6 | 6 |
| 2 | 子集、空集与包含三律 | 10 | 10 |
| 3 | 并、交、差与幂集 | 11 | 11 |
| 4 | 外延性与集合等式 | 8 | 8 |
| 5 | 序对与笛卡尔积 | 7 | 8 |
| 6 | 关系 | 8 | 23 |
| 7 | 函数 | 9 | 9 |
| 8 | 像与原像 | 9 | 30 |
| 9 | 等势（基数 Ⅰ） | 7 | 13 |
| 10 | 可数与 Cantor 定理（基数 Ⅱ） | 7 | 9 |
| 11 | 论域与 Russell 悖论 | 5 | 5 |
| 12 | 综合与读证明 | 9 | 9 |

> 「练习」= 画布上还留着 `sorry` 的声明数；「解答（checked）」= 解答文件里
> `decl.checked` 的条数——两者**不必相等**（解答可以多证几个画布上的演示定义）。
> 记法对照页（`units/notation-cheatsheet.sokonanoda` + 它的解答）也进同一套判据：
> **18 checked + 3 练习 / 解答 21 checked**。

`lib/` 共 9 个文件：8 个模块 + 自检入口 `Demo`。门禁实测（0.59.0，P4 之后）：
**Logic 0（空壳）** · Set 23 · Exists 3 · Prod 6 · Rel 7 · Fun 14 · Image 6 · Equiv 5 · Demo 10。
**口径**：这一行**不重复计** `Demo`；门禁 `--json` 的 `summary.checked` 把 `Demo` 算两次
（`lib Demo` 与 `lib 自检` 判同一个文件），所以汇总额比逐行相加多 10。
分层与名字纪律见 `docs/design/course-stdlib.md`。

**`lib/Logic` 已退化成空壳（P4，2026-09-19，台账 L-01/L-02 的收尾）**：文件里**一条声明都没有**，
只剩注释——30 个名字现在由 **prelude** 自带（`True`/`True.intro`/`False`/`False.rec`/`False.elim`、
`And.*`、`Or.*`（构造子是**点号名** `Or.inl`/`Or.inr`）、`Not.*`/`absurd`、`Iff.*`、
`Eq.symm`/`Eq.trans`/`congrArg`）。34 个 `import lib.Logic` **一字未改**（空模块可 import）；
课程侧 65 处项位裸名 `inl`/`inr` 改成点号名。**别往这里加声明**——它是入口，不是库；
prelude 的签名才是唯一真相（`crates/front/src/compile/prelude.rs` 的 `PRELUDE_L1_SRC`）。

**`lib/Exists` 已升级（G-03 / 0.59.0，台账 `fixed_in`）**：从公理三件套
（`axiom Exists` / `Exists.intro` / `Exists.elim`）换成**真归纳**——
`Exists.intro` 是归纳块的构造子（规范名 `Exists.intro`，G-02 起的命名空间）、
`Exists.elim` 由自动派生的 `Exists.rec` **定义**出来（不是第二条公理）。
名字与签名**逐字不变** ⇒ `units/` 里 186 处点名调用（`Exists.intro A p w hw` /
`Exists.elim A p Q h f`）零改动。**"大消去"仍不可用，而且这是正确的行为**
（`Exists.rec` 的 motive 只能落 `Prop`，要取数据的引理得走数据版，台账 L-06）——
细节见文件头，别试图绕过。

## 单元的定义完成（DoD）

1. 大纲条目（单元号 / 靶子 / 先修 / 练习类型配额）；
2. 画布：演示 + 练习，`-- soko:hint` 三段（思路 / 目标形态 / 关键件，**不写答案**）；
3. 解答：洞全填、0 判负；
4. `course.json` 加一行；
5. `python3 courses/set-theory/tools/check.py` 全绿（连同 `--selftest`；判据 G1–G5）；
6. 撞到的缺口登记（`gaps/` → `docs/gaps/`）；
7. 引用的教学主张要么给出处、要么标"设计判断"（见大纲 §1.4 引用纪律）。
