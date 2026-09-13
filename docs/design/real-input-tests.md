# 真人输入测试（`sokonanoda` 四写法共存）

> 触发（用户原话）：「你要设计真人相同的输入测试，多设计测试，覆盖 sorry intro apply 和 by
> 这些 command 共存会引发的复杂。」
>
> 本文是**测试设计与实施计划**；相关设计：`docs/design/term-intro.md`（值位 `intro`）、
> `docs/design/term-apply.md`（值位 `apply`，待实现）、`docs/TESTING.md`（测试地图）。

## 0. 问题的本质

现有测试大多**构造最终文本**（`did_open(src)` 一次到位）。但学习者是这样用编辑器的：

```
敲 `:=` → 回车 → 敲 `intro`（补全弹出）→ Tab 接受 → 变成 fun 骨架 → 发现太宽 → 撤销 → 改敲 `by …`
```

「最终文本」一样，**中间态**完全不同——而 bug 全在中间态：
`intro` 补全曾经只在 token 内部命中（真实输入时光标在词尾，看不到）；
`apply` 设计里的合成洞与源码 `sorry` 的分派差异（见 `term-apply.md` §6）
也只在「还没接受补全」的那一刻存在。

所以本文的目标是：**把输入序列当成被测对象**，并且让这个过程可复现、零 flake。

## 1. 三层，各管一段

| 层 | 输入形态 | 确定性 | 覆盖 |
|---|---|---|---|
| **front（进程内）** | 文本版本序列，直接调 `Session::update` | 完全确定（同步，无异步） | 增量决策：谁被复用、谁被重算、span 怎么 remap |
| **LSP 协议（进程内）** | `didOpen(v1)` → `didChange(v2..vn)` | 完全确定（`wait_diagnostics` 即就绪信号） | 补全/hover/inlay/诊断在**每个中间态**的表现 |
| **VS Code（进程外）** | 真实 `type` + `triggerSuggest` + `acceptSelectedSuggestion` | 有 UI 反应窗口（唯一允许 sleep 的地方） | 端到端手势：建议真的弹、Tab 真的替换 |

**纪律**：功能正确性一律压在前两层（确定性）；VS Code 层只做**每条链路一个**端到端手势烟测，
不追求覆盖组合。这条纪律来自教训——VS Code 层是唯一出现 `sleep(300/500)`
的地方（`editor/vscode/src/test/extension.test.js:333、369`），是最大的 flake 源。

## 2. 新增基建：输入脚本（本设计的核心可复用件）

> **实现轮修正（2026-09-13，同日）**：本节最初设计的 API 是
> `type_script(...) -> Vec<String>`（跑完整条脚本、返回每步之后的文档全文）。
> 落地时发现它**根本不能用**：**服务器只持有最新状态**，历史文本快照拿回来
> 什么也断言不了——攒到最后再断言，验到的只是最后一步。
> 据此写出的第一个版本是**假测试**（在「已敲 `i`」的快照上，服务器其实已经
> 是「已敲 `intro`」，断言看起来还通过了）。
>
> 现形态（已落地）只给**「一步」**这个原语，断言由测试在**步与步之间**做：
> 见 §2.1。教训值得单独记一笔：**测试基建设计错了，比没有基建更危险——
> 它会产出看起来通过的假测试。**

### 2.1 `crates/lsp/src/testutil.rs` 新增（已落地）

```rust
/// 输入脚本的一步：把当前文本 [offset, offset + delete) 换成 insert。
/// delete == 0 = 纯输入；insert == "" = 纯删除；两者都有 = 选中重打。
pub(crate) type TypedStep<'a> = (usize, usize, &'a str);

/// 每一步一次 didChange + 等到诊断落地，原地推进 cur / version。
pub(crate) async fn type_step(
    service: &mut LspService<Backend>,
    socket: &mut ClientSocket,
    cur: &mut String,
    version: &mut i32,
    step: TypedStep<'_>,
);

/// 把 text 展开成「在 offset 起逐字符输入」的步骤（纯函数，不驱动服务）。
pub(crate) fn char_steps<'a>(text: &'a str, offset: usize) -> Vec<TypedStep<'a>>;

/// 一次 didChange（FULL sync），version 严格递增。
pub(crate) async fn did_change(service: &mut LspService<Backend>, version: i32, text: &str);
```

