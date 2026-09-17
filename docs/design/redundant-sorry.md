# 多余的 `sorry`：把「还没证明出来」与「多写了一个 sorry」分开

> 触发：用户 2026-09-17 在 `playground.sokonanoda` 326–327 行的实测反馈。
> 状态：**已落地**（2026-09-17 第九十一轮续；as-built 与验收证据见 §8.4）。
> 判定必须走 kernel（REQUIREMENTS §2 第 4 条）。

## 0. 用户原话与现象

> 「326和327行有问题。编译器的信息应该是 sorry 没有用，而不是
> `declaration 'exists_intro_rule' uses 'sorry' (exercise not yet solved)`。
> sorry 去掉你试试，就能编译通过了。差别很大，会让用户觉得没有证明出来，
> 而不会怀疑是 sorry 的问题。」

学生把答案 `Exists.intro Person P w hw` 写在 `:=` 右边，却保留了原来那行
`sorry`（换行缩进）。结果是：

- 编辑器（VS Code / LSP）说 **"该声明用了 sorry，练习尚未解决"**；
- 学生据此以为"我还没证明出来"，而真相是**他的项已经完整证明了目标**，
  多出来的 `sorry` 只是被当成了第 5 个实参。

两类状态的**学习含义完全相反**，却共用同一句话。

## 1. 现状与根因

| 环节 | 位置 | 行为 |
|---|---|---|
| 分类 | `crates/front/src/compile/check.rs:1257` | 值里含洞 ⇒ 一律 `PendingOp::OpenExercise` → `DeclStatus::Open` → `exercise.open`；**从不过问洞是否承重** |
| 编辑器文案 | `crates/lsp/src/lib.rs:364` | 对每个 Open 声明统一发 `declaration '{}' uses 'sorry' (exercise not yet solved)` |
| CLI 文案 | `crates/cli/src/check.rs:93` / `json_report.rs:66` | `exercise open (fill the sorry)` |

**根因是一行的静默放弃**：`crates/front/src/compile/goals.rs:891` 的
`func_spine_case` 处理"超量应用"（实参个数 > 头部望远镜）时，会把结果类型按
def 体展开去找箭头（`And.right a (Not a) x` + `sorry` 就是这样拿到期望类型的）；
一旦展开不出箭头，`goals.rs:974` 直接 `_ => return None` ——整个 case 交回
generic fallback，**"这个实参根本没人要"这个事实被丢掉**，只剩一个
`ty: None` 的洞。

关键：`by` 块那条路**已经报对了**（实测例 6）：

```
elab-tactic-failed: by 块里没有待解目标
```

所以这不是"缺少机制"，而是**值位与 `by` 位两条路的诊断质量不一致**。

## 2. 实测证据（7 例探针，发布版 0.56.1）

| # | 形状 | 洞的期望类型 | 今天的结果 | 应该是 |
|---|---|---|---|---|
| 1 | `:= f h` + 换行 `sorry` | **`None`** | `exercise.open` | **多余的 sorry** |
| 2 | `:= f` + 换行 `sorry` | `A` | `exercise.open` | 不变（真缺实参） |
| 3 | `:= f h`（无 sorry） | — | `decl.checked` | 不变 |
| 4 | `:= And.intro A B ha` + `sorry`（缺第二个证明） | `B` | `exercise.open` | 不变（真缺） |
| 5 | 洞在中间 `And.intro A B ⟨sorry⟩ hb` | `A` | `exercise.open` | 不变（真缺） |
| 6 | `by exact f h` + 换行 `sorry` | — | **`elab-tactic-failed`（已正确）** | 不变 |
| 7 | `fun (h : A) => f h` + 换行 `sorry` | **`None`** | `exercise.open` | **多余的 sorry** |

用户那行 `exists_intro_rule:0` 的 `ty` 同样是 `null`（`query holes`），与 1、7 同签名。

## 3. 目标

把两种状态**在数据与文案上分开**，且不改变 `exercise.open` 的既有语义：

