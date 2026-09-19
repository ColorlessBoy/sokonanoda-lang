# 设计：教学项目（第二大课）—— 从集合论到分析（2026-09-18）

> 触发（用户）：「我想基于这个项目，复刻一个大一点的教学项目，比如我自己仓库下的
> analysis 项目，当然可能过于大了，sokonanoda 缺少很多基础设施，需要足够多的细分的
> 计划，比如先做『集合论』教程。在这个项目的制作过程中，可以不断搜集 sokonanoda-lang
> 本身这个项目的不足，让另外的 agent 实现。」
>
> 本文 = 靶子标定 + 今日能力**实测** + 项目形态 + 课程体系 + **缺口台账协议** +
> 分期计划 P0–P7 + 验收 + 风险 + 待拍板。**只出计划**；动工按 §8 分期，每期独立过 gate。
>
> 证据分三类标注：**[实测]** = 本机用发布版内核跑过（命令与输出见附录 B）、
> **[文档]** = 仓库现有文档记载、**[推断]** = 写明依据的估计。

---

## 0. TL;DR（一屏）

1. **靶子不是 Mathlib，是 analysis 的「课程工程」**：章节编号 + 练习即洞 + 每章
   epilogue + 书页 + agent 调度纪律。内容本身 5.3 万行 Analysis I 本体、16 GB 依赖树
   不可移植；可移植的是**流程与结构**（§1）。
2. **集合论今天就能写**：`Set α := α → Prop`、`Set.mem/subset/empty/union/power`、
   子集传递性与并集左包含的完整证明，**已实测通过内核**（§2.1）。
3. 但有 **8 条 blocker** 必须先清（全部本轮实测复现，非推测）。独立仓库最先撞到的两条：
   - **G-11 启动器在非 Rust 仓库没有版本源**（课程仓拿不到版本 ⇒ 拒绝运行 + 裸 `ENOENT`）；
   - **G-12 相对路径入口 + 祖先清单 ⇒ 模块根退化成空路径** ⇒ `units/` 里 `import lib.Set`
     全部报 `import-not-found`（同一文件换绝对路径就绿）——正是课程仓的布局；
   语言/工具面上的另外六条：
   - **G-10 `query check` 对「解析失败」报全零 + `ok:true` + 退出码 0**（agent 主通道假绿）；
   - **G-01 开练习的签名不做类型检查**（`theorem t : 3 := sorry` 报 `exercise_open`，0 诊断）——**0.59.0 已修**（WO-004）；
   - **G-02 构造子无命名空间且全局唯一**（`Pair.mk` 不存在；两个 `ctor mk` 直接撞名）；
   - **G-03「Prop 结果 + Type 参数 + 单构造子 + 自有字段」的 inductive 被内核断言拒绝**
     （`Exists` 因此只能立公理）；
   - **G-04 没有 `notation`/`infix`**（`∈`/`⊆`/`∪` 写不出来，集合论可读性）；
   - **G-06 `sokonanoda course` 只按单文件聚合**（单元一旦 `import` 共享库，聚合必报 failed）。
4. **核心机制 = 缺口台账**：`docs/gaps/ledger.jsonl` + 最小复现 + 工作单（WO）；
   台账**可执行**——复现进测试，语言修好那天测试变红，逼你回来关账 + 升级课程（§6）。
5. **落点（D-1 已定）**：**独立文件夹 / 独立仓库** `sokonanoda-set-theory/`（与
   `sokonanoda-lang` 同级），消费已发布工具链；语言仓只留缺口台账 + 语法增量 + 入门课
   回归。**前置缺口 G-11**：启动器在非 Rust 仓库没有版本源（§3.3）。
6. 本轮已产出 P0 的第一批证据与工具：**附录 A 的缺口（语言/工具 15 条 + 标准库 5 条）+
   `docs/gaps/` 台账 + `scripts/gap.py` + `scripts/new-course-repo.sh`（一条命令生成课程仓）**。
7. **卷 I 前两个单元已"真做一遍"**（`docs/gaps/spike/README.md`）：2 个单元 16 道题 + **66 条标准库**
   全部真内核判卷 0 failed；由此得到 **L-01…L-05** 五条标准库欠账与 **G-14/G-15** 两条新语言
   缺口，并锁定三层分界（`docs/design/course-stdlib.md`）。

---

## 1. 靶子：analysis 项目到底「大」在哪（实测数字）

> 来源：对本机 `/Users/penglingwei/Documents/lean/analysis` 的实测统计。

| 维度 | analysis | sokonanoda 现状 | 差距 |
|---|---|---|---|
| 编译单元 | 110 个 `.lean` | 58 个 `.sokonanoda`（含 en/解答） | ×2 |
| 有效行数 | **124,491**（Analysis I 本体 55,888 / MeasureTheory 65,202） | **4,290**（`course/` 全部） | **×29** |
| 顶层声明 | ≈5,300（theorem 2,097 / lemma 1,221 / example 599 / abbrev 333 / def 280 / instance 197） | 85 道练习 + 若干演示 | ×30+ |
| 集合论一章 | §3 六节 + epilogue = **6,883 行 / 488 声明** | — | 这是卷 I 的直接靶子 |
| 基建代码 | ≈800 行（`literate.toml` 303 / README 195 / AGENTS 156 / serve+build 130） | CLI/LSP/站点/技能 ≈ 数万行 | 我们**已经**有基建 |
| 依赖树 | 16 GB（Mathlib 10 GB + Verso + doc-gen4） | 0（单二进制 + 内核） | 我们的优势 |
| 质量门 | 只有「0 error / 0 warning」纪律；无 lint/测试套件；`-Dwarn.sorry=false` | `scripts/soko gate` + 888 测试 + golden + e2e | 我们的优势 |

**结论（推断）**：analysis 值得抄的是**三件事**，不是它的技术栈：

1. **章节协议**：`Section_<章>_<节>.lean` + 1,203 个编号 docstring（书号锚点）+
   README 每节三链接（书页 / API 文档 / 源码）→ 可索引、可对照教材；
2. **epilogue 机制**：每次换地基（换定义/换库）就写一个小文件证明两套定义同构并弃用前者
   （`Section_3_epilogue.lean` 只用 101 行把第 3 章的公理实例化到 Mathlib 的 `ZFSet`）——
   这是「公理不是空中楼阁」的教学高潮，**我们完全可复刻**（卷 I 的 ZF/类型论对照单元）；
3. **agent 调度纪律**：主线程先分解 → subagent 只做 ≤100 行小块 → temp 文件 ≤300 行
   （500 硬顶）→ 每次改 1–3 行立刻诊断 → 同错 3 次换策略 → 经验台账（143 会话）。
   这套纪律与语言无关，**可逐字移植**（换成我们的 4 个动词）。

**不可移植项（直说）**：`class SetTheory` + 强制转换体系、563 处 `_root_.` 消歧、
`notation`/`@[simp]` 自动化、`noncomputable`/选择公理、Verso/doc-gen4 三个 lake target。
其中 `notation` 与自动化正是我们**要为课程补的语言能力**（§2.2 G-04）。

---

## 2. 今日能力实测（决定「集合论现在能不能写」）

### 2.1 已经能做的（[实测]，探针 `docs/gaps/repro/OK-set-spike.sokonanoda`）

探针文件用**今天的发布版内核**判过，`decl_checked=11 / failed=0`：

