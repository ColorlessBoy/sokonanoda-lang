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
python3 courses/set-theory/tools/check.py --selftest  # 判据通道自检（故意坏文件必须被拒 + G6 清单自检 + 台账字段）
python3 courses/set-theory/tools/check.py --json      # 机器可读（含计数；CI 用 --report 落盘）
python3 courses/set-theory/tools/check.py --only "单元 5" --bisect   # 二分到第一个判红的声明
python3 courses/set-theory/tools/check.py --ledger    # 追加一条成本台账（默认 docs/courses/ledger.jsonl；默认关闭）
python3 courses/set-theory/tools/test_manifest_v2.py  # 清单 v2 / G6 / 台账的单测（秒级、零工具链）
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
记账本。

**成本台账**（设计 `docs/design/course-manifest-v2.md` §4.6）：`--ledger [路径]`
追加一条 `soko.course-ledger/1`（日期 / 课程名 / 目标数 / checked / open / 判负 /
用时 ms / 版本 + 逐目标 `rows`）到 `docs/courses/ledger.jsonl`。**默认关闭**——
跑门禁的机器不该往仓库里写文件（CI 不写、也不自动提交），`--ledger` 是**人工收尾**
动作：发版/里程碑时跑一次，把那一行提交进仓库当趋势点。

**一次报多份清单**（§4.5，多课程/多卷聚合）：`sokonanoda course` 现在收多个路径，
每个路径是清单文件或**含 `course.json` 的目录**；`--all` 把目录**递归**展开成它下面
每一个 `course.json`：

```bash
node scripts/soko course "$PWD/course/course.json" "$PWD/courses/set-theory"   # 两份一起报
node scripts/soko course --all "$PWD/courses" --json                            # courses/ 下所有清单
```

>1 份清单时每个 `course.unit` 多出 `manifest`、`course.summary` 多出 `manifests`
（单清单调用一个键都不多）；一批里有一份读不了 ⇒ 整体失败且**零事件**。

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
| `tools/check.py` | 课程门禁（判据 G1–G6，与规模无关；`--selftest` / `--bisect` / `--json` / `--ledger`） |
| `tools/test_manifest_v2.py` | 清单 v2 展平 + G6 + 成本台账字段的单测（`check.py` 的纯清单面，零工具链） |
| `course.json` | **单元清单 v2**（`schema: soko.course/2`：卷 → 章 → 单元 + 先修/标签/配额） |

台账文件在课程目录外：`docs/courses/ledger.jsonl`（仓库根；`--ledger` 人工跑一次追加一行）。

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

**门禁实测（2026-10-01，单元㉕ 落地后）：70 个目标 · 687 checked · 230 open · 0 判负**

**门禁实测（2026-10-01，单元㉔ 落地后）：68 个目标 · 661 checked · 220 open · 0 判负**

**门禁实测（2026-10-01，单元㉓ 落地后）：66 个目标 · 639 checked · 210 open · 0 判负**

**门禁实测（2026-10-01，单元㉒ 落地后）：64 个目标 · 614 checked · 200 open · 0 判负**

**门禁实测（2026-10-01，单元㉑ 落地后）：62 个目标 · 591 checked · 190 open · 0 判负**

**门禁实测（2026-10-01，单元⑳ 落地后）：60 个目标 · 575 checked · 180 open · 0 判负**

**门禁实测（2026-10-01，单元⑲ 落地后）：58 个目标 · 556 checked · 170 open · 0 判负**

**门禁实测（2026-10-01，单元⑱ 落地后）：56 个目标 · 533 checked · 160 open · 0 判负**

**门禁实测（2026-10-01，单元⑰ 落地后）：54 个目标 · 515 checked · 150 open · 0 判负**

**门禁实测（2026-10-01，单元⑯ 落地后）：52 个目标 · 499 checked · 140 open · 0 判负**

**门禁实测（2026-10-01，单元⑮ 落地后）：50 个目标 · 474 checked · 130 open · 0 判负**

**门禁实测（2026-10-01，单元⑭ 落地后）：48 个目标 · 458 checked · 120 open · 0 判负**

**门禁实测（2026-10-01，单元⑬ 落地后）：46 个目标 · 436 checked · 110 open · 0 判负**
（**差值是合法生长**：新库 `lib/Order` 48 条（`lib/Ordinal` 同日重定形）+ 单元⑬ 画布/解答各 14 条
+ 单元⑭ 画布/解答各 13 条 + 单元⑮ 画布/解答各 13 条。
**这些数字都别在别处再抄**，要现算就跑 `python3 courses/set-theory/tools/check.py`，它每次都会重数。）