- **真缺口**（洞有期望类型）：`exercise open (fill the sorry)` —— 一个字不改；
- **多余的 `sorry`**：新稳定 code **`redundant-sorry`**，走既有 warnings 通道
  （`CompileWarning`，像 `reserved-declaration-name` 那样），文案直说结论与动作：
  - message：`这一行的 sorry 是多余的：前面的项已经完成了证明，sorry 不能再接在这里。`
  - hint：`删掉这一行 sorry，这条声明就会通过内核检查；若还想继续写，请把它换成真正缺少的那部分。`
- 该声明的编辑器诊断**不再**说 "exercise not yet solved"（那正是误导来源）。

## 4. 判定规则（sound 的候选 + kernel 终审）

1. **候选（elaborator，可判定的强信号）**：`func_spine_case` 在超量应用分支里，
   剩余实参是 `Expr::Hole`，而**结果类型展开不出箭头**（`goals.rs:974` 那条
   `return None` 的路径）。此时该实参**无论填什么都不是良类型的应用**——
   这不是"缺一块"，是"多一块"。
2. **终审（kernel，硬规则）**：把该实参从应用 spine 上删掉，合成一条**无洞**
   声明交完整 kernel（复用 `front::judge` 的合成/终审路径）。**kernel 接受**
   → 报 `redundant-sorry`；否则**退回** `exercise.open`（保守，绝不误报）。
3. **反例护栏**（必须不误报）：`And.intro A B ha` + `sorry`（洞有期望类型 `B`）
   与 `And.right … x` + `sorry`（结果展开成箭头、洞真的承重）都**不能**被判成多余。
4. **性能**：探针只在"存在无期望类型的洞"时触发——正常检查与热路径零改动
   （REQUIREMENTS §3）。

## 5. 影响面（同一轮必须同步）

| 面 | 改动 | 状态 |
|---|---|---|
| `crates/front/src/compile/{goals,warning,check}.rs` | 候选信号 + kernel 终审 + 新 `WarningKind::RedundantSorry`；终审显式传 `EnvLimit::ByIndex(env_before)` | ✅ 已落地（§8.3） |
| `crates/kernel/src/{tc,util}.rs` | `check_declar_at` / `try_check_declar_at`（只加不改语义，进 `docs/architecture.md` §6 适配表） | ✅ 已落地 |
| `crates/front/src/session.rs` | 内核终审过的 warning 按命令进快照（否则 LSP/增量路径看不到）+ 坐标重映射 | ✅ 已落地 |
| `docs/protocol.md` | 新 code `redundant-sorry` + `exercise.open` 语义澄清（**事件类型不变，只增字段/新 warning**） | ✅ 已落地 |
| `crates/cli/src/{check,json_report}.rs` | 人类视图与 `--json` 走既有 warnings 通道（**未改代码**，通道本来就够） | ✅ 无需改动 |
| `crates/lsp/src/lib.rs` | 抑制 "not yet solved"；`redundant-sorry` 带 message + hint | ✅ 已落地 |
| `crates/front/src/query/**` | 同一真相：洞级 `redundant` 标记（`HoleInfo`/`LocatedHole`），`query goals`/`holes` 与 LSP `soko/goals` 逐字段一致 | ✅ 已落地（两视图契约测试 `crates/cli/tests/query.rs`） |
| `skills/sokonanoda-teacher` | 判卷决策表加一行：`redundant-sorry` ⇒ 让学生删掉那行，**不给提示阶梯** | ✅ 已落地（`SKILL.md` + `references/events.md`） |
| `editor/vscode/` + `skills/` + `AGENTS.md` + `docs/vscode-dev-guide.md` | 用户可见反馈变更 ⇒ 同轮同步（收尾义务）；**版本号 bump** | ✅ 已落地：`README.md`（Honest warnings）、`CHANGELOG.md`（`## [0.56.2]`）、`Cargo.toml` + `package.json` = **0.56.2**（patch）；余下只有 commit/push（触发 auto-tag，非本地步骤） |
| 课程 | `course/**` golden 事件计数**不得变化**（现有 open 全是真缺口，需实测确认） | ✅ 实测不变（`cargo test --workspace --locked` 全绿） |