```lean
inductive Or (A B : Prop) : Prop
ctor inl (a : A) : Or A B
ctor inr (b : B) : Or A B
end

def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x
def Set.union (α : Type) (A B : Set α) : Set α := fun (x : α) => Or (A x) (B x)
def Set.power (α : Type) (A : Set α) : Set (Set α) := fun (B : Set α) => Set.subset α B A

theorem subset_trans (α : Type) (A B C : Set α) (h1 : Set.subset α A B) (h2 : Set.subset α B C) :
    Set.subset α A C :=
  fun (x : α) => fun (hx : A x) => h2 x (h1 x hx)
```

读出来的四条事实：

- **`def` 可以当类型别名用**：`Set α` 在应用位置能展开成 `α → Prop`（`A x` 直接可用），
  内核 def-eq 兜住了；`abbrev` 不是必需（G-08 因此只是 nice）。
- **点名可以带点**：`Set.mem` / `And.intro` 这种「伪命名空间」写法今天就能用。
- **参数化归纳、非索引 `Prop` 归纳可用**，`Or.rec` 自动派生（构造子要用**裸名** `inl`）。
- **项目模式可用**：`import` + 跨模块复用 + `query project` 闭包状态都实测正常。

### 2.2 卡住的地方（[实测] 20 条：语言/工具 15 + 标准库 5；完整台账见附录 A 与 `docs/gaps/ledger.jsonl`）

| ID | 症状 | 最小复现 | 今天的内核表现 | 判定 |
|---|---|---|---|---|
| **G-11** | **启动器在非 Rust 仓库没有版本源** | `repro/G11-launcher-version-source.sh` | 空缓存时 `version --json` 的 cli/lsp 为 `{}`；有旧缓存时报裸 `No such file or directory` + exit 1 | blocker（独立仓库） |
| **G-12** | **相对路径入口 + 祖先清单 ⇒ 模块根 = 空路径** | `repro/G12-relative-entry-ancestor-manifest.sh` | `import` 全报 `import-not-found`；同一文件绝对路径即绿；`query project` 的 `root=''` | blocker（课程仓布局） |
| **G-10** | **`query check` 对解析失败的文件报"全零 + ok"** | `query check --text 'infix:50 " e " => mem'` | `counts` 全 0、`failed: []`、`ok:true`、**exit 0**；同一文件 `grade` 正确报 `unexpected-token`、exit 1 | blocker（agent 通道假绿）——**0.59.0 已修**（WO-003） |
| **G-01** | **开练习的签名不做类型检查** | `theorem t9 : 3 := sorry` | 修前：`exercise_open`，**0 诊断**；`query goals` 给出 `⊢ 3`。修后（0.59.0/WO-004）：`kernel-expected-sort` + exit 1，**不发** `exercise.open` | blocker ——**0.59.0 已修**（WO-004） |
| **G-02** | **构造子无命名空间且全局唯一** | `Pair.mk`；两个 inductive 各写 `ctor mk` | `unknown identifier Pair.mk`；`duplicate declaration mk` | blocker |
| **G-03** | **Prop 结果 + Type 参数 + 单构造子 + 自有字段**的 inductive 被拒 | `inductive Bar (A : Type) : Prop` / `ctor mk (a : A) : Bar A` | 内核**断言失败** `left: 1 right: 0`；`Exists` 只能立公理 | blocker |
| **G-04** | 无 `notation`/`infix` | `infix:50 " ∈ " => mem` | 修前：parse `unexpected-token`；修后（0.59.0/WO-011 **第一刀**）：`∈`/`⊆`/`∅` + 通用 `infix`/`infixl`/`infixr`/`notation` 可用，点名形式与记法判卷一致；`𝒫`/`''`/`⁻¹'`/`×ˢ` 待第二刀 | blocker ——**第一刀已修**（WO-011） |
| G-05 | 无 `namespace`/`open` | `namespace Foo … end Foo` | parse `unexpected-token` | painful（可绕：点名带点） |
| **G-06** | **`sokonanoda course` 只按单文件编译** | 单元 `import Lib2` 后用 `mythm`（单文件判卷绿） | 聚合报 `failed: 1` | blocker（库化课程） |
| G-07 | 课程清单扁平（`{file,title,unit}`） | — | 无卷/章/先修/标签；`title_en`/`en_file` 只是 site 生成的约定 | painful |
| G-08 | 无 `abbrev` | `abbrev Set (α : Type) := α -> Prop` | parse error | nice（`def` 可替代） |
| G-09 | 内核断言以裸文本外泄 | 同 G-03 | 消息是 `left: 1 right: 0` + 通用「类型不匹配」hint | painful（诊断质量）——0.59.0 已修，见附录 A |
| G-13 | **~~`axiom` 不吃 binder 参数表~~（已修，WO-008）** | `axiom Foo (α : Type) : Sort 1` | 修前 parse `expected axiom type, found LParen`；0.59.0 起与 `def`/`theorem` 同序（柯里化写法照旧） | painful（一致性）→ 已修 |

> **标准库欠账（L-01…L-05）**来自"真做一遍"（`docs/gaps/spike/README.md`）：卷 I 只写了两单元，
> 就需要 66 条库支撑，其中 24 条本该由 prelude / 课程标准库提供，学习者一行都不该写。
> 判据与处置见 `docs/design/course-stdlib.md`。

**四次现场教训（写进台账的理由）**：

- G-12 是**做脚手架时自己撞出来的**：为了让生成器产出的 `units/` 能 import `lib/`，
  我按文档用相对路径判卷，结果全报 `import-not-found`，换绝对路径就绿——顺手把
  `query project` 的 `root=''` 也钉成了证据。**建仓这个动作本身就是一次全链路测试**。

- G-01 是**我差点被骗过去**的地方：我先用 `course` 聚合一个 `import` 的单元，看到
  `open=1 / failed=0` 以为「聚合认 import」——其实那条练习的签名根本没被检查，
  换个 `checked` 声明才暴露 G-06。**开练习不校验 ⇒ 所有"绿"都要重新定义**。
  **as-built（0.59.0，WO-004 已修）**：签名现在与值位同罪——先 elaborate、再问内核
  「是不是一个类型 / `theorem` 的是不是 Prop」，不过就报 diagnostic（span 取签名
  自身）且**不发** `exercise.open`。修前修后的对照（实测，同一条课程变异）：

  | 变异（`mem_of_subset` 的签名） | 修前 | 修后 |
  |---|---|---|
  | 引理名拼错（`Set.subset` → `Set.subsets`） | exit 0 · `exercise.open`×6 · 0 诊断 | **exit 1** · `elab-unknown-identifier` · `exercise.open`×5 |
  | 结论写成 `Set α`（不是 Prop） | exit 0 · `exercise.open`×6 · 0 诊断 | **exit 1** · `kernel-theorem-not-prop` · `exercise.open`×5 |

  这正是「**判据要盯 `failed`/`diagnostic`，不要盯 `exercise_open` 计数**」这条
  纪律的样板：`exercise.open` 只说明"这是个练习"，它对签名腐烂**永远**是盲的。
- G-10 是**台账机制自己抓到的**：为了给 G-04 的复现留一条"今天的表现"，我按文档
  （`crates/front/src/query/mod.rs:594` 的注释明写「`check` 会带着 parse 诊断返回」）
  预期 `query check` 报错，实测却是全绿。**写台账这个动作本身就在产出新缺口**——
  这就是 §6 要把它做成流程的原因。
- G-03 不是新问题：单元⑧早就把 `Exists` 立成**公理三件套**（`course/unit8:101-103`），
  文档里也写了「官方 Lean 的 `Exists` 是 inductive」——但**它从未进过 backlog**。
  这正是本计划要的机制：**被绕过去的东西必须留下账**（§6.1）。

