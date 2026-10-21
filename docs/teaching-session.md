# 教学循环：playground.sokonanoda（agents 开课前必读） <!-- 本文提到的 `docs/**` 指针都是**历史位置**（那批设计文档已收口删除）：原文 ⇒ `git log --all -- <path>`。doc-refs: history-only -->

> 画布：仓库根 `playground.sokonanoda`（学习者与 agent 共同编辑）。
> 依据：`REQUIREMENTS.md` §6（教学工作流）；事件协议：`docs/protocol.md`。
> 本文档 = 开课手册 + 事件决策表 + 全部练习的解答钥匙（agent 专用，勿直接给学习者）。

## 0. 课程层与执行层的关系（用户原则，2026-09-07）

**`course/` 只是大模型的路线图/素材库；执行层必须按用户灵活适配。**

- course/ 的 **11 个单元** = 知识点顺序、题池、可解性守护（CI 验证解答）；
- **真正教学时，agent 必须依据当前用户的反馈历史动态重组**：
  - 用户在某概念反复出错 → 追加同概念的变式练习，或先放一个"填好
    的演示声明"再重新出题；
  - 用户进度快 → 跳过热身、合并单元；进度慢 → 拆小步、增加提示层；
  - 用户的兴趣/背景（程序员/学生/纯新手）→ 调整比喻与例子；
- 用户面对的**永远只是当前画布**（playground.sokonanoda 或其分支），
  不是 course/ 文件本身；course/ 文件永不直接丢给用户当"课程"读。

## 1. 开课手册（3 步循环）

1. **讲课**：agent 往画布里写 `--` 讲解 + 定义（演示）+ 练习（`sorry` 洞声明）。
   语法点永远先出现在讲解注释里，再出现在练习里（白名单即课程）。
2. **作答**：学习者编辑画布填洞（支持部分作答：先写几层 fun，最后一层留
   `sorry`，剩余目标会显示在 hover/诊断里）。
3. **判卷**：agent 跑
   ```bash
   SOKO="$HOME/.local/share/sokonanoda/bin/sokonanoda"   # Release 二进制，零 cargo
   "$SOKO" --json playground.sokonanoda
   ```
   （二进制的获取见 `skills/sokonanoda-teacher` §1。）
   读结构化事件（不是 exit code）决定反馈；一个练习红了不影响其他练习
   （逐声明容错）。然后再回到第 1 步追加内容。

## 2. 事件决策表（已用真实内核验证）

| 事件 / code | agent 解读 | agent 动作 |
|---|---|---|
| `decl.checked`（原练习名） | 练习解出 | 肯定 + 追加下一个概念/练习 |
| `exercise.open` 持续 | 未做/卡住；若同时有 `uses-open-exercise` warning，说明引用了未完成练习（正常状态，不是错误） | 给一层提示（画布注释里有），永不直接给答案；提醒「补完被引用的练习才算真正证完」 |
| `elab-unknown-identifier` | 拼写错（open 声明现在**进环境**，与 Lean 4 的 `sorry` 一致） | 先查 open 列表再判拼写；「先做练习 N」 |
| `elab-duplicate-declaration` | 重名 | 讲“单赋值世界”，换名 |
| `elab-hole-misplaced` | 洞不在可恢复位置（嵌套洞/非直接实参，如 `n + sorry`；答案尾巴、构造子 spine 与已知函数直接实参都合法） | 讲“洞只能放答案末尾，或已知函数/构造子的直接实参位” |
| `kernel-rejected`（def_eq failed） | 填了类型而不是证明项 / 方向反了 / 宇宙忘了 `.{1}` / 忘了 Not 会展开 | 对比声明类型与所填项的形状，让学习者逐参数预言类型 |
| `kernel-rejected`（app arg def_eq failed） | 部分应用 / 参数顺序错 | 一起数构造子签名参数 |
| 无诊断但语义不对（如 `double := fun n => n`） | 内核只判类型不判意图 | 设计“证明形状”的需求（见 two_def 模式） |

## 3. 第一课练习清单与解答钥匙

