# P2-12 **S4c 研究**：S4b 之后剩下的 52 次 `ok_notation_unsolved` 定性（2026-11-01 15:20）

> 实施单：[`reports/2026-11-01-p212-s4c-spec.md`](2026-11-01-p212-s4c-spec.md) §2.1（本报告只做**定性**，不改判定）。
> 上游读数：[`reports/2026-11-01-p212-s4b-readings.md`](2026-11-01-p212-s4b-readings.md) §7/§13 ·
> [`reports/2026-11-01-p2-s4b-setle-rootcause.md`](2026-11-01-p2-s4b-setle-rootcause.md)。
> 分支 `strict/2026-10-29-kernel-app`，HEAD = `dba09a05`。**没有 `git commit` / `git push`**。

## 0. 一句话结论

**52 次全部归例外（D4），`old_ok` 不可能到 0**：其中 **26 次是真实源码位置**（6 个不同站点，全是 `Set.Le` 的
`∅ ≼ A` / `A ≼ ∅`）、**另外 26 次是同一批站点在 `judge_infer` 合成的 `#check` 文档里被重新 elaborate 一遍**
（坐标 = 真实坐标 + 1534 − 入口自己的 `import` 行数，逐条验算见 §3）。**探针实测：52/52 次的跳过原因都是
`prefix_meta`** —— 老路自己的求解器（`elab::notation_prefix_args` 的 E19 待定档）对这批站点只能交回**占位符**
（源级 `\0soko_m*` ⇒ 内核 `Expr::Meta`），而内核在**任何工作之前**就拒绝带元变量的输入
（`crates/kernel/src/elab_app.rs:184-186` `Unsupported("input contains a meta")`）⇒ S4b 的重试按设计挡住它们
（§13 的第一道闸 `has_meta`）。要清掉它们只有两条路：**让内核接受带元变量的输入**（内核刀）或
**给占位符新造一种猜法**——两条都被实施单 §4 排除。

## 1. 读数身份（跨轮比较的前提）

| sha16 | 来源 | 用途 |
|---|---|---|
| **`513bb374a143b170`** | HEAD `dba09a05` 的 `target/release/sokonanoda`（14:31，= S4b 交付态） | §2 的位置清单、§4 的基线（**52 = 52**） |
| `4608889bca1b22e4` | 同源码 + **临时探针**（`SOKO_S4C_PROBE`，测完已 `git checkout` 还原） | **只**用于 §4 的跳过原因；不出现在任何判据里 |

每个长跑前 `cp target/release/sokonanoda /tmp/soko-<sha>`，此后只用副本（`scripts/strict/fallback-one.sh` 自己
每次新建缓存目录）。**52 次在两个二进制上逐条相同**（§4 的探针行数与 §2 的清单 1:1）。

## 2. 52 次的 `line:col` 清单

口径：`SOKO_STRICT_KERNEL_APP=1` + `--dialect strict` + `SOKO_KERNEL_APP_LOG=<path>`，
日志行 `<tag> <route><why> <head> <line>:<col>`（`crates/front/src/compile/kernel_app_census.rs:190-207`）。
判定规则（实施单 §2.1）：**行 > 文件行数** 或 **列 > 该行长度** ⇒ **合成文档坐标**；否则真实源码坐标。

### 2.1 `courses/set-theory/units/solutions/I.8/unit19-solution.sokonanoda`（**72 行**，30 次）

| `line:col` | 次数 | 判定 | 该行长度 | 指向的源码 |
|---|---:|---|---:|---|
| `43:43` | 2 | **真实** | 50 | `theorem empty_le (α : Type) (A : Set α) : ∅ ≼ A :=` —— 列 43 = `∅`（`∅ ≼ A` 的起点） |
| `59:46` | 4 | **真实** | 77 | `theorem le_empty … (h : A ≼ ∅) : … :=` —— 列 46 = `A`（`A ≼ ∅` 的起点） |
| `65:56` | 8 | **真实** | 72 | `theorem subset_of_le_empty … (h : A ≼ ∅) : A ⊆ ∅ :=` —— 列 56 = `A`（第一个 `A ≼ ∅`） |
| `1573:43` | 10 | **合成** | 行 > 72 | = `1534 + 43 − 4` ⇒ 就是 `43:43` 那个站点（§3） |
| `1589:46` | 6 | **合成** | 行 > 72 | = `1534 + 59 − 4` ⇒ 就是 `59:46` 那个站点（§3） |
| 合计 | **30** | 真实 14 · 合成 16 | | 6 个不同站点里的 3 个 |

