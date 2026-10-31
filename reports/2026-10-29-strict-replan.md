# strict 重构：现状核实 · 前一版的错误 · 重做计划（2026-10-29）

> 依据：分支 `origin/strict/2026-10-29-pipeline` @ `f151eaba`（merge-base `fd9fb215`），
> 在只读 worktree `/tmp/strict-wt` 实测；release 构建 sha `e71fd1052706`。
> 目标以 `REQUIREMENTS.md` §9.39 为准：strict 是**绑定策略**，隐式位 = typed metavariable，
> **禁止 print→parse**，`strict ≤ loose` 是硬不变量。

---

## 1. 现状（实测）

### 1.1 用户能用到的三档

| 档 | 开法 | 在哪 | unit01 | lib 17 模块 |
|---|---|---|---|---|
| loose | 默认 | main / 0.97.11 | 1.6s · 通过 | 17/17 通过 |
| strict（文本 oracle） | `--dialect strict` | main / 0.97.11 | 17–19s · 通过 | **17/17 通过** |
| strict + typed app | 再加 `SOKO_STRICT_TYPED_APP=1` | **仅旧分支** | 失败（`lib.Set` / `lib.Rel` 先挂） | 2/17（只有 Logic、Sum），337 条诊断 |

⇒ **main 上 strict 的 lib 是对的，只是慢**；旧分支的 typed app 是把它改坏了的那条实验路径。
main 的 `crates/` 自 0.97.11（`fd9fb215`）以来未变 ⇒ 缓存的 0.97.11 二进制就是 main 的读数。

**整门课（`scripts/strict-score.py --course`，冷跑）**：112 目标两档相同、checked 433/433，
但 strict **failed=1**：`units/I.4/unit12-synthesis.sokonanoda` 报
`elab-unknown-identifier` "unknown identifier `\0soko_m58`"（line 0）—— 内部元变量名经
print→parse 漏进了回读文本，正是 §9.39 禁止的那条通道造成的**已发布正确性 bug**。
整门课耗时 strict 515s vs loose 137s。

### 1.2 strict 慢在哪（unit01，0.97.11，`SOKO_JUDGE_STATS=1`）

| 计数 | loose | strict | 倍数 |
|---|---|---|---|
| `JUDGE_STATS` 判定调用 / 耗时 | 16 / 642ms | 16 / 9,747ms | 耗时 ≈15× |
| `JUDGE_STATS` 前缀字节 | 496,355 | 827,947 | ≈1.7× |
| `JUDGE_INFER` 调用 / 耗时 / 失败 | 51 / 173ms / 0 | **217** / 5,096ms / 138 | 调用 ≈4.3× |
| `JUDGE_INFER_SPLIT` 命中 / 未命中 | 46 / 5 | 200 / 17 | — |

多出来的是"问操作数的类型文本"（strict 要全显式 pp 才看得见前导隐式 → 解析回来），即 §9.39 禁止的
print→parse。（旧分支另有逐调用点计数 `JUDGE_INFER_SITES`：就地查询 19,296 vs 991、文本回落 17,824 vs 3,885；
那套 instrument 只在分支上。）

### 1.3 新流水线 `compile/strict/`（非测试 ≈7,400 行）

- 唯一调用点 `strict/shadow.rs:326`；`strict/report.rs` 无任何非测试调用者；`SOKO_STRICT_V2=on` 被当成 off。
- 影子档比较"新流水线 vs 旧 typed app"，两边输入同源：层表都来自 `elab.rs:4357` 的
  `implicit::telescope(签名文本)`，实参类型都来自 `type_oracle`。
- 语料（111/112 文件有读数）：calls 482,467 · agree 90.76% · disagree 0 ·
  弃权桶合计 0.49% · **`legacy_none` 8.75% 未被观测**。

### 1.4 关键反例：agree ≠ 正确

`lib/Exists.sokonanoda:110` `Exists.imp`：旧 typed app 报 `ok=2`，影子报 `agree=2 disagree=0`，
**内核拒绝**：`期望 Sort(1)，实际是 ∀(w'' : A), Sort(0)`。trace 显示解出的隐式实参是源级项
（`Exists (fun (w'' : A) => q w'')`），回填后再 elaborate —— 根因待定位。

### 1.5 工程状态

- `cargo test -p sokonanoda-front --lib` 默认栈下栈溢出中止
  （`probe_that_exhausted_the_old_budget_still_judges_equal`，**main 上也有**，macOS 实测；
  main 的 CI（ubuntu）是绿的 —— 差异待查）；分支上 `RUST_MIN_STACK=64MB` 下 1260 全过。
- 旧分支 docs-lint 3 条红、`audit-strict-paths` `2/6/1`；**main docs-lint 绿、CI 绿**（`c022d82`）。

---

## 2. 前一版的错误

1. **量错了东西**：优化"新旧一致"（agree），而两边输入同源、一致在结构上必然；被比较的旧路径本身让
   lib 15/17 失败。agree 88.8% → 90.8% 期间 lib 欠账一直是 198。