课程练习与解答钥匙已迁到 `course/` 的 unit 文件与 `course/solutions/`（本表 2026-10-22
收口；开课手册与事件决策表见 §0–§2）。**不要**把答案直接丢给学习者：按 §1 的
三步循环一次只给一层提示。

## 4. Gotchas（全部验证过，别踩）

1. **裸 `Eq` 默认 u=0**（Prop 层）；Nat 级必须 `Eq.{1}`/`Eq.refl.{1}`/`Eq.subst.{1}`。
2. ⚠ **更正（2026-09-30）**：**隐式实参可省**（R2.5 / D6 已落地，见 `docs/notes/course-lean-style/notation-rewrite-brief.md` §1.4 与 `docs/design/implicit-arguments.md`）；`@` = 显式脊（不补隐式位）。
   教学画布的 axiom 全用显式 binder（`And.intro a b ha hb` 风格），只有 prelude 的
   Eq 与毕业题 12 用隐式。
3. **open 声明进环境**（2026-10-21 对齐 Lean 4）：`sorry` 按同签名 axiom 进环境
   （Lean 是 `sorryAx`）⇒ 引用正常、goal state 照常，只给 `uses-open-exercise` warning（不是 error）。
4. **kernel 只判类型**：`double := fun n => n` 也能过；语义要求用证明形状表达。
5. **`Type n` = `Sort (n + 1)`**：`Type 0` 等于 `Sort 1`，单独一个 `Type`
   也是 `Sort 1`；`Type u`（宇宙变量）不支持，写 `Sort u`。层级算术
   （0.61.0）支持 `Sort (u+1)` / `Sort u+1` / `Type (u+1)` / `Eq.{u+1}`
   ——`Eq.mp`/`Eq.mpr`/`cast` 的签名就用它（白名单
   `docs/design/type-level-syntax.md` §5）。
6. **命名要防撞 prelude**：若画布自己声明 `Eq`/`Eq.refl`/`Eq.subst` 任一，
   整个 Eq prelude 跳过（all-or-nothing，与显式 `Nat` 块行为一致）。
7. **排中律不在课内**（`em`/`by_contra` 在白名单外）。`Or` 在**单元⑨**升级为
   真正的 inductive（前端自动派生消去子 `Or.rec`），`or_comm`/`or_elim`/`or_id`
   都是该单元的练习；**单元⑨之前**（画布第一课、单元①–⑧）不要许诺 `Or.rec`。

## 5. 后续课程（单元④–⑪已上线；⑪ = `course/unit11-modules-projects/`）

`by` 写法（tactic 证明）与值位 `intro`/`apply` 对照在 **course/ 单元④**
（0.18.0 起，P2 从旧单元⑥提前）；宇宙（`Sort n`/`Type n` 阶梯、`Eq.{1}`
由来、`#check` 读输出）在 **单元⑤**。显式 `inductive Nat` 块 +
`Nat.rec`/iota 归纳、`match` 分情况/递归在 **单元⑥**（归纳与递归 Ⅰ）；
参数化 `Option`/依赖 match/嵌套与通配模式/带索引 `Vec` 在 **单元⑦**
（归纳与递归 Ⅱ）。手写 `eq_trans`（以及 `eq_symm`、等式接力）在
**course/ 单元②**。量词（`forall`/`Exists`，Person 论域 + `Exists`
公理三件套）在**单元⑧**（0.26.0 起），就是画布的第二课。关系与联结词
（`Or` 升级为真 inductive + 自动派生 `Or.rec`、`Iff` 定义、`Le`/`Even`
归纳关系 + 手写消去子）在**单元⑨**（P3，2026-09-16 上线，见“第三课”）；
读证明与综合（自解释三问、formal↔informal 互译、评阅错证明、期末小项目）
在**单元⑩**（P3）——`or_comm`/`or_elim`/`Or.rec` 都从单元⑨起才出现，
单元①–⑧没有。语料参照 `examples/py-nat.sokonanoda`
与 `examples/fol-basics.sokonanoda`。