---

## 3. 落点（**D-1 已定：建在当前项目内**，2026-09-18 用户）

> 用户两次定调：先「完全单独一个文件夹，甚至本身一个子仓库」，后「**课程建在当前项目内
> 新建一个文件夹**」。**当前执行的是后者**：课程在语言仓内 `courses/set-theory/`
> （已建，见 §3.1）。独立仓库形态（连同建仓生成器 `scripts/new-course-repo.sh`）保留为
> **将来抽取**的路径，抽取清单见 §3.3。

### 3.1 现状：`courses/set-theory/`（已落地）

```text
courses/set-theory/            # 卷 I 集合论（与入门课 course/ 并列，互不影响）
├── README.md                  # 怎么判卷 + 现状表 + 单元 DoD
├── AGENTS.md                  # 课程线 agent 手册（写作循环 + 判卷三纪律 + 教学纪律）
├── sokonanoda.toml            # 模块根 = 本目录（requires = "0.59"）
├── course.json                # 单元清单（12 单元）
├── lib/                       # L2 课程标准库（名字用 Loogle 取证版）
│   ├── Logic.sokonanoda       # 逻辑与等式骨架 26 条（prelude 0.59.0 也自带 30 个名字，见 L-01/L-02）
│   ├── Set.sokonanoda         # 12 个定义 + Set.ext 公理 + 11 条定义展开引理（L-04）
│   ├── Exists/Prod/Rel/Fun/Image/Equiv.sokonanoda   # 量词 / 序对 / 关系 / 函数 / 像 / 等势
│   └── Demo.sokonanoda        # 库自检入口（门禁会跑）
├── units/                     # 单元画布（演示 + 练习；12 单元 + 记法对照页）
│   └── solutions/             # 解答钥匙（agent 专用）
├── gaps/                      # 发现端（权威台账仍在 docs/gaps/）
└── tools/check.py             # 课程门禁：判据 G1–G5（--selftest/--bisect/--json/--report；已接 gate + CI）
```

**边界**：课程内容与门禁归 `courses/set-theory/`；**缺口台账权威**仍是 `docs/gaps/`
（修复发生在语言侧）；语法增量仍走语言仓的三件套。

### 3.2 为什么这样也行（与"独立仓库"相比）

- **判卷路径现成**：课程就在语言仓里，`scripts/soko` 与 `scripts/gap.py` 直接可用，
  没有版本源问题（G-11 只在"非 Rust 仓库"才挡路）；
- **缺口闭环最短**：课程撞到缺口 → 同一棵树里改语言 → 课程门禁立刻复验；
- **代价**：语言仓的体积与测试时间会随课程增长（需要时按 §3.3 抽出去）。

### 3.3 将来抽成独立仓库的清单（**现在不做**）

1. 用 `scripts/new-course-repo.sh <目标目录>` 生成自包含骨架（已实测绿）；
2. 把 `courses/set-theory/` 的 `lib/`、`units/`、`course.json` 原样搬过去
   （路径检查：`lib.Logic` 这类模块名与目录结构不变）；
3. 修 **G-11**（启动器版本源），否则新仓库只能 `SOKONANODA_BIN` 兜底；
4. 课程自己的 CI（`setup → tools/check.py`）与站点；
5. `docs/gaps/` 的台账留在语言仓，课程侧只留 `requires = "x.y"`。

## 4. 课程体系：卷 I《集合论》大纲（决策点 D-2/D-6）

> **大纲已另立并锁定：`docs/design/set-theory-syllabus.md`**（2026-09-18；含教材取证、
> 证明助手先例、学习障碍、十单元表、每单元"必证/必破"、记法引入顺序）。
> 下面 §4.1–§4.3 是**上游草案与口径**；冲突时**以大纲文档为准**。

### 4.1 卷/章/单元三层

- **卷 I 集合论**（本计划的第一交付物）：10 单元，靶子 = analysis §3（6,883 行 / 488 声明）。
- **卷 II 数系**（自然数 → 整数 → 有理数 → 实数）：先只留名字，等卷 I 的经验。
- **卷 III 分析初步**（序列/级数/极限/连续）：等 §9 的 scale gate。

规模目标（推断，按 §2.1 的写法密度折算）：卷 I ≈ **3,000–5,000 行**画布 + 解答 +
共享库，**120–200 道练习**（每单元 6–12 题 + 3–6 道读/评阅题）。对照：analysis §3 = 488 声明。

### 4.2 卷 I 单元草案（对齐 Tao §3.1–3.6 + epilogue）

| # | 单元 | 靶子（Tao / analysis） | 内容要点 | 语言依赖 |
|---|---|---|---|---|
| 1 | 集合与隶属 | §3.1 前半 | `Set α := α → Prop`；隶属 = 应用；外延（`Prop` 等价 vs `Eq`）；空集/单元素/配对 | 今天够 |
| 2 | 子集、幂集、分离 | §3.1 后半 | `⊆` 自反/传递/反对称；幂集；分离公理（specification）；并/交/差 | **G-04**（`⊆`/`∪`/`∩` 记法） |
| 3 | 序对与笛卡尔积 | §3.1 尾 + §3.5 | `Prod` 归纳；序对唯一性；`A × B`；函数图 | **G-02**（`Prod.mk`） |
| 4 | 关系与等价关系 | §3.3 前半 | 关系 = `A -> B -> Prop`；逆/复合；等价关系三律；划分 | 今天够 |
| 5 | 函数 | §3.3 | 函数即满足单值性的关系 vs 直接 `A -> B` 对照；复合律；单射/满射/双射；逆 | 今天够 |
| 6 | 像与原像 | §3.4 | 像、原像、前推/回拉性质、纤维、单调性 | **G-04** |
| 7 | 基数 Ⅰ：等势与有限集 | §3.6 前半 | 等势、有限集（`Fin`-式枚举）、鸽笼原理 | 今天够 |
| 8 | 基数 Ⅱ：可数与 Cantor | §3.6 后半 | 可数集、Cantor 定理（幂集严格大）、选择公理/选择函数 | **G-03**（`Exists`）、**G-01**（大量签名） |
| 9 | Russell 悖论与「论域」 | §3.2 + epilogue | 为什么没有全集；`Set α` 为什么依赖 `α`；与 ZF 的关系（对照单元，可零 `sorry`） | 今天够 |
| 10 | 综合、读证明与评阅 | §3 全书 | 形式↔散文互译、给错证明找错、期末小项目（一条链：集合 → 关系 → 函数 → 基数） | **G-01** |

依赖树：1 → 2 → 3 → {4, 5} → 6 → 7 → 8 → 9 → 10（9 可在 2 之后随时插）。

### 4.3 刻意不做的（写在大纲里当教学卖点）

- **不用商类型**：等价关系用 **setoid 风格**（在 `A` 上带一个等价关系做运算），
  既绕开语言缺口，又是分析里「有理数/实数构造」的正统做法；
- **不引入类型类/结构体**：所有结构显式传参（与现有 11 单元一致）；
- **不追 Mathlib 命名**：但我们自己的库**必须**是可索引的点名（`Set.mem`），
  这也是 G-02/G-05 要修的原因。

---

## 5. 生产流程：每单元一个「定义完成」（DoD）

每个单元是一轮，DoD 九条（缺一条不算完成）：