2. **造了第二套类型论**：在源级 AST 上重做 isDefEq（α/η/多 binder Π/δ/记法摊平）+ 内核→源级转换 +
   层元变量 + 富余实参 δ+β。每多一种形状补一条规则 —— 这才是"一刀一刀修不收敛"的原因。
   现成的没用：内核 `Expr::Meta`（带 kind）、`crates/kernel/src/mvar.rs` 的赋值 / occurs 检查，
   前端已把 store 元变量映射成 `Expr::Meta`（`elab.rs:5202`、`:5509`）。
3. **病根诊断错**："三份描述不一致"不成立 —— 构造子 telescope 由 `align_implicit_prefix(params.len())`
   从注册表对齐，prelude 两者出自同一份 AST ⇒ 他提的不变式测试必过、抓不到问题。真问题是**消费方读文本签名**。
4. **文本调用范围划错**：说 `strict/**` 只剩 4 处；strict 路径上实际 ≥12 处（含上游 `elab.rs:4357`、
   `type_oracle` 8 处）。`type_oracle/level.rs:317` 是 print→parse，旁注却写"不做文本猜测"。
5. **没看 `legacy_none`**：8.75% 旧路径解不出的应用，恰是新流水线该接管的部分，影子在那里直接 return。
6. **无视性能红线**：默认 strict 慢 ≈11× 且已发布，交接书未提。
7. **接管点有洞**：§3.4 只在 legacy 返回 None 时接管 ⇒ `Exists.imp` 这类"报 ok 实际错"原样留下。
8. **过程**：134 提交大量是 `plan:`，报告 2,400+ 行，docs-lint 红着推送。过时注释例：
   `strict/mod.rs` 称 compare "未开始"（已实现有测试）；`local_convert` 称默认关（实为默认开）；
   `telescope.rs` 称"无消费者"（已有）。
9. **读数口径**：abstain 0.27% 漏算 `leak_skip`（实为 0.49%）；"1260 测试通过"在默认栈下不成立。

---

## 3. 第一性原理：strict 应当**不慢于** loose

- 两档差别只在绑定策略。loose 每个应用都要多答一问："位置实参是只绑显式位，还是旧的全参数写法？"
  （`elab.rs` 的 `fits_old_style` / `starts_old_style` / `args.len() > explicit_arity()` / 第二候选）。
- 隐式求解成本两档相同（`Exists.intro w hw` 两档都要解 `α p`；写 `@` 两档都不用解）。
- ⇒ loose = strict + 消歧 + 可能的第二次尝试；出错时 strict 首次失败即报，更快。
- ⇒ 今天的 ≈11× 是表示层缺陷（§1.2），不是 strict 本身更贵。

---

## 4. 计划

### 原则

- **判定只交给内核**：隐式位 = 内核 `Expr::Meta`（类型由内核 telescope `Param.ty` 实例化）；
  显式实参带期望类型 elaborate；合一走内核 def_eq。**无源级类型论、无任何文本**。
- **新标尺**（取代影子 agree）：
  1. lib 17/17 在 strict 下通过；
  2. 整门课 strict 全部目标通过；
  3. **计数**判据（AGENTS.md 禁止墙钟判据）：同一文件上 strict 的 `judge_infer` 调用 / 就地查询 /
     文本回落 **≤ loose**。今天 unit01 是 296 vs 51 ⇒ 判据会判红，不是空转守卫。
- 新路径**直接替换**旧路径，不并行两套。
- 对照 Lean 4 `src/Lean/Elab/App.lean` `ElabAppArgs`：隐式 `mkFreshExprMVar`、显式
  `elabTermEnsuringType(arg, binder 类型)`、可推迟的推迟、最后 `isDefEq(结果, 期望)` + synthesize。
  ⚠ 本机无 `~/Documents/lean/lean4`，开工前先拉锁版本源码。

### P0 基线与止血（从 main 新开分支）

- 计分脚本 `scripts/strict-score.py`：一条命令出 JSON —— lib 17 × {loose, strict} 退出码 / 诊断 /
  checked；unit01 / unit12 / Equiv 的 `JUDGE_*` 计数（strict vs loose）；可选整门课；附构建身份；
  `--env K=V` 注入实验开关（P1 的新路径用它量）。
- 单独修测试栈溢出（main 也有）。
- P1 前置调研：拉 Lean 4 源码做 `ElabAppArgs` 对照；核实内核 `Expr::Meta` 能否在 def_eq 中被赋值；
  估算从旧分支摘 `telescope.rs` 的成本（它把 `KnownTable` 改成了 `KnownTable<'a>`）。
- **判据**：脚本在 main 上复现 lib strict 17/17、unit01 `JUDGE_INFER` 217 vs 51；计数判据今天判红；
  对旧分支二进制 + `SOKO_STRICT_TYPED_APP=1` 复现 2/17（证明 lib 判据咬得住）。**✓ 已达成**
  （main sha `1eac0efd`：lib 17/17 · work 三文件全红 —— unit01 217/51 · unit12 574/67 · Equiv 216/64；
  坏二进制 2/17；同 sha 两次计数逐项相同；`course_strict` 红 = unit12 那条漏名）。
