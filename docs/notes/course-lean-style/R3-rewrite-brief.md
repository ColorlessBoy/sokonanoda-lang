# R3 改写手册：入门课 `course/`（CN+EN，画布+解答）

> 给执行 R3 的 subagent / 人的**操作手册**。设计依据：`docs/design/course-lean-style.md`
> §C2（C2.1–C2.11）、§C4（记法边界）；记法集见 `docs/design/notation-subset.md`。
> **已完成的样板 = 单元①**（CN/EN 画布 + CN/EN 解答）：照它改。
>
> **判据只有一条**：`scripts/soko grade <绝对路径>` 退出码 0，且**事件计数不变**
> （见 §5）。**内核零改动**。

---

## 1. 这一轮改什么、不改什么

| 改 | 不改 |
|---|---|
| 代码里的**连接符**：`And a b` → `a ∧ b`、`Or a b` → `a ∨ b`、`Not a` → `¬ a`、`->` → `→`；`Iff`（若本文件有）→ `↔` | **引理/构造子名**照旧点名（`And.intro`、`Or.inl`、`False.rec`）——记法是连接符的糖，不是引理名的糖 |
| **解答**（`course/solutions/*`、`course/en/solutions/*`）的证明改成 `by` tactic 块；**练习占位一律 `:= by sorry`**（设计 **D3**：`:= sorry` → `:= by sorry` 事件流逐字节相同，零风险；第 114 轮实测入门课 104 处） | **画布**里的**演示**（`example`、`demo_*`）：单元①②③⑤⑥⑦⑧⑨⑩⑪ 的**教学点就是项模式**（设计 C2.2），演示**保持项模式**（只换记法） |
| 骨架块（`axiom And…`、`inductive Or…`）的**结果类型**用记法（`a ∧ b` / `A ∨ B`） | **骨架的删除**（设计 C2.5）**本轮不做**——要拍板，且会打红 13 个计数测试 |
| 注释里的陈词（"本教学语言没有 ↔ 记号"这类）——**只在你自己那一步顺带** | `#check` / `#reduce` 演示行（入门课的教具） |
| `course/shared/*` 的规范副本**必须同轮同步**（`course_shared.rs` 会拦） | 文件的**声明顺序、声明名、练习题号**（`solution_covers_every_canvas_exercise` 按名判） |

**tactic 白名单（本课）**：`intro` / `exact` / `apply` / `assumption` / `rfl` / `have` / `sorry`。

**按单元不同**（**判卷器说了算**：能过就用，不能过就点名写）：

| 单元 | 骨架形态（实测） | 可用的额外 tactic |
|---|---|---|
| ①④⑧ | `axiom And` + `axiom Or` | **无**（`constructor`/`cases`/`left`/`right` 都不可用——要真归纳） |
| ⑨⑩⑪ | `axiom And` + **`inductive Or`** | **`left` / `right` / `cases`**（`Or` 是真归纳；`And` 上仍然不行） |
| ②③⑤⑥⑦ | 无 And/Or | 无 |

`And.intro` / `And.left` / `And.right` **位置参数个数与 prelude 的真归纳一致**（4/3/3），
点名照写即可；`Or.inl` / `Or.inr` 亦然（4 参）。⚠️ 不要用 `exfalso` / `use`（本课
没有 `False.elim` 的 tactic 封装与 `∃` 的 `use` 支持面，点名写 `False.rec` / `Exists.elim`）。

**记法可用面（实测，按单元）**：`∧ ∨ ↔ ¬ → = ≠` **全课可用**（内建；`↔` 的目标是本文件
自己声明的 `Iff`，⑨⑩⑪ 有）；`∀` 是关键字 ✓；**`∃` 不是内建**——单元⑧ 想写
`∃ (x : Person), P x` 必须在本文件加一行 `binder_notation "∃" => Exists`
（记法声明**不产生事件**，counting-neutral ✓，设计 G-04）。`∈ ⊆ ∪ ∩ 𝒫` 等**卷 I 的
lib 记法入门课用不了**（没有 `import`）。

---

## 2. 记法表（本课可用）

| 记法 | 目标 | 例 |
|---|---|---|
| `∧`(35) `∨`(30) `↔`(20) `¬`(40) `=`(50) `≠`(50) | **内建**，零声明 | `a ∧ b`、`¬ a`、`a = b` |
| `→` | `->` 的**词法别名** | `(a : Prop) → a` |
| `∀ x, p x` | 关键字（声明级 binders 照旧 `(a : Prop)`) | `∀ (n : Nat), p n` |
| `∈ ⊆ ∪ ∩ \ ∅ 𝒫 ᶜ '' ⁻¹'` | **卷 I 的 lib**（`courses/set-theory/lib/Set.sokonanoda` 等） | ⚠️ **入门课没有 `import`**（`course_shared.rs` 会拦）⇒ **入门课用不了这些**，除非本文件自己声明 |

