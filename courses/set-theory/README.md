# 卷 I《集合论》（course: set-theory）

> 教学项目「从集合论到分析」的第一卷，建在语言仓内（`courses/set-theory/`）。
> 总体计划：`docs/design/teaching-project.md`；大纲：`docs/design/set-theory-syllabus.md`；
> 分层判据（L1/L2/L3）：`docs/design/course-stdlib.md`；缺口台账：`docs/gaps/`。
>
> 与入门课（`course/`，11 单元）的关系：本卷**假设读者已学完入门课**（逻辑、`by`、
> 归纳、量词），直接从"集合 = 谓词"开始。

## 怎么判卷（一条命令）

```bash
python3 courses/set-theory/tools/check.py             # 判据 G1–G6 + 人读表 + 汇总
python3 courses/set-theory/tools/check.py --selftest  # 判据通道自检（故意坏文件必须被拒 + G6 清单自检）
python3 courses/set-theory/tools/check.py --json      # 机器可读（含计数；CI 用 --report 落盘）
python3 courses/set-theory/tools/check.py --only "单元 5" --bisect   # 二分到第一个判红的声明
python3 courses/set-theory/tools/test_manifest_v2.py  # 清单 v2 / G6 的单测（秒级、零工具链）
```

**判红只有六条，全部与课程规模无关**（设计 `docs/design/course-gate-in-ci.md` §3、
`docs/design/course-manifest-v2.md` §4.2）：

| # | 判据 |
|---|---|
| G1 | 每个目标 `grade` 退出码 0（只认退出码，不认 `query check`——台账 G-10） |
| G2 | 目标存在：`course.json` 条目 / `lib/` / 每个画布的解答文件 |
| G3 | 解答：`exercise.open == 0` 且 `decl.checked > 0`（退出码对合法 open 返回 0，只靠 G1 抓不到没填的洞） |
| G4 | 解答覆盖画布每个具名练习（防"删题代替填洞"） |
| G5 | `lib` + Demo：`exercise.open == 0`（库里有 `sorry` 会让引用它的单元判卷失真） |
| G6 | **清单自洽**（v2）：volume/chapter id 唯一且非空、每个 unit 恰好属于一个 chapter、`prereqs` 指向存在的 chapter id |

计数（checked / open / diagnostics）与**配额差额**只进报告与台账，从不参与判红：
课程还在长，锁死 `checked == N`（或 `quota.exercises == N`）会让门禁从质量闸退化成
记账本。台账用 `--ledger [路径]` 追加一条 JSONL（默认 `docs/courses/ledger.jsonl`，
只在该跑的时候手动跑）。

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
| `lib/Set.sokonanoda` **末尾的记法块** | 卷 I 的五个**集合论专用**数学符号（G-04 第二刀，0.60.0）：`𝒫`（`prefix:100`）/ `ᶜ`（`postfix:100`）/ `''`（`infixr:80`）/ `⁻¹'`（`infixr:80`）/ `×ˢ`（`infixr:80`）。**记法不是声明**（零事件、不进声明表），所以这一段**不改变任何计数**；它随 `import lib.Set` 传播到每个单元（跨 `import` 的记法，见下） |
| `units/solutions/` | 解答钥匙（agent 专用；画布不 import 它） |
| `gaps/` | 本卷撞到的**新**缺口（收编进 `docs/gaps/ledger.jsonl`） |
| `tools/check.py` | 课程门禁（判据 G1–G6，与规模无关；`--selftest` / `--bisect` / `--json`） |
| `tools/test_manifest_v2.py` | 清单 v2 展平 + G6 的单测（`check.py` 的纯清单面，零工具链） |
| `course.json` | **单元清单 v2**（`schema: soko.course/2`：卷 → 章 → 单元 + 先修/标签/配额） |

## 清单 v2（`course.json`，台账 G-07）

清单从 v1 扁平数组升成 **`soko.course/2`** 结构化对象（设计
`docs/design/course-manifest-v2.md`）。**v1 仍然合法**（入门课 `course/course.json`
就是 v1，一个字节都没改），所有消费者两种都读：

```jsonc
{ "schema": "soko.course/2", "name": "set-theory", "title": "卷 I《集合论》",
  "volumes": [ { "id": "I", "title": "卷 I 集合论", "chapters": [
    { "id": "I.1", "title": "集合、子集与集合运算",
      "prereqs": [], "tags": ["membership", "subset", "powerset"],
      "quota": { "exercises": 35 },
      "units": [ { "file": "units/unit01-….sokonanoda", "title": "单元① 集合与隶属",
                   "title_en": "Unit 1 — Sets & Membership", "unit": 1 }, … ] }, … ] } ] }
```

| 字段 | 语义 | 判红？ |
|---|---|---|
| `schema` | 格式标签 `soko.course/2`；**其它值直接拒读**（不猜），没有 `schema` 的对象按 v2 读 | 未知 schema ⇒ exit 非 0 |
| `volumes[].id` / `chapters[].id` | 卷 id / 章 id，课程内唯一且非空 | **G6** |
| `chapters[].prereqs` | 先修 **chapter id** 列表（可空；允许前向引用） | **G6**：指向不存在的章 |
| `chapters[].tags` | 主题标签（自由字符串） | 否 |
| `chapters[].quota.exercises` | **计划**练习数 | **否，永不**：差额只报告 |
| `chapters[].units[]` | 与 v1 条目**同形**（`file`/`title`/`title_en`/`unit`） | 是（沿用 G2：缺 `file`/`unit` 判负） |

一条纪律：**一个 unit 恰好属于一个 chapter**（同一 `file` 出现两次由 G6 判红）。
机器读到的结构会出现在三处：CLI `course.unit` 事件的 `volume`/`chapter`/`tags`
（`docs/protocol.md`）、VS Code 课程树的卷→章→单元分组、站点卷 I 页面的分组目录。