- 冷跑口径：除两个 `NO_*_ARTIFACTS` 外，**全局编译缓存**（`compile/cache.rs::root()`）命中会整份回放、
  一行 `JUDGE_*` 都不打 ⇒ 每次运行 `SOKONANODA_CACHE_DIR=<新临时目录>`；方言用顶层 `--dialect`
  （`course` 也认，优先于 `SOKO_STRICT_AT`）。
- **P2 的验收集加一条**：unit12 strict 通过（新路径不经文本 ⇒ 漏名这一类按构造消失）；P1 只照报读数。

### P0 产出：Lean 4 对照（v4.35.0 `eb6cafb`，`~/Documents/lean/lean4`）

| Lean | Lean 做法 | 我们 | 决定 |
|---|---|---|---|
| `Elab/App.lean:745-760` | `{x}` ⇒ `mkFreshExprMVar binderType`；`@` 下隐式位吃位置实参 | 隐式位 = 内核 `Expr::Meta`，类型 = 实例化后的 `Param.ty` | 同；`[x]` 无 TC ⇒ 当 `{x}` |
| `Elab/App.lean:563-608` | 第一个显式实参前**一次** `isDefEq(期望, 结果)`；期望是 `Prop` / 结果依赖未吃实参 / 实参是 `_`/`by` 时跳过 | 同，三条跳过条件照抄 | 同 |
| `Elab/App.lean:433-443` | 显式实参带 binder 类型立即 elaborate + `ensureHasType` | 同；无 coercion、无 postpone 队列 | 期望仍是 `?m` 头 ⇒ 报"期望类型未知" |
| `Elab/App.lean:610-656` | `finalize`：`isDefEq(期望, 推出类型)`；未赋值隐式**声明末尾**才报 | 同；**应用末尾**就报，点名 binder | strict 没有后续求解器，等无意义 |
| `Meta/ExprDefEq.lean:1477-1521` | Miller pattern `?m xs =?= v` + 一阶/常量近似（elaborator 开 `quasiPatternApprox`） | 只做 `?m =?= t` + Miller pattern | 不做近似 ⇒ 确定性；解不出让用户写 `@`/命名实参 |

形状结论：`Exists.intro`（有期望）/ `And.intro` / `Set.mem` **一阶够**；`Exists.elim` 需 Miller pattern
`?p x =?= P x`（或把签名里 `λx. ?p x` η-约成 `?p`）；`Exists.intro` 无期望需 quasi-pattern ⇒ **不支持，报错**。
内核最小能力：可赋值 typed meta（scope / occurs / 类型检查）· 带 meta 的 β/δ/η 与 binder 开放 ·
Miller pattern · **失败回滚**（Lean `checkpointDefEq`）· 层元变量（`Exists` 的 `Sort u`，未追）。

### P0 产出：内核元变量现状（spike `/tmp/p0-spike`，复跑 4/4 与报告一致）

内核 def_eq **能**给 `Expr::Meta` 赋值，但只有一阶，且三处会 panic：

| 位置 | 现象 | 后果 |
|---|---|---|
| `eval_type.rs:39` `value_type` | Lam/Pi ⇒ panic | `?p := fun x => …`、`?α := A → B` 赋不上；期望类型含 λ（`∃ x, P x` 的常态）直接 panic |
| `eval.rs:783` `apply` | 头是 `Value::Meta` ⇒ panic（`Value::Meta` 无 spine） | `?p w` 表示不了 ⇒ 无 Miller pattern、无卡住项 |
| `eval.rs:449-455` `closed_eval_cache` | 无元变量守卫 | 赋值前求过值的 `?p` 一直是旧值（spike case 3 vs 4） |

另：公开入口只有 panic 式 `assert_def_eq`（`tc.rs:296`），无 `is_def_eq -> bool`、无回滚、无
`instantiate_mvars`；binder 下赋值无 scope 纪律。`add_declar` 入口拒任何元变量（`builder.rs:596`）
⇒ **最终声明永远无元变量**，信任边界不变。层元变量已有（`Level::MVar`、`level.rs:150-175`）。

telescope：**按需从内核声明类型派生**（`builder.with_env(|ef| ef.declars.get(..))`，先例
`check/decl_card.rs:54`）+ `implicit_prefix` 覆盖构造子参数，≈ 2–4 文件；移植旧分支方案 ≈ 19–28 文件
（`KnownTable<'a>` 生命周期 + wire 冷暖对拍）⇒ 选按需派生；暖路径查不到声明 ⇒ 回落今天的路径（仅 P1）。

### P1：内核元变量补齐 + 内核级 elab_app（限 2 天，到期无结论先汇报）