**优先级**（记法都是语言内建/自带，不需要声明）：`¬`/`𝒫` 100 > `''` 80 > `∩`/`\` 70 > `∪` 65 > `=`/`≠` 50 > `∧` 35 > `∨` 30 > `↔` 20。
⇒ `a ∧ b ∨ c` 读作 `(a ∧ b) ∨ c`；有疑问就**加括号**（课程代码优先可读性）。

**改写顺序**（建议）：先把**语句/签名**改完（计数不变，随时可判），再改**解答的证明体**
（改一条判一条：`scripts/soko grade "$PWD/course/solutions/<file>"`）。

---

## 3. CN/EN 同步（设计 C2.1）

CN 与 EN 的**代码逐字相同**（只有注释不同；unit1 的 EN 多 3 个空行）。
⇒ **先改 CN，再把代码块原样搬进 EN**，注释**分别**润色（EN 是按语义重构的，不是翻译）。
判据：`python3 -c "..."` 去注释后 diff（或直接看 `course.rs` 的
`en_mirrors_match_chinese_event_counts` + `course_shared.rs` 的副本检查）。

**CN/EN 事件计数必须相同**（两个测试钉着：`en_mirrors_match_chinese_event_counts`、
`en_solutions_match_chinese_event_counts`）。

---

## 4. 每步的验收命令

```bash
scripts/soko grade "$PWD/course/<unit>.sokonanoda" --json      # 画布（CN）
scripts/soko grade "$PWD/course/en/<unit>.sokonanoda" --json   # 画布（EN）
scripts/soko grade "$PWD/course/solutions/<unit>-solution.sokonanoda" --json
scripts/soko grade "$PWD/course/en/solutions/<unit>-solution.sokonanoda" --json
cargo test -p sokonanoda-cli --test course --test course_shared   # 计数/镜像/副本
```

**判卷纪律**（`courses/set-theory/AGENTS.md` 同款）：一律**绝对路径**；只认 `grade` 的
**退出码**与 `--json` 事件流；`span.start.line` 才是行号（**别**拿字节 offset 切字符）。

---

## 5. 计数纪律（**最容易翻车的地方**）

`course.rs` 的 `GOLDEN`（11 条 `(checked, open, reduced)`）与
`course_status.rs`、`cli.rs:2097` 的 **86/65** 是**契约**。

- **纯记法改写是 count-neutral 的**（单元① 实测：13/6/1 改前改后逐项相同）⇒
  正常改写**不该**动 GOLDEN。
- **动了就说明你改了语义**（多/少一条声明、把一个练习填成完整证明、删了一个
  `#reduce`）。此时**必须**从内核重新取数再钉，**不许手算**：
  ```bash
  scripts/soko course "$PWD/course/course.json" --json | python3 -m json.tool
  ```
  并把新数字同时更新 `course.rs`/`course_status.rs`/`cli.rs`。
- **不要**为了凑计数而删练习或改 `course.json`（那会让 `solution_covers_every_canvas_exercise`
  或 `course_json_lists_the_eleven_units_in_order` 翻）。

---

## 6. 单元④ 是**特例**（设计 C2.6，单独一刀）

`unit4-by-tactics.sokonanoda` 教的就是 `by`：它的**画布演示要改成 `by` 写法**
（其余单元的演示保持项模式），且：

- 今天的顺序是 `intro → exact → assumption → apply → rfl`，练习
  `by_ex1`(:54) / `by_ex2`(:60) / `by_ex3`(:66) / `by_ex4`(:72) / **`by_ex6`(:83，编号跳过 5)**；
- 解答里 `by_ex6` 回退成项模式，另有 **9 条画布上没有的遗留声明**
  （`h_s`/`Pfam`/`Qfam`/`f_dep`/`qfam_true`/`val_apply_imp`/`val_apply_dep`/`by_ex7`/`by_ex8`）；
- ⇒ **补上缺的 5 号**、**清掉遗留声明**、tactic 顺序重排（新白名单多了 `have`）。

单元④ 单独一轮做，做完**重新取数**再钉 GOLDEN。

---

## 7. 阻塞报告格式（撞到引擎缺口时照抄）

```
文件:行        想写的形状        期望（Lean 4 语义）      今天的行为（原始诊断一行）
最小复现       6–12 行，能单独 grade 出红
```
先写进 `docs/notes/course-lean-style/R3-blockers.md`（不存在就建），**不要**自己绕过去——绕法要写进文件头注释，像 `unit12-solution.sokonanoda` 那样。