调用形态（断言在中间态）：

```rust
let mut cur = initial.to_string();
let mut version = 1;
for (index, step) in char_steps("intro", caret).iter().enumerate() {
    type_step(&mut service, &mut socket, &mut cur, &mut version, *step).await;
    // ← 这里 service 正持有「已敲 index+1 个字符」的状态，可以断言
}
```

为什么要「每段一次 didChange」而不是一次性 `did_open(最终文本)`：`did_change` 走的是
`refresh` → `Session::update`（`lsp/lib.rs:785-795`）的真实增量路径，中间态会经过
「旧快照 + remap」，这正是缺陷藏身处。已有先例：
`completion_expands_value_intro_after_a_line_break_edit`（`lsp/lib.rs:2840`）用的
就是这个模式，只是没有 helper，所以只有一条。

### 2.2 就绪信号（不许 sleep）

| 层次 | 信号 | 依据 |
|---|---|---|
| LSP | `wait_diagnostics(&mut socket, "…")`：排空消息直到本 URI 的 `publishDiagnostics` 到达 | `testutil.rs:147-163`；`didOpen`/`didChange` 后必须紧跟 |
| LSP（无诊断可等时） | 驱动一次 `textDocument/hover`，非空即代表编译完成 | 这是「本文件应零诊断」场景的规范手法（`docs/TESTING.md:285` 记录） |
| front | 无需等待，`Session::update` 是同步的 | `session.rs` 全部测试 |
| VS Code | `waitFor(desc, cond, timeout=30s, poll=100ms)`；**先用「hover 非空」确认服务就绪**再断言「没有诊断」 | `extension.test.js:98-107`、`:190-194` |

### 2.3 flake 预算

- LSP/front 层：**零 sleep**，不许出现 `tokio::time::sleep`；
- VS Code 层：整个文件 ≤ 2 处 sleep，且只作为「UI 反应窗口」出现在
  `triggerSuggest` → `acceptSelectedSuggestion` 之间；新增用例若要 sleep，
  必须在此处登记理由。

## 3. 共存风险矩阵（调研产出，行=两种写法，格=风险 / 现有覆盖）

`by` 与值位关键字在**同一值位内语法互斥**（`parser.rs:154-165`），所以多数组合是
**同文档跨声明**或**前瞻性**风险。

| 组合 | 洞 / 子目标 | 光标（nextHole / inlay / stateAt） | 诊断 | 增量（session） |
|---|---|---|---|---|
| `sorry` × `intro` | intro 洞 span=`intro` token、`sub_goals` 空；真 sorry 的多洞有 `sub_goals`。**已覆盖**（`inlay.rs:299`、`compile/tests.rs:904-1003`） | nextHole 只测过真 sorry（`lsp/lib.rs:2042`）；**intro 的 nextHole 无测试** | 二者 warning 同 code 同 span（`lsp/lib.rs:216-241`）；**无对照测试** | intro 洞 remap 有（`session.rs:839`）；**「sorry→intro」增量无 front 测试** |
| `sorry` × 值位 `apply`（待建） | **高危**：apply 若产出多个子目标，洞 span 目前会**全部相同**（见 §4.1） | **高危**：重复 span 会卡 nextHole、inlay 叠位、`sub_goals` 匹配错 | 洞非空 → 同 sorry warning；**未定义** | **未定义**，需复刻 `session.rs:839` |
| `sorry` × `by` | by 尾部洞 span=末个 tactic；闭合则无洞。已覆盖（`compile/tests.rs:3066`、`lsp/lib.rs:3592`） | stateAt × by 覆盖充分（`lsp/lib.rs:2123/2149/2166`） | warning 不吞后续注释已覆盖（`lsp/lib.rs:3558-3589`） | `by_steps` remap 有（`session.rs:806`）；**「sorry→by」增量无测试** |
| `intro` × 值位 `apply` | 两者都单关键字；apply 多子目标时与 `inlay` 的「`holes.len()==1`」分派（`inlay.rs:68`）分叉；**无测试** | 命中规则是否复用未定（`term-apply.md` §7） | 都会触发 sorry warning；文案未定 | 均需 remap holes/sub_goals |
| `intro` × `by` | 单声明内不可能；跨声明无字段冲突。各自有覆盖 | **同文档混排的 stateAt 优先级无测试** | **混排对照无测试** | **「同文档四写法 + 改一行」无测试** |
| 值位 `apply` × `by`（内层 tactic `apply`） | 两条判定路径并存（`by.rs:275-368` vs 新 `compile/apply.rs`）；**无测试** | 未定义 | 未定义 | 未定义 |

