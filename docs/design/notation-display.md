# 记法转化的**唯一接口**（阶段 U 的设计；`e2-plan.md` T-U1 ✓）

> **一句话**：**AST/文本 → 给人看的文本与分段**这件事，全仓**只有一个入口**
> `DisplayNotations::render(...)` ✓ —— 谁需要"带记法的文本"或"带记法的分段"，
> 都必须经它 ✓；绕过它 = 又长出一套实现 ✗（守卫：`scripts/audit-notation-paths.py` ✓）。

## 1. 为什么要定这个（真实事故，不是洁癖）

2026-09-25 用户报告：**Infoview 顶部「目标」里 `∃` 没有转化** ✗。查证结果：
**不是一处 bug，而是四处各自为政的实现** ✗：

| # | 实现 | 位置 | 干什么 | 谁在用 |
|---|---|---|---|---|
| ① | `display::print_back`（+ `fold_collecting`） | `crates/front/src/display.rs:142` | **真的转化**（文本→文本 ✓） | `kernel_phase.rs:104/123/321`（`ty_text`/`val_text`）、`check/mod.rs:393` |
| ② | `semantic::tag_runs_with_notations` | `crates/front/src/semantic.rs` | **只打标签**（不转化 ✗） | `query/mod.rs:488` 的 `runs(…)` ⇒ `goal_runs`/`ty_runs`/`goals_runs`（**Infoview 渲染读它** ✓） |
| ③ | `display::render_expr`（=`proof::render_expr`） | `crates/front/src/proof.rs` | **只渲染**（AST→文本 ✗） | `goals::open_goal`、`walk.rs:735` |
| ④ | 内核 pp（`info.goal`） | 内核 | 点形式 ✗ | `walk.rs:555/778`（tactic 步进目标） |

**后果**："渲染"与"折叠"被拆成两步 ✓，而**目标生产链**（③④ ⇒ ②）**只渲染、不折叠** ✗
⇒ 顶部「目标」永远是点形式 ✓。`AGENTS.md` 的 R-1/R-2 教训"真相与显示是两条路"
在这里升级为"**连显示自己都分了四条路**" ✗。

**已被这个分叉咬过两次**（同一族）：
* **R-2**（`semantic.rs:308`）：内建记法表（含 `"="`）喂给词法 ⇒ `fun (x) => z` 里 `=>` 的 `=`
  被吃掉 ⇒ 整段降级 ✗；
* **③**（2026-09-25）：`token.rs` 的声明符号匹配在常规分支**之前** ⇒ 同样的 `=` 抢走 `=>`
  ⇒ `print_back` 解析失败 ⇒ **整条 bail** ⇒ 凡类型含 `fun … =>` 的声明全退化 ✗
  （修法：基础多字符算符更长时**让路** ✓）。

## 2. 唯一接口的形状

```rust
/// 记法表（一次建好，随编译闭包传播）。
impl DisplayNotations {
    /// **唯一的"AST → 给人看的东西"入口**（阶段 U）。
    ///
    /// 三段**固定顺序**，任何一段都不许在别处重做：
    ///   1. `render_expr(expr)`            —— AST → 点形式文本（`proof::render_expr` ✓）
    ///   2. `print_back(&text, self)`      —— **点形式 → 记法**（真正的转化 ✓）
    ///   3. `tag_runs(&folded, …)`         —— 给文本**打分段标签**（kind/scope ✓，不转化 ✗）
    pub fn render(&self, expr: &Expr) -> Rendered;

    /// 已经有文本时走这条（tactic 步进的 `info.goal` 等 ✓）：**同样必过 2、3** ✓。
    pub fn render_text(&self, text: &str) -> Rendered;
}

/// **`text` 与 `runs` 是同一次转化的两个投影** ✓。
pub struct Rendered {
    /// 给人看的**文本**（已带记法 ✓）。
    pub text: String,
    /// 同一份文本的**分段**（供高亮/悬浮；`runs` 的 `text` 拼接**必须逐字节等于** `text` ✓）。
    pub runs: Vec<RunInfo>,
}
```