**门禁实测（2026-09-21，0.62.0 二进制，卷 I Lean 化收尾后）：36 个目标 · 328 checked · 99 open · 0 判负**
（上一行 2026-09-19 的 329 是当时的实测；差 1 来自其后某轮的声明增删——**这两个数字都别在别处再抄**，
要现算就跑 `python3 courses/set-theory/tools/check.py`，它每次都会重数。）

**门禁实测（2026-09-19，0.60.0 二进制，清单 v2 之后）：36 个目标 · 329 checked · 99 open · 0 判负**
（`python3 courses/set-theory/tools/check.py --json`：`canvas_open` 96 / `solutions_open` 0 /
`lib_open` 0；P4 前是 355 checked —— 差额 = `lib/Logic` 空壳化少掉的 26 条声明，**open 不变**。
清单 v2（G-07）**不改判卷计数**：展平后与 v1 的目标/计数逐个相同，只多出卷/章结构与配额报告）。
数字只作现状记录——门禁本身不锁计数（课程还在长），所以这份表随每次重算更新，
**不手写旧数字**。收尾轮（多清单聚合 + 成本台账）同样**不改判卷计数**：
`node scripts/soko course --all "$PWD/courses" --json` 报 `manifests: 1` / 12 单元，
`docs/courses/ledger.jsonl` 的第一条就是本轮实测（36 目标 · 329 checked · 99 open ·
0 判负 · 20024 ms · v0.60.0）。

按清单 v2 的**卷 → 章**分组（配额 = `quota.exercises`，门禁只报告差额、绝不判红）：

| 章 | 标题 | 先修 | 标签 | 计划练习 | 画布实测 | 单元 |
|---|---|---|---|---|---:|---:|---|
| I.1 | 集合、子集与集合运算 | — | membership · subset · powerset | 35 | 35 | 1–4 |
| I.2 | 序对、关系与函数 | I.1 | ordered-pair · product · relation · function | 24 | 24 | 5–7 |
| I.3 | 像、原像与基数 | I.2 | image · preimage · cardinality · cantor | 23 | 23 | 8–10 |
| I.4 | 论域、悖论与综合 | I.1 · I.3 | universe · russell · synthesis | 14 | 14 | 11–12 |
| **I.5** | **序关系与良序** | I.2 | order · partial-order · linear-order · strict-order · well-order · well-founded | 21 | 21 | **13–14** |
| **I.6** | **序数** | I.5 | ordinal · transitive-set · successor · well-founded · extensionality · zero | 20 | 20 | **15–16** |
| **I.9** | **集族、广义积与 n 元关系** | I.2 · I.5 | family · arbitrary-union/intersection · monotone · image · preimage · indexed-family · product · currying | 30 | 30 | **21–23** |
| **I.8** | **基数与基数算术** | I.3 | cardinal · injection · equinumerous · cardinal-arithmetic · product · commutativity | 20 | 20 | **19–20** |
| **I.10** | **关系闭包与等价关系** | I.2 | relation · closure · reflexive/symmetric/transitive-closure · equivalence-relation · equivalence-class · quotient · partition | 20 | 20 | **24–25** |
| **I.7** | **选择公理与 ZF 公理体系** | I.2 | axiom-of-choice · choice-function · surjective · right-inverse · zf-axioms · regularity · extensionality | 20 | 20 | **17–18** |

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
| **13** | **I.5** | **偏序、全序与严格序** | **11** | **11** |
| **14** | **I.5** | **良序与良基** | **10** | **10** |
| **15** | **I.6** | **传递集与序数** | **10** | **10** |
| **16** | **I.6** | **序数的序：外延性、反对称与零序数** | **10** | **10** |
| **17** | **I.7** | **选择公理** | **10** | **10** |
| **18** | **I.7** | **ZF 公理体系与正则性** | **10** | **10** |
| **19** | **I.8** | **基数的比较** | **10** | **10** |
| **20** | **I.8** | **基数算术：积** | **10** | **10** |
| **21** | **I.9** | **集族与广义并交** | **10** | **10** |
| **22** | **I.9** | **像、原像与广义并交** | **10** | **10** |
| **23** | **I.9** | **广义积与柯里化** | **10** | **10** |
| **24** | **I.10** | **关系闭包** | **10** | **10** |
| **25** | **I.10** | **等价关系与划分** | **10** | **10** |

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
   `Set.prod` 在 `lib/Prod.sokonanoda`（E02 起收进库）。用到某个符号时那个名字必须在作用域里，
   否则报 `elab-notation-unknown-target`（报错点名缺的是哪个名字）。
2. **同一个符号全课程只能声明一次**（重复声明是 parse 错误）——库声明过的，
   单元不能再声明一遍。
3. **一元记法在实参位要加括号**：`f (𝒫 A)` / `f (Aᶜ)`；`Eq.{1} (Set α) (Aᶜ) (…)`
   里的那对括号是必须的。