### 2.2 `courses/set-theory/units/I.8/unit19-cardinal-le.sokonanoda`（**158 行**，22 次）

| `line:col` | 次数 | 判定 | 该行长度 | 指向的源码 |
|---|---:|---|---:|---|
| `85:43` | 3 | **真实** | 53 | `theorem empty_le (α : Type) (A : Set α) : ∅ ≼ A := by` —— 列 43 = `∅` |
| `141:46` | 3 | **真实** | 80 | `theorem le_empty … (h : A ≼ ∅) : … := by` —— 列 46 = `A` |
| `149:56` | 6 | **真实** | 75 | `theorem subset_of_le_empty … (h : A ≼ ∅) : A ⊆ ∅ := by` —— 列 56 = `A` |
| `1615:43` | 6 | **合成** | 行 > 158 | = `1534 + 85 − 4` ⇒ 就是 `85:43` |
| `1671:46` | 4 | **合成** | 行 > 158 | = `1534 + 141 − 4` ⇒ 就是 `141:46` |
| 合计 | **22** | 真实 12 · 合成 10 | | 6 个不同站点里的 3 个 |

⇒ **52 次 = 6 个不同源码站点 × 各自的 elaborate 次数**（两个文件各 3 个站点：`∅ ≼ A` 1 个 + `A ≼ ∅` 2 个）。
没有一处列 > 行长而仍落在文件行内的情形（§2 的 26 个"真实"位置的列都指向该处记法的**第一个操作数**，
与 `kernel_app_census::pending_notation` 记的 `span` = 整条记法应用的起点一致 ✓）。

## 3. 合成坐标的机制（把它钉死，而不是"看着像"）

合成坐标来自 `judge_infer` 的 `#check` 文档：`synthesized_prefix(extra_prefix, prefix_src)`
（`crates/front/src/judge.rs:3624-3629`）把 **库层闭包前缀** 接在入口自己的前缀**前面**，再追加一条
`#check fun … => …`。而库层前缀是 `project::importless_source` 逐模块拼接（`compile/check/mod.rs:1227-1240`：
**去掉 `import` 行**、每行补 `\n`）。所以：

```
doc_行(入口第 L 行) = Σ(闭包里每个模块的 行数 − 其 import 行数) + (L − 入口在 L 之前的 import 行数)
```

两个文件的 import 闭包相同（`lib.Equiv · Set · Image · Fun · Rel · Logic · Exists`，**7 个模块 1550 行、
其中 16 行是 `import`**）⇒ **Σ = 1534**。入口自己各有 4 行 `import`（`unit19-solution` 在 1–4 行；
`cardinal-le` 在 28–31 行）⇒ 代入：

| 合成坐标 | 验算 | 真实坐标 |
|---|---|---|
| `1573:43` | `1534 + 43 − 4` | `43:43` ✓ |
| `1589:46` | `1534 + 59 − 4` | `59:46` ✓ |
| `1615:43` | `1534 + 85 − 4` | `85:43` ✓ |
| `1671:46` | `1534 + 141 − 4` | `141:46` ✓ |

**四条一位不差**（同列、同头名），且 **65:56 与 149:56 没有合成孪生**（会落在 `1595` / `1679`，日志里没有）
—— 这正是"某条声明被 `judge_infer` 重编了几次"的差别，不是两批不同的站点。⇒ 26 次合成**不是新缺口**，
它们是 §2.1/§2.2 那 6 个站点的**重放**；"修源码"对它们一个字节都动不了（源码位置本身已经计在真实那 26 次里）。

## 4. 探针读数：52/52 都是 `prefix_meta`（决定"能不能到 0"的那一条）

临时探针（已删，构建 `4608889bca1b22e4`）在 `kernel_app_notation::try_notation`/`retry_with_prefix` 的**每个**
早退点记一行 `<reason> <head> <line>:<col>`（`SOKO_S4C_PROBE=<path>`）。逐文件聚合：

| 文件 | `retry_none` + `prefix_meta`（= 重试被"前导值含占位符"挡掉） | 其余原因 | 该文件的 `ok_notation_unsolved` |
|---|---:|---:|---:|
| `unit19-solution` | `43:43 ×2` · `59:46 ×4` · `65:56 ×8` · `1573:43 ×10` · `1589:46 ×6` = **30** | **0** | **30** ✓ |
| `unit19-cardinal-le` | `85:43 ×3` · `141:46 ×3` · `149:56 ×6` · `1615:43 ×6` · `1671:46 ×4` = **22** | **0** | **22** ✓ |