**路线**：扩展内核**已有**的元变量机制（`mvar.rs`），不在前端另写一套 isDefEq。
⚠ 与 Lean 的偏离：Lean 内核不认元变量，合一在 `Lean.Meta`。偏离理由：本仓内核已承载一阶元变量合一
（IA-4）；另写一套 = 第二份 defeq（旧分支病根，只是换了表示）。信任边界靠 `add_declar` 入口拒元变量 +
最终项实例化后走**不变的**判定路径重检。

| 刀 | 内容 | 写者 | 判据 |
|---|---|---|---|
| K1 | `Value::Meta` 加 spine（`apply` 产卡住项，quote/conv 认它）；`value_type` 的 Lam/Pi 改为对 quote 项 `infer`；`closed_eval_cache` 元变量守卫；公开 `is_def_eq -> bool` + checkpoint 回滚；`instantiate_mvars`；Miller pattern 臂（实参两两不同的局部变量 ⇒ `?m := λxs. t`，scope + occurs + 类型检查） | 内核**唯一**写者 | spike 四例转正式测试：期望含 λ ⇒ 解出 `?p`；无期望 ⇒ **干净的 false**、无 panic；缓存不陈旧；`?p x =?= P x`（x 为 binder 变量）⇒ Miller 解出。**红线**：内核既有测试 + front lib 1099 + golden 逐字节不变 |
| F1 | 按需 telescope helper | 前端写者（与 K1 文件不相交，可并行） | 单测：`Exists.intro`/`And.intro`/`Set.mem`/`Exists.elim` 的 binder 档与注册表一致 |
| F2 | 内核级 `elab_app`（`SOKO_STRICT_KERNEL_APP=1`，仅 strict），按 Lean 对照 7 步；未解隐式在应用末尾报错并点名 binder；解不出 ⇒ 回落今天的路径（仅 P1）；stderr 计数 `KERNEL_APP tried= ok= fallback=` | 前端写者（K1、F1 之后） | `strict-score.py --env SOKO_STRICT_KERNEL_APP=1`：`lib_strict` 仍 17/17、`lib_loose` 不变；`lib/Exists`、`lib/Set` 上 `ok > 0`（证明真接上）；命中的应用零文本 / 零 `judge_infer`；三文件计数相对 P0 基线**下降**并照报（≤ loose 是 P2 硬门） |

不通过 ⇒ 汇报后再定（候选：前端 Meta 层合一，只喂内核项）。

### P1 实际落地（2026-10-30，分支 `strict/2026-10-29-kernel-app` @ `47109c44`）

每刀合并前：内核/front 单测复跑 + 全语料 357 文件 × 两档 = 714 组 `grade --json` 逐字节对拍（开关关）。

| 刀 | 提交 | 内容 |
|---|---|---|
| F1 | `f5f83b32` | `kernel_tel`：按需从内核声明类型取 telescope（428 声明扫描 0 失配，暖路径 0 差异） |
| — | `351e3574` | 用户拍板：显式层之后的隐式 binder 直接报错（`elab-implicit-binder-after-explicit`） |
| K1a–c | `d71e0cdf`…`1cfb73a8` | `Value::Meta` spine；可回滚 `is_def_eq`；`instantiate_mvars`；Miller pattern；赋值须封闭；`add_declar` 补扫定理证明项（main 上的信任边界漏洞） |
| F2a | `f30b4cc1` | 跨 arena 搬运（`transfer.rs`）+ builder 侧代换 + `solve_metas` —— 解出的项不再经 pp→parse |
| F2b-1 | `673e1783` | 内核 `elab_app`（Lean `ElabAppArgs`）；隐式位用**带上下文的元变量** `?M x1..xd` |
| K1d | `879136d8` | 会话层元变量可赋值 ⇒ `Eq` 家族等宇宙多态头可走新路径；修 `instantiate_mvars` 开放实参下标 |
| F2b-2 | `9f70d63d` | 前端接线，`SOKO_STRICT_KERNEL_APP=1` 才走；解不出一律回落旧路径 |
| — | `c2d293c6` `47109c44` | 拆 `mvar_spine.rs`（docs-lint ⑨）；工作量门改比 `judge_infer` **未命中** |

**开关打开的读数**（冷跑，单独计时）：lib strict 17/17；新路径解出 94–98% 的应用；
unit01 2.4s vs loose 2.3s（持平）· Equiv 6.0s vs 4.8s · unit12 22.6s vs 9.6s。

**剩余慢的唯一原因**（实测归因）：`judge_infer` 缓存**未命中**多（unit12 30 vs 9），每次 = 整份前缀重编
（strict ≈0.44s）。命中几乎免费（557 次合计 3ms）。unit12 的 30 次里 24 次出自同一处：回落应用走旧文本
oracle（`elab.rs:4821` `pp_shape::arg_types` → `operand_type_expr` → `infer_type_text`）。
`prefix_bytes` 大 2.85× **不是**第二个原因：同一次判定的前缀两档逐字节同长（19 次 tactic 判定都是
739,407 B），差额全是未命中次数 × 前缀长度（strict 的额外未命中落在文件后部的大前缀上）。
⇒ 修法 = 减少回落：回落最多的是 `argelab`（实参无期望类型时 elaborate 失败，如无标注 binder 的 `fun`），
对齐 Lean 先推迟、带期望类型再 elaborate（`Elab/App.lean:433-443`）—— P2-1 进行中。