1. **大纲条目**：单元号、靶子（Tao 节号）、先修、练习类型配额（T/B/L/R/X 至少 3 种）；
2. **画布** `courses/set-theory/unitN-*.sokonanoda`：演示 + 练习（`sorry`），
   自给自足或 `import` 共享库（G-06 已落地：`course` 认 `import`，WO-007）；
3. **hints**：`-- soko:hint` 三段（思路 / 目标形态 / 关键件），**关键件只写触发条件 + 引理名**，
   绝不写完整答案（teacher skill 铁律）；
4. **解答** `solutions/unitN-*-solution.sokonanoda`：全填、0 诊断、0 open；
5. **判卷证据**：`scripts/soko query check|goals` 的计数写进课程清单/golden；
6. **测试**：单元进课程清单后由课程测试跑（G-06 已落地：有 `import` 的单元走项目
   闭包聚合，`course` 的计数与 `grade` 同判；G-07 的卷/章分层仍待办）；
7. **文档**：`courses/set-theory/README.md`、大纲文档、本文件 §4.2 的进度列；
8. **缺口台账**：本轮撞到的每条都进 `docs/gaps/ledger.jsonl`（含最小复现）；
9. **门禁**：`scripts/soko gate` + 课程测试全绿（在仓库内阶段）。

**顺序原则**：先写**不依赖 blocker 的单元**（1、4、5、7、9）把流水线跑顺，
同时把 blocker 的 WO 发出去；等 G-01/G-02/G-03/G-06 修好再写 2、3、6、8、10。

---

## 6. 缺口台账协议（本计划的核心机制）

### 6.1 为什么需要：四个真实教训

1. **绕过即遗忘**：`Exists` 用公理绕了 G-03，两年（轮次意义上）没人记账；
2. **假绿**：G-01 让「开练习」全绿，我先据此得出了错误结论（§2.2）；
3. **文档与实现分叉**：G-10 —— `query/mod.rs:594` 的注释承诺「`check` 会带着 parse
   诊断返回」，实测是全零 + `ok:true`；没有台账就没人会发现这句注释在说谎。
   **as-built（0.59.0，WO-003 已修）**：注释与实现重新对齐——`check` 现在真的带
   parse 诊断（`failed[]`）+ 退出码 1，同族的 `goals`/`holes` 假绿（G-17）一并修掉；
4. **回归无痕**：语言仓的 golden 只看计数，签名级错字不改计数 ⇒ 课程烂掉不报警。

### 6.2 数据结构：`docs/gaps/ledger.jsonl`（一行一条，机器可读）

```json
{"id":"G-01","title":"开练习的签名不做类型检查","kind":"language","severity":"blocker",
 "status":"open","found":"2026-09-18","found_by":"course-agent",
 "where":{"area":"elab/开练习路径（open_goal）"},
 "repro":"docs/gaps/repro/G01-open-exercise-signature.sokonanoda",
 "today":"exercise_open，0 诊断；query goals 给 ⊢ 3",
 "expected_lean":"unknown identifier `3` 之类：签名必须能通过 elaborate",
 "workaround":"把签名复制进一个 checked 声明跑一遍（人肉，不可扩展）",
 "blocks":["卷 I 全部单元","任何 >100 题的课程"],
 "wo":null,"wo_planned":"WO-002","fixed_in":null}
```

> `wo` 只在**工作单文件已经写出来**时填路径（状态同时改 `wo-filed`）；
> 还没写就放 `wo_planned`（计划编号），别指向不存在的文件。

- `kind`：`language`（语法/elaborator/kernel）/ `tooling`（CLI/LSP/VS Code）/ `infra`（清单/CI/站点）/
  **`library`（标准库欠账：Lean core / Mathlib 里现成、我们却没有）** / `doc`；
  `library` 条目另有 `owner`（`prelude` / `course-lib` / `exercise`）与 `lean_names` 两个字段——见
  `docs/design/course-stdlib.md` 与 `docs/gaps/spike/README.md`（试做稿）；
- `severity`：`blocker`（不修就写不了这类内容）/ `painful`（能写但成本×N 或会教坏学生）/ `nice`；
- `status`：`open` → `wo-filed` → `fixed`（`fixed_in` 必填版本）／`workaround`（长期绕行，须写清代价）／`wontfix`（须写理由）；
- **复现文件必须最小且入库**（`docs/gaps/repro/<id>-*.sokonanoda`）；
- **`expected_lean` 必填**：硬规则 3（教学语法是真实 Lean 4 的子集）要求我们写清
  「官方 Lean 里这段是什么行为」，它就是 WO 的验收判据。

### 6.3 工作单（WO）：交给「另一个 agent」的唯一接口

每条升格为开发任务的缺口写一份 `docs/gaps/WO-<nnn>-<slug>.md`，模板：

```markdown
# WO-001 开练习的签名不做类型检查（G-01）
- 用户可见症状 / 最小复现 / 今天的表现（贴命令与输出）
- 期望行为（官方 Lean 4 语义 + 本教学子集的边界）
- 范围：parser / elab / front / project / CLI（**是否动内核**：预期否）
- 不做的事（明确排除，防止顺手扩大）
- 验收（三层）：front 单测 + CLI e2e + 课程用例；**影响面**：事件计数是否变（golden 双改）
- 文档同步清单：白名单 / docs/protocol.md / skills / VS Code / HANDOVER
- 门禁：`scripts/soko gate`
```

### 6.4 可执行台账：「缺口即测试」——**本轮已落地**（`scripts/gap.py check`）

机制分两层，第一层已经能跑：

1. **自断言复现**（✅ 本轮）：每条缺口的 `.sh` 复现遵守统一退出码——
   **0 = 缺口仍在且与台账 `today` 一致 · 1 = 行为变了（回来更新台账） · 2 = 环境不满足**；
   `.sokonanoda` 复现的判据是「是否干净判卷 + 有没有 checked 声明」（**故意不看
   `exercise_open` 计数**，G-01 就是这么假绿的）。
2. **`python3 scripts/gap.py check`**（✅ 本轮）：跑全部复现，逐条对照台账状态打印判定，
   任何不一致 exit 1。红了就是语言变了而台账没跟上（或修好忘了关账）。
   `close <id> --version X` 会先复跑复现，仍复现则**拒绝关账**。
3. 后续（P0.2 余项）：把它包成 `crates/cli/tests/gap_ledger.rs` 进 `scripts/soko gate`，
   顺带加一条「故意把台账改错会红」的自检。

### 6.5 排序规则（谁来修、先修谁）

优先级 = `severity` 序 → `blocks` 的单元数 → 修复规模（S/M/L）。每轮课程线把
**top-1 的 blocker** 升级成 WO 交给语言线；语言线一次只接一张 WO（仓库惯例：
一次一刀 + 二进制对拍）。`scripts/gap.py next`（P0 落地）直接打印「下一张 WO +
可直接粘贴给另一个 agent 的 prompt」。

---

## 7. 两个 agent 的协作回路

```
课程线（agent A，本计划）                语言线（agent B，sokonanoda-lang）
─────────────────────────                ────────────────────────────────
写单元 → scripts/soko 判卷
   │ 绿 ⇒ 下一单元（进 DoD）
   └ 撞墙 ⇒ 登记 G-xx + repro + WO
                     │
                     ├── 人/脚本把 WO 转交 ──────────► 读 WO → 设计先行（docs/design/）
                     │                                   → TDD 三层 → gate → bump → release
                     ◄──────── 新版本 + CHANGELOG ──────┘
   升级钉版本 → 复跑 repro → 台账写 fixed_in → 继续写
```

