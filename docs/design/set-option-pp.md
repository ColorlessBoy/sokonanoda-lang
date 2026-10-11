# `set_option pp.*`（命令输出的显示档）+ `Iff.refl` 签名对齐 Lean 4

**状态**：**已落地**（2026-10-11，用户拍板 A + B）
**起因**（用户实测 `courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda:35`）：
`#print demo_mem_def` 印出 `… := Iff.refl`，看起来「`Iff.refl` 一个实参都没有却过了内核」。
查清后是**两件事叠加**，内核没有放水：① `Iff.refl` 的 `A` 是**隐式**参数 ⇒ 裸写就是完整项；
② 内核 pp 默认 `explicit: false`（`crates/kernel/src/pretty_printer.rs:279`）⇒ 隐式实参被印掉
（`unfold_apps_pp`，同文件 `:652`）⇒ `@Iff.refl (Set.mem a A)` 印成 `Iff.refl`。
**反向验证**（同轮实测，两条都判红）：`by rfl` 证非 defeq 的 `(a ∈ A) ↔ True` ⇒
`elab-tactic-failed`；直接写 `Iff.refl` 证同一目标 ⇒ `kernel-rejected`，诊断里「实际」逐字
就是 `Iff (Set.mem a A) (Set.mem a A)` —— **隐参被补上的样子**。

## §1 Lean 4 对照（源码级）

| 事 | Lean 4 | 本仓（改前 → 改后） |
|---|---|---|
| `Iff.refl` 签名 | `@[refl] theorem Iff.refl (a : Prop) : a ↔ a`（**显式**，`Init/Core.lean:1022`） | 隐式 ✗ → **显式** ✓（`prelude/L1.sokonanoda:42`） |
| 隐式版兄弟 | `protected theorem Iff.rfl {a : Prop} : a ↔ a := Iff.refl a`（`:1025-1026`） | 已一致 ✓（`:46`） |
| `rfl` tactic | `macro_rules \| `(tactic\| rfl) => `(tactic\| exact Iff.rfl)`（`:1029`） | 候选裸 `Iff.refl` ✗ → **`Iff.rfl`** ✓（`crates/front/src/by.rs`） |
| 显示隐式实参 | `set_option pp.explicit true`（`Lean/PrettyPrinter/Delaborator/Options.lean:154`，默认 `false`）· `pp.all`（`:19`）连 notation/宇宙/证明一起开 | 没有开关 ✗ → **同名两条** ✓ |

## §2 范围（本语言支持的子集）

- **语法**：`set_option <名字> true|false`，**文件作用域**（从该行起对**后续命令**生效；
  与 Lean 的 section/文件作用域同形 ⇒ 是 Lean 语法的子集 ✓）。
- **名字白名单只有两条**：`pp.explicit`、`pp.all`。其它名字（含 `pp.universes` /
  `pp.notation` / `pp.proofs`）⇒ **专用诊断**（`set-option-shape`），不静默忽略 ✓。
  为什么只两条：`#print`/`#check` 的内核 pp 走**点名形式**（实测 `Iff`/`Set.mem` 不折记法；
  内核 pp 的记法表来自 export file，前端不填它 —— 记法由**显示层**另折），而 `proofs`
  已被内核阶段**全局置 true**（`kernel_phase.rs`）⇒ 其余三条今天**不可观察**，暴露它们
  等于撒谎 ✗。
- `set_option … in <命令>` **不做**（同 `open scoped … in …` 的先例）：专用提示，让选项
  单独写一行 ✓。
- **影响面只有命令输出**：`#print` / `#check <表达式>` / `#reduce`（`PendingOp::{Print,
  Check,Reduce}` 各带一份档）。**不动** `ty_text` / 目标 / Infoview / hover —— 那条路有
  自己的**可回读**策略（G-71：tactic 上下文要把类型文本再回读成项）。**这是对 Lean 的
  有意偏离**（Lean 的 `pp.explicit` 影响所有 pp），如实登记在 §6。
  LSP 的声明卡片走 `judge_print_of`（合成 `#print`，前缀含本行）⇒ **与 CLI 的 `#print`
  自动一致** ✓。
- **默认行为逐字节不变**：不带 `set_option` 的文件，事件流 / golden / `--json` 一个字节
  都不动 ✓（档初值 = `PpOptions::default()`）。

## §3 实现点（as-built）

1. `crates/front/src/ast.rs`：`Command::SetOption { name, value, span }` —— **不是声明**、
   不产事件、不进声明表（与 `Command::Import` 同族）。
2. `crates/front/src/parser.rs`：`parse_set_option` + `is_reserved_command` 收 `set_option`；
   三条形状闸各给专用码 `set-option-shape`；白名单/着色同步在 `semantic.rs` 的 `KEYWORDS`
   与 `editor/vscode/syntaxes/sokonanoda.tmLanguage.json`（契约测试 `extension.rs` 双向比对）。