课程侧用法：单元③ 的 `demo_mem_powerset_notation` 附近、单元⑧ 的
`image_mono` 附近各有一条 `example` 演示（用 `example` 是为了**不动
`decl.checked` 计数**：演示不该往单元里塞具名声明）。

## 逐单元 as-built（**每单元一行的交付明细**；大纲 §8 只留状态，明细在这）

| 单元 | 交付 | 撞到/绕开的 |
|---|---|---|
| ⑬ 偏序、全序与严格序 | 新库 `lib/Order`（词汇 6 性质 + 4 序类 + 两个翻译函数 + 展开引理 + **取用子** + **智能构造子**）；画布 3 演示 + 11 练习；解答 0 open | **G-72**（`lib/Order` 最初写成字面箭头返回 ⇒ 单文件绿、`import` 红）· **G-73 形状 ①②③**（证明全靠库层绕法） |
| ㉕ 等价关系与划分 | `lib/Order` +`IsEquivalence` 三条取用子与智能构造子；`lib/Rel` +`Rel.EquivClass` / `Rel.Classes`（**商集写成"是某个等价类"这条性质** —— 谓词式，**不需要新类型**）；画布 3 演示 + 10 练习；解答 0 open · **I.10 章收口** | **同类点有同样的类** · **类相同 ⟺ 相关**（旗舰）· **不同的类不交** · **划分三条**（非空 · 覆盖 · 有公共点 ⇒ 同类）· 「等价关系 = 划分」这一定理的两半 |
| ㉔ 关系闭包 | `lib/Rel` +`Rel.reflClosure` / `symClosure` / `transClosure` + 展开与取用子（**传递闭包照 Halmos 的"交集式"定义** —— "在每个包含 `r` 的传递关系里都成立" ⇒ **不需要递归**）；画布 3 演示 + 10 练习；解答 0 open · 新章 **I.10** | 三种闭包的**两条定律**各一份（**包含** + **极小性**）· 传递闭包另外两条（**自己传递** · **幂等**）· **定义头形态的闭包怎么"用"**（一律先 λ 绑定，库给了取用子）|
| ㉓ 广义积与柯里化 | `lib/SUnion` +`Set.pi` / `mem_pi_iff` / `mem_pi_elim` / `mem_pi_intro` / `pi_mono`（**索引族的积** —— 元素是函数，与二元 `Set.prod` 的序对相对）；画布 2 定义（柯里化/反柯里化）+ 3 演示 + 10 练习；解答 0 open | **某个因子空 ⇒ 积空** · **积与交互相拆装（两半）** · **柯里化往返** · **必破旗舰：选择公理正是"每个因子非空 ⇒ 积非空"**（`lib.Choice` 直接给出 —— 这就是为什么"积非空"不是无条件定理）|
| ㉒ 像、原像与广义并交 | `lib/SUnion` +`Set.mem_sUnion_elim`/`_intro`/`Set.mem_sInter_elim`；`lib/Image` +`Set.mem_image_intro`/`_elim`/`Set.mem_preimage_intro`/`_elim`（**把 `∃` 的拆装收进库** —— 手写嵌套 `Exists.elim` 实测极易少数右括号）；画布 3 演示 + 10 练习；解答 0 open · **I.9 章收口** | **原像是"好"的**（`⁻¹'` 与 `⋃₀`/`⋂₀` **四个方向全交换**）· **像是"半好"的**（与 `⋃₀` 两个方向都对，与 `⋂₀` **只有一半**）· 「像不还原原像」（`f '' (f ⁻¹' B) ⊆ B`，反向**是假的**）· **等式一律拆成两个 `⊆`**（`Set.ext` 在深层嵌套下不稳，实测） |
| ㉑ 集族与广义并交 | 画布 3 演示 + 10 练习；解答 0 open · 新章 **I.9** | **两条刻画定理**（`⋃₀ F ⊆ B` ⟺ 族里每个成员 `⊆ B`；`B ⊆ ⋂₀ F` ⟺ 每个成员 `⊇ B`）· **单调 vs 反单调**（`F ⊆ G` 时 `⋃₀` 单调、`⋂₀` **反**单调 —— 最反直觉的一条）· 空族的并 · **`⋃₀` 里面推不出空集的类型参数**（记法边界，写成恒假谓词） |
| ⑳ 基数算术：积 | `lib/Prod` +`Set.mem_prod_iff` / `Set.mem_prod_mk` / **`Prod.eta`**（η 律 —— 本语言**没有 η 转换**，必须显式证并显式用）；画布 3 演示 + 10 练习；解答 0 open · **I.8 章收口** | **交换律**（`(s ×ˢ t) ≈ (t ×ˢ s)`，跨类型的 `≈`）· **`⊆` 逐坐标函子性** · **交换律与 `⊆` 可交换** · **基数乘法的良定义性如实不排**（要从 `∃` 取数据 ⇒ **G-58**） |
| ⑲ 基数的比较 | `lib/Equiv` +`Set.InjOn` / `Set.Le`（`≼`，含记法）/ `Set.Le.elim` / `Set.Equiv.elim`；画布 3 演示 + 10 练习；解答 0 open · 新章 **I.8** | **`A ≼ B` = 存在单射**（**存在命题**，不是数据 —— 与 `≈` 的关键差别）· **`≼` 是预序**（自反/传递）· 包含给出 `≼` · 空集最小 · **Schröder–Bernstein 如实不排**（结论是数据，要造第二把映射 ⇒ **G-58**） |
| ⑱ ZF 公理体系 | `lib/Rel` 收编 `rel_subst_left/right`（**从 `lib/Order` 迁出** —— 它们是"关系 + 等式"的通用搬运，与"序"无关）；画布 4 演示 + 10 练习；解答 0 open · **I.7 章收口** | **正则性怎么用**（取极小元 · 扔掉它的成员）· **空集唯一**（外延性）· **外延性的反向**（相等 ⇒ 元素相同，两条合起来才是"元素相同 ⟺ 相等"）· **必破两道**：空族的并（化归 + 合成）· **G-73 实例 ⑦⑧**：无序对/并的**唯一性与对称性**如实不排 |
| ⑰ 选择公理 | 画布 3 演示 + 10 练习；解答 0 open · 新章 **I.7** | **旗舰：满射可裂**（`Surjective f → ∃ g, RightInverse g f`，把 AC 用在关系 `f x = y` 上）· **AC 的输出是 `∃`，可以继续往下顺**（不取出数据 ⇒ 绕开 G-58）· **必破**：空族假设真空 · 单点族**不用 AC** 就能写出选择函数 |
| ⑯ 序数的序 | `lib/ZF` +3 条（`Extensional`，谓词层的外延公理）· `lib/Ordinal` +6 条（`IsZero` 一族）；画布 3 演示 + 10 练习；解答 0 open | **G-74（新）：没有排中律 ⇒ 序数三歧性写不出证明** —— 探针把 Halmos 的极小反例证明写到最后一卡，卡在消双重否定（**不是难，是没有规则**）⇒ **三歧性如实不排**；另有 **G-73 形状 ①②③ 的三个新实例**（带 `=` 的 `↔` 取不出方向、`=` 的 `∀` 当合取项会毒掉投影、`Or.inr` 的类型实参写成 `y = y` 判红） |
| ⑮ 传递集与序数 | **`lib/Ordinal` 重定形**（见下）+ 画布 3 演示 + 10 练习；解答 0 open | **教材核心定理「序数的元素是序数」可直接证**（重定形前不可证）· **G-73 形状 ①②**（取用子 + 前导实参写全） |
| ⑭ 良序与良基 | `lib/Order` +6 条（`hasMin_def`/`hasMin_intro`/`hasMin_elim`/`hasMin_witness`/`isWellFounded_intro`/`EmptyRelation`）；画布 3 演示 + 10 练习；解答 0 open | **G-73 形状 ②**（`Exists.intro`/`Exists.elim` 的前导实参必须写全）· 形状 ③（`≠` 不能出现在 λ 的**绑定类型**里 ⇒ 谓词改成部分应用 `r a`、像写成具名 `SeqImage`） |

**被 G-73 挡住、如实不排的题**（等语言线关账再补，**不改题绕开**）：「`⊆` 不是全序」·
「严格部分不是偏序」（I.5）—— 它们的证明恰好全落在形状 ③ 上。

`lib/` 现共 **17 个文件**（16 个模块 + 自检入口 `Demo`；2026-10-01 新增 `Order`）。
**⚠ 下面这一段是 0.59.0 的快照，别当现状读** —— 要现算就 `ls courses/set-theory/lib/` +
`tools/check.py --json`。门禁实测（0.59.0，P4 之后）：
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
名字与签名**逐字不变** ⇒ `units/` 里的点名调用（`Exists.intro A p w hw` /
`Exists.elim A p Q h f`）零改动。**2026-09-21 实测**：这类调用现在是 **102 行、14 个文件**（7 画布 + 7 解答）——比 G-03 当时的 186 处少，是因为全课程 Lean 化之后命题位改用了 `∃ (x : α), p x` 记法（`lib/Exists` 的 `binder_notation`），构造子/消去子的**点名调用本身照旧**（记法是类型的糖，不是引理名的糖）。**"大消去"仍不可用，而且这是正确的行为**
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