**交接的三条硬要求**（沿用仓库既有纪律）：

1. WO 必须自带复现，且复现是**一条可执行命令**（`scripts/soko grade docs/gaps/repro/…`）；
2. 语言线的验收必须包含**课程用例**（硬规则 3 的「三件套」），不接受只改测试；
3. 任何用户可见改动，语言线同一轮同步 `editor/vscode/` + `skills/` + `AGENTS.md`（仓库硬规则）。

---

## 8. 分期计划（P0–P7，每期独立可验收）

> 规模：S ≈ 半天、M ≈ 1–2 天、L ≈ 3 天+（agent 轮次口径）。**P1 与 P2/P3 可并行**。

### P0 侦察、试点与建仓（本轮已做一半）— S
- **P0.1 ✅（本轮）**：10 条缺口实测 + `docs/gaps/` 台账骨架 + `docs/gaps/repro/` 最小复现；
- **P0.0 落点**：✅ **已定：语言仓内 `courses/set-theory/`**（2026-09-18 用户）。
  课程骨架已建（README/AGENTS/`sokonanoda.toml`/`course.json`/`lib/`/`units/`/`gaps/`/
  `tools/check.py`），门禁 **22 checked · 16 open · 0 判负**；
  独立仓库生成器 `scripts/new-course-repo.sh` 保留备用（`/tmp` 实测绿）；
- **P0.2 ✅ 本轮已落地（python 形态）**：`scripts/gap.py`（`list|show|next|check|close`）+
  四条自断言复现脚本（G-06/G-10/G-11/G-12）+ `gap.py check` 全绿；**剩**＝包成
  `crates/cli/tests/gap_ledger.rs` 进 `scripts/soko gate`（Rust 形态，CI 用）；
- **P0.3**：把 §2.1 的探针扩成**一个完整单元的最小切片**（单元 1 的前 3 题），确认
  「一单元 ≈ 多少轮 agent 工作」；
- **验收**：课程仓 `check-course.py` 绿（0 单元也算）；`gap.py next` 能打印 WO-001 的
  可粘贴 prompt；台账测试绿（能抓住故意改错的台账）。

### P1 ✅ **已完成**（0.59.0，2026-09-19）— M/L

> **收官（第一百〇六轮 / 0.59.0）**：P1 的八张 WO（WO-001…007 + WO-011）全部落地，
> 台账里 **18 条 `fixed_in = 0.59.0`**（`python3 scripts/gap.py list`：12 条 blocker 里
> **11 条 `fixed`**，只剩 L-04 是 `workaround`——课程库已手写那 8 条 Set 展开引理；
> 其余未关账的 6 条全是 `painful`/`nice`）。实测：复现件全部翻成修后形状，
> `python3 scripts/gap.py check` **全绿（exit 0）**——G-09 的复现件与 G-03 共用同一份文件、
> 已随 G-03 转绿，故该条改判 `fixed` 并**撤下 `repro`**（两半结论写进 `notes`：包装层已有
> 稳定码 `kernel-internal` + 「这不是你的代码问题」；唯一已知可达触发路径随 G-03 关闭）；
> G-01 则相反——它的"修好"就是判红，故加显式 `repro_expect: "rejected"`（`gap.py` 新增该字段
> 与 `selftest`，判定规则本身也有自检）、
> `scripts/soko gate` 带课程门禁**与台账门禁**绿（CI 同款 step：`Gap ledger is consistent`）、
> 入门课双 GOLDEN 与卷 I 门禁**同数不回归**。**G-12 补丁保留**：相对路径今天也能判绿
> （实测 `node scripts/soko grade courses/set-theory/units/unit02-subsets-empty.sokonanoda`
> = exit 0），但课程门禁**有意**继续一律绝对路径（`--bisect` 的前缀文件必须落在目标同目录，
> 理由见 `docs/design/course-gate-in-ci.md` §9.2-2）。

按序：**WO-001 G-11**（启动器版本源——独立仓库的**入场券**）→ **WO-002 G-12**
（模块根绝对化 + 空 parent 护栏；**修好前课程仓只能用绝对路径**）→ **WO-003 G-10**
（`query check` 带上 parse 诊断，agent 判卷通道；**as-built（0.59.0）**：`failed[]`
合成 `parse_error`、退出码 1，同族的 G-17（`goals`/`holes` 假绿）同轮修掉，
判据见 `crates/cli/tests/query.rs` 的四条 parse 用例，repro 见
`docs/gaps/repro/G10-query-check-parse-error.sh`）→ **WO-004 G-01**（开练习签名校验，
改 elab/开练习路径）→ **WO-005 G-02**（构造子命名空间，注意兼容旧的裸名，别一次改崩
unit9/unit11 的 golden）→ **WO-006 G-03**（Prop+Type 参数归纳的 recursor 派生，像 H6-C
一样在前端修；若必须动内核，按 `docs/architecture.md` §6 走）→ **WO-011 G-04**
（用户自定义记法；**as-built（0.59.0）第一刀**：`infix:N`/`infixl:N`/`infixr:N`/
零元 `notation` 四条命令 + 数学符号独立 token + elab 内源到源重写并自动补前导类型
参数；文件内作用域、零事件、点名形式永久可用且两种写法判卷一致；课程**零改动**。
设计 `docs/design/notation-subset.md`（§9 as-built），三层测试见
`crates/cli/tests/notation.rs`。`𝒫`/`ᶜ`/`''`/`⁻¹'`/`×ˢ`、跨 `import` 的记法、
binder 记法、记法重载留第二刀）→ **WO-007 G-06**
（`course` 走项目闭包；**as-built（WO-007）**：`CompileOptions` 保持只有 `prelude`
一个字段——模块根属于"计划"（`plan_project` 的 `root_override`；塞进 `Copy` 的
`CompileOptions` 会污染单文件缓存键）。落地记录见
`docs/gaps/WO-007-course-import.md`，判据见 `crates/cli/tests/course_project.rs`）。
- **验收（0.59.0 实测 ✅）**：七条 repro 全部从「失败签名」翻成「0 failed」
  （`python3 scripts/gap.py check` **全绿 exit 0**——G-09 的复现件就是 G-03 那个文件，
  随 G-03 一起转绿后已撤下）；台账相应条目 `fixed_in = 0.59.0`（17 条）；
  全量 `scripts/soko gate`（含课程门禁与台账门禁）+ 既有 11 单元 golden 不回归；**G-12 额外要求**：
  `units/` 用相对路径判卷与绝对路径结果一致 ✅（实测 exit 0）——门禁**有意**继续用绝对路径。

### P2 课程骨架（不依赖 P1）— S/M
- `courses/set-theory/`：`README.md`、`course.json`（先扁平，等 G-07 再升卷/章）、
  目录约定、单元命名、依赖表；
- 课程测试接入（仓库内阶段：`crates/cli/tests/set_theory_course.rs`）；
- **验收**：空课程骨架能被 `sokonanoda course` 聚合（0 单元也算绿）。

### P3 共享库（不依赖 P1 的部分先做）— M
- `courses/set-theory/lib/`：`Logic.sokonanoda`（True/False/And/Or/Iff/Not）、
  `Set.sokonanoda`（Set/mem/subset/empty/singleton/pair/union/inter/power）、
  `Rel.sokonanoda`、`Fun.sokonanoda`、`Prod.sokonanoda`、`Exists.sokonanoda`（先公理版，G-03 修好后换归纳版）；