**不变量（本次 bug 的直接守卫 ✓）**：对任意 `expr`/`text`，
`rendered.runs.map(|r| r.text).concat() == rendered.text` ✓ ——
它把"① 与 ② 各算各的"这种分叉**在类型层面暴露出来** ✓。
判据：`crates/front/src/display.rs` 里一条 `render_matches_between_text_and_runs` ✓
（对语料里的声明逐个断言 ✓）。

## 3. 迁移表（谁 → 改成什么）

| 调用点 | 现在 | 改成 | 所属环节 |
|---|---|---|---|
| `kernel_phase.rs:104` `ty_text` | `print_back(&text, display)` | `display.render(ty).text` | T-U5 |
| `kernel_phase.rs:123` `val_text` | 同上 | `display.render(val).text` | T-U5 |
| `kernel_phase.rs:321` | 同上 | 同上 | T-U5 |
| `check/mod.rs:393` | `print_back(text, display)` | `display.render_text(text).text` | T-U5 |
| `query/mod.rs:488` `runs(…)` | `tag_runs_with_notations(text, …)` | `display.render_text(text).runs` | T-U5 |
| `goals::open_goal`（`:1033`/`:1051` 的 `render_expr`） | 只渲染 ✗ | `display.render(ty).text` | T-U4 |
| `walk.rs:735` | 已改 `render_folded` ✓（第一步 ✓） | `display.render(ty).text` | T-U4 |
| `walk.rs:555/778`（`info.goal`） | 内核 pp 文本 ✗ | `display.render_text(&info.goal).text` | T-U4 |
| `semantic::tag_runs_with_notations` | 公开 | **降为 `display` 内部实现细节**（`pub(crate)` ✓） | T-U5 |

## 4. 调用白名单（**可执行**，守卫 `scripts/audit-notation-paths.py` ✓）

**允许**直接调用 `render_expr(` / `print_back(` / `tag_runs_with_notations(` 的文件：

* `crates/front/src/display.rs`（唯一接口的**实现** ✓）
* `crates/front/src/proof.rs`（`render_expr` 的**定义** ✓）

**其余任何文件**出现这三个调用 ⇒ **判红** ✓（报出文件:行 ✓ 与"请改用
`DisplayNotations::render`/`render_text`"的修法 ✓）。
**反向验证是硬要求** ✓：把守卫指向**本阶段之前的版本**必须报红 ✓
（咬不住的守卫等于没有 ✓ —— 照 `audit-wire-fields.py` 的先例 ✓）。
进 `scripts/soko gate` 与 CI ✓（T-U3 ✓）。

## 5. 判据（三层齐 ✓，每层一条就够）

| 层 | 判据 |
|---|---|
| **front 真相** | `render_matches_between_text_and_runs` ✓（不变量 ✓）+ 全语料 `--json` 逐字节不变（`scripts/kernel-diff.sh --fast` ✓） |
| **wire 字段存在性** | `crates/lsp/src/tests/`：`goal` 与 `goal_runs` **同源**（拼接逐字节相同 ✓） |
| **真宿主 e2e（用户看得见 ✓）** | 夹具 `exists_fun` 的**顶部「目标」出现 `∃`** ✓（用例已在 `editor/vscode/src/test/extension.test.js` ✓，未提交 ✓） |

## 6. 落地顺序（与 `e2-plan.md` 的 T-U* 一一对应 ✓）

1. **T-U1** 本文（设计 ✓）
2. **T-U2** 实现 `render`/`render_text`（**先零行为变化** ✓：内部就是那三段 ✓，
   先让**已有**调用点改调它 ✓ —— 全语料逐字节对拍必须**零差异** ✓）
3. **T-U3** 立守卫（白名单 + 反向验证 ✓，进 gate/CI ✓）
4. **T-U4** 目标链（用户看得见的那条 ✓）→ e2e 绿
5. **T-U5** 类型/值链 + 不变量断言
6. **T-U6** 词法根因的独立判据（`token.rs` 的算符让路 ✓）
7. **T-U7** e2e 入册 · **T-U8** 收尾（文档 + 一次 push → CI → bump → release ✓）

> **为什么这个顺序**：先有接口（2）、再有守卫（3）——**守卫必须在迁移之前立起来** ✓，
> 否则迁移过程中又会长出第五套 ✗；先做**用户看得见**的那条（4）✓，
> 再做内部两条（5）✓。