## 6. 验收（TDD 三层 + 反例）

1. **front 单测（红先）**：例 1/7 报 `redundant-sorry`；例 2/4/5 仍 `exercise.open`
   且 `ty` 不变；例 6 仍是 `elab-tactic-failed`；
2. **CLI e2e**：`--json` 出现 `{"type":"warning","code":"redundant-sorry"}`，
   人类视图一行 `warning[redundant-sorry]`；删掉该行后 `decl.checked`；
3. **LSP**：该声明**不再**出现 "not yet solved"，改出现 `redundant-sorry`
   （含 hint）；
4. **回归**：`cargo test --workspace --locked` 全绿、`scripts/soko gate` PASS、
   `examples/**` 与 `course/**` golden 计数不变；
5. **反向证据**：真缺口样本（例 2/4/5）在实现前后输出**逐字节相同**。

## 7. 已知边界（v1 不做）

- 洞在 `fun` 体里但**有**期望类型（如 `fun (x : A) => sorry`）是真缺口，不在此列；
- 一次多处多余洞：v1 按"存在即报"（每处一条 warning），不做删除组合搜索；
- 更深的嵌套多余（`f (g sorry)` 里 `g` 已饱和）：候选条件只认**直接实参**，
  更深的一律退回 `exercise.open`（保守）；
- **头是 λ 绑定的局部函数**（`fun (g : Prop -> Prop) => fun (h : Prop) => g h` + `sorry`）：
  `local_func_templates` 不为这种 binder 建模板，候选**不触发**（实测 `spans=0`），
  仍报 `exercise.open`。比误报好，但要在下一版补。

## 8. as-built（2026-09-17 第九十一轮 · 实现就绪，**只差终审的环境可见前缀**）

**已打通（有埋点实测证据）**：

1. `WarningKind::RedundantSorry`（code `redundant-sorry` + hint）已加；
2. 候选项从 `func_spine_case` 的超量应用分支（原 `_ => return None` 处）经 sink
   参数 `redundant: &mut Vec<Span>` 贯穿 `goal_under_binders` → `open_goal` →
   `check.rs` 三个构造点，**行为不变**（仍然交回 generic fallback）；
   `cargo test -p sokonanoda-front --lib` = **409 passed / 0 failed / 2 ignored**；
3. 探针构造正确：实测 `»PROBE name=_soko_redundant_sorry_0 ty=(h : A) -> B
   val=fun (h : A) => f h`（`spine_without_arg` 已能穿透声明级 binder 的 λ 包装）；
4. pass 2 已接线：`»CHECK n=1`（探针确实被送到终审点）。

### 8.1 根因（第九十一轮续 · 5 分钟实验已做，**旧假说被推翻**）

**旧假说（作废，别再照着修）**：「探针引用的常量 `NamePtr` 与环境键不是同一个
身份（interning / `decl_idx` 那一层）」。反证见 §8.2 第一行：报出来的未知常量
**就是**环境里 `A` 的规范节点。

**真根因**：终审走 `ExportFile::check_declar` → `check_simple_declar`
（`crates/kernel/src/tc.rs:94-108`），它用**声明自己的名字**来定环境的可见前缀：

```rust
let env = self.new_env(EnvLimit::ByName(d.info().name));   // tc.rs:102
```

而 `EnvLimit::ByName` 对**从没进过环境的名字**取 0：

```rust
EnvLimit::ByName(n) => match n.as_ref().decl_idx() {
    crate::name::NO_DECL => 0,                             // env.rs:257-260
    idx => idx as usize,
},
```

`decl_idx` 只在 `EnvBuilder::add_declar` 时写进去（`builder.rs:238-248`，
`NO_DECL` 是 `NameNode` 出生值，`name.rs:31/80`）。探针名
`_soko_redundant_sorry_i` 是**故意不入环境**的（§4）⇒ 它的名字 `NO_DECL`
⇒ `cutoff = 0` ⇒ **终审看到的是一个空环境**，探针类型里第一个 `Const`（`A`）
立刻变成 `const_head_type: unknown const`（`eval.rs:694-696`）。

