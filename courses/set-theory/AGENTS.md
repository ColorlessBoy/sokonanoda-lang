# AGENTS.md —— 课程线 agent 手册（卷 I 集合论）

你在语言仓 `sokonanoda-lang` 内的课程目录 `courses/set-theory/` 工作。
语言侧的一切（判卷器、台账工具、缺口实现）都在同一棵树里。

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
