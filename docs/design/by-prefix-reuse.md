# 按前缀复用（by-prefix reuse）：编辑一个画布时，前 18 遍到底在重算什么

> 计划条目 **T-K10**（线 K：内核提速）。这份文档只做一件事：把"每次 `update`
> 都要把**前缀**重新过一遍内核"这件事讲清楚，并把三个候选方案摆在一张桌上
> ——**它们的侵入面 / 解锁收益 / 正确性风险**，以及**为什么推荐 K1-b 而不是 K1-c**。
>
> 写作纪律（吃过教训）：**下面每一处 `file:line` 都是写文档时实测过的**
> （调研稿里有几处行号已经漂了，例如 `judge_pairs_uncached` 实际在
> `crates/front/src/judge.rs:399`，不是 418）。改代码后请顺手更新本文的引用。

---

## 1. 现状：一次 `update` 的调用链（实测）

```
session.rs:265  let trust = TrustPlan { … };          // 只记"前缀到哪"
session.rs:276  run_incremental(&file, &options, &trust, &prefix_failures)
                    └─ compile/check/mod.rs:500  run_incremental
                         └─ walk.rs  Walk（逐命令）
                              ├─ walk.rs:330  lower_value(…)      ← **每次都在跑**
                              ├─ walk.rs:378  if trusted { … }    ← 只决定"要不要 push PendingOp"
                              │     walk.rs:388  build_def(…)     ← **trusted 分支里也在跑**
                              │     walk.rs:409  return
                              └─ 非 trusted：push PendingOp::Decl
                         └─ kernel_phase：把 PendingOp 逐条送**内核**终审
```

一份画布（`playground.sokonanoda` 或单元解答）里，**前 18 遍左右的 `update`**
（用户每敲一次 `by` / `intro` / `exact`）都会把**同一条前缀**重新 elaborate、
重新构造 `Declar`，然后把它们送进内核。前缀越长，这份重复越贵。

`judge_pairs_uncached`（`crates/front/src/judge.rs:399`）是另一条独立入口：
它为了判一条 `#prove`/judge，会把**整份前缀**再跑一遍
（`judge.rs:594` 的注释自己就写着"每一次 `judge_pairs_uncached` 都要把整份前缀重跑一遍"）。

---

## 2. 两条与直觉相反的纠正（**必须知道**，否则会挑错方案）

### 2.1 `TrustPlan` **不是**环境复用：它只省"内核重查"，不省 elaborate

`front::session` 的 `TrustPlan`（`crates/front/src/compile/check/mod.rs:120`）看起来像
"前缀不用重算了"，**不是**。`walk.rs:137` 只是算出一个 `trusted` 布尔：

```rust
let trusted = trust.is_some_and(|t| idx < t.before);   // walk.rs:137
```

而 trusted 分支**在跑完 `lower_value` 之后**才分叉，并且**照样 `build_def`**：

```rust
let lowered = match lower_value(…);                    // walk.rs:330（trusted 与否都跑）
…
if trusted {                                           // walk.rs:378
    if let Ok(decl) = build_def(…) { … }                // walk.rs:388（**照样构造 Declar**）
    return;                                            // walk.rs:409（只是**不 push** PendingOp）
}
```

`theorem` 同理（`walk.rs:561` lower_value → `:586` trusted → `:593` build_theorem）。
**它省下来的只有"把前缀再送一次内核终审"**，而 elaborate 与 `Declar` 构造一个字都没省。

> 这也是为什么"给 `TrustPlan` 加缓存"不是提速方案：真正贵的那一段它压根没碰。

### 2.2 真正的障碍不是 arena 借用，而是 `NameNode::decl_idx`

想"把上一次的内核对象留下来复用"时，第一个想到的墙是 arena 生命周期
（`docs/architecture.md` 的政策段写着："Session 每次 update 都开新 arena——
跨 update 只复用渲染后的快照，不复用内核对象"）。但那堵墙**不是最硬的**。

最硬的是 **`NameNode::decl_idx`**：一个**挂在被 intern 的 `NameNode` 上的全局可变槽位**：

* 写：`crates/kernel/src/builder.rs:244`
  `name.as_ref().set_decl_idx(u32::try_from(idx)…)`；
* 读：`crates/kernel/src/env.rs:273` / `:297`（`n.as_ref().decl_idx()`），
  **读的时候不做名字校验**。

`NameNode` 是**跨 builder 共享**的（intern 表的语义就是"同一个名字给你同一个节点"）
⇒ 用 builder A 造的 `Declar` 去查 builder B 的环境，会**静默取到别人的声明**
（槽位被 B 覆盖过），而不是报错。**这是"跨 update 复用内核对象"必须先解决的东西**，
也是 K1-b 把 `with_env` 做成"显式环境"而不是"靠名字查"的原因。

#### 被否的方案（K1-c）：把 `&ArenaRef` 存进 `ExportFile`

看起来最省事："让 `ExportFile` 记住自己的 arena，下次接着用"。
**走不通**，两条：

1. `ExportFile: Sync` 是**并行检查**需要的 —— `tc.rs:210 check_all_declars_par`
   经 `tc.rs:223 thread::scope` 把声明分给多个线程；