- 命名纪律：**点名带点**（`Set.mem`），构造子用 `mk_*` 前缀直到 G-02 修好；
- **验收**：库自身 0 诊断；每个模块能被一个自检入口 `import` 后判卷（照 `course/shared/Demo` 的做法）。

### P4 逐单元生产（主体工作量）— L ×**10**
- 顺序（§5 的「先不依赖 blocker」原则）：**1 → 4 → 5 → 7 → 9 →（P1 落地后）2 → 3 → 6 → 8 → 10**；
- 每单元一轮：DoD 九条（§5）；
- **验收**：每单元的画布/解答/提示齐；计数进课程清单；台账更新；gate 绿。

### P-C ✅ **已完成**（0.59.0，2026-09-19）— L

> **收官（第一百〇六轮 / 0.59.0）**：P-C1…P-C8 全部 ✅，P-C6 由本轮收掉——课程门禁
> （判据 G1–G5，`courses/set-theory/tools/check.py`）已接进 `scripts/soko gate` **与**
> CI（设计/as-built `docs/design/course-gate-in-ci.md`；CI 侧是 `test` job 里的 step +
> `course-gate-report` artifact，课程红自动挡住 `auto-tag` 发布）。内容侧同时收在
> **12 单元 / 36 个目标（含记法对照页）/ 355 checked / 99 open / 0 判负**，
> 站点卷 I 页面（`site/set-theory.html`）的计数就是这条门禁实测出来的。
> **P4 的课程仓跟随已完成（2026-09-19）**：`lib/Logic` 退化成只有注释的空壳
> （0 条声明，34 个 `import lib.Logic` 一字未改），课程侧 **65 处项位裸名**
> `inl`/`inr` 改成点号名 `Or.inl`/`Or.inr`（+27 行注释同步改写）；
> 验收 `python3 courses/set-theory/tools/check.py` = **exit 0 · 36 目标 ·
> 329 checked · 99 open · 0 判负**（checked 的 −26 正是 `lib/Logic` 少掉的 26 条
> 声明，**open 不变** ⇒ 练习与题义未动）。P4 的原始方案见提案
> `docs/design/prelude-l1-proposal.md` §1.4/§6；课程侧口径见 `course-stdlib.md`。
> **仍未做（留给后续）**：`lib/Set` 还有 10 条待移的 L3 引理（L-05 遗留）、记法第二刀。

> 独立于 P1（语言线）：内容可以先写，写的过程就是缺口探测器。设计 =
> `docs/design/course-stdlib.md`；现场 = `docs/gaps/spike/README.md`。
- **P-C1 ✅**：卷 I 单元 1–2 真做一遍 + 66 条标准库，全部判卷 0 failed；产出 G-14/G-15/L-01…L-05；
- **P-C2 ✅ 本轮**：课程**搬进语言仓** `courses/set-theory/`；`lib/Set` 改用 **Loogle 取证名**
  （`Set.notMem_empty`/`Set.mem_powerset_iff`/`Set.mem_sdiff`/`Set.Subset.refl`…）；
  **L3 拆分第一刀**：`subset_refl/trans/antisymm`、`empty_subset`、`subset_empty_iff`、
  `eq_empty_iff_forall_notMem` 六条**移出 lib、进单元②**（台账 L-05），lib 现在只有
  定义 + `Set.ext` + 定义展开引理；
- **P-C3 ✅**：向语言线提 **L1 prelude 提案** —— 已**落地**（0.59.0，P1/P2/P3）：
  `PRELUDE_L1_SRC` + 族让位（B1–B7）+ 30 个 `PRELUDE_NAMES`；入门课 golden 一次改完
  （`unit2 = (3,5,2)`、summary `checked 85→86`，另加 `cli.rs` 的第三处）。
  设计 `docs/design/prelude-l1-proposal.md`（含 as-built 修正）；**P4 课程仓跟随已完成**（同日）；
- **P-C4 ✅**：单元 3–12 **全部落地**（12 单元 93 题 / 解答 0 open / 门禁 34 目标 · 308 checked · 0 判负）；
  方法 = 两轮 workflow（17 + 14 个子代理：实现→独立复核闭环 + 10 张 WO + 4 份设计/调研）；
  `lib/Set` 里还剩 10 条待移的 L3 引理（属单元③④，未做，记在 L-05）；
- **P-C5 ✅**：`gap.py list --kind library` 视图（标准库欠账单独可见）；
- **P-C7 ✅**：Demo 自检已 import 全部 8 个 lib 模块（每模块一条冒烟定理）；
- **P-C8 ✅（本轮 workflow 产出）**：10 张 WO 落盘（`docs/gaps/WO-001…010.md`）+ L1 prelude 提案
  （`docs/design/prelude-l1-proposal.md`）+ 课程门禁 CI 接线设计（`docs/design/course-gate-in-ci.md`）
  + 中文教材调研（`docs/notes/settheory-survey/chinese-textbooks.md`）；
- **P-C6 ✅（0.59.0 收尾）**：把课程门禁接进 `scripts/soko gate` 或 CI
  （`courses/set-theory/tools/check.py`）——**两条都接了**；设计/as-built
  `docs/design/course-gate-in-ci.md`（判据 G1–G5 与规模无关、`--selftest`/`--bisect`、
  python3 缺失 ⇒ exit 3、本地与 CI 同一条命令）；索引见 `courses/set-theory/README.md`；
- **验收**：`python3 courses/set-theory/tools/check.py` 全绿；台账 `library` 条目都有
  `owner` 与 `lean_names`。

### P5 阅读面（决策点 D-4）— M
- 站点加「课程」页：从 `course.json` + 画布生成单元目录 + 进度；可选：
  把注释当散文渲染成「书页」（`literate.toml` 的极简替代，零构建 HTML）；
- **验收**：`scripts/check-site.py` 绿 + 站点可见卷 I 目录与每单元计数。

### P6 度量与规模化 — S/M
- 进度看板：`sokonanoda course` 的计数 + 缺口燃尽（`gap.py list --stats`）；
- 成本台账：每单元 agent 轮次/耗时（照 `docs/perf/ledger.jsonl` 的做法记进 `docs/courses/ledger.jsonl`）；
- **验收**：能回答「卷 I 还要几轮」「哪个单元最贵」。

### P7 卷 II 及以后（scale gate 之后）— 另立设计
- **scale gate（进卷 II 的硬条件）**：① 卷 I 完成且 blocker 清零；② 每单元平均 ≤ N 轮；
  ③ 库能支撑「不复制粘贴」；④ 语言侧对「商类型/结构体/自动化」有了明确取舍结论。
- 卷 II 需要的新能力（预判，先记着，届时不惊讶）：`structure`/记录、
  setoid-商、更多等式推理（`rw` 的极简替代）、`Fin`/可判定性。

---

## 9. 验收与度量

**卷 I 完成的定义**：10 单元 × DoD 九条全绿 + 课程聚合计数稳定 + 台账无 `blocker`
且 `painful` 都有 `workaround` 或 `fixed_in` + 站点可见 + 至少一条端到端「学习者路径」
（零 cargo：`scripts/soko setup` → 打开画布 → 判卷 → 看提示）人工走通一遍。

**度量**（P6 起逐轮记录）：单元数/练习数/解答数、缺口数（按 severity 分）、
每单元轮次、CI 时长、每单元行数（对照 §1 的 analysis 密度）、提示泄题复查通过率。

---

## 10. 风险与取舍

