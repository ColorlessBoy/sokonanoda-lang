# 全课程 Lean 4 化（0.62.0 批次）——给站点 / 文档 agent 的事实清单

> **这份文档是给"要写别人读的东西"的 agent 的输入**（站点重构、发布说明、教程）。
> 它把这一批**已经实现并验证**的功能特性与课程事实列成一份可引用的清单，
> 免得看代码或旧文档猜。
>
> **⚠️ 版本状态**：下面这些都在**工作树**里，对应 **0.62.0**；截至 2026-09-21
> 最后一次已知**已发布**版本是 **0.61.0**。站点写"已发布版本的事实"时，**先确认
> 0.62.0 是否已发**（`scripts/soko version --json` + `git log`），不要把未发布行为
> 写成线上事实。这条陷阱的完整说明在 `docs/design/site-rebuild/STATE.md` #13
> （「本工作区 `crates/` 是脏的，`scripts/soko` 量的是未发布代码」）。
>
> **权威出处优先于本文**：本文只做导览，冲突时以各设计文档 + 代码 + 判定为准。

---

## 1. 用户可见的特性（这一批新增/变更）

| # | 特性 | 怎么用（最小例子） | 权威出处 |
|---|---|---|---|
| 1 | **逻辑连接符是语言内建记法**，任何文件零声明可用 | `a ∧ b` / `a ∨ b` / `a ↔ b` / `¬ a` / `a → b` / `a = b` / `a ≠ b`；全称是关键字 `∀ (x : A), p x` | `docs/design/notation-subset.md`；测试 `crates/cli/tests/notation.rs::core_logic_notation_is_built_in` |
| 2 | **`→` 是 `->` 的词法别名** | 两种写法同一个词，可混用 | 同上 |
| 3 | **`∃` 不是内建**：由课程标准库用 `binder_notation "∃" => Exists` 声明 | 卷 I：`import lib.Set`（经 `lib/Exists`）后写 `∃ (x : α), p x`；入门课单元⑧ 在自己的文件里声明一行 | `courses/set-theory/lib/Exists.sokonanoda`；`docs/design/notation-subset.md` §10 |
| 4 | **集合论符号由课程库声明**（不是 prelude） | `∈ ⊆ ∪ ∩ \ ∅ 𝒫 ᶜ '' ⁻¹' ×ˢ`，声明点在 `courses/set-theory/lib/Set.sokonanoda`，**随 `import` 传播**到入口文件；同一符号全课程只声明一次 | 同文件顶部注释；测试 `notation.rs::the_shipped_course_library_declares_the_five_symbols` |
| 5 | **tactic 白名单**（`by` 块里可用） | `intro` / `exact` / `apply` / `assumption` / `rfl` / `match` / `constructor` / `left` / `right` / `use` / `exfalso` / `cases` / `have` / `⟨a, b⟩` / `sorry` | `docs/design/by-tactics.md`（§12 有本批的 as-built）；`skills/sokonanoda-teacher/SKILL.md` |
| 6 | **`by` 块的 tactic 分隔**：`;` 或**换行**（可混用），不缩进敏感 | `theorem t : A → A := by` ⏎ `  intro h` ⏎ `  exact h` | `docs/design/by-tactics.md` §11 |
| 7 | **练习占位统一 `:= by sorry`**（`:= sorry` 事件流逐字节相同，二者都合法） | `theorem ex : A ∪ B = B ∪ A := by sorry` | `docs/design/course-lean-style.md` §1.2（实测）、D3 |
| 8 | **隐式实参（路线 C）**：签名有前导隐式 binder 时，点名叫法可以省掉它们；`@` 关闭插入 | 签名 `def id2 {α : Type} (a : α) : α` ⇒ 可以写 `id2 a`；`@id2 a` 表示"别插" | `docs/design/implicit-arguments.md`（§9 as-built） |
| 9 | **新诊断码 `elab-implicit-argument-unsolved`**（隐式实参补不出来时**单独一条**，不再混进泛化错误） | 见 `docs/protocol.md` 的码表 | `docs/protocol.md`；`crates/front/src/compile/error.rs` |
| 10 | **记法输入提示**：hover 显示"这个符号怎么敲"（`\in` / `\sub` …） | 编辑器 hover 一个 `∈` | `docs/design/notation-input.md`；VS Code 的缩写改写器（NI-2） |
| 11 | **判定侧修复（用户可感知）**：① 判定合成声明**携带声明的宇宙参数**（目标含 `Sort u`/`Eq.{u}` 时 `by` 可用）；② `cases` 的头解析**认记法**（库定义体写记法后 `cases` 不再被打断）；③ binder 记法的实参位两处同款 | 例：`theorem Eq.symm {u} : … := by …` 现在能过 | `docs/design/by-tactics.md` **§12**（三处 as-built，各有回归测试） |
| 12 | **`kernel-expected-sort` 的提示会指根因**：点名调用漏了前导类型参数时（`Set.mem a A`），提示直接说"漏了前导类型参数 + 可用记法" | 见 §3 的"仍欠"：`by` 路径那一半还没修 | `crates/front/src/compile/error.rs`；缺口台账 **G-21**（`docs/gaps/ledger.jsonl` + `docs/gaps/repro/G21-omitted-type-argument.sh`） |

