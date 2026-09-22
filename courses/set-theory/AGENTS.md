# AGENTS.md —— 课程线 agent 手册（卷 I 集合论）

你在语言仓 `sokonanoda-lang` 内的课程目录 `courses/set-theory/` 工作。
语言侧的一切（判卷器、台账工具、缺口实现）都在同一棵树里。

## 记法规则（2026-09-21 起，硬规则，脚本判红）

**课程一律写 Lean 4 记法，不写「点名 + 前导类型/宇宙实参」的旧写法。** 判据一条命令：

```bash
python3 scripts/notation-lint.py                 # 全课程（卷 I + 入门课 + playground）
python3 scripts/notation-lint.py --root <file>   # 单文件
```

- `Eq.{1} T a b` → `a = b`（用户原话：`Eq.{1}` 直接就是一个等于号）；`Ne` 同理；
  `And/Or/Iff/Not/forall/Exists/->` → `∧ ∨ ↔ ¬ ∀ ∃ →`；`Set.*` → `∈ ⊆ ∪ ∩ \ ᶜ 𝒫 ∅ '' ⁻¹' ×ˢ {a} {a,b}`。
- 基础类型**省前导隐式实参**（对齐 Lean）：`And.intro h1 h2` / `And.left h` /
  `Or.inl h` / `Exists.intro w hw` / `Exists.elim h f`……能判绿就省。
- **代码与注释（含 `-- soko:hint`）都算**；`units/notation-cheatsheet*.sokonanoda`
  整文件豁免（它是教学装置）。
- **明说的边界**（保留点名 + 行内 `-- soko:notation-ok: <理由>`）：等式族证明项
  （`Eq.refl/symm/trans/subst`、`congrArg`）的宇宙层级；`congrArg` 参数顺序是
  Lean 的 `{α β} {a b} (f) (h)`；`Set.univ α`；
  `intro` 派生的目标/假设、`Exists`-headed def、嵌套 `Exists.elim`、
  复合记法操作数（`{aa} ∩ {bb}`、字面 λ 的 `''`/`⁻¹'`）、`And.left h x` 续应用等。
  **`by rfl` 已能认 `=` 记法目标**（2026-09-21 修）。
  细则见 `docs/notes/course-lean-style/notation-rewrite-brief.md`。
- **画布（`units/*.sokonanoda`）里的 tactic 块不动，term 保持 term**；纯记法改写必须
  **计数中性**：门禁仍是 `36 目标 · 328 checked · 99 open · 0 判负`。

## 解答写法：**项风格**（2026-09-21 用户拍板，**覆盖**早先的"tactic 先不动"）

> 「tactic 先不动」是我写的，但是现在看性能差太多了，solution 没必要损失性能。

`solutions/*.sokonanoda` 一律写**项风格**（lambda / 直接给证明项），**不用 `by`**。

**为什么**：`by` 块的判定代价是 **O(前缀 × `by` 块数)**——每个 `by` 块都要把整份
前缀重新 elaborate 一遍。实测 `unit12-solution`（25 个 `by` 块）冷跑 **120 秒**，
其中判定占 **68%**；同一批定理写成项风格实测快 **8.4×**，`unit12-solution` 那种
大文件 **25×**（`docs/PERF.md` 有分阶段表）。

**解答同时是"提示"**：学习者做画布上的题卡住时，老师（LLM）读解答拿到**证明骨架
与关键件**，再用 tactic 写法讲给学习者听。两边一一对应：

| 项风格 | tactic 写法 |
|---|---|
| `fun (h : P) => e` | `intro h; exact e` |
| `f a b` | `apply f; exact a; exact b`（或 `exact f a b`） |
| `Iff.intro P Q h1 h2` | `constructor; exact h1; exact h2` |
| `Set.ext α A B (fun x => …)` | `apply Set.ext; intro x; …` |

写解答时**必须**：
1. 声明签名逐字不变（只换 `:=` 后面的证明项）；
2. 不用 `sorry`、不削弱命题；
3. 保留全部教学注释与 `-- soko:notation-ok: …` 标记；
4. 判绿（`node scripts/soko grade "<绝对路径>"` 退出码 0、无诊断）。

**已知陷阱（G-30）**：`Iff.intro` / `And.intro` 的分支里含集合字面量（`{a}`）时，
**前导 Prop 实参必须显式写出来**（`Iff.intro ({a} = {b}) (a = b) …`），否则报
`期望 Sort(0)，实际是 (Set.[] $2)`。

## 写作循环（每个单元一轮）

1. 读大纲（`docs/design/set-theory-syllabus.md` §3 的单元表）与分层判据
   （`docs/design/course-stdlib.md`）：**先决定这道题是 L2（进库）还是 L3（练习）**；
2. 写画布：演示（已证）+ 练习（`sorry`）+ `-- soko:hint` 三段；
3. 写解答并判卷：`node scripts/soko grade "$PWD/courses/set-theory/units/solutions/<file>"`；
4. `course.json` 加行；跑 `python3 courses/set-theory/tools/check.py`；
5. 撞到缺口 → `gaps/` 记一条（最小复现 + 期望的 Lean 4 语义 + 今天的表现），
   再收编进 `docs/gaps/ledger.jsonl`（`python3 scripts/gap.py list` 看全貌）；
6. 收尾：README 的"现状"表 + `docs/design/teaching-project.md` 的进度。

## 判卷纪律（三条，全部有实测教训）

- **判据用 `grade` 的退出码**判"有没有坏"——课程门禁保持这一条不变；G-10 已修
  （≥0.59.0）：`query check` 对解析失败也带 parse 诊断并 exit 1，可以拿它与 `grade`
  交叉复核（两条通道同口径）；
- **一律给绝对路径**（台账 G-12）；
- **span 可信，量具要选对**：`span.offset` 是**字节**偏移，`span.start/end.line`/`column`
  才是内核给的行列——要行号就直接用事件自带的 `line`/`column`（`query check` 的
  `failed[].start_line`/`start_col` 同一批数字），**别**拿 offset 去切字符（中文、`α`
  是多字节，量出来会偏 —— 台账 G-15 原来记的「span 漂到后面的声明」就是这么来的假象）。
  二分法（"截断到第 N 个声明再判卷"）留作解析失败/多条错误时的定位手段。

## 写作纪律（教学侧）

- **答案绝不进 hint**：关键件只写"触发条件 + 该用哪条引理"；
- **每句教学主张要么给出处、要么标"设计判断"**（大纲 §1.4 引用纪律：有序对与选择公理
  **没有**实证研究，不得写成"研究表明…"）；
- **提示密度随单元递减**（前三个单元每题都有 hint，中段只给关键件，后段只给思路）；
- 单元开头声明"本单元只允许用什么"（对应 stg4 的逐步解锁）。

## 零 cargo

课程线与语言线都用 `scripts/soko`（版本锁定、幂等）。不要为跑课程装 Rust；
需要改语言时才进贡献者路径（`scripts/soko gate`）。