## 4. 调研发现的**真实缺陷**（必须由本计划覆盖并修）

### 4.1 `by` + `apply` 留下多个未解子目标时，所有洞 span 相同

- `assemble` 对每个未闭合叶子都写 `Expr::Hole { span: hole_span }`（`by.rs:263、375、382`），
  而 `hole_span` 取**最后一个 tactic 的 span**（`by.rs:269-271`）。
- 若最后一个 tactic 是 `apply` 且它产生了 ≥2 个子目标，则 `holes = [s, s]`、
  `sub_goals[i].span` 也全等于 `s`（`goals.rs:497-505 / 558-564`）。
- 后果：`nextHole` 按 offset 排序后重复 → 前后向 `find` 永远命中同一条，**子目标之间跳不动**；
  inlay 的类型解析 `sub_goals.iter().find(span == hole)` 命中**第一条**，多条 hint 叠在同一位置
  且类型可能是错的（`inlay.rs:65-74`）。
- **当前零覆盖**：`by_block_with_apply_and_rfl_checks`（`compile/tests.rs:2914-2928`）apply 后
  立刻 `exact` 全闭合；`apply_with_mismatched_function_is_a_tactic_error`（`:3052`）直接报错。
- **根因是身份模型，不只是 span 计算**（实现轮复核后修正）：`assemble` 把同一个
  `hole_span` 传给树里**每一个**叶子洞（`by.rs:372-397`），而 `hole_span` 是
  「最后一个 tactic 的 span」（`by.rs:269-271`）。就算改成「每个叶子用它**自己**由哪个
  tactic 产生的 span」，`by apply h` 的两个前提子目标仍然同源、仍然同 span——
  **它们在源码里确实没有各自的坐标**。
- 因此修法必须是**给洞一个 (span, index) 复合身份**，而不是继续在 span 上做文章：
  1. `soko/goals` 已经在 `holes[i]` 上带了按索引唯一的 `id`（protocol 侧已有位置），
     `nextHole` 与 inlay 应改用它，而不是用 offset 反查；
  2. `soko/nextHole` 的返回需要能表达「同一个 offset 上的第几个洞」——这是
     **协议形状变更**（`docs/protocol.md` + `editor/vscode/extension.js` + 契约测试
     同步改动），不是内部小修；
  3. `inlay` 的 `hint_label` 目前用 `sub_goals.iter().find(span == hole)` 反查
     （`inlay.rs:65-74`），要改成按洞索引对齐 `sub_goals`。
- 处置：**先写红测试钉住现状，再按上面的身份模型修**；因为涉及协议，单独一轮做，
  不要塞进 `apply` 的实现轮里。

### 4.2 三套「位置选取」逻辑并存

`intro_at`（洞闭区间 + 同行尾随空白，`lsp/lib.rs:648-663`）、`select_state_at`（声明 span
包含 + by_steps，`lsp/lib.rs:558-594`）、`render::decl_at`（半开区间，`render.rs:298-302`）
各自实现。命令 span 相邻时边界归属可能不一致。值位 `apply` 会引入第四个调用方，
**必须先把选取逻辑收敛成一套**（`term-apply.md` §7 的通用 `keyword_at`）。