### P2 接管

- strict 档全部应用走新路径；删旧 typed app 与影子档；诊断复用 `strict/report.rs`。
- **判据**：lib 17/17；整门课 strict 全部目标通过；计数 ≤ loose；loose 档 golden 与 `--json` 逐字节不变。

**拆刀（2026-10-31 定，进度以本表为准；每刀 ≤60 分钟可提交，子代理实现、主会话审+并入）**：

**对拍节奏**（用户 2026-10-31 20:09 同意提速）：从 P2-11b 起，**每刀只跑单测 + 复现件/课程计数 + 计分板**；
全语料两对逐字节对拍改为**每两刀合并后跑一次**（主会话在主仓库跑，基线与新二进制同一检出目录）。
出了差异 ⇒ 用两刀各自的二进制二分定位。内核改动的刀仍各自跑开关关那一对（判定红线）。

**第一批验收 ✓**（P2-11a-A + P2-11a-B，HEAD `0c95f317`，二进制 `a356d3d8`，主仓库，2026-10-31 21:14）：内核 lib / front `kernel_app` rc=0；
语料开关关、开关开各 714 行，空输出只有 `lib/Logic`，与参照唯一差异为 G77 两行（路径相关；哈希 `b9feb53e` = 主仓库基线）；
计分板 rc=0：lib 17/17 两档，`work_le_loose` 三文件绿（未命中 5/5、4/4、4/4）。墙钟在并发负载下量，只作参考。

**第二批验收 ✓**（P2-10b + P2-L-fix + 反向臂补丁，HEAD `49364ae1`，二进制 `401782ca`，主仓库，基线 = 第一批输出）：front lib 1141/1141、内核 lib rc=0；
计分板 rc=0（lib 17/17 两档，未命中 5/5、4/4、4/4）。语料与第一批的差异 4 行，**全部是 §9.39 允许的拒绝→接受**（P2-L-fix 修的层级缺陷，strict 档也用这段代码——P2-L 只量了 loose，这里补上）：
- `unit12-synthesis` strict 开关关：rc 1→0，`flawed_equalities_refuted` 由 `Sort(0)/Sort(1)` 拒绝变为签名通过（open 8→9），其余 3 checked / 8 open 不变；
- `unit12-solution` strict 开关关、开关开：rc 1→0，checked 8→9，诊断 1→0（L171 同一缺陷）。对拍里开关关那行是空输出（并发负载下 900 秒超时，脚本按设计 exit 4），单独串行重跑得到上述结果（721s / 387s）；开关开重跑哈希与对拍行一致。
无任何接受→拒绝。

**第三批验收 ✓**（P2-11b + P2-11c，HEAD `ff23149a`，二进制 `e8f70e48`，主仓库，基线 = 第二批输出，`unit12-solution` 开关关一行用串行重跑的哈希补上）：
内核 lib 123、front lib 1145 全过；语料开关关、开关开各 714 行，空输出只有 `lib/Logic`，与基线**差异 0 行**；计分板 rc=0（lib 17/17 两档，未命中 5/5、4/4、4/4）。
开关开也是 0 差异：内核路径解出的项与旧路径逐字节相同，判卷与事件数自然不变。

**第三批全课程普查**（二进制 `e8f70e48`，开关开，strict，251 个文件；rc≠0 只有两个本来就该拒绝的 `gaps/` 复现件，unit12-solution 已转绿）：

| 计数 | P2-C（`2fdb470c`） | 第三批（`e8f70e48`） |
|---|---|---|
| tried / solved | 917,633 / 908,821（99.04%） | 887,187 / 885,504（99.81%） |
| `fallback_argmismatch` | 5,573 | 2 |
| `fallback_unsolved` | 3,122 | 1,629 |
| `fallback_unsupported_surplus` | 29 | 0 |
| 宇宙多态头 | 19 | 0（另 `ho_motive` 3，`Eq.subst` 有意挡住） |
| `input_contains_a_meta` | 45 | 32 |
| `shape` / `argelab` / `expected` | 12 / 9 / 3 | 12 / 0 / 5 |
| `KERNEL_APP_MISS.total` | 2,862 | 2,843 |
| **`untried_universe_app_head`**（P2-11c 新加，之前看不见） | — | **141,258** |

⇒ 删旧路径前最大的一块是 `untried_universe_app_head`：每个 `=`/`≠` 都没进内核路径（P2-11e 在修）。
`fallback_unsolved` 1,629 集中在 unit19-sol 160、unit12-sol 97、unit10-sol 92、unit19 79、unit09-sol 74（P2-11g 的输入）。
`expected` 3→5 小幅上升，归因留给 P2-11g 一并做。