| 风险 | 取舍 |
|---|---|
| **G-02 修复会撞既有课程**（`Or.inl`/`inl` 命名、golden 计数） | 兼容别名（裸名保留）+ 一次改完 + 二进制对拍；先写设计再动 |
| **G-03 可能牵动内核**（内核冻结快照） | 先按 H6-C 的经验在前端派生侧修；确需动内核时走 `docs/architecture.md` §6 清单 + 三层回归 |
| 缺口修完课程没跟上（或反之） | §6.4 的「缺口即测试」把这条变成红测试 |
| 规模失控（照 analysis 全抄 = ×29） | §9 的 scale gate；卷为单位、每单元可独立交付；不追 Mathlib |
| 无 `notation` 前可读性差，学习者流失 | 卷 I 前两个单元先写「点名形式」，notation 落地后**同轮**补一版记法对照（教学上反而更好） |
| 双语镜像成本 ×2 | 决策点 D-2：卷 I 先只做中文，英文镜像等卷 I 冻结 |
| 独立仓库的版本钉不住 | **G-11 就是它**（§3.3）：修好版本源链之前，课程仓只能 `SOKONANODA_BIN` 兜底；P1 的 WO-001 优先做 |

---

## 11. 待拍板（请用户定）

- **D-1 落点**：✅ **已定并落地（2026-09-18 用户：「课程建在当前项目内新建一个文件夹」）**：
  课程在语言仓内 `courses/set-theory/`（见 §3.1，已建、门禁全绿）；
  **独立仓库形态改为"将来抽取"**（§3.3 清单 + `scripts/new-course-repo.sh` 保留）。
- **D-2 语言与读者**：中文为主；英文镜像**卷 I 先不做**（推荐）还是同步做？读者默认「学过一点数学、没写过证明」还是「已会 Lean 4」？
- **D-3 语法增量边界**：同意为课程新增 `notation`/`infix`（G-04）、`namespace`/`open`（G-05）、
  `abbrev`（G-08）吗？（按硬规则 3，每项都要课程 + 测试 + 白名单三件套，且照 Lean 4 真实语法子集设计。）
  构造子命名空间化（G-02）允许做**兼容性**调整吗（裸名保留 + 新增 `Type.ctor` 形式）？
- **D-4 发布形态**：卷 I 是否要求**书页/网站**作为验收项？（P5 默认「要」）
- **D-5 交接方式**：WO 由谁转交——人工复制（默认）／`scripts/gap.py next` 输出 prompt 后自动开新会话？
- **D-6 停止线**：卷 I 完成后是否继续卷 II（数系）？还是先回头把入门课也升级成新机制？

---

## 附录 A：已验证缺口清单（24 条，2026-09-18 首测 / 2026-09-19 **0.59.0 对账**）

> **对账口径**：唯一真相 = `docs/gaps/ledger.jsonl`，看全貌用 `python3 scripts/gap.py list`
> ——24 条 = language 10 + library 6 + tooling 7 + infra 1。**blocker 12 条里 11 条
> `fixed_in = 0.59.0`**，只剩 L-04 是 `workaround`（课程库已手写那 8 条 Set 展开引理）；
> 其余未关账的 5 条全是 `painful`/`nice`（L-03 / G-05 / G-07 / G-08 / L-06），
> 加上 L-04 共 6 条未关账。下表逐行标注本轮（0.59.0）状态。

| ID | 级别 | 一句话 | 复现文件 |
|---|---|---|---|
| G-11 | blocker（独立仓库） | 启动器在非 Rust 仓库没有版本源（`repoVersion()` 只读 `Cargo.toml`），课程仓报裸 `ENOENT` 且拒绝运行——**0.59.0 已修**（WO-001：版本钉源链 `$SOKONANODA_VERSION` → `sokonanoda-version.txt` → 清单 `requires` → `Cargo.toml`；解析不出就**拒绝 exec**） | `repro/G11-launcher-version-source.sh` |
| G-12 | blocker（课程仓布局） | 相对路径入口 + 祖先清单 ⇒ 模块根退化成空路径 ⇒ 所有 `import` 报找不到（绝对路径即绿）——**0.59.0 已修**（WO-002：模块根绝对化 + 空 parent 护栏）；修后相对路径实测 exit 0，课程门禁仍**有意**用绝对路径 | `repro/G12-relative-entry-ancestor-manifest.sh` |
| G-10 | blocker | `query check` 对解析失败的文件报全零 + `ok:true` + exit 0（`grade` 正确报错）——**0.59.0 已修**（WO-003：`failed[]` 合成 parse 诊断、**exit 1**，与 `grade` 同口径） | `repro/G10-query-check-parse-error.sh` |
| G-01 | blocker | 开练习签名不做类型检查（`theorem t : 3 := sorry` 也绿）——**0.59.0 已修**（WO-004）：签名先 elaborate、再过内核的类型/Prop 判定，坏签名报 diagnostic 且不发 `exercise.open` | `repro/G01-open-exercise-signature.sokonanoda`（修后应 exit 1）+ `repro/G01-course-signature-mutations.sokonanoda`（课程级变异体） |
| G-02 | blocker | 构造子全局唯一且不可带前缀（`Pair.mk` 不存在；两个 `mk` 撞名）——**0.59.0 已修**（WO-005：规范名 `Ind.ctor`，裸名降为闭包级别名，撞名报 `elab-ambiguous-ctor-alias`） | `repro/G02-ctor-namespace.sokonanoda` |
| G-03 | blocker | Prop+Type 参数+单构造子+自有字段的 inductive 被内核断言拒绝（`Exists` 只能立公理）——**0.59.0 已修**（WO-006：派生 recursor 的宇宙参数镜像内核）；课程侧 `lib/Exists` 已随之升级成真归纳 | `repro/G03-prop-type-param-inductive.sokonanoda` |
| G-04 | blocker | 无 `notation`/`infix`（`∈`/`⊆`/`∪` 写不出来）——**0.59.0 第一刀已修**（WO-011）：`∈`/`⊆`/`∅` + 通用四条命令可用；`𝒫`/`''`/`⁻¹'`/`×ˢ` 与跨 `import` 待第二刀 | `repro/G04-notation.sokonanoda`（修后判卷干净） |
| G-05 | painful | 无 `namespace`/`open`——**0.59.0 仍 open** | `repro/G05-namespace-open.sokonanoda` |
| G-06 | blocker | `sokonanoda course` 只按单文件编译，不认 `import`——**0.59.0 已修**（WO-007：有 `import` 的单元走同一份项目闭包，`failed == 0` ⇔ `grade` exit 0）；课程**聚合与本单元判卷同判** | `repro/G06-course-import/` + `repro/G06-course-import.sh` |
| G-07 | painful | 课程清单扁平（无卷/章/先修/标签）——**0.59.0 仍 open**（留给 P6 度量与看板） | —（清单格式问题） |
| G-08 | nice | 无 `abbrev`（`def` 可替代）——**0.59.0 仍 open**（不阻塞课程） | `repro/G08-abbrev.sokonanoda` |
| G-09 | painful | 内核断言以裸 `left: 1 right: 0` 外泄，hint 是通用「类型不匹配」——**0.59.0 已修**（两半：包装层稳定码 `kernel-internal` + 「这不是你的代码问题」提示；唯一已知可达触发路径随 G-03 关闭 ⇒ 撤下 `repro`，见 `notes`） | —（复现件已随 G-03 转绿撤下） |
| G-13 | painful | `axiom` 不吃 binder 参数表（`def`/`theorem` 吃；调研探针发现）——**0.59.0 已修**（WO-008） | `repro/G13-axiom-binder-params.sh` |
| G-14 | ~~painful~~ **nice**（勘误） | 一个声明只允许一个宇宙层级 binder（`{u v}`/`{u} {v}` 都解析失败）⇒ 跨宇宙引理写不出来——**0.59.0 已修**（WO-009：`{u v}` / `{u, v}` / `{u} {v}` 等价） | `repro/G14-single-universe-binder.sokonanoda` |
| G-15 | painful | **重新定义**（勘误）：`query check` 的 `failed[]` / `warnings[]` 只给裸字节 offset（没有行列、没有单位）——**0.59.0 已修**（WO-010：additive 加 1 基 `start_line/start_col/end_line/end_col`，`start/end` 仍是字节 offset、坐标空间 = 入口文件；`--bisect` 仍是解析失败/多条错误时的定位手段） | `repro/G15-query-check-bare-offsets.sh`（修后 exit 1） |
| L-01/L-02 | blocker（库） | prelude 缺 Lean core 的逻辑与等式骨架（True/False/And.elim/Or.elim/Not/absurd/Iff/Eq.symm/trans/congrArg）——**0.59.0 已修**（P1/P2/P3，`PRELUDE_NAMES` 12 → 42；P4 课程仓跟随同日完成：`lib/Logic` 空壳 + 65 处点号名） | `repro/L01-l1-prelude-logic-skeleton.sh`、`repro/L02-eq-core-lemmas.sh` |
| L-03 | painful（库） | `Eq.subst` 的 motive 只能 `α → Prop` ⇒ Type 层重写（`Eq.mp`/`cast`）不可表达——**0.59.0 仍 open** | —（最小反例在台账里） |
| L-04 | blocker（库） | 课程标准库缺 8 条 Set 定义展开引理——**0.59.0 仍 `workaround`**：课程把 8 条写进自己的 `lib/Set.sokonanoda`（自己的库自己补），prelude 不给 | — |
| L-05 | painful（方法学） | 16 条**内容型**引理被误放进库——应当是练习——**0.59.0 第一刀已修**（6 条移出 lib 进单元②；`lib/Set` 还剩 10 条待移，记账在 P-C 遗留） | — |
| G-16 | blocker | 启动器在「版本未知」时会 exec 缓存里的陈旧二进制（绕过「过期即拒绝」守卫）——**0.59.0 已修**（WO-001 同族：解析不出期望版本 ⇒ 缓存与仓库构建都拒绝） | `repro/G11-launcher-version-source.sh`（同一夹具的第二处缺陷） |
| G-17 | painful | `query goals`/`holes` 对解析失败也假绿（空数组 + ok:true）——**0.59.0 已修**（WO-003 同轮：`not-parsable` + `ok:false` + exit 1） | `repro/G17-query-goals-holes-parse-error.sh` |
| G-18 | painful | `def f.{u}` 被静默解析成名字 `f.`（声明消失且不报错）——**0.59.0 已修**（WO-008） | `repro/G14-single-universe-binder.sokonanoda` |
| L-06 | painful（边界） | 无累积性 + `Exists.elim` 的 Q 只能是 Prop ⇒ 等势只能 Prop 值、取数据的引理写不出来（课程统一改数据版）——**0.59.0 仍 open**（卷 II+ 的事） | — |
| **勘误** | — | **G-14 降级**为 nice（`{u, v}` 逗号写法可用）；**G-15 重新定义**为「query check 的 failed[] 只给裸字节 offset」（原先的"span 漂移"是把字节当字符的量具缺陷，见 `docs/LESSONS.md`） | — |

