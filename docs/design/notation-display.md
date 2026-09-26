# 记法转化的**唯一接口**（阶段 U 的设计；`e2-plan.md` T-U1 ✓）

> **一句话**：**AST/文本 → 给人看的文本与分段**这件事，全仓**只有一个入口**
> `DisplayNotations::render(...)` ✓ —— 谁需要"带记法的文本"或"带记法的分段"，
> 都必须经它 ✓；绕过它 = 又长出一套实现 ✗（守卫：`scripts/audit-notation-paths.py` ✓）。

## 1. 为什么要定这个（真实事故，不是洁癖）

2026-09-25 用户报告：**Infoview 顶部「目标」里 `∃` 没有转化** ✗。查证结果：
**不是一处 bug，而是四处各自为政的实现** ✗：

| # | 实现 | 位置 | 干什么 | 谁在用 |
|---|---|---|---|---|
| ① | `display::print_back`（+ `fold_collecting`） | `crates/front/src/display.rs` | **真的转化**（文本→文本 ✓） | `kernel_phase.rs`（`ty_text`/`val_text`）、`check/mod.rs` |
| ② | `semantic::tag_runs_with_notations` | `crates/front/src/semantic.rs` | **只打标签**（不转化 ✗） | `query::runs(…)` ⇒ `goal_runs`/`ty_runs`/`goals_runs`（**Infoview 渲染读它** ✓） |
| ③ | `display::render_expr`（=`proof::render_expr`） | `crates/front/src/proof.rs` | **只渲染**（AST→文本 ✗） | `goals::open_goal`、`walk.rs` |
| ④ | 内核 pp（`info.goal`） | 内核 | 点形式 ✗ | `walk.rs`（tactic 步进目标） |

**后果**："渲染"与"折叠"被拆成两步 ✓，而**目标生产链**（③④ ⇒ ②）**只渲染、不折叠** ✗
⇒ 顶部「目标」永远是点形式 ✓。`AGENTS.md` 的 R-1/R-2 教训"真相与显示是两条路"
在这里升级为"**连显示自己都分了四条路**" ✗。

**已被这个分叉咬过两次**（同一族，都是**内建 `=` 抢 `=>`**）：**R-2**
（`semantic.rs:308` 把含 `"="` 的内建表喂给词法 ⇒ `fun (x) => z` 整段降级 ✗）与
**③**（`token.rs` 的声明符号匹配在常规分支**之前** ⇒ `print_back` 解析失败、
**整条 bail** ⇒ 凡类型含 `fun … =>` 的声明全退化 ✗）。修法：基础多字符算符
更长时**让路** ✓。

## 2. 唯一接口的形状（**as-built**，2026-09-26 对齐 ✓）

> ⚠ **本节曾与代码不符**（照它写会编译不过 ✗）：原文写着 `render_text` / `Rendered`
> 返回值 / `render(...).text` —— 而代码里 `render`/`fold` 返回 **`String`**、
> **`render_text` 根本不存在**、`Rendered` **无人构造**（只有
> `Rendered::is_consistent` 一条不变量判据在用 ✓）。下面是实际形状 ✓。

```rust
/// 记法表（一次建好，随编译闭包传播）。
impl DisplayNotations {
    /// **文本 → 带记法的文本**（唯一转化入口 ✓）。= `print_back(text, self)`。
    pub fn fold(&self, text: &str) -> String;

    /// **AST → 带记法的文本**（唯一入口的 AST 版 ✓）：渲染**之后必过折叠** ✓。
    pub fn render(&self, expr: &Expr) -> String;   // = self.fold(&render_expr(expr))

    /// **给已经折过的文本打分段标签**（唯一分段入口 ✓，**不转化** ✗）。
    /// `self` 不参与（分段只看文本 + 名字表）⇒ 拿不到表的消费者用
    /// `DisplayNotations::default()` 调它是**接口约定**，不是绕过 ✓。
    pub fn runs(&self, folded: &str, decls: &[…], binders: &[String],
                notations: &[String]) -> Vec<Run>;
}
```

**两个返回值的分工**（as-built）：`fold`/`render` 只给**文本**；需要分段时按
**固定顺序**再调 `runs`（`runs` 的入参**必须是** `fold`/`render` 的产物 ✓ ——
传点形式文本进来就得到点形式分段，2026-09-25 的用户报告正是这么来的 ✗）。

**不变量（本次 bug 的直接守卫 ✓）**：`runs` 的文本拼接**逐字节等于**它收到的那份
文本 ✓ —— 它把"① 与 ② 各算各的"这种分叉**在接缝处暴露出来** ✓。
判据：`crates/front/src/display.rs` 的 `Rendered::is_consistent` ✓（LSP 侧另有
`goal`/`goal_runs` 同源的 wire 契约 ✓）。