---

## 8. 单元④ 的 C2.6 专项（**四条已全部做完**）

**已做（第 112/113 轮）**：代码换记法（`Or True False` → `True ∨ False`、`And a b` → `a ∧ b`、
`->` → `→`），三条演示拆成多行 `by` 块。

**四条结构专项也已落地（第 113 轮，画布 `(13,5,0)` → `(14,6,0)`）**：

| # | 动作 | 结果 |
|---|---|---|
| 1 | **补练习 5**（教 `have`） | `by_ex5 : (a : Prop) → (b : Prop) → (a → b) → a → b ∧ a`，解答用 `have hb : b := f ha` |
| 2 | **清解答的 9 条遗留声明** | `h_s`/`Pfam`/`Qfam`/`f_dep`/`qfam_true`/`val_apply_imp`/`val_apply_dep`/`by_ex7`/`by_ex8` 全删（解答 `checked` 29→20） |
| 3 | **tactic 教学顺序重排** | 新增演示 `demo_by_have : (a : Prop) → a → a ∧ a`（`have h : a := ha` 再 `exact And.intro a a h h`） |
| 4 | 「首期五个 tactic」措辞 | 改成「本单元教**六条**」+ 白名单说明 + 「其余 tactic 随后面的类型解锁：⑧ `use`、⑨ `left`/`right`/`cases`」 |

⚠️ **计数钉了四处**（改前必读）：`course.rs` 的 `GOLDEN` 第 4 项、`course_status.rs`
的逐单元表第 4 行 + summary 的 `87/66`、`cli.rs` 的 warm-cache `87/66`。

⚠️ **本课 `Or` 的两种形态**：单元④ 是 **`axiom Or`**（与 ①⑧ 同），而 ⑨⑩⑪ 是
**`inductive Or`** ⇒ `left`/`right`/`cases` 只在 ⑨⑩⑪ 可用（实测）。单元④ 的 `Or`
相关的练习照旧点名 `Or.inl` / `Or.inr` + `apply`。

---

## 9. 收尾（第 113 轮：subagent 半途停掉后的接管记录）

**发生了什么**：按本手册派出的四个 subagent（单元 2/3/5、6/7、8、9/10/11）里
**只有单元 8 跑完并回报**，其余三个**在 token 配额耗尽时停掉、没有收尾消息**。
所以本轮后半段是把它们留下的**半成品**接管并验完的——过程本身值得记下来，因为
「没有收尾消息」不等于「没改文件」：

| 症状 | 怎么发现的 | 处理 |
|---|---|---|
| 文件**改了一半**：计数对、能判卷，但还有旧写法残留 | **计数中性只能证明「没改语义」，不能证明「改完了」**。写脚本按「剥掉 `--` 注释后扫旧写法（`->`、`And X Y`、`forall`、`Exists X Y`）」逐文件列残留 | 逐条补：`unit11-project/`（**整目录漏改**）、单元⑥/⑦ 的 `Nat.rec` 块 `->`、单元⑤ 解答的 `Eq.symm` 项模式证明、单元⑩/⑪ 解答的 `Or B C` |
| 解答里**写重了一份**（我自己的编辑失误） | 判卷计数 `EN=29 vs CN=20` 一眼看出 | 删重复块 |
| subagent 之间**碰不到的地方**（`course/shared/Nat.sokonanoda` 规范副本、`playground`） | `course_shared.rs` 当场打红 | 规范文本与 8 份副本同轮换记法；playground 的 `∃` 段补记法声明 |
| 改写**反过来要求语言改动** | 判卷器报 `universe variable u is not declared` 与「被匹配项必须是书写类型为 `Or …` 的局部变量」 | 见 `docs/design/course-lean-style.md` §9「R3」的两处语言刀（kernel 零改动） |

**给下一次的教训（已生效的纪律）**：
1. **验收不看 subagent 的自述，只看三件事**：① 每个文件 `grade` 退出码 0 且诊断 0；
   ② 计数与 `GOLDEN`/`course_status` 逐项一致；③ **CN/EN 代码逐字节一致**。
   前两条证明「没改坏」，第三条证明「没改歪」——但**都不证明「改完了」**，所以要
   第 ④ 条：**旧写法残留扫描**（脚本化，别靠眼睛）。
2. **计数中性是必要条件、不是充分条件**：纯记法改写必然计数中性（单元① 已验证），
   所以「计数没动」只能说明你没改语义；漏改的文件同样计数中性。
3. **接线处最容易漏**：不在 `course.json` 里的 `unit11-project/`、只被
   `course_shared.rs` 逐字守着的规范副本、根目录 `playground.sokonanoda`——三处
   都不在任何一次「单元改写」的清单里，要单独点名。