## 现状（2026-09-19，12 单元全部落地）

**门禁实测（2026-09-19，0.60.0 二进制，清单 v2 之后）：36 个目标 · 329 checked · 99 open · 0 判负**
（`python3 courses/set-theory/tools/check.py --json`：`canvas_open` 96 / `solutions_open` 0 /
`lib_open` 0；P4 前是 355 checked —— 差额 = `lib/Logic` 空壳化少掉的 26 条声明，**open 不变**。
清单 v2（G-07）**不改判卷计数**：展平后与 v1 的目标/计数逐个相同，只多出卷/章结构与配额报告）。
数字只作现状记录——门禁本身不锁计数（课程还在长），所以这份表随每次重算更新，
**不手写旧数字**。

按清单 v2 的**卷 → 章**分组（配额 = `quota.exercises`，门禁只报告差额、绝不判红）：

| 章 | 标题 | 先修 | 标签 | 计划练习 | 画布实测 | 单元 |
|---|---|---|---|---|---:|---:|---|
| I.1 | 集合、子集与集合运算 | — | membership · subset · powerset | 35 | 35 | 1–4 |
| I.2 | 序对、关系与函数 | I.1 | ordered-pair · product · relation · function | 24 | 24 | 5–7 |
| I.3 | 像、原像与基数 | I.2 | image · preimage · cardinality · cantor | 23 | 23 | 8–10 |
| I.4 | 论域、悖论与综合 | I.1 · I.3 | universe · russell · synthesis | 14 | 14 | 11–12 |

| 单元 | 章 | 标题 | 练习 | 解答（checked） |
|---|---|---|---|---|
| 1 | I.1 | 集合与隶属 | 6 | 6 |
| 2 | I.1 | 子集、空集与包含三律 | 10 | 10 |
| 3 | I.1 | 并、交、差与幂集 | 11 | 11 |
| 4 | I.1 | 外延性与集合等式 | 8 | 8 |
| 5 | I.2 | 序对与笛卡尔积 | 7 | 8 |
| 6 | I.2 | 关系 | 8 | 23 |
| 7 | I.2 | 函数 | 9 | 9 |
| 8 | I.3 | 像与原像 | 9 | 30 |
| 9 | I.3 | 等势（基数 Ⅰ） | 7 | 13 |
| 10 | I.3 | 可数与 Cantor 定理（基数 Ⅱ） | 7 | 9 |
| 11 | I.4 | 论域与 Russell 悖论 | 5 | 5 |
| 12 | I.4 | 综合与读证明 | 9 | 9 |

> 「练习」= 画布上还留着 `sorry` 的声明数；「解答（checked）」= 解答文件里
> `decl.checked` 的条数——两者**不必相等**（解答可以多证几个画布上的演示定义）。
> 记法对照页（`units/notation-cheatsheet.sokonanoda` + 它的解答）也进同一套判据：
> **18 checked + 3 练习 / 解答 21 checked**。

### 记法（G-04 第二刀，0.60.0）

记法是**源级糖**（设计 `docs/design/notation-subset.md`）：`a ∈ A` 与 `Set.mem α a A`
是**同一个项**，走同一个内核；记法命令**不产生任何事件**、不进声明表。

* **第一刀（0.59.0）**：Lean core 级四条 `infix:N` / `infixl:N` / `infixr:N` /
  `notation`。`∈`/`⊆`/`∪`/`∅` 这四个 core 级符号课程库**不声明**——记法对照页
  自己写一份（那是"第二遍"教学的一部分）。
* **第二刀（0.60.0）**：加两条一元命令 `prefix:N` / `postfix:N`，并让**被导入
  模块声明的记法在入口文件里直接可用**（跨 `import` 传播）。于是卷 I 的五个
  集合论符号写在 `lib/Set.sokonanoda` 末尾，单元只要 `import lib.Set` 就能写
  `𝒫 A` / `Aᶜ` / `f '' A` / `f ⁻¹' B` / `A ×ˢ B`——**不重声明**。

三条要记住的边界（都能在 `docs/design/notation-subset.md` §10–§12 找到实测）：

1. **目标名在使用点解析**：`''`/`⁻¹'` 的目标在 `lib/Image.sokonanoda`、`×ˢ` 的目标
   `Set.prod` 是**单元⑤ 给出的词汇**。用到某个符号时那个名字必须在作用域里，
   否则报 `elab-notation-unknown-target`（报错点名缺的是哪个名字）。
2. **同一个符号全课程只能声明一次**（重复声明是 parse 错误）——库声明过的，
   单元不能再声明一遍。
3. **一元记法在实参位要加括号**：`f (𝒫 A)` / `f (Aᶜ)`；`Eq.{1} (Set α) (Aᶜ) (…)`
   里的那对括号是必须的。

课程侧用法：单元③ 的 `demo_mem_powerset_notation` 附近、单元⑧ 的
`image_mono` 附近各有一条 `example` 演示（用 `example` 是为了**不动
`decl.checked` 计数**：演示不该往单元里塞具名声明）。

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
4. `course.json` 加一行（v2：挂到对应的 `chapters[].units[]` 下，顺手更新那一章的
   `quota.exercises`——配额是**计划**，差额由 G6 报告、不判红）；
5. `python3 courses/set-theory/tools/check.py` 全绿（连同 `--selftest`；判据 G1–G6）；
6. 撞到的缺口登记（`gaps/` → `docs/gaps/`）；
7. 引用的教学主张要么给出处、要么标"设计判断"（见大纲 §1.4 引用纪律）。