2. `stumpalo::ArenaRef` **既非 `Send` 也非 `Sync`**（它内部是裸指针风格的分发器）
   ⇒ 把 `&ArenaRef` 存进 `ExportFile` 会让 `ExportFile` 不再是 `Sync` ⇒
   并行路径编译不过（或被迫串行化，把已有的并行收益吐回去）。

---

## 3. 三个候选（三栏齐全）

| 候选 | 侵入面 | 解锁收益 | 正确性风险 |
|---|---|---|---|
| **K1-a** 纯 front 的 judge TrustPlan 复用 | **front 约 60–150 行**（`judge.rs` 三处入口 + 把失败表从 `run_pass` 穿到 `lower_value`/`run_by`/judge）；**内核 0 行** | 每个 pass 省掉"前缀所有声明的**内核检查**"。占比 × ~19 遍由 T-K03 定：占一半 ⇒ 36.1s→~18s；占 80% ⇒ →~8s | **低**：复用的是**已上线、已有逐字节对拍**的机制（`crates/cli/tests/judge_batch.rs:52 assert_same_both_ways`） |
| **K1-b** `EnvBuilder::with_env`（**推荐**） | 内核**一处**：给 builder 一个"显式环境"入口（`EnvLimit` 的兄弟），front 侧把"上一次的前缀环境"喂回来 | 省掉**前缀的 elaborate + Declar 构造 + 内核检查**（K1-a 只省最后一项）——这才是"19 遍重复"的大头 | **中**：必须绕开 §2.2 的 `decl_idx` 静默取错（显式环境 = 不靠名字查 ⇒ 从设计上避开）；`skip` 语义要与 check-then-add 逐字一致 |
| **K1-c** 跨 update 直接复用 arena/`ExportFile` | 看起来最小（"记个指针"） | 理论最大（连 elaborate 产物都留着） | **不可能**：§2.2 末尾两条（`ExportFile: Sync` vs `ArenaRef` 非 `Send`/`Sync`）⇒ 编译不过；硬要做得给 arena 加同步层 ⇒ 侵入面反而最大 |

**为什么 K1-b 优于 K1-c**：K1-c 想省的东西**更多**，但它撞的是**类型系统的墙**
（不是工程量的墙）——「`&ArenaRef` 进 `Sync` 结构体」在当前 `stumpalo` 版本下
**没有安全写法**；要绕开就得自造 arena + 同步层，那是**把内核的分配器换掉**，
远超"按前缀复用"这件事该有的侵入面。K1-b 把问题缩小成
"**给我一个显式环境，我保证只查它**"——一发一中，且**从设计上**消掉
`decl_idx` 那条静默取错的路径。

---

## 4. 每个候选的正确性风险与验收

**共同红线**（内核解冻后不变）：同一批输入**接受/拒绝不变、事件计数不变、
golden 与 `--json` 逐字节不变**。三者都必须过 `scripts/kernel-check.sh` 五步
（见 `docs/architecture.md` §6.1）。

### K1-a（先做，零内核风险）

- 开关：`SOKO_JUDGE_ENV_REUSE=0/1`（仿 `judge.rs:588` 的 `SOKO_NO_JUDGE_BATCH`）。
- 验收：两态下**全语料 `--json` 逐字节相同**（`assert_same_both_ways` 已具备）；
  `unit12-solution` 的耗时数字进 `docs/perf/ledger.jsonl`。
- 三处必须守死：① 只有外层 pass **对前缀的判定可担保**时才允许复用；
  ② trusted 前缀**不产 `DeclState`/事件** ⇒ `judgement_of` 只读 `_soko_judge_k`，
  **必须加断言**防止"报告缺状态"被误用；③ `skip` 语义与 pass 2 的 check-then-add **逐字一致**。

### K1-b（推荐，单次内核改动里性价比最高）

- 验收：K1-a 的全部，**再加**"前缀 elaborate 只做一次"的可观测证据
  （例如内核侧计数器或 front 侧 debug 计数进 `perfNote`）。
- 风险点：`decl_idx` 的静默取错 —— 判据要**故意构造**"两个 builder、同名不同声明"
  的小用例钉住（内存 API 层，`crates/kernel/tests/memory_api.rs` 是那层最硬的钉子）。

### K1-c（不做，理由见上）

- 若将来 `stumpalo` 提供了 `Send + Sync` 的 arena 句柄，**重新评估**；
  在那之前它属于"看起来最省、实际要换分配器"的方案。

---

## 5. 依赖与顺序

```
T-K01 全语料逐字节对拍（kernel-diff.sh）──┐
T-K02 验收五步 + 语料收集修复 ────────────┼─→ K1-a（T-K11，零内核改动，先做）
T-K03 分阶段 profile（36.1s 花在哪）──────┘        └─→ K1-b（T-K12，推荐）
                                                        └─→ K1-c：不做（记录在案）
```

**顺序的理由**：K1-a 的收益上限**取决于 K1-a 之外的那部分占多少**
（内核检查在 prefix pass 里占几成）——那是 T-K03 的 profile 要回答的。
所以：**先量（T-K03）→ 先做零风险的那个（K1-a）→ 再动内核（K1-b）**。