| 刀 | 内容 | 判据 | 状态 |
|---|---|---|---|
| P2-1…P2-8 | 推迟实参、多轮、重检、记法、探测、缓存键 | 见交接书 `2026-10-31-strict-handoff.md` §1 | ✓ |
| P2-9 | 内核：`expected` 里允许外层元变量（登记 + 内层 id 从 `max(SESSION_META_BASE, 最大外部 id+1)` 起防撞号）；`NeedArgs.blocked` 带域；拆 `elab_app_holes.rs` | 内核单测（外部元变量被赋值 / 留空 ⇒ Unsolved / 撞号反向验证）；开关开/关全语料逐字节 | ✓ `bef50d65`（714×2 两对 diff 皆空，主会话复核；内核 120、front kernel_app 25） |
| P2-C | **普查**（只读）：整门课开关开，按回落原因计数；钉住 R1/R2 两档分歧 | 见下「P2-C 结论」 | ✓ |
| P2-10 | 前端：`elab_ready` 把 blocked 的非 λ 位按带元变量的域作提示（P2-7 方案 A 第 5 步） | `unsolved` 回落降 | ✗ **降级、不并入**：重试在 7 个文件上 `fallback_unsolved` 一次没降、`KERNEL_APP_MISS` 不变 ⇒ 只多做功。存档 `p2-10-blocked-hints.bundle`（`b5dec5b0` 内核小改：全 blocked ⇒ `NeedArgs{domains:[]}`）+ `p210-front-retry.diff`（无测试；计数静音范围过宽，复用前要修） |
| P2-10b | 前端：**带注解 binder 的 λ 实参**，域已知后让 λ 体对着域的 codomain elaborate（Lean `elabAndAddNewArg` + `Binders.lean` `elabFunCore`/`propagateExpectedType`）。可能要推迟带类型的 λ 位，或域就绪后给体一个提示 | `fallback_unsolved` 降（基线：lib/Exists 1、lib/Image 1、lib/Order 32、unit19 84、unit12 66、unit10-sol 134）；`KERNEL_APP_MISS` 不升；反向验证 | ✓ `4d9986c2`（新文件 `kernel_app_lambda.rs`）：unsolved Exists 1→0、Image 1→0、Order 32→9、unit19 84→69、unit12 66→54、unit10-sol 134→92、unit01 20→5；rc/诊断与 `KERNEL_APP_MISS` 全不变；计分板绿；守卫测试改在 `TEST_NO_TYPED_LAMBDA` 下跑、断言未放宽；主会话抽查 Exists/Order。域仍带元变量的 λ 退回旧做法（留给 P2-7 方案 A）。语料随第二批统一跑 |
| P2-11a | `argmismatch` 根因（只读）| 见 `reports/2026-10-31-p2-11a-argmismatch.md` | ✓ 主会话复现 mem/iffn 计数 |
| P2-11a-A | 前端：第一趟把**裸隐式常量**实参（`univ`，`implicit_prefix > 0` 的 Ident/记法）推迟，交给 `elab_ready` 按域 elaborate（Lean `addImplicitArg` `App.lean:745` + `elabAndAddNewArg` :433）。约 6 行，`kernel_app::run_once` | `Set.mem`/`Set.subset`/`Set.powerset` 的 argmismatch 归零（基线：只 import `lib.Set` = 12） | ✓ `411145fe`：只 import `lib.Set` 12→0、`a ∈ univ` 16→0、`pow` 28→2（2 处转 unsolved，域里还带 `Set ?α`）；unit10-sol 回落 225→148；`KERNEL_APP_MISS` 不升；两对语料仅 G77（路径相关，同一检出目录下逐字节相同）；主会话抽查 empty/mem/sub/pow |
| P2-11a-B | 内核：`elab_app.rs` 显式实参闸门前，若本会话已有赋值，`dom` 从 `instantiate_mvars(quote(dom))` 重 eval（`Value` 闭包里捕获的是 `app_propagate` 赋值**之前**的值）。约 4 行。Lean 的 Expr 无陈旧闭包（`ExprDefEq.lean:1402`）。长期正解在 `def_eq_at` 本身 | `Iff.intro` 家族（330 次）归零；内核单测 + 反向验证；两对语料逐字节 | ✓ `0c95f317`：复现单测在旧代码上 `ArgMismatch{arg:0}`、常驻反向断言；iffn 2→0、lib/Order argmismatch 7→0；`KERNEL_APP_MISS` 不变；开关关语料仅 G77（路径相关）；主会话抽查 iffn/rel_empty/Order。开关开语料随第一批统一跑 |
| P2-11b | `surplus`：过量应用（`And.right h x`，Lean `whnfForall` 结果类型 `App.lean:458`） | `fallback_unsupported_surplus` 归零 | ✓ `b95866f8`：根因 (b) 与 P2-11a-B 同族（`?b` 已赋值但 `fty` 是旧 `Value::Meta`）；`app_reeval_pi` 只在实参有剩、非 Π、本会话有过赋值时重算（Lean `whnfForall` `App.lean:247/373`）；未赋值 `?b` 照旧 `surplus`（Lean 也报错）。11 个文件 surplus 29→0、unsolved 184→174、`KERNEL_APP_MISS` 97→93；开关关语料 714 行逐字节相同（主会话复核）；主会话抽查 unit04-sol/C-112-cb-prop |
| P2-11c | 宇宙多态头（`Eq`/`Eq.symm`/`Eq.subst`）仍被前端 `universes.is_empty()` 挡在门外（K1d 内核已支持） | 计数归零（m5 已由 P2-L-fix 两档修好）；另把 `UniverseApp` 头的不计数放弃（`=` 记法降成 `Eq.{u}`，P2-L 发现）补成计数器，删旧路径前要看得见 | ✓ `7865ed5b`：三处闸门统一读 `kernel_app_hint::universe_gate`；头剥层交内核 K1d 发会话层元变量（`Solved` 层已具体，`infer_const_universes_with` 对它 no-op）。13 个文件宇宙多态头回落 19→0，`KERNEL_APP_MISS` 117→117，rc/诊断不变；主会话抽查 unit02。`Eq.subst` 仍挡（高阶 motive，放行后试了再付老路），计 `fallback_unsupported_universe_poly_ho_motive`。新观测计数 `untried_universe_app_head`：**13 个文件 7,802 次**（`=`/`≠` 记法降成 `Eq.{u}`，从不进内核路径） |
| P2-11e | **`UniverseApp` 头进内核路径**（`Eq.{u} a b`：按显式层建 `Const`，内核原样用）——删旧路径前必须做，否则每个 `=` 都靠旧路径 | `untried_universe_app_head` → 0；`KERNEL_APP_MISS` 不升；两档判卷不变 | 待第三批 |
| P2-11f | `Eq.subst` motive 推断（Lean `elab_as_elim` / `ElabElim`），先只读设计 | 设计稿 + 复现件 | 待定 |
| P2-11g | 剩余 `unsolved`（P2-10b 后约 229 次，多为域里仍带元变量的带注解 λ，走 `fill_late`）| 先普查归因 | 待第三批普查 |
| P2-D | **只读盘点**：删旧路径（P2-13/P3）要动哪些代码，哪些被 loose / goal 显示 / hover / tactic 共用必须留 | 盘点表 + 风险 | 进行中 |
| P2-L | **loose 档**修 m5（REQUIREMENTS §9.39）：先只读定旧路径 `=`/`≠` 记法选错 sort 的根因，再修；对齐 Lean `binrel%` 先 elaborate 两边 | m5 两档接受；loose 语料只在含此形状的文件上变、只许拒绝→接受、逐行列出；strict 不受影响 | 根因 ✓（`reports/2026-10-31-p2-l-m5-root-cause.md`，主会话读码核实）：`elab.rs:2274` `universe_level_text_of_operands` 的 `?` 在第一个操作数推不出类型时整函数返回 `None` ⇒ 层级默认 `"0"` ⇒ `Eq.{0}`；与注释「逐个操作数试」不符。**两档共用** ⇒ 一处修两档（strict 的 m5 头是 `UniverseApp`，从不进内核路径）。修法 `?` → `continue`（Lean `Extra.lean:275` `isUnknown → hasUnknown` 继续）：loose 357/357 逐字节不变（仓库里没有这种写法 ⇒ 必须补回归测试） |
| P2-L-fix | 实现上面的修法 + 回归测试（m5 族 5 种写法 + 不受影响的对照）+ 反向验证 | front 单测；strict 语料随第二批统一跑 | 进行中 |
| P2-11d | 余量：`input contains a meta` 19 处、`shape` 8、`argelab` 3、`expected` 3 | 各归零或定为报错 | 待前几刀 |
| P2-12 | strict 默认走内核路径：去开关；解不出 ⇒ 报错**用人能读的名字**点名 binder（不许出现 `?<id>` / `\0soko_m…`），不回落。**前提**（P2-D 风险表）：① P2-11e（`=` 进内核路径）；② 内核路径自己的报错文字，替代旧路径的 `strict_rejection_message` 等，CLI 断言 strict 文字的测试同步改；③ `@` 脊 / 具名实参 / prelude 安装仍走旧路径，先决定移植还是保留；④ 报错处的 hover 不能丢；⑤ 去 `env_on` 会让 strict 缓存键整体失效，`kernel_app_keys.rs` 重写。unit12-solution L171 已由 P2-L-fix 修好，无需改写法 | lib 17/17；整门课 strict 112/112；loose 逐字节；R1 报「无法推断」类错误 | 待 P2-11e |
| P2-13 | **砍掉共享代码里的 strict 分支**（不是删模块：旧 typed app、`pp_shape`、`judge_infer`、`implicit*` 都是 loose / tactic / goal / LSP 共用，见 `reports/2026-10-31-p2-d-old-path-inventory.md`）。strict 专属的只有 `elab.rs:4505-4540`、`:4780-4794`、`:4885-4900` 等 | 同上 + 摘旧分支 `f151eaba` 的 `scripts/audit-strict-paths.py` 并按本分支重定基线、只降不升 | 待 P2-12 |

