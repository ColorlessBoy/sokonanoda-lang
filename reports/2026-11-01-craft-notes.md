# 接手笔记：上一棒两天的做法（2026-10-30 / 10-31 共 64 笔），哪些要抄

> 起因：2026-11-01 用户点名「学习这个程序员这两天的计划和开发」。本文只写**有证据的做法**，
> 每条都给提交 sha 或文件行号；最后一节是**我这一棒的对照检查**（哪里没达到这个标准、已经改了哪些）。
> 这不是评价，是给自己下一刀用的检查表。

## 1. 节奏：一刀 = 两笔提交（代码 + 文档）

那 64 笔里 `feat/fix/test` 与 `docs(strict)` 交替出现，几乎每笔代码后面紧跟一笔
`docs(strict): P2-XX 已并入（<sha>）…读数`（例：`429c6edf` → `211c9dad`、`4d9986c2` → `49364ae1`、
`b95866f8` → `ec50cd16`）。⇒ **计划文档永远不落后于代码**，下一棒读到的 `replan §4` 就是最新读数。

我这棒的问题：S0b + S4a 的文档攒成一笔（`e36d2a4a`）才补。**下一刀起改成两笔。**

## 2. 提交信息里有一段独立的「判定正确性」论证

`0c95f317`（P2-11a-B）有专门一节 *Judgement correctness*：
「只在旧检查说 false 的地方重试 / 只接受实例化后真 defeq 的对子 / `Solved` 仍过 `instantiate_mvars` +
`app_recheck` + `add_declar` 重检 / 闸门关时这条代码路径不可达」。
`b95866f8` 同款。⇒ 读者不必读 diff 就知道**为什么接受/拒绝不变**。

我这棒：S4a 的提交里写了「陈旧值/各处域」，但**没有**这种"逐条排除"的论证。

## 3. Lean 4 块写到 file:line，纯搬移写 `Lean4: n/a`

`0c95f317` 的 Lean4 块给了 4 个位置（`Meta/ExprDefEq.lean:1402`、`:146/:268`、
`Meta/WHNF.lean:621`）+ Lean 怎么做 / 我们怎么做 / 差异 / 决定。纯搬移的 `57318279`、`e1b65f28`
直接一行 `Lean4: n/a（纯搬移，行为不变）`。⇒ 钩子（`lean4-guard`）从来不会卡他们。

我这棒：`5192a0d9` 因为漏了 `Lean4: n/a` 被 commit-msg 钩子打回、重做了一次。

## 4. 需要行数时**先**腾，别撞线再回头

`57318279`：`notation_level_texts` 搬到 `notation_levels.rs`，`elab.rs 8857 ⇒ 8824 ≤ 基线 8841`。
`e1b65f28`：`loose_bvars` 搬到 `kernel_app_postpone.rs`，`kernel_app.rs 501 ⇒ 474 ≤ 500`。
两笔都是**独立 `refactor:` 提交**，提交信息里直接写前后行数。

我这棒：`conv.rs` 892→913 撞了 `docs-lint ⑨` 才回头拆成 `conv_hint.rs`（多花一轮）。

## 5. 报告用 Q1…Qn 结构，每个数字标「实测 / 推断」

`p2-11a-argmismatch.md`：`Q1 Repros — measured` · `Q2 Root causes` · `Q3 Lean 4` · `Q4 Proposed fixes (design only)` ·
`Q5 Expected effect (inferred from samples, not re-measured over 251 files)`。
`p2-l-m5-root-cause.md` 同款。⇒ **inferred 不许冒充 measured**，读者一眼看得见证据强度。

我这棒：S4b 普查报告的读数（子代理产出）带了构建身份与口径，但我自己在 replan 里转述时没标来源强度。

## 6. 负结果**也**落一笔提交，并把 diff/bundle 存档

`42d9b0de`：P2-10 降级不并入 —— 写清「重试在 7 个文件上 `fallback_unsolved` 一次没降、`KERNEL_APP_MISS` 不变
⇒ 只多做功」，存档路径进交接书 §6。
`fcdb66a8`：P2-11e 方案 B 不提交 —— 「`fallback_shape` 0→103、`solved` −754 未解释」。
⇒ **"没做成"本身是交付**，而且写清"为什么不做"比写"做了什么"更省下一棒的时间。

## 7. 测量脚本自带自检（把事故变成约束）

`140d1ca0`：`corpus-hashes.sh` 改成「不再原地覆盖钉住副本」+ **自检行数**（`2 × 文件数`）+
**空哈希位置**（只许 `lib/Logic`）。起因是 2026-10-31 19:28 那次 12 个进程卡死。
⇒ 事故 → 脚本里的一条断言，而不是一句话。

## 8. 判据分四层，缺一层就是洞

每刀的判据都是：**复现件（修前红）→ 修（绿）→ 反向开关（关掉必须红）→ 端到端**（`add_declar` 接受 /
CLI `grade` 的 rc 与诊断）。例：`p211b_*` 除了正向还写了 `p211b_genuine_surplus_still_unsupported`
（**假阳性对照**），我这两天的 S0b/S4a 也照这个补了（`kernel_names_never_leak`、`filled_late` 关掉记 0）。

---

## 9. 我这一棒的对照检查（逐条打分，诚实）

| # | 他们的标准 | 我这棒 | 处理 |
|---|---|---|---|
| 1 | 一刀两笔提交（代码 + docs） | ✗ 攒了两刀才补 docs | 下一刀起两笔 |
| 2 | 提交里有「判定正确性」论证 | ✗ S4a 缺 | S3/S4b 并入时补这一节 |
| 3 | Lean 4 块 file:line / 搬移写 n/a | △ 有块但漏了 clippy 那笔 | 每次 `git commit` 前先自查两个关键词 |
| 4 | 先腾行数再动码 | ✗ 撞线才拆 | **动码前先 `wc -l` 预算**（`elab.rs` 8841 顶格、`kernel_app.rs` 现 482） |
| 5 | 报告标 measured / inferred | △ | 报告里每个数字带来源 |
| 6 | 负结果落提交 + 存档 | △ 写了但没落独立提交 | S4b 若出现负结果，照此办 |
| 7 | 测量脚本自检 | ✓ 新增的 `strict_fallback_census.rs` 就是对账自检 | 保持 |
| 8 | 四层判据 | ✓ 复现/修/反向/端到端都有 | 保持 |

## 10. 下一刀（S3 审核 + S4b）按这张表执行

- **S3 审子代理产出**：查 ① 开关关语料 0 差异；② c1 文字无内核名且 span 不在 0:0；③ `Fallback` 剩余原因
  是否与 `kernel_app_report` 的四种例外**一一对应**；④ `elab.rs` 是否仍 8841；⑤ 反向：关掉新开关 ⇒ 逐字节回到 S2。
- **S4b 第一刀 = `Set.Le` 记法族**（121 次）：先只读报告（Q1 复现件 / Q2 根因 / Q3 Lean 4 / Q4 修法 / Q5 推断效应），
  **不要**先写码；复现件必须修前红。位置坐标先按普查 §5 换算回真实行。