这也解释了旧记录里那个"矛盾"：相邻 `PendingOp::Decl` 用**同一个 API** 检查真实
声明却成功——因为真实名字有 `decl_idx`（等于它进环境时的下标），它引用的前缀常量
自然可见。**与指针身份无关，与"可见范围"有关。**

### 8.2 实验（最省的一条，可直接复现）

复现文件（A/B/f + 练习 + 尾随 `axiom g` 给终审一个"下一条真实声明"的名字）：

```sokonanoda
axiom A : Prop
#print A
axiom B : Prop
#print B
axiom f : A -> B
#print f
theorem t (h : A) : B := f h
 sorry
axiom g : B
```

```bash
SOKO_EXP_RS=1 cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json tmp-exp-rs/rs.sokonanoda
```

（`tmp-exp-rs/` 是临时目录，跑完即删；埋点当天已全部还原。复现只要三处：
① 文件里加 `#print A/B/f`，在 `PendingOp::Print` 分支记下 `ptr`——那是环境里
的**规范**名字地址；② 终审前 `eprintln!` 探针名与 `try_check_declar` 结果；
③ 用 `ops.peek()` 拿"下一条真实声明"的名字替换探针的 `info.name`（`ty`/`val`
不动）再查一次。）

```
»EXP declars=16 probe=NamePtr(0x12180c2a0) canon=[("A", NamePtr(0x12180c4f0)), ("B", NamePtr(0x12180c4b0)), ("f", NamePtr(0x12180c3e0))] as_is=Err(Rejected("const_head_type: unknown const NamePtr(0x12180c4f0)"))
»EXP sub_name=NamePtr(0x12180c260) sub=Ok(())
»EXP real=NamePtr(0x12180c3e0) real_res=Ok(())
```

读法（三行就是全部证据）：

- `as_is`：原探针 `Rejected`，**未知常量的地址 = 环境里 `A` 的规范节点地址**
  （`0x12180c4f0`）⇒ 身份没问题，是环境空；
- `sub`：把探针的 `info.name` 换成**下一条真实声明 `g`** 的名字
  （`decl_idx = 15` = 该练习的 `env_before`，同一行里 `declars=16`），
  **ty/val 指针一个字节没动** ⇒ `Ok(())`；
- `real`：同一个 env 上真实 `f` 声明 ⇒ `Ok`（与旧观察一致，但原因如上）。

**端到端验证**（顺手做掉，证明除限界外整条实现都对）：临时把终审换成"用下一个
真实声明的名字"后，CLI 立刻产出（`--json` 事件流原样，只省掉 `human` 字段）：

```json
{"code":"redundant-sorry","hint":"删掉这一行 sorry，这条声明就会通过内核检查；若还想继续写，请把它换成真正缺少的那部分。","message":"这一行的 sorry 是多余的：前面的项已经完成了证明，sorry 不能再接在这里。","span":{"start":{"line":8,"column":2,"offset":104},"end":{"line":8,"column":7,"offset":109}},"type":"warning"}
```

span 精确覆盖那个 `sorry` token（偏移 104–109），其余事件里
`exercise.open` 照旧、零 error。验证完实验代码已全部还原（树里不留埋点）。

### 8.3 修法（已落地）

**A（采用）**：给终审一个**显式的可见前缀**，内核**只加不改语义**（已写进
`docs/architecture.md` §6 适配表，与既有 `try_check_declar` 同规格）：

- kernel：`ExportFile` 新增 `pub fn check_declar_at(&self, d, limit: EnvLimit)`
  与 `pub fn try_check_declar_at(&self, d, limit) -> Result<(), CheckError>`；
  `check_simple_declar` 的限界改为参数传入，`check_declar` 保持原行为
  （= `check_declar_at(d, EnvLimit::ByName(name))`），批量路径
  （`run_session_inner`）逐条显式传同一个 `ByName` ⇒ **热路径零改动、行为
  逐字节不变**；inductive/quot 走各自既有的限界（front 只对 def/theorem 探针
  用新入口）；