**P2-C 结论**（2026-10-31，分支二进制 `2fdb470c`，开关开，251 个 tracked 文件，冷跑；主会话抽查 m5/q2 复现）：
- 整门课计分板 4 门全绿：lib 17/17 两档；工作量 strict = loose；课程 112/112 目标、0 失败、433 checked，两档相同。
- 内核路径解出 908,821 / 917,633 = **99.04%** 的应用。回落按原因（次数 / 不同位置上限）：`argmismatch` 5,573 / 38（`Set.mem` 5,132、`Iff.intro` 330、`Set.subset` 68）· `unsolved` 3,122 / 183（`Exists.intro` 2,684、`And.intro` 160、`≼` 119）· `input contains a meta` 45 / 19 · `surplus` 29 / 25 · 宇宙多态头 19 / 7 · `shape` 12 · `argelab` 9 · `expected` 3。次数大多是同一批 lib 位置在每个导入方里重 elaborate；不少位置落在文件末尾之后（推断是 `judge_infer` 拼的前缀文本），不同位置数是上限。
- 对照 Lean `Elab/App.lean`：所有回落原因都是 Lean 能解的（`propagateExpectedType` :563、`elabAndAddNewArg` :433、`postponeElabTerm` :1314、`whnfForall` :458）。唯一该报错的是 R1 那种约束不足的元变量（Lean `TermElabM.lean:934` 在声明末尾报）。
- **main 上已有的 bug（与本分支无关，主会话复现过）**：`{∅} = @Set.univ (Set Nat)`（m5）在两个二进制、两档、开关开关下**全部拒绝**（Sort(0)/Sort(1)）——左边的类型只能由右边定时，旧路径选错 sort。交换两边就过。Lean 的 `binrel%`（`Elab/Extra.lean:256`）先 elaborate 两边，应当接受（推断，未跑 Lean）。走 P2-11c 后在 strict 修掉；loose 是否另修待用户定。
- R1/R2 两档分歧 main 上就有：R1 在 strict 拒绝（main 上漏 `\0soko_m10`）；R2 在 loose 拒绝（即 m5）。
- `units/solutions/I.4/unit12-solution` L171 在 strict 拒绝（main 同样），是 unit12 L285 那处的答案版，要同样改写法（并入 P2-12）。`gaps/C-04`、`gaps/C-112-cb-direct-def-reject` 本来就是拒绝复现件。
- 子代理报的「`Exists.intro 0 rfl` 开关开时 rc=1、可能回退」**不成立**：8 种配置全是 `unknown identifier rfl`（课程库无 `rfl`），复现件写错。