**折叠的两趟**（A1 ✓）：`fold` = 最多两趟 `fold_once`。第一趟折记法与 `->`；
`fold_spine` / App 父节点会用 `render_expr` **重渲染整段**，而它是**回读通道的
输入**（判卷靠它）⇒ 永远打 ASCII `->` ⇒ 被吞掉的由**第二趟**补折（第二趟输入已是
显示文本 ⇒ 记法不会再折一次 ⇒ 两趟有界、不振荡）。
判据：`arrows_fold_to_the_unicode_arrow`。

## 3. 迁移表（谁 → 改成什么）

| 调用点 | 现在 | 改成 | 所属环节 |
|---|---|---|---|
| `kernel_phase.rs` 的 `ty_text`/`val_text` | `print_back(&text, display)` | `display.fold(&text)` ✓ | T-U5 ✓ |
| `check/mod.rs` 的显示副本 | `print_back(text, display)` | `display.fold(text)` ✓ | T-U5 ✓ |
| `query::runs(…)` | `tag_runs_with_notations(text, …)` | `DisplayNotations::runs(…)` ✓ | **T-N4 ✓** |
| `walk.rs` 的三处目标 | 只渲染 ✗ | `display.fold(&render_expr(ty))` ✓ | T-U4 ✓ |
| `by.rs` 的 8 处**诊断消息** | `render_expr(x)` ✗ | `fold_for_display(prefix_src, &render_expr(x))` ✓ | **T-N4 ✓** |
| `goals.rs` 的 17 处 | 只渲染 ✗ | **不折**（源级渲染面）+ 立判据 ✓ | T-U4 ✓ |

## 4. 调用白名单（**可执行**，守卫 `scripts/audit-notation-paths.py` ✓）

**允许**直接调用 `render_expr(` / `print_back(` / `tag_runs_with_notations(` 的文件：

* `crates/front/src/display.rs`（唯一接口的**实现** ✓）
* `crates/front/src/proof.rs`（`render_expr` 的**定义** ✓）

**其余任何文件**出现这三个调用 ⇒ **判红** ✓（报出文件:行 ✓ 与"请改用
`DisplayNotations::{fold, render, runs}`"的修法 ✓）。
**反向验证是硬要求** ✓：把守卫指向**本阶段之前的版本**必须报红 ✓
（咬不住的守卫等于没有 ✓ —— 照 `audit-wire-fields.py` 的先例 ✓）。
进 `scripts/soko gate` 与 CI ✓（T-U3 ✓）。

## 5. 判据（三层齐 ✓，每层一条就够）

| 层 | 判据 |
|---|---|
| **front 真相** | `Rendered::is_consistent` ✓（不变量 ✓）+ `display.rs` 的折叠单测（`arrows_fold_to_the_unicode_arrow` / `set_literals_fold_back_to_braces` ✓）+ 全语料 `--json` 逐字节不变（`scripts/kernel-diff.sh --fast` ✓） |
| **wire 字段存在性** | `crates/lsp/src/tests/`：`goal` 与 `goal_runs` **同源**（拼接逐字节相同 ✓） |
| **真宿主 e2e（用户看得见 ✓）** | 夹具 `exists_fun` 的**顶部「目标」出现 `∃`** ✓；A1/A2 另加 `→` 与 `{a}` 两条 ✓（`editor/vscode/src/test/extension.test.js` ✓） |

## 6. 落地顺序（与 `e2-plan.md` 的 T-U* 一一对应 ✓）

**T-U1** 本文 → **T-U2** 实现 `fold`/`render`（**先零行为变化** ✓，全语料对拍零差异 ✓）
→ **T-U3** 立守卫（白名单 + 反向验证 ✓，进 gate/CI ✓）→ **T-U4** 目标链（用户看得见的
那条 ✓）→ **T-U5** 类型/值链 + 不变量 → **T-U6** 词法根因 → **T-U7/T-U8** e2e 与收尾 ✓。

> **为什么这个顺序**：守卫（T-U3）必须在迁移之前立起来 ✓，否则过程中又会长出第五套 ✗；
> 先做**用户看得见**的那条（T-U4）✓，再做内部两条 ✓。
>
> **后续（批次 N ✓）**：T-N4 把 `query::runs` 与 `by.rs` 的 8 处用户可见诊断消息迁到
> 本接口（基线 68 → **59** ✓）；逐条结论见 `notation-paths-audit.md` ✓。
