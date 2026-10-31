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

| 刀 | 内容 | 判据 | 状态 |
|---|---|---|---|
| P2-1…P2-8 | 推迟实参、多轮、重检、记法、探测、缓存键 | 见交接书 `2026-10-31-strict-handoff.md` §1 | ✓ |
| P2-9 | 内核：`expected` 里允许外层元变量（注册 + 内层 `meta_base` 防撞号）；`NeedArgs.blocked` 带域；拆 `elab_app_holes.rs`（P2-7 设计 §1/§6 第 3–4 步） | 内核单测（外部元变量被赋值 / 留空 ⇒ Unsolved / 撞号反向验证）；开关关 38 文件逐字节 | 进行中 |
| P2-C | **普查**（只读）：整门课开关开，每文件按回落原因计数；钉住 P2-7 §5 R1+R2 两档分歧 | 一张表 + 结论回写本节 | 进行中 |
| P2-10 | 前端：`elab_ready` 把 blocked 的非 λ 位按带元变量的域作提示 | R3/R4 计数降；`TEST_NO_EXTERNAL_METAS` 反向 | 待 P2-9 |
| P2-11 | 按普查结果清剩余回落原因（每类：修 or 定为报错） | 普查表每类归零或有处置 | 待 P2-C |
| P2-12 | strict 默认走内核路径：去开关；解不出 ⇒ 报错点名 binder，不回落 | lib 17/17；整门课 strict 全通过；loose 逐字节 | 待 P2-11 |
| P2-13 | 删旧 typed app / 文本回落 / 影子档 | 同上 + audit 归零（衔接 P3） | 待 P2-12 |

### P3 清理

- `audit-strict-paths` 三项归零、基线改 0；删 `SOKO_STRICT_TYPED_APP` / `SOKO_STRICT_V2` /
  `SOKO_STRICT_ALLOW_FALLBACK` / `SOKO_V2_LOCAL_CONVERT` 与 `type_oracle` 文本回落。
- **判据**：audit 三项 0；grep 不到这些开关；docs-lint 全绿。

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