### P3 清理

- ~~删 `SOKO_STRICT_TYPED_APP` / `SOKO_STRICT_V2` / `SOKO_STRICT_ALLOW_FALLBACK` / `SOKO_V2_LOCAL_CONVERT` 与 `type_oracle`~~：
  **本分支上本来就没有**（P2-D 盘点，主会话 grep 复核 0 处），是旧分支的东西 ⇒ 不用做。`SOKO_STRICT_AT` 是有文档的逃生门，**保留**。
- 剩下的：`SOKO_STRICT_KERNEL_APP` 在 P2-12 去掉；`audit-strict-paths` 摘过来后把 strict 专属的文本调用降到 0
  （loose / tactic / goal 共用的不算）；§9.39「零文本」还覆盖记法层级、match/let、具名实参、goal 视图等非应用路径，逐项列在 P2-D 盘点里，按需另开刀。
- **判据**：audit strict 专属项 0；grep 不到 `SOKO_STRICT_KERNEL_APP`；docs-lint 全绿。

### P4 发版

- 默认档由用户拍板（REQUIREMENTS §9）；批次制，一次 push。

---

## 5. 旧分支的处置

不合并。只摘：

- `compile/telescope.rs` + `rebuild_from_kernel`（内核 telescope，P1 输入）；
- `strict/report.rs`（诊断出口）；
- `strict_app/gate.rs` 的"不回退"规则、`scripts/audit-strict-paths.py`；
- `strict/tests/*` 的形状用例（改写成新路径的行为测试）；
- 交接书 §6 负结果表（背景）。

其余 shadow / compare / convert / normalize / binding_view / view 丢弃；P1 证明内核元变量可用则 store 一并丢。

---

## 6. 未核实

- `Exists.imp` 根因未定论（旧分支的问题，新路线不依赖它）；
- 暖路径（artifact 装载）上内核声明是否总在 builder 里未验（F1 以回落兜底）；
- loose 的隐式插入是否与 strict 走同一求解路径，只读了入口；
- 全部读数只在 macOS/arm64 上量过；语料调用数比交接书少 8,467（可能是构建差异）。