- front：`PendingOp::OpenExercise` 带上 pre-pass 本来就记着的 `env_before`
  （`check.rs` 命令循环开头 `let env_before = builder.declaration_count();`），
  终审改成 `env.try_check_declar_at(&declar, EnvLimit::ByIndex(env_before))`，
  这次探针检查计入 `stats.kernel_checks`；
- **sound**：真实声明进环境时的下标**就是** `env_before`（同一条命令、一次
  `add_declar`），而 `check_simple_declar` 对真实声明用的正是
  `EnvLimit::ByName(真名)` = 同一个 cutoff ⇒ 探针看到的可见前缀与"学生真把这条
  声明补完"逐字节一致（前瞻引用照样看不见，有反例测试钉住）。

**会话/LSP 侧**：内核终审过的 warning 必须**按命令进快照**——`session.rs`
此前每轮只用 `collect_warnings` 重算语法级 warning，把 `redundant-sorry` 丢了
（LSP 因此看不到）；现在 `CmdSnapshot.warnings` 按 span 归属、随快照跨版本
复用并做坐标重映射，`WarningKind::is_kernel_verified()` 明确区分两类。

**B（未采用）**：文本级 / 重跑整份流水线（`judge_hole_fill` 那条路）本来是要
"编译里再编译"，撞 REQUIREMENTS §3 的每键预算。§8.2 已证明：**限界一对，
现有实现一次就过**，不用重编译。

**被排除的两条（省得再走）**：

- "探针借用当前练习的名字身份"——**无效**：开放练习不入环境（`check.rs` 的 open
  分支在 `add_declar` 之前 `continue`），练习自己的名字同样 `NO_DECL`；只有
  **已入环境**的名字才有可解析的 `decl_idx`。
- "把探针 `add_declar` 进环境、拿它的下标"——**语义不允许**：探针（去掉 sorry
  的完整证明）一旦进环境，后面的命令就能引用它，等于替学生把练习证了（§4 的
  "绝不入环境"）。

### 8.4 验收（全部落地，2026-09-17 第九十一轮续）

| §6 验收 | 证据 |
|---|---|
| 1. 探针 `ty` 不变、通过 | kernel `tests/memory_api.rs::synthetic_declaration_needs_an_explicit_environment_limit`（8 passed），front 单测 2 条 `#[ignore]` 摘除后绿 |
| 2. CLI：`--json` 出 `warning[redundant-sorry]`、人类视图一行、删行后 `decl.checked` | `crates/cli/tests/protocol.rs::redundant_sorry_warns_and_the_declaration_stays_open` + `crates/cli/tests/cli.rs::cli_redundant_sorry_warns_on_stderr_and_stays_successful` |
| 3. LSP：不再 "not yet solved"，改出 `redundant-sorry`（含 hint） | `crates/lsp/src/by_sorry_range_tests.rs::redundant_sorry_replaces_the_not_yet_solved_warning`（同一测试里真缺口照旧报 `sorry`） |
| 4. 回归：全绿 | `cargo test --workspace --locked` 全绿（front 413 / lsp 118 / kernel 8 …，0 failed）；`scripts/soko gate` PASS |
| 5. 反向证据：真缺口不得误报 | front 3 条反例（真缺实参 / 缺第二个证明 / 项不是目标类型）+ **可见前缀护栏**（同一形状只把 `g` 挪到练习后面 → 前瞻引用不可见 ⇒ 不报） |

**当前状态**：`redundant-sorry` **已经产出**（用户 playground 326–328 的形状：
答案写全、只多留一行 `sorry`）；语义不变（仍是 `exercise.open`，退出码 0），
两条曾经的 `#[ignore]` 已摘。

```bash
cargo test --workspace --locked    # 全绿（0 failed / 6 ignored：缺 fixture 的 kernel 用例）
scripts/soko gate                  # PASS（fmt + clippy + test + playground 锚点）
```