另有三条**已知 backlog**（不重复记账，指针在此）：P7 的 `[deps]`、`namespace`/`open`、
`watch` 项目模式、decl 级产物（`docs/HANDOVER.md` §3 G / `ROADMAP.md` §10）；
启动器在非 Rust 仓库没有版本源（§3.3）；课程站点无书页（P5）。

## 附录 B：本轮探针证据（可复跑）

```bash
# 1) 集合论可行性（应 11 checked / 0 failed）
scripts/soko query check --file docs/gaps/repro/OK-set-spike.sokonanoda --compact

# 2) 缺口复现（各自应命中台账里写的失败签名）
bash docs/gaps/repro/G11-launcher-version-source.sh                       # 课程仓没有版本源
bash docs/gaps/repro/G12-relative-entry-ancestor-manifest.sh              # 相对路径 ⇒ 模块根空
scripts/soko query check --file docs/gaps/repro/G01-open-exercise-signature.sokonanoda --compact
scripts/soko query check --file docs/gaps/repro/G02-ctor-namespace.sokonanoda --compact
scripts/soko query check --file docs/gaps/repro/G03-prop-type-param-inductive.sokonanoda --compact
scripts/soko grade     docs/gaps/repro/G04-notation.sokonanoda            # 期望 parse 诊断 + exit 1
scripts/soko grade     docs/gaps/repro/G05-namespace-open.sokonanoda      # 期望 parse 诊断 + exit 1
scripts/soko grade     docs/gaps/repro/G08-abbrev.sokonanoda              # 期望 parse 诊断 + exit 1
bash docs/gaps/repro/G06-course-import.sh                                 # course 聚合不认 import
bash docs/gaps/repro/G13-axiom-binder-params.sh                           # axiom 不吃 binder

# 一条命令跑全部复现并对照台账（推荐）：
python3 scripts/gap.py check

# 3) G-10：同一份 parse 失败文本，两条通道答案不一致（前者假绿）
scripts/soko query check --text 'infix:50 " e " => mem' --compact ; echo "exit=$?"   # 期望: 全零 + exit 0（错）
scripts/soko grade       docs/gaps/repro/G04-notation.sokonanoda       ; echo "exit=$?"   # 期望: parse 诊断 + exit 1（对）

# 4) 对照：G-06 的同一单元在项目模式下判卷是绿的
scripts/soko query check --file docs/gaps/repro/G06-course-import/proj/U4.sokonanoda --compact
```

## 附录 C：analysis → sokonanoda 组件对照（摘要）

| 组件 | analysis 做法 | 我们的替代 | 优先级 |
|---|---|---|---|
| 章节协议 | `Section_3_1.lean` + 书号 docstring + README 三链接 | `courses/set-theory/unitN-*.sokonanoda` + 靶子节号 + README 索引 | must |
| 练习 | 练习即 `sorry`（无解答目录，答案在分支） | 练习即 `sorry` + `solutions/` 钥匙（我们已有） | must |
| 换地基 | 每章 `epilogue` 证明两套定义同构 | 单元 9（Russell/论域）+ 卷 II 的 `epilogue` 单元 | must |
| agent 纪律 | 主线程分解 + temp ≤300 行 + 1–3 行编辑环 + 经验台账 | 逐字移植，命令换成 4 个动词（`grade`/`query`/`hints`/`next`） | must |
| 质量门 | 只有 0 error/0 warning（无测试套件） | `scripts/soko gate` + golden + 课程测试 | 我们更强 |
| 自动推论 | `@[simp]`/`omega`/`rw` | 手写引理链 + hint 只给引理名 | should（教学卖点） |
| 出版 | Verso 书页 + doc-gen4 + Pages | 零构建 HTML 站点（P5） | should |
| 依赖 | Mathlib 16 GB | 无（内核自带） | 我们更强 |
| 不可移植 | `class SetTheory`/coercion/`_root_.`/`noncomputable` | 显式传参 + setoid 风格 | —— |