## 5. 测试清单（可直接执行）

命名规范：`<层次>_<输入形态>_<断言>`；`<输入形态>` 用 `typed` / `replayed` 区分是否走输入脚本。

### 5.1 front（`compile/tests.rs`、`session.rs`）——确定性最高，先写

| # | 测试名 | 输入序列 | 断言 |
|---|---|---|---|
| F1 | `session_switches_sorry_to_intro_and_back` | `:= sorry` → `:= intro` → `:= sorry` | 三轮的 `status`/`holes` 长度/`intro_skeleton`；每轮 `stats.kernel_checks` |
| F2 | `session_switches_sorry_to_by_and_back` | `:= sorry` → `:= by exact h` → `:= sorry` | `by_steps` 出现/消失；洞 span 随源文本走 |
| F3 | `session_switches_sorry_to_apply_and_back`（随 `apply` 落地） | 同上，`:= apply h` | `apply_skeleton` 出现/消失；洞数=前提数 |
| F4 | `session_rewraps_value_across_four_layouts` | 同一值反复变形：同行 → 折行 → 折行+尾随空格 → 同行 | 每轮 `goal`/`binders`/骨架字符串**一字不差**（只有 span 变） |
| F5 | `session_four_spellings_in_one_document_edit_one_line` | 一份文档含 `sorry`/`intro`/`apply`/`by` 四个声明，改第 3 个 | `recompiled_from` 精确；前缀三类声明的快照被复用且 span 已 remap |

### 5.2 LSP 协议（`lsp/*.rs`）——用 `type_script` / `type_chars`

| # | 测试名 | 输入序列（关键步骤） | 断言 | 状态 |
|---|---|---|---|---|
| L1 | `typed_intro_chars_only_expand_on_the_whole_word` | 逐字符 `i`,`n`,`t`,`r`,`o`（`char_steps` + `type_step`） | `i`/`in`/`int`/`intr` **不**给展开项；`intro` 给出且 `textEdit.range` 恰为 token | ✅ 已落地 |
| L1b | `typed_intro_after_retyping_sorry_and_wrapping` | `:= sorry` → 选中重打为 `intro` → 折行缩进 | 每个中间态都能拿到展开项；编辑范围恒为 token | ✅ 已落地 |
| L2 | `typed_apply_chars_only_complete_on_the_whole_word` | 逐字符 `a`…`y`，再空格 + `h` | 同上；外加「`apply` 后空格再敲名字时补全仍可用」 | 待 I10 合流 |
| L3 | `typed_intro_then_space_still_completes` | 逐字符 `intro`，再插一个空格 | 展开项仍在（第 38 轮的回归，扩展到逐字符形态） | 待做 |
| L4 | `typed_by_then_apply_keeps_tactic_completion` | `:= by ` + 逐字符 `apply` | 给的是 **tactic** 补全语义（不是值位关键字），且解析进 `Tactic::Apply` | 待做 |
| L5 | `replayed_intro_expansion_keeps_inlay_and_next_hole` | `:= intro` → 接受展开（把文本换成骨架） | 展开后 inlay 数=1、`nextHole` 命中骨架里的 `sorry`、诊断只剩 sorry warning | 待做 |
| L6 | `replayed_apply_expansion_keeps_sub_goals_addressable` | `:= apply h`（多前提版本） | 每个子洞可被 `nextHole` **独立**寻址（依赖 §4.1 的修复） | 待 I10 合流 |
| L7 | `replayed_four_spellings_state_at_and_next_hole` | 四声明文档，光标依次落在四处 | `stateAt`/`nextHole` 在 `by` 与值位关键字混排时不互相污染 | 待做 |
| L8 | `replayed_comment_edit_remaps_synthetic_holes` | 四声明文档，在文件头插入注释行 | 每类声明的洞 span 都按新坐标平移（复用 `session.rs:806/839` 的判据） | 待做 |

### 5.3 VS Code 端到端（`extension.test.js`）——每条链路一个手势