3. `crates/front/src/compile/check/walk.rs`：`Walk::pp`（`PpFlags { all, explicit }`）+
   **`WalkCheckpoint`**（T2-B 续编）与 **`ResumeState`**（T1-A 跨模块）**都带上它** ——
   漏一处就是"同一份源码在 CLI 与 LSP 两条路上印出不同文本" ✗（对拍判据见 §5）。
4. `crates/front/src/compile/check/kernel_phase.rs`：执行三条命令前把档**叠加**到
   `env.config.pp_options`（`apply_pp_flags`，只碰 `all`/`explicit`/`universes`/`notation`，
   **不碰 `proofs`**）、执行完**还原**（模板 = `#check` 现成的 `saved_explicit` 那一对）。
5. **顺带修（对拍逮到）**：`compile/units.rs::split_report` 里 `checks[].cmd` /
   `prints[].cmd` **漏了重基** ⇒ 整条一趟那条路的报告里这两个字段留在闭包坐标系、而同报告
   的 `event_cmds` 是本模块序（自相矛盾 ✗）。现在与 `decls`/`hover_cmds` 同法重基
   （单文件时 `start == 0` ⇒ 恒等、逐字节不变 ✓）。
6. 缓存 / 增量：档是**前缀文本的函数** ⇒ 缓存键自动带住 ✓；续编走 §3.3 的两处快照 ✓。

## §4 `Iff.refl` 对齐（同一轮，用户拍板）

- `prelude/L1.sokonanoda:42` 改显式 `(A : Prop)`；镜像 `prelude/Prelude.sokonanoda` 由
  `SOKO_WRITE_PRELUDE=1 cargo test -p sokonanoda-front --test prelude_mirror` **重新生成**
  （不手抄 ✓）。
- `by.rs::rfl_candidate` 的 `Iff` 分支与 `suggest.rs` 的两条 quick-fix 改用 **`Iff.rfl`**。
- 课程同步：`lib/Rel.sokonanoda` 三处、两份解答、以及描述证明项的注释 ⇒ `Iff.rfl`。
- `scripts/notation-lint.py`：`Iff.refl` 的 `allowed 0 → 1`（现在**必须**写实参），
  并新增 `Iff.rfl`（`allowed 0`）；`--census-selftest` 两条新样例 ✓。

## §5 判据（可执行）

| # | 判据 | 命令 |
|---|---|---|
| 1 | 默认档逐字节不变 + 开档露出隐参 + 作用域 + 关档回默认 | `cargo test -p sokonanoda-front --lib set_option_pp_explicit` |
| 2 | `pp.all ⊃ pp.explicit`（还关记法：`Prop` 印成 `Sort 0`） | `cargo test -p sokonanoda-front --lib set_option_pp_all` |
| 3 | `#check <裸常量>` 走 U2 签名源文本 ⇒ **有意不受档影响**（§6 边界） | `cargo test -p sokonanoda-front --lib set_option_pp_explicit_covers_check` |
| 4 | 三条形状闸（未知名字 / 坏值 / `… in …`） | `cargo test -p sokonanoda-front --lib set_option_shape_errors_are_dedicated` |
| 5 | 端到端（含坏选项判红 + help 自文档化） | `cargo test -p sokonanoda-cli --test cli set_option` |
| 6 | **跨模块携带 + 两条路报告逐字相同**（档设在被检查点的模块里） | `cargo test -p sokonanoda-front --test t1a_module_checkpoint_parity` |
| 7 | `Iff.refl` 对齐 | `cargo test -p sokonanoda-front --lib rfl` + `--test prelude_mirror` + `python3 courses/set-theory/tools/check.py` |
| 8 | 课程记法 | `python3 scripts/notation-lint.py`（+ `--selftest` / `--census-selftest`） |

## §6 边界与发现（如实登记）

- `set_option … in …` 不做（专用提示）；只两条选项名，其余报错（不静默）；**只影响命令输出**
  —— `ty_text` / hover / 目标保持 G-71 策略（**偏离 Lean**，理由见 §2）。
- **`#check <裸常量>` 有意不受档影响**：它走 U2 的**签名源文本**捷径（G-63：裸写法默认层 0，
  直接渲签名才不撒谎）⇒ 显式档对它无意义（§5 判据 3 把它与 `#check <表达式>` 一起钉住）。
- **发现（已修）**：`split_report` 的 `checks[].cmd`/`prints[].cmd` 坐标系不一致 —— 由
  T1-A 对拍判据逮到（`#print` 的 `cmd` 6 vs 2），见 §3.5；`docs/visible-changes.md` 有记。
