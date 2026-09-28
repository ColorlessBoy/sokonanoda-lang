# HANDOFF-0.77 —— 交接书（2026-09-28 · v0.77.0 已发布）

> **读者**：接手 v0.78 / ST6·ST7·ST9·ST11 的新会话。**先读 `AGENTS.md`**，再读本文。
> 本文只写「现在在哪、下一步怎么走、哪些坑别踩」——**过程不在这里**（在 commit message 与 `STATUS.md`）。

## 1. 当前状态（v0.77.0 已发布 ✓）

- **版本**：`0.77.0` 已发布（tag `v0.77.0` → `70d3eca7`；release workflow `36382099817` **11/11 success**）。
- **发版依据轮**：CI `36378945287` ⇒ `scripts/ci-green.py --run` **exit 0**（重活 10/10 实跑且 success）。
- **课程门禁**：**43 个目标 · 375 checked · 99 open · 0 个被判负**。
- **内核**：`git diff crates/kernel/` **为空** ✓（v0.77.0 一行内核判定都没改）。
- **v0.77.0 的验收产出物** = **`docs/design/v077-kernel-deficiencies.md`**（内核不足清单，
  3 blocker + 5 painful；每条带自断言复现件）。

### 1.1 已完成章节

ST1（边界决策）· ST2（`Quot` 进源语言）· ST3（分离）· ST4（集族并交）· ST5（不交并/函数空间）·
ST8（序数谓词式）· ST10（基数 = 类型的商）· ST12（选择公理）· ST13（ZF 公理表）· ST14（`propext`/`funext`）·
ST15（不足清单）。

### 1.2 卡住的章节（**blocked-by-kernel**，别硬做）

| 章节 | 内容 | 被谁挡住 |
|---|---|---|
| **ST7** | 秩 `rank` | **G-56 + G-58**（良基递归 + 大消去，两条都要） |
| **ST6** | 传递闭包（递归定义） | G-56 |
| **ST9** | 超限递归 / `V` 层级 | G-56 + G-58 |
| **ST11** | 序型 / Aleph / ω₁ / Cantor 正规形 | G-56 + G-58 |

## 2. 内核缺口现状（**这是本版最值钱的部分**）

### 2.1 带索引归纳类型：**两道门**（本版最关键的发现）

**目标**：`Acc`（Lean core `src/Init/WF.lean`）—— 良基递归的地基，ST6/7/9/11 全要它。

**写法 A（下标写块头）**：`inductive Acc (α : Type) (r : α → α → Prop) (x : α) : Prop` ⇒ 撞 **G-56**。
- 判定本体：`crates/kernel/src/inductive.rs:153`（`check_uniform_inductive_occurrences_at`），
  **唯一调用点** `:44`，前端**零镜像**。
- **精确机制（插桩实测，2026-09-28）**：`_at` 只在 `args_rev.len() <= num_params` 时进断言。
  对 `Acc`（出现 `Acc α r y`，2 个实参 > `num_params=1`）**卫为假 ⇒ 完全跳过**。
  **判红其实来自另一个块**（`num_params=3`、单构造子）：它的 3 个实参是
  `Var(4) · Var(3) · Var(1)`（期望 `4,3,2`）⇒ 前两个是参数、第三个不是 ⇒ 断言触发。
- ⚠ **放宽不是挪一个比较符**：把卫改成 `>=`（只校验**前** `num_params` 个实参）
  **会打破上面那个今天能过的块**（它原本靠"跳过"过关）⇒ **实测判红**。
  ⇒ 正解要**区分「参数位」与「指标位」两段语义**，并保留"末位非参数时跳过"那一档的既有行为。
- **分支状态**：`kernel/g56-indexed-inductives`（本地，**未推**）—— 里面是那次的实验
  （含正确的 `skip(len - num_params)` 修正），**不是可合入状态**，只作参考。

**写法 B（下标写返回位，Lean 官方写法）**：`inductive Acc (α : Type) (r : α → α → Prop) : α → Prop`
+ `ctor Acc.intro (x : α) (h : ∀ y, r y x → Acc α r y) : Acc α r x` ⇒ 撞 **G-64**。
- **它过了** uniform 与 `SPEC0`（实测 `local_params.len=2 indices.len=1` ✓）——
  **比写法 A 窄得多**，只剩最后一步。
- **卡点**：`crates/kernel/src/expr.rs:381` 的 `subst_expr_levels`：
  ```rust
  if ks == vs || self.read_levels(ks).is_empty() {
      assert_eq!(self.read_levels(ks).len(), self.read_levels(vs).len());  // ← G-64
      return e;
  }
  ```
  `ks` 空（目标常量无宇宙参数）、`vs` 有 1 个（递归子多出的 motive 层级）⇒ `left: 0 / right: 1`。
