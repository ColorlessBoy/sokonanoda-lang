# 「治本」计划：把"求一个项的类型"从文本重读改成检查模式

日期 2026-11-03 · 状态：**待用户确认** · 硬约束：714 行逐字节 0 差异 · 四门绿 · 课 258/2191/0
· 源码文件 ≤500 行（`elab.rs` 在行数上限，只许减）· G3 不泄漏内部名

## 0. 已知与未知（先说清）

**已知**：`unit06-solution` / `G30` 各 2 次 `judge_infer` 失败，失败码 `elab-implicit-argument-unsolved`；
term 是渲染出来的**证明项**（丢了前导隐式实参）；调用链 `operand_type_expr_explicit → infer_type_text(explicit=true)
→ judge_infer_explicit`；loose 一次都不问；每次失败重编整份前缀（≈50 KB）。

**未知（必须先查）**：**是哪个源级构造触发了那次"求操作数类型"**。unit06 的定理陈述里 `∧` 没有前导隐式，
所以触发点不是它 —— 若方案按错误假设设计就会打偏。**Stage 0 的唯一目的就是消掉这个未知。**

## 1. 分阶段（每阶段独立可交付、可回退）

| 阶段 | 做什么 | 判据（缺一不可） | 估时 | 消掉 2 个的概率 |
|---|---|---|---|---|
| **0 定位** | 用 `SOKO_KERNEL_APP_LOG` + span 过滤 + `#[track_caller]` 把触发点钉到**源级构造**（哪个记法/哪一行）；做成最小复现件 | 复现件 strict 下出现那 2 次失败、loose 下 0 次 | **0.5 天** | —— |
| **1 就地化** | 该调用点改成"**就地 elaborate**（`judge_infer_inplace_*` 已有）；失败返回 `None`，**不再重编整份前缀**"。若就地路不可用 ⇒ 改用零内核的 `operand_type_src`（`expected_spine.rs` 已有），扩到"有前导隐式的头" | 714 行 0 差异 · 2 文件 fails→0 且 calls/prefix = loose · 四门绿 · 课 258/2191/0 | **1 天** | 50–60% |
| **2 检查模式（真根）** | 让"求类型"走 elaborate 的检查模式：内核 `elab_app` 的域交付**放宽到"含未解元变量也交"**，前端把 term 交给内核自己 `infer` 再与域合一（内核本来就会）；前端拿不到期望类型时不再渲染文本 | 同 Stage 1 + 内核 131/131 | **2–3 天** | 85% |
| **3 判卷器快路** | 慢路（合成 `#check` 文档、整份重编）降级为**对拍专用**；常规路径全走就地/域检查 | 全语料 strict 的 `JUDGE_STATS.prefix_bytes` 不再高于 loose | **2–3 天** | 99% |

**合计**：Stage 0+1 ≈ **1.5 天**（这是我建议先授权的范围）；做到 Stage 3 ≈ **5–7 天**（保守 10 天）。

## 2. 每阶段的验证成本

一次改动 = `cargo build --release` 2 min + 714 行对拍 2.5 min + 四门 1 min + 整门课 2 min ≈ **8 min/轮**，
一天可跑 4–6 轮。**全量对拍不能省**（逐字节 0 差异是硬约束，引擎换路最易破它）。

## 3. 风险与应对

| 风险 | 应对 |
|---|---|
| Stage 0 定位推翻假设（触发点不是记法） | Stage 0 交付"复现件 + 调用链"后才定 Stage 1 方案；方案变了先回来报告 |
| 换引擎路径改 golden | 每步全量 `--json` 对拍；任何差异**停下来定性**，不许"顺手修" |
| `elab.rs` 只许减不许增 | 任何新增逻辑先**拆出去**（`operand_type_expr_explicit` 一族本来就该搬），拆完再加 |
| 判卷器独立性被削弱 | Stage 3 保留慢路做**对拍开关**（`InPlace` 影子模式已有），两条路必须逐字相同 |
| 工作量低估 | Stage 1 结束重新估一次，再决定是否进 Stage 2/3 |

## 4. 我会怎么汇报

每个阶段结束给：改了什么（文件/行）、判据读数（714 行差异数、2 文件的 fails/calls/prefix、四门、课程）、
以及"下一步是否需要你拍板"。**没达判据就不说完成。**

## Stage 0 结果（2026-11-03 完成）

用 `infer_type_text` 入口探针（打印操作数 span + 渲染文本，跑完已还原）定位到**唯一触发点**：

| 文件 | 触发 span | 触发者 |
|---|---|---|
| `unit06-solution` | `331:6`（外层 `And.intro` 的**第二个实参** = 内层 `And.intro (fun…) (fun…)`） | `try_implicit_application` 的 `arg_types` → 第二候选 `alt_type` |
| 同上（递归复入） | `1:117`（同一段 1462 字节文本，行号从 1 起 = **判卷器合成文档**） | 判卷器重编时又问了同一个问题 |
| `G30` | `43:27..43:30`（`mem_singleton_iff` 假设）× 2 + `47:15`（lambda） | 同一族 |

**结论**：提问发生在「**隐式实参插入**求解前缀之前、要拿每个实参的类型」这一步
（`crates/front/src/compile/elab.rs` 的 `arg_types` / `alt_type`，`try_implicit_application` 内）。
实参是**应用形态**（不是 Ident/lambda/常量）⇒ 主候选（源级线索）答不出 ⇒ 第二候选去问判卷器
⇒ 判卷器把这段 1462 字节的渲染文本当 `#check` 的查询项重编 ⇒ 文本丢前导隐式实参 ⇒
`elab-implicit-argument-unsolved` ⇒ 每次失败重编整份前缀。

