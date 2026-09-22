# 编辑并发：别的 LSP 怎么做（调研 + 我们的方案）

> 2026-09-21。起因：用户问
>
> > 并发正确性……这个我认为完全可以优化，你调研一下其他开源项目怎么做的。
> > **编辑同一个地方，那就取消前一个编译。编译不同的地方，那代码块都不一样，
> > 触发的编译热更新的地方都不一样。不管怎么样，多次编辑不应该导致性能变差。**
>
> 结论：**三条都对**，而且 Lean 自己的做法比这三条更进一步。下面是三家
> 成熟实现的机制（都读过源码/设计文档，链接在每节末尾），以及我们要做什么。

## 1. clangd：每个文件一个 worker + 丢掉过时的写 + 防抖

[clangd 的 threads 设计](https://clangd.llvm.org/design/threads.html)：

> its methods should not block - that would block incoming messages which could
> be independent (code completion in a different file) or relevant
> (**cancelling a slow request**).

* **`TUScheduler` 为每个打开的文件维护一个 `ASTWorker`**（各自一个队列 + 一个
  线程）。⇒ **一个文件上的操作不阻塞另一个文件**。
* worker 的循环做两件事：
  * **扔掉过时的操作**：被取消的读；**"写后面紧跟写"（例如连续两次击键）**；
  * 执行第一个仍然有效的操作：写 ⇒ 重建 AST（必要时重建 preamble）并发布诊断；
    读 ⇒ 把 AST 交给回调。
* **防抖**：对重建较慢的文件，**收到第一个改动不立刻开始重建**——等到"来了一个
  读"**或**"一个很短的截止期到期（用户停手了）"。
  文档里给的理由很具体：用户快速敲 `foo();` 时，敲完 `f` 就开始建，会
  "always see a diagnostic 'unknown identifier f'"，而且"we'll never see the
  correct diagnostics until after 2 rebuilds"。
* 代码补全**不等** AST 是最新的：它用"此刻手上有的那一份"（补全对延迟最敏感）。

⇒ 用户的第一条（同处编辑取消前一次）与第三条（多次编辑不该更慢）在这里是
**同两条机制**：丢掉"写紧跟写" + 防抖。

## 2. Lean 4 的 server：按**命令**切任务链 + 从改动处重启

[`Lean.Server.FileWorker`](https://leanprover-community.github.io/mathlib4_docs/Lean/Server/FileWorker.html)：

> elaboration is executed in a **chain of tasks**, where each task corresponds to
> the elaboration of **one command**. When the elaboration of one command is
> done, the next task is spawned. **On didChange notifications, we search for the
> task in which the change occurred. If we stumble across a task that has not yet
> finished before finding the task we're looking for, we terminate it and start
> the elaboration there**, otherwise we start the elaboration at the task where
> the change occurred.

* **粒度是"一条命令"**，不是整个文件 ⇒ 改动**之前**那些命令的成果**全部保留**。
  这正是用户的第二条（"编译不同的地方……热更新的地方都不一样"）——
  Lean 把它做到了极致：不是"整份重编"，而是"从改动那条命令起重编"。
* 请求**不阻塞主线程**：请求自己是一个 task，沿着任务链走；如果它等的那个 task
  被终止了（说明请求要找的命令**之前**有改动），请求返回
  **"content changed"** 错误，而不是傻等。
* `WorkerState.reporterCancelTk : IO.CancelToken` —— "Token flagged for aborting
  `doc.reporter` **when a new document version comes in**"。
* `WorkerContext.maxDocVersionRef` —— "Latest document version received by the
  client, used for **filtering out notifications from previous versions**"。
* 另有 `handleCancelRequest`（LSP 的 `$/cancelRequest`）。

⇒ 这就是用户三条直觉的完整形态：**同处编辑取消**（terminate + restart at the
change）、**异处编辑保留前面的成果**（task chain）、**多次编辑不变慢**
（版本过滤 + 取消令牌）。

## 3. rust-analyzer：salsa 的按需取消

[`CheckCanceled`](https://docs.rs/ra_ap_base_db/0.0.28/ra_ap_base_db/trait.CheckCanceled.html)：
salsa 的每个 query 在边界上可以 `check_canceled()`；有新版本（revision）时它返回
`Cancelled`，整条计算链**自己退出**。不需要外部的取消信号——
**"有没有更新的输入"本身就是取消条件**。

## 4. 我们要做什么

现状（实测，见 `docs/PERF.md` 与缺口 G-29）：

| | 今天 |
|---|---|
| 编辑一次 | 3–5s（**整条 import 闭包从零重编**） |
| 编译期间 | **整个 LSP 冻结**（第一个只读请求等 8907ms） |
| 连续编辑 | 串行处理，**每个都编一遍** |

四条，按依赖顺序：

### 4.1 编译 spawn 出去，消息循环不阻塞（T-A30）

clangd 那句话就是判据："methods should not block"。改法：

* `did_change` 的 handler **不 await** 编译，spawn 一个任务；
* 但**文本要同步落进文档**（否则后续编辑与只读请求看到的是旧文本）；
* 写回时按**版本号**校验（新版本来了就丢弃这份结果）；
* 编译期间到达的只读请求：**用手上前一份已完成的报告**回答，
  而不是等（clangd：补全"用此刻手上有的那一份"）。

> 踩过的坑（第一版为什么失败）：① 只把编译挪出锁不够——**消息循环是串行的**，
> 后面的请求等的是 handler；② 版本校验若用"文本相等"，**首次打开**时文档还是
> 空的 ⇒ 结果被丢弃（18 个测试红）。要用**版本号**，并且在取快照时就把它推上去。

### 4.2 同处连续编辑：丢掉中间态（防抖 + 取代）

* **防抖**：收到 `didChange` 不立刻开编，等 `-- soko:debounce` 默认 **~150ms**
  的静默期，或"来了一个只读请求"（clangd 的两条件）。
  这直接消灭"敲 7 个字母编 7 次"。
* **取代**：新版本到达时，**已经在飞的那次编译结果作废**（版本校验即可，
  不必真的 kill 线程——Rust 里没法安全地中断内核）。这是 clangd 的
  "writes immediately followed by writes"。

### 4.3 异处编辑：保留改动之前的成果（T-K12/K13 + T-K20）

这是**真正的大头**，也是用户第二条的完整实现。分两层：

* **模块层**（K1，本次优先）：`import` 闭包里**没变的依赖**不重编
  ⇒ 编辑 unit08 省掉 2.16s / 4.89s。
* **命令层**（K2，Lean 那一套）：文件内**改动之前**的声明不重查
  ⇒ 改文件末尾一条定理不必重查前 26 条。
  这需要"每个声明一个检查点"的能力（`EnvBuilder::snapshot()`），
  而且有一个已知陷阱：`conv.rs:169` 按**指针**比较 `NatLit`
  ⇒ 克隆出来的快照必须与新建声明在**同一 arena** 里，否则判定会变
  （`docs/architecture.md` §6 有记）。

### 4.4 多文档：每份文档一个队列（clangd 的 `ASTWorker`）

现在所有文档共用一把 `Mutex<Docs>` ⇒ 编 A 的时候 B 的请求也堵。
改成**每份文档一把锁**（或一个 worker 队列）之后，A 在编不影响 B 的读。
这一条与 4.1 是同一件事的两半。

## 5. 与本计划的对应

| 机制 | 环节 |
|---|---|
| 编译 spawn + 版本校验 + 只读请求用旧报告 | **T-A30**（已量清，见 `docs/PERF.md`） |
| 防抖 + 丢掉中间态 | 新增（T-A30 的一部分） |
| 依赖不重编（模块层） | **T-K12 / T-K13**（K1） |
| 改动之前的声明不重查（命令层） | **T-K20 / K2**（设计文档 `closure-incremental.md`） |
| 每文档一个队列 | T-A30 的另一半 |

**先做哪一条**：4.1 + 4.2 是**纯 LSP 侧**（零内核风险），而且立刻把"冻结 9 秒"
与"敲 7 个键编 7 次"消灭掉；4.3 要动内核，按计划先补 T-K01/K02 的护栏。