| # | 测试名 | 手势 | 断言 |
|---|---|---|---|
| V1 | `typed intro then accepting expands the token` | 已存在（`extension.test.js:340-374`） | 保留 |
| V2 | `typed apply then accepting expands the token`（随 `apply` 落地） | `type "apply h"` → `triggerSuggest` → accept | 文档出现骨架 |
| V3 | `hover on value apply carries a clickable expand command` | hover → 取 `command:` 载荷 → `executeCommand("sokonanoda.expandApply", payload)` | markdown 受信且放行该命令；替换后无 error 诊断 |
| V4 | `typing inside a by block still offers tactic completions` | `type ":= by "` → `triggerSuggest` | 不出现值位关键字的展开项（防串台） |

## 6. 与「三件套」硬规则的关系

- 新增值位 `apply` = **新增语法**，按 `REQUIREMENTS.md` §2 第 3 条必须齐
  「课程 + 测试 + 白名单」三件套；本计划只负责**测试**这一件，
  课程与白名单见 `ROADMAP.md` §10 的 I10。
- 增删/改名测试要同步 `docs/TESTING.md` 的测试地图（那里是「测试在哪」的权威）。

## 7. 执行顺序与 subagent 切分

1. **B0（前置，主会话自己写）**：§2.1 的 `type_step` / `char_steps` / `did_change`
   基建 + L1/L1b 两条用例。理由：这是**公共接线点**，按 LESSONS「并行 subagent 必须
   文件集互斥、主会话独占公共文件」，必须先由主会话落地，再派人。
   **✅ 已落地（2026-09-13，见 §8）**。
   > 顺序修正：原本把 §4.2 的 `keyword_at` 收敛也放在 B0。实现轮判断**后移到
   > I10-S3**——现在只有一个值位关键字（`intro`），把 `intro_at` 提前泛化成
   > `keyword_at` 是**没有第二个调用方可验证的抽象**，多半会在 `apply` 落地时
   > 返工。等 `apply` 存在、两个调用方都在时再抽，收益与可验证性都更高。
   > 代价：`apply` 落地前位置选取仍有三套逻辑，属已知债务（§4.2）。
2. **B1（subagent-1）**：§4.1 的缺陷——先加红测试，再修 `by.rs` 的 `assemble` 洞 span。
   允许改：`crates/front/src/by.rs`、`crates/front/src/compile/tests.rs`。
3. **B2（subagent-2）**：§5.1 的 F1–F5（front/session），只改 `crates/front/src/session.rs`
   的测试模块与 `compile/tests.rs`。
4. **B3（subagent-3）**：§5.2 的 L3/L4（逐字符输入的后两条），只改 `crates/lsp/src/lib.rs`
   的测试模块。依赖 B0。
5. **B4（主会话 + subagent）**：§5.2 的 L5–L8、§5.3 的 V1–V4——涉及 `apply`，与 I10 合流。
6. 每步验收命令固定：`cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check`
   && `cargo clippy --workspace --all-targets` && `cargo test --workspace --locked`，
   外加 `cd editor/vscode && npm run test:unit`（改了扩展时）。

## 8. B0 落地记录（2026-09-13）

| 交付 | 位置 |
|---|---|
| `TypedStep` / `type_step` / `char_steps` / `did_change` | `crates/lsp/src/testutil.rs` |
| L1 `typed_intro_chars_only_expand_on_the_whole_word` | `crates/lsp/src/lib.rs` 测试模块 |
| L1b `typed_intro_after_retyping_sorry_and_wrapping` | 同上 |
| `completion_expands_value_intro_after_a_line_break_edit` 改用 `did_change`（去掉内联 notify 样板） | 同上 |

顺带修正的一致性缺陷：值位 `intro` 的**补全项文档**此前仍写「一次把目标剩下的 binder
全写成 `fun`」，与 hover 已改的「不替换也完全等价」口径不一致——现已统一（编辑器里
两处文案不该打架）。

验收：`cargo test --workspace --locked` **517 passed / 0 failed**（本轮 +2），
fmt clean，clippy 无新警告。