**对方案的含义**：Stage 1 的"就地化"救不了这个点 —— 就地路径同样要先 elaborate 那个实参，
而设计上实参**必须等前缀解出来**才 elaborate（期望类型要代换）。
⇒ 真正的修法是给 **"求应用类型"** 一条不经过文本的路：让内核**自己 infer 已 elaborate 的项**
（Lean `synthesizeType` / `inferType`），也就是 Stage 2 的检查模式。**Stage 0+1 的 50–60% 预期下调。**

## Stage 1 结果（2026-11-03）：**Lean 顺序那条路已经存在，但只对 loose 开着**

`crates/front/src/compile/elab.rs:4733`：

```rust
if !strict_at && (args.len() > declared.explicit_arity() || fits_old_style) {
    ...
    for (i, a) in args.iter().enumerate() {
        let expected_src = crate::spine::substitute(&layers[i].domain, &sigma);
        let t = elab_expr(builder, a, …, None, Some(&expected_src), ctx)?;   // ← 对着域 elaborate
        out = builder.mk_app(out, t);
    }
}
```

这正是 **Lean `elabAndAddNewArg` 的顺序**（逐个实参对着**域** elaborate，解出来的值代换进后续域），
但它被 `!strict_at` 挡掉 —— strict 走的是另一条「**先要类型、再解前缀**」（`arg_types` → 判卷器）。

**为什么 strict 不能直接开这条**：它把源级实参**逐位**贴到 `layers[0..]`，而 strict 的规则是
「位置实参只按**显式**形参绑定」。对 `And.intro (fun…) (fun…)`（layers = `a`隐 `b`隐 `ha` `hb`），
逐位贴会把两个 lambda 塞进**隐式**位 `a b` ✗ —— 那是 loose 的读法。

**⇒ Stage 2 的精确工作面**（也是目标里那句"逐个交域"的落点）：

| 要做的 | 落点 |
|---|---|
| 让 strict 也能"对着域 elaborate 实参"，但**只贴显式层**、隐式层走**内核元变量** | `elab_app`（内核会话已有元变量）+ `run_once`（把每位的 `want` 从 `expected_srcs[i]` 换成内核交出的域） |
| 位次与配对：显式实参 ↔ 望远镜里的**显式层**（跳过隐式层） | `run_once` 的循环 |
| 域可含未解元变量时也要能交（现在 `app_need_args` 把这类判 blocked 不交） | `app_need_args` |
| 拿不到期望类型时**不再渲染文本去问判卷器**（消掉那 2 次 `fails`） | `pp_shape::arg_types` / `elab.rs:4855` 的第二候选 |

判据不变：714 行 0 差异 · 四门绿 · 课 258/2191/0 · 2 个文件 calls/prefix = loose。

⚠ 风险如实标注：`arg_types` 那条路同时服务 **IA-1 插入**与**群体解**，动它影响的文件面很大
（我上一轮用形状谓词试过，714 行对拍出现 512 行差异 ⇒ 立刻回滚）。所以 Stage 2 必须**先加开关**
（`SOKO_STRICT_DOMAIN=1`），对拍通过后才默认开。

## Stage 2 探针（2026-11-03）：两条实验，都回滚了（负结果有价值）

### 2a. 内核已交出 `blocked` 位，前端一直丢掉 —— 但补上**对这两个文件无效**

`AppOutcome::NeedArgs { domains, blocked }` 里 `blocked` = 「域里还带未解元变量」的位（内核已经
`export_owned` 好了）；`grep blocked crates/front` = **0 命中** ⇒ 前端从来不用。

补上"把 blocked 位也 elaborate 一次、回填、下一轮内核合一"（开关 `SOKO_BLOCKED_ARGS`）后：
**两个文件的 calls/prefix/fails 一字未变**。原因清楚：这两个站点的提问发生在 **IA-1**
（`try_implicit_application_with`）里，**根本没走到 `elab_app`** ⇒ blocked 通道够不着。

### 2b. 写全参数就**跳过 IA-1**（开关 `SOKO_SKIP_IA1_FULL`）—— 消掉了失败，但**改了判卷**

| 文件 | 关 | 开 |
|---|---|---|
| `G30` | prefix 4,640 · fails 2 · json `2f66beef` | **prefix 0（= loose）· fails 0** · json **`1d33fa3f`** ✗ |
| `unit06` | calls 26 · prefix 1,512,484 · fails 2 | calls **29** · prefix **1,642,160**（更差）· fails 0 · json **`4efd5388`** ✗ |

⇒ **结论（这轮最有用的一条）**：IA-1 与它后面的老路**语义不同** —— IA-1 是 strict 的「隐式前缀求解」
（按显式层对齐、用实参类型反解隐式位），老路是 loose 的「逐位贴」读法。所以"绕过 IA-1"必然改判卷 ✗。

### 因此 Stage 2 的正确定义

> **不是**绕过 IA-1，而是**把 IA-1 的求解器换成内核元变量版**：保留它今天的**绑定规则与判卷结论**，
> 只把"求实参类型"这一步从「渲染文本问判卷器」（有损、每次失败重编整份前缀）换成「内核自己 infer
> 已 elaborate 的项」。判据仍是：714 行 0 差异（**判卷结论不变**）+ 两个文件的 calls/prefix 降到 loose。