## 2. 课程内容的**事实变化**（站点课程页要跟着改的点）

| 事实 | 现在的值（2026-09-21 实测） | 怎么复现 |
|---|---|---|
| **卷 I（`courses/set-theory/`）已全面 Lean 化**：连接符用记法、解答全 `by` tactic、练习占位 `by` 形式 | 门禁 **36 个目标 · 328 checked · 99 open · 0 判负** | `python3 courses/set-theory/tools/check.py` |
| **入门课（`course/`）已全面 Lean 化** + 练习占位统一 `:= by sorry` | 课程汇总 **checked 54 · open 66 · failed 0**（11 单元） | `scripts/soko course "$PWD/course/course.json" --json` |
| **`playground.sokonanoda` 已 Lean 化** | `decl.checked 30 · example.checked 2 · exercise.open 4`（+1 条既有 warning：`axiom Prop` 撞内核保留名） | `scripts/soko grade "$PWD/playground.sokonanoda" --json` |
| **入门课不再自建 `And`/`Or` 骨架**：`And`/`Or`/`Not`/`Iff` 一律用 **prelude 的真归纳**（C2.5，2026-09-21 用户拍板）。后果：`constructor`/`cases`/`left`/`right` **全课程可用** | `course/shared/{And,Or}.sokonanoda` 两个规范模块**已删除**，只剩 `Nat`（8 份副本）；`course/shared/Demo.sokonanoda` 只 `import Nat`，And/Or 两条演示改用 prelude 真归纳 | `bash scripts/soko grade "$PWD/course/shared/Demo.sokonanoda"` |
| **入门课里 `True`/`False` 仍是自建 `axiom`**（单元① 用它讲"`axiom` 是什么"）；单元⑧ 的 `Exists` 仍是自建公理 | 见各单元头注释 | `scripts/soko grade "$PWD/course/unit1-propositions-proofs.sokonanoda" --json` |
| **单元④ 是唯一教 `by` 的单元**（其余单元的画布演示**保持项模式**，那是教学点本身） | 见 `docs/design/course-lean-style.md` §C2.2 | — |

> **⚠️ 旧数字别再用**：站点或文档里如果还写着「卷 I 329 checked」「入门课 87 checked /
> 65 open」「`And` 25 份 / `Or` 12 份拷贝」「每单元自带 `axiom And`/`Or` 块」，
> 那些都是**改造前的值**，现在都变了。（这些数字**不要在任何地方再抄一份**——每次现算，
> 命令见上表。）

## 3. 站点不该误解的三件事（我**没有**改 `site/`，这是给你们的输入）

1. **`by` 路径的一条报错仍欠**（台账 **G-21**，状态 `open`）：点名调用漏前导类型参数时
   （`Set.mem a A`），**声明位**已经报得准（`kernel-expected-sort` + 指根因的提示），
   但**写在 `by` 块里**的那种（`… : Set.mem a A → A a := by intro h; exact h`）仍报
   "同形对照"（`` `exact` 类型不匹配：期望 `A a`，实际是 `Set.mem a A` ``）。
   写教程/FAQ 时**别把这条说成已修**；`docs/gaps/repro/G21-omitted-type-argument.sh`
   会告诉你当前是哪一半。
2. **记法在"实参位 + 操作数是记法或集合字面量"时补不出类型参数**
   （`elab-notation-argument-unsolved`）：`Set.image Two Two f1 ({aa} ∩ {bb})` 报错，
   而单独写 `{aa} ∩ {bb}` 没问题。课程里遇到就写点名形式。
3. **`notation-cheatsheet` 那一页**（卷 I 的 `units/notation-cheatsheet.sokonanoda`）
   **故意**并列"点名形式 ↔ 记法"两种写法——那是**教学装置**（"这个符号底下站着什么"），
   不是没改完的残留。别把它当"还有一处旧写法"来报。

## 4. 这一批的语言改动**只碰了 `crates/front`**；`crates/kernel` 零改动

- 红线（`REQUIREMENTS.md` §2 硬规则 1）：内核冻结。这一批所有语言侧改动都在
  `crates/front/src/{by.rs, compile/elab.rs, compile/error.rs, compile/check/*.rs}`，
  `git diff --stat -- crates/kernel/` 必须为空。
- 判定永远走内核（硬规则 8）：所有"改对了"的结论都由 `scripts/soko grade` 的退出码
  与事件流给出，**没有**文本比对。

## 5. 一条命令确认这份文档没过期

```bash
scripts/soko gate            # fmt + clippy + test + playground 锚点 + 卷 I 课程门禁 + 缺口台账门禁
python3 courses/set-theory/tools/check.py   # 卷 I：36 目标 · 328 checked · 99 open · 0 判负
scripts/soko course "$PWD/course/course.json" --json | tail -1   # 入门课汇总：checked 54 · open 66
```

数字对不上就是这份文档过期了——**以命令输出为准**（见 §2 的警告）。