- **✅ 已验证的修法（实测有效，但只推进到下一道门）**：把 `ks.is_empty()` 那一支**单独提前返回**，
  不再顺手断言长度（`vs` 多出来的层级**没有消费者**，多几个都不改变结果）：
  ```rust
  if ks == vs { return e; }
  if self.read_levels(ks).is_empty() { return e; }   // 无可代入 ⇒ 原样返回
  if let Some(cached) = … { return cached }
  self.expr_cache.subst_cache.clear();
  assert_eq!(self.read_levels(ks).len(), self.read_levels(vs).len());  // ks 非空仍判红 ✓
  ```
  改完实测：`left: 0 / right: 1` **消失** ✓，但 variant ② **换成 G-59 的判红**：
  「期望 `Pi (α : Sort(1)), … Pi (motive : …, Sort(0)), …`，实际是 `…, Sort(2)`…」。
  ⇒ **G-64 是第一道门，G-59 是下一道**。补丁原文见 `/tmp/g64_patch.diff`（本次会话）与本文 §2.2。
- **反向判据**（改这条时必须配）：`ks` **非空**且长度与 `vs` 不等 ⇒ **仍须判红**
  （那一支的断言要留着）。

### 2.2 大消去（G-58 / G-59）：`Acc` 之后、`rank` 之前的门

- **G-59**：`large_elim_test` 对 `Acc` 这种「单构造子 + 非 Prop 字段恰好是指标」的 Prop 块
  **没放行大消去** ⇒ 递归子 motive 被钉死在 `Sort 0` ⇒ 用户写 `Sort 2` 的 motive 判红。
  内核自身的注释（`inductive.rs:1150-1163`）说这种块**应当**允许大消去
  （例子 `MyTypeLarge`），但实测**不放行** ⇒ **根因未定位**。
- **G-58**：`And` 这类「字段全是 Prop」的块也消去不到 `Type`，且**对照组**（消去到 `Prop`）能过
  ⇒ 不是 recursor 写法错。
- ⚠ **即使 G-56/G-64 全修好，G-58/G-59 不修 ⇒ ST7 的 `rank` 仍返回不了 `Ordinal`**
  （`Acc` 在 `Prop`、`Ordinal` 在 `Type`）。**这条必须与 G-56/G-64 一起评估**。

### 2.3 其余缺口

**G-60**（花括号不是记法形状）· **G-61**（没有 η ⇒ `Quotient`/`Setoid`）·
**G-62**（def 形态与展开形态不同一）· **G-63**（`Quot.lift` 的宇宙实参难对准）。
四条都是**前端/语法侧**，与内核判定无关，可独立做。

## 3. 下一步（建议顺序）

1. **先做 ST16–ST19**（低风险、能清就清，**在 `main` 上**）：三元素的教学处理 ·
   `abbrev`/`scoped` 真实用法 · 速查表清账 · 补真能用的记法。
2. **再评估内核**：G-56（写法 A 的正解：参数位/指标位两段语义）→ G-64（已验修法）→
   G-58/G-59（大消去）。**每次只动一条判定**，走 §4 的四道保险。
3. bump **`v0.77.1`**（本组续版；**不要动 `v0.78.0`**——那是 E17/E18「性能与纪律固化」）。

## 4. 硬规则（一条都不能忘）

- **内核改动四道保险**：先留判红证据 → **内核单独一个 commit** → 三件套
  （三层回归 · `kernel-diff.sh` 全语料对拍 · `--json` 逐字节不变）**实测数字贴 commit message**
  → **最小档 + 反向判据**（坏类型仍被拒）。
- **提交前 `git diff crates/kernel/` 必须为空**（除非那个单独的内核 commit）。
- **`grep '@@@' crates/kernel/src/` 必须 0 处**（临时插桩一律还原；本轮踩过一次：
  WIP commit 里留了一行，已删）。
- **不许为绿色改判据、不许压低课程难度**。
- **`scripts/soko gate` 不许并发跑**。
- **性能判据不许用绝对毫秒**（跨机不可转移，实测 44ms↔2431ms）⇒ 用**同机比值 + 宽天花板**。
- **轮询 ≤90 秒**；长命令落盘 + 超时（`timeout N cmd > /tmp/x.log 2>&1 &`）。
- **发版**：bump → 一次 commit（含 CHANGELOG）→ push → **CI 真绿** ⇒ auto-tag 自动发版。

## 5. 本轮踩过的坑（别人别再踩）

1. **`git reset --hard` 会连带回退未提交的台账/文档改动** —— 本轮把 G-64 登记与 G-56 的机制
   一起回退了，被迫重建。**换分支前先 `git stash` 或先 commit**。
2. **内核 `try_check_declar_at` 自己装了静默 panic hook**（`util.rs:674`）
   ⇒ `catch_unwind` 拿不到栈。要定位内核 panic：**改那个 hook** 打 `Backtrace::force_capture()`，
   别改 `main.rs`（会被 `quiet()` 覆盖）。
3. **`assert_eq!` 的 payload 会把左右值 `Display` 出来**（`left: 0 / right: 1`），
   所以「同值不同 Debug 格式」的 `assert_eq!` 也能出现这种消息 —— 别只盯着整数。
4. **`gh run rerun` 在 run 未结束时无效**；run 结束后 rerun 会**整轮重跑**（`attempt=2`）。
5. **CI `concurrency: ci-${ref}` 按 ref 隔离** ⇒ 推**分支**不会取消 main 的 run ✓。