**1:1 全对上**（每个站点恰好一次 `retry_with_prefix` 尝试、一次 `prefix_meta` 早退；探针里 `would_cut` /
`sites[…]` / `prefix_len` / `prefix_elab_fail` / `retry_unsolved` **一条都没有**）⇒ 这 52 次**没有别的成因**。

机制（与 S4b §13.1 同一条，本刀用读数补全"真实 vs 合成"这一维）：

1. `notation_prefix_args` 对 `Set.Le {α β} (A : Set α) (B : Set β)` 要补 2 位；
   `∅ ≼ A` / `A ≼ ∅` 里空集那一侧的元素类型**没人定**（`?β ≟ ?δ` 闭环）⇒ 求解器的 E19 待定档只能
   **记成待定占位符**（`implicit::fill_pending_by_shape` 要借"同形已解兄弟"，这两个站点借不到）。
2. 占位符在内核项里是 `Expr::Meta` ⇒ `has_meta`（`kernel_app_notation.rs:382-401`）判真 ⇒ **不重试**。
3. 就算硬重试：内核 `elab_app` 先扫输入，见元变量立刻 `Unsupported("input contains a meta")`
   （`crates/kernel/src/elab_app.rs:184-186`）⇒ 老路照样接住 ⇒ `fallback_old_ok_*` 只是**换个原因**，
   不会减少（S4b §13.1 实测：`88 次重试 = 58 Solved + 30 Unsupported`）。

## 5. 结论与边界（给 S4c 实施部分与 S6 的口径）

1. **归例外（D4 已决）**：这 52 次是"内核按设计拒绝带元变量的输入"这一条的**可见后果**，与那
   30 + 14 次 `ok[_notation]_unsupported_input_contains_a_meta` **是同一件事的不同趟**
   —— 本报告用读数把它钉死：**同一批站点、同样的前导值、同样的占位符**。
2. **`old_ok` 到不了 0**：清掉它们要么改 `elab_app`（接受元变量 / 给占位符合一），要么给前端新造一种
   "占位符怎么补"的猜法。实施单 §4 两条都排除 ⇒ 本刀**不宣称**这一项为 0。
   **能到 0 的那部分**已由 S4b 做完（`120 → 52`），剩下的 52 应当按 D4 写进 `REQUIREMENTS §9.39` 的已知偏差。
3. **对"真余量 10 次"的影响**：那 10 次里 **`ok_unsolved` 4** 与这 52 次**同族**（内核 `Unsolved` 而老路
   靠占位符形状借用接住），只是走的**应用臂**而不是记法臂（`Set.inter` / `Set.image`，见
   `reports/2026-11-01-p212-s4c-readings.md` §5 的逐类定性）⇒ 本刀只对**能靠"把题出完整"消掉**的那几类负责。
4. **σ 观察（回写给 S6 的验收设计）**：52 次里有一半（26 次）**根本不是源码位置**，而是 `judge_infer`
   重编前缀时的**重放**。⇒ 任何拿 `fallback_old_ok` 当"用户可见缺口数"的口径都要先做这一层过滤，
   否则判据会把"判定跑了几遍"算成"缺口有几个"。

## 6. 复现命令（逐条可执行）

```bash
bin=/tmp/soko-513bb374a143b170        # = HEAD 的 target/release/sokonanoda（sha16 513bb374a143b170）
# ① 位置清单（含合成坐标）
SOKO_KERNEL_APP_LOG=/tmp/log19.txt scripts/strict/fallback-one.sh $bin \
  courses/set-theory/units/solutions/I.8/unit19-solution.sokonanoda
grep '^ok notation_unsolved' /tmp/log19.txt | awk '{print $NF}' | sort -t: -k1,1n -k2,2n | uniq -c
# ② 合成坐标的验算（1534 = 7 个库模块 1550 行 − 16 行 import；入口 4 行 import）
#    1573 = 1534 + 43 − 4 · 1589 = 1534 + 59 − 4 · 1615 = 1534 + 85 − 4 · 1671 = 1534 + 141 − 4
# ③ 跳过原因（探针构建，已删）：SOKO_S4C_PROBE=/tmp/probe.txt + 同一条命令 ⇒ 全是 prefix_meta
```
