# 设计：内核声明表持久化 / COW（**T2-A**）

> **性质**：T2-A 的**契约**（形状 / 不变式 / 判据 / 反例）。过程进 commit message 与
> `STATUS.md`，读数进 `docs/perf/ledger.jsonl`。
> **上游**：`docs/notes/PLAN-align-lean4.md` §3.3（T2-A 的三条候选形状与五条判据）。
> **Lean 侧引用**：`~/Documents/lean/lean4/src/Lean/Data/SMap.lean` ·
> `src/Lean/Environment.lean` · `src/Lean/Server/Snapshots.lean`（v4.28.0 本机源码）。

## 0. 问题与判据

**根因（实测）**：`DeclarMap` 是 `FxIndexMap`（`crates/kernel/src/env.rs:256`）
⇒ `EnvBuilder::clone` 逐条复制声明表 + 六张 intern 表。结构计数（先建先红 ✓
`crates/kernel/src/builder.rs` 的 `env_clone_cost_is_constant_in_declaration_count`）：
**64 条声明 = 196 条目、512 条 = 1540 条目**（`declars` 512 + `Dag` 1028）⇒ **O(#decls)**。
⇒ 入口趟按命令边界取快照在数据结构上是 **O(N²)** ✗。

**判据（缺一不算 · 照 PLAN §3.3）**：① 命令边界克隆复制的**条目数** O(1)（结构计数，两臂相等）；
② 指针同一性**两向**判据保持绿；③ 全语料 `--json` **逐字节**；④ 内核单测 + `kernel-diff.sh --fast`
零差异；⑤ **不许**引入第二份 intern 表。

## 1. Lean 4 对照（逐条机制，不抄代码）

| 机制 | Lean 4 | 本文（我们） |
|---|---|---|
| 环境常量表 | `Environment.constants : ConstMap`（`Environment.lean:232`）· `abbrev ConstMap := SMap Name ConstantInfo`（`:96`） | `DeclarMap` |
| 分层 | `SMap { stage₁ : Bool, map₁ : Std.HashMap, map₂ : PHashMap }`（`SMap.lean:28-33`） | `base`（平坦）+ `local`（即 `map₂` 位） |
| 切换 | `switch`：把插入从 `map₁` 切到 `map₂`（`:96-98`）；导入期 `stage₁ = true`（`fromHashMap m false` 用于导入，`Environment.lean:2241`） | `seal()`：库层趟结束 ⇒ 插入切到 `local` |
| 查找 | `find?`：stage₂ 先 `map₂` 再 `map₁`（`:53-56`） | `get`：`local` 先、`base` 后 |
| 为什么这样切 | 原话（`:17-25`）：导入条目**远多于**本地；HashMap 比 PHashMap 快；读导入文件时**独占** ⇒ 走破坏性写 | 同构：库层趟独占、入口趟增量 |
| 逐命令状态 | `Command.State`（含 `env`）＋ `Snapshot`（`Server/Snapshots.lean:28-33`） | 本文 §2.3 的 `EnvSnapshot`（T2-B 的载体） |

**一处刻意偏离** ✗：Lean 的 `map₂` 是**真 PHashMap**（HAMT，插入 O(log n)）；
我们的 `local` 用 **`Arc` + COW**（插入摊还 O(1)，但快照后**第一次**插入付一次
O(#local) 复制）。理由：本例 `#local` = **入口文件自己的声明数**（课程 ≤ ~120），
而 `#base`（库层）才是 O(#decls) 的来源 ⇒ 把"不该复制的部分"变成共享，收益的
**量级**已经拿到 ✓；换成 HAMT 的边际收益 ≤ 常数倍，却要新造一整个持久化索引结构
（含**位置索引**与**保序迭代**两个额外要求）。**边界写在这里**：若某天入口文件
大到 `#local` 与 `#base` 同量级，再按 `SMap.map₂` 换 HAMT（判据不变）。

## 2. 形状

### 2.1 分层 `DeclarMap`

```rust
pub(crate) struct DeclarMap<'a> {
    /// 库层（导入层）：平坦 `FxIndexMap`；**封层前**独占、允许破坏性写（对齐 `map₁`）。
    base: Arc<FxIndexMap<NamePtr<'a>, Declar<'a>>>,
    /// 本地层（入口趟新增）：`Arc` + COW ⇒ **克隆只动根指针** ✓（对齐 `map₂` 的位）。
    local: Arc<FxIndexMap<NamePtr<'a>, Declar<'a>>>,
    /// `true` ⇒ 插入进 `base`；`false` ⇒ 插入 `local`（对齐 `SMap.stage₁`）。
    stage1: bool,
}
```

`Clone` = 两次 `Arc` 提升 + 一个 `bool` ⇒ **O(1)** ✓（判据①）。
保持既有 API：`len` · `get` · `contains_key` · `get_index` · `iter` · `keys` ·
`Index<usize>` —— **`get_index`/保序迭代必须留**（`inductive.rs:28/400` 按**下标区间**
扫归纳块；`pretty_printer.rs:356` 保序迭代）。

### 2.2 `Dag`（intern 表）：**不进快照**

**单调 intern 引理**：intern 表只增不删、节点在 arena 内**不可变**（`intern(v)` 命中即返回
**同一个**指针）⇒ 表在时刻 `t` 的状态恒是其后任意状态的**子集**，且对已存在项
`intern` 恒返回同一指针。⇒ **用"更新的表"重放**旧命令，得到的指针与首次运行**逐字节相同** ✓。

⇒ 命令边界快照**不含 `Dag`**：一趟一个活表，所有快照共享它（判据⑤：**只有一份** intern 表 ✓）。
收益：快照的 `Dag` 分项从 O(#interned) 直接归 **0**（不是"少复制一点"）。

### 2.3 `EnvSnapshot`（命令边界的**环境**句柄）

```rust
pub struct EnvSnapshot<'a> {           // Clone = O(1)
    pub declars: DeclarMap<'a>,
    pub notations: NotationMap<'a>,     // 同一套 Arc + COW
    pub mutual_block_sizes: ...,
}
```
`EnvBuilder::restore(&mut self, snap)` 把三张表装回（`Dag` **不动** ✓）。
命令边界上 `block_in_progress` 恒为 `None`（归纳块在一条命令内闭合）⇒ 不进快照。

### 2.4 `notations` / `mutual_block_sizes`

同 `local` 的形态（`Arc` + COW）⇒ 克隆 O(1) ✓。它们只随 `notation` 命令 / 归纳块增长
（**稀有**）⇒ COW 的复制几乎不发生。

## 3. 不变式（破一条就是静默错判 ✗）

1. **`decl_idx` = 位置**：`add_declar` 拿 `declars.len()` 当槽位（`builder.rs:491`）
   ⇒ 分层后位置 = `base.len() + local 序号`；**封层后 base 位置永不改变** ✓。
2. **指针同一性**：快照/恢复只搬 `Arc` 与指针，**从不重建**项（`ExprPtr` 的
   `PartialEq` 比地址位）—— 既有两向判据（`builder.rs`）不许放松。
3. **`cutoff` 语义**：`EnvLimit::ByName(n)` 的可见前缀 = `decl_idx` 之前
   ⇒ `Env::get_old_declar` 改"**按名字查 + `idx < cutoff`**"（不再是 `declars[idx]`），
   语义等价（同名项唯一）且不再依赖位置索引 ✓。
4. **一份 intern 表**：所有快照共享同一个 `Dag`；`restore` 不动 `Dag`。
5. **封层后不许再写 `base`**：`seal()` 之后 `insert` 一律走 `local`（否则共享的
   `Arc` 被 `make_mut` 整份复制 ⇒ 判据①当场退回 O(#decls) ✗）。

## 4. 判据与反例

| # | 判据 | 位置 |
|---|---|---|
| ① | 克隆条目数**两臂相等**且 ≤ 小常数（64 vs 512 条声明） | `builder.rs::env_clone_cost_is_constant_in_declaration_count`（**先建先红** ✓） |
| ② | 指针同一性**两向** | `builder.rs::a_reused_environment_is_pointer_identical_and_a_rebuilt_one_is_not` |
| ③ | 全语料 `--json` 逐字节 | `scripts/check-json-identity.py`（发版大节点） |
| ④ | 内核单测全绿 + `kernel-diff.sh --fast` 零差异 | 既有 |
| ⑤ | **反例**：装载/恢复后同一名字**不许**造出第二个 `NamePtr` | 既有 `from_export_file_carries_the_intern_tables_not_a_rebuilt_dag` 的判据② |

**反向验证**（撤掉修复必须重新判红）：把 `seal()` 之后的插入改回写 `base`
⇒ 判据① 的两臂相等**当场判红** ✓。

## 5. 与 T2-B 的接口（不在本文实现）

快照点 = `crates/front/src/compile/check/walk.rs:457` 命令循环的**循环体末尾**
（此处 `builder` 恰好含 `[0..=idx]` 的全部声明）。复用判据**沿用** `EntryCache`
（命令文本 + 起点）+ 依赖指纹 + 库层键；新增读数 = **elaborate 命令数**
（改最后一条 ⇒ **1**）。

---

## 6. T2-B（**下一件** · 入口趟命令级环境快照）—— 可落地方案

### 6.1 形状

* **快照点** = `crates/front/src/compile/check/walk.rs` 的**命令循环体末尾**
  （`for (idx, &(unit_idx, command)) in flat.iter().enumerate()`，循环体在 `:666` 闭合 ⇒
  在此处 `builder` 恰好含 `[0..=idx]` 的全部声明 ✓）。
* **快照内容**（计划 §3.3 的 `(EnvBuilder, PassTables, 报告片段)` 的**精确化**）：
  **整份 walk 状态**，不是只有环境 —— `EnvSnapshot`（§2.3）+ `known` / `inductives` /
  `defs` / `out` / `ops` / `cmd_hovers` / `decl_states` / `ns` / `exports` / `example_idx`
  （字段清单 = `walk.rs:117-203`）。理由：任何一件没留，恢复出来的就是**另一个判定**
  ⇒ 静默错编 ✗（红线）。
  **Clone 现状（2026-10-09 编译探针实测 ✓）**：`Walk` 的字段类型**全部可 `Clone`** ——
  探针查出只有 `PendingOp` / `CmdHover`（及其内层 `HoverNode`）缺 `#[derive(Clone)]`，
  **已补**（纯加法、零行为变化 ✓）⇒ 第 1 步只剩"逐个字段搬一遍"的机械件 ✓。
* **恢复**：以它当新 `Walk` 的起点，只走 `flat[k..]`。

### 6.2 三个硬约束（都不是"选择"）

1. **arena 必须活过按键**：`Walk<'arena, 'shadow>` 借 arena，而入口趟今天用的是
   **栈上 arena**（`session.rs` 的 `run_entries` 不走泄漏路）⇒ 快照跨 `didChange`
   会**悬空** ✗。出路：入口趟改走**泄漏 arena**（`Box::leak`，同 T1-A 的
   `LEAKED_LIB_ARENAS` 手法与 `MAX_LEAKED_LIB_ARENAS=8` 上界）。
2. **快照成本必须与 #decls 无关**：`EnvSnapshot` ✓ 已经 O(1)（T2-A）；
   其余累加器今天都是 `Vec` / `FxHashMap` ⇒ 每命令 O(状态) ⇒ 全留是 **O(N²)** ✗。
   两条出路：**(a) 有界窗口**（W=4：编辑点落在最后 W 条内 ⇒ 命中；成本 O(W·状态)）；
   **(b) 把累加器也做成共享 / 持久结构**（大件，照 §2 的分层手法逐个来）。
   **先做 (a)**（它已经满足判据的"改最后一条 ⇒ 1"✓），(b) 留给"改第 k 条 ⇒ N−k+1"。
3. **复用判据一律沿用现成的、不新造键**（计划 §3.3 的纪律）：`EntryCache.keys`/`starts`
   （命令文本 + 起点）+ `dependency_fingerprint` + `lib_key` ⇒ **结构相等**，
   **不是哈希缓存**（避开 B4 被否的那条路：错键 = 静默用旧目标判定 ✗）。

### 6.3 判据（读数**已先建先红** ✓）

* `elaborated_commands_total()`（`crates/front/src/compile/mod.rs`）：**改最后一条 ⇒ 1**
  （今天该夹具 = **43**，入口 12 条 + 库层 + pass2）；改第 k 条 ⇒ N−k+1；
  **反向验证**：改依赖文件 ⇒ 回到 N。
* 守卫：`crates/front/tests/keystroke_structure.rs::
  t2b_last_command_edit_still_reelaborates_every_entry_command`（今天断言 `≥ N`，
  **T2-B 落地后改判成 `== 1`，不许放宽** ✗）。
* 红线：全课程 `--json` 逐字节 + `keystroke_structure.rs` 既有四条断言**一个字不放松**。

### 6.4 as-built（2026-10-09 落地 ✓）

* **第 1 步（形状）**：`WalkCheckpoint`（**只装 `Walk` 的字段** + 四个循环局部量 ⇒
  恢复时前缀**整段跳过**，不必"重放记账" —— 重放会把 `namespace`/`open` 的累加抹掉 ✗）
  ＋ `Walk::checkpoint()` / `restore_from()`；`run` 收 `resume: Option<&WalkCheckpoint>` 与
  `snapshot_tail`，前缀用 `continue` 跳过 ✓。
* **第 2 步（跨按键）**：**关键前置件 = 把 units 与 arena 寿命解绑**
  （`run_pass_with` / `install_all_preludes` / `run_entries` 的 `&[SourceUnit<'_>]`）——
  否则"入口趟跑在 `'static` arena 上"与"units 是本次调用的"不可能同时成立 ✗。
  检查点存线程局部 `ENTRY_TAILS`（`RefCell<Vec<EntryTail<'static>>>`，去重 + 上界 8 ✓），
  只在 `LibCheckpoint` 是 `'static` 的三条路（LRU 命中 / 磁盘产物 / 整条重建）接；
  栈上 arena 那两条（CLI / 前缀续编）传 `None` ⇒ **与今天逐字节相同** ✓。
* **本轮的界**：只留**一个**边界（`cp.idx = 命令数 − 2`，W = 1）⇒ 满足判据
  "改**最后一条** ⇒ 1" ✓；"改第 k 条 ⇒ N−k+1"要**有界窗口 W>1**（每边界一份）⇒ 下一刀 ✓。
* **读数收口**：判据只数**入口趟**（`count_entry_commands`；不数库层/别的趟 ✗）
  ⇒ 今天 = **入口命令数 N**（本夹具 12）；先建先红那一版是 43（没收口时）✓。
* **会话接线（同日第二轮 · 已落地 ✓）**：线程局部 `ENTRY_TAILS`（按 `(库层键, 入口)`
  去重 + 上界 8）；只在 `LibCheckpoint` 是 `'static` 的三条路接（LRU 命中 / 磁盘产物 /
  整条重建），栈上 arena 那两条传 `None` ⇒ **与今天逐字节相同** ✓。
  **判据当场达标**：改最后一条命令 ⇒ 入口趟 `elaborated = **1**` ✓。
* ⚠⚠ **两道闸门（都是被守卫逼出来的 ✓，缺一就是静默错编 ✗）**：
  1. **报告所有权**：快照**不装回**报告侧累加器（`out`/`ops`/`cmd_hovers`/`decl_states`）
     —— 前缀那段的报告由**既有的 `EntryCache` 拼接**提供 ✓。装回会让每条声明**出现两次**
     （22 vs 13）✗（守卫 `t2b_resumed_report_has_no_duplicate_declarations` 逮到 ✓）。
  2. **信任闸门**：只在 `plan.before > cp.idx`（前缀**确实被信任**）时才续编 ——
     否则拼接不会提供那段报告 ⇒ 报告**缺声明** ✗（`query::tests::
     entry_trust_skips_the_prefix_only_when_it_is_unchanged` 逮到 ✓）。
  3. **快照点必须在循环体内**（`idx + 2 == flat.len()`）✗→✓：循环**之后**取，
     `self` 已经是"全部走完"的状态却被标成 `len-2` ⇒ 最后一条被**加两次**
     ⇒ `duplicate declaration d02` ✗（同一个守卫逮到 ✓）。
* **本轮已备好的前置件**（都在位、且**行为零变化** ✓）：`WalkCheckpoint` /
  `checkpoint()` / `restore_from()` / `run` 的 `resume`+`snapshot_tail` /
  `run_pass_with` 的三个参数 / units 与 arena **寿命解绑**（`run_pass_with` ·
  `install_all_preludes` · `run_entries`）/ 线程局部读数（`note_elaborated_command`）。

> ⚠ **它单独不解决 goal 延迟**：T2-B 只回答"**从哪条开始编**"；要 goal 进 3ms 量级
> 还需要**按命令发布** goal 状态（Lean 的 `AsyncList` + `waitFindAtPos`，见
> `docs/notes/perf-lean4-bench.md` §8.1/§8.3）✓。

## 7. 第三件：**按命令发布 goal**（对齐 Lean `AsyncList`/`waitFindSnapAtPos`）—— 可落地方案

**读数定位**：T2-B 落地后 goal 口径 **13.48ms → 8.72ms** ✓（Lean 4 = 3.1ms）⇒ 还差
**"把 goal 从'每次重算'变成'从已发布的快照里取'"** 这一件 ✓。

### 7.1 Lean 怎么做（`~/Documents/lean/lean4/src/Lean/`，v4.28.0 ✓ 已核）

| 机制 | Lean 4 | 我们 |
|---|---|---|
| 快照链 | `Snapshot { stx, mpState, cmdState }` **每条命令**一份，`AsyncList` 串成任务链（`Server/Snapshots.lean:28-33` · `Server/AsyncList.lean:21-23`） | 只有**每趟**的报告（`DocumentReport`），没有"每命令一份" |
| 取法 | `withWaitFindSnapAtPos p = waitFind? (fun s => s.endPos ≥ p)` ⇒ **二分找 ≤ 光标的最近一份**（`Server/Requests.lean:340/357-363`） | `soko/goals` 现从**整份报告**里按声明找 ⇒ 每次按键后才有答案 |
| 代价 | 查询**不触发重算**（快照已在链上） | 8.72ms 里含"这一刀的 elaborate + 内核检查 + 报告序列化" |

### 7.2 我们的形状（可落地）

1. **入口趟每条命令结束时发布一份 `CmdSnapshot { cmd_idx, end_pos, goal_view }`**
   （`end_pos` = `command.span().end` 已有 ✓；`goal_view` = 该命令之后
   `DeclState.goal`/`by_root` 的那一份 ⇒ 不必新算，**切出来共享**即可 ✓）；
   会话持有 `Vec<CmdSnapshot>`（`Rc`/`Arc` 共享，同 `WalkCheckpoint` 的纪律：**只借 arena** ✓）。
2. **`soko/goals` 改走"取 ≤ 光标的最近一份"**（二分 ✓）⇒ 请求**不触发编译**；
   响应仍走同一份 `GoalPayload`（协议不变 ✓ `docs/protocol.md`）。
3. **发布表与报告的所有权二选一** ⚠（同 T2-B 那条纪律）：goal 由**发布表**给，
   报告仍给诊断/洞 ⇒ 两者**不重复计算** ✓。

### 7.3 判据（结构计数，不用墙钟）

* ① **不重算**：一次 `soko/goals` 请求的 `elaborate`/`by_calls` **增量 = 0** ✓
  （今天 > 0 —— 请求要等/触发那一刀）；
* ② **答案等价**：同一 fixture 下"发布表取出的 goal" vs "整趟重编后的 goal"
  **逐字段相同** ✓（反向验证：故意把发布表的 `end_pos` 改错 ⇒ 必须判红 ✓ 咬得住）；
* ③ **陈旧面**：改**最后一条**之后，光标在**倒数第二条**上取到的 goal 必须**与改前相同** ✓、
  在**最后一条**上必须**变了** ✓（这条钉住"陈旧但不该变 vs 该变没变"两种错 ✗）。

### 7.4 顺序（每步自带判据，缺一步不许往下）

1. **先量账**：把 8.72ms 拆成"elaborate 1 条 + 内核检查 + 报告序列化 + 请求往返"
   （`SOKO_STAGE_STATS` + LSP 侧计时）⇒ 才知道第三件能拿回多少 ✓（**不许**跳过这步直接写 ✗）；
2. 发布表**只写不读** ＋ 判据 ②；
3. `soko/goals` 改走发布表 ＋ 判据 ① ＋ ③；
4. 若 ① 达标而 goal 仍 > 3ms ⇒ 余量在**序列化/往返** ⇒ 那时才动 wire（另立设计 ✗）。

### 7.5 ⚠ **量账结果（2026-10-09）：7ms 里不含编译** —— 第三件要**重新定位** ✗→✓

同一构建、同一夹具、只换量具的 settle 模式（**无需探针** ✓）：

| 臂 | `goal_ms`（5 轮） | `first_diag_ms` |
|---|---|---|
| `--settle warm`（默认：等后台预热落定） | 6.93 / 8.93 / 7.04 / 6.97 / 6.93 | 21.4–24.8 |
| `--settle diag`（**先等诊断落定再敲下一刀**） | 7.03 / 8.90 / 7.41 / 7.43 / 7.45 | 22.3–23.3 |

⇒ **几乎不变** ✓ ⇒ **`goal_ms` 里不含"那一刀的 elaborate + 内核检查"**（那部分在
`first_diag_ms ≈ 22ms` 那一侧 ✓）。

**这条推翻了 §7.4 的默认假设**：把 goal 改成"从已发布的 per-command 快照里取"，
**在 `goal_ms` 上省不到东西** ✗（编译本来就不在计时窗里）。⇒ 重新定位：

* **真正的 7ms 在哪**：请求处理 + **响应负载（序列化/体积）** + stdio 往返
  （两侧用**同一** Python 夹具 ⇒ 客户端开销是**共同项** ✓，可比 ✓）。
* **下一步（§7.4 的替代）**：先量 **`soko/goals` 响应的字节数**与它的组成
  （是否把整份报告/全部声明的 goal 都发了 ✗）；Lean 侧同一请求的负载大小做对照 ✓。
  判据仍是**结构计数**：负载字节数 + 服务端处理步数（不许用墙钟 ✗）。
* **仍然成立的部分**：`CmdSnapshot` 的形状与 §7.3 的三条判据（"请求不触发编译"）
  —— 只是它服务的是**别的**目标（多光标/陈旧面语义、查询不打编译），**不是** 3ms ✗。

### 7.6 ⭐⭐ **负载量出来了：91 KB / 一次单点 goal 查询** —— 95% 与 goal 无关 ✗

客户端侧量（**零探针** ✓：`/tmp` 里的临时 LSP 客户端，服务端二进制 = 本轮 test profile ✓）：

```
soko/goals 响应 = 91358 B · decls 27 条 · goal_ms = 7.4ms（与独立量具 6.93–8.93 一致 ✓）
每字段合计：ty_runs 46136 B (51%) · value_runs 17819 B (20%) · goals_runs 10796 B (12%)
            goal_runs 10778 B (12%) · binders 4884 B (5%) · range 2111 B (2%)
            ty 1666 B · holes 1325 B · value 815 B
```

⇒ **"run"类（语法/语义高亮的 run 数组）占了 95%** ✗，而这次查询**要的只有光标那一条**的
goal 视图（`goal`/`binders`/`by_root`/`holes` ≈ 1–2 KB ✓）。**这就是那 ≈7ms 的去处**
（序列化 + 传输 91 KB，而 Lean 的 `plainGoal` 只回该位置的 goal ✓）。

### 7.7 下一刀（**具体到接口** ✓）

1. `soko/goals` 只回**该位置需要的**那份（`goal`/`binders`/`by_root`/`holes`/`sub_goals`
   ＋声明定位所需的 `name`/`kind`/`range`/`status` ≈ **1–3 KB** ✓）；
2. **run 类字段**（`ty_runs`/`value_runs`/`goals_runs`/`goal_runs`）留给**专门的高亮/装饰请求**
   （编辑器侧的语义高亮本来就有独立通道 ✓）；
3. **判据（结构计数 ✓ 不用墙钟 ✗）**：同夹具同位置回归 ⇒ **响应字节数 91358 → ≤ ~3000**
   （≥30× ✓）且 **`goal_ms` 随之下移**（只作参考 ✓）；**契约不许松**：
   `scripts/audit-wire-fields.py` 仍绿 ✓（消费方真正读的字段一个都不许少 ✗ ——
   扩展侧实测读了 `.decls`/`.errors`/`.warnings` ⇒ 先确认 `decls` 的**哪些子字段**
   真被渲染，再决定裁到哪一层 ✓）。

### 7.8 ⭐⭐⭐ **决定性对照：`goal_ms` 与负载大小同阶** ⇒ 3ms 就在裁剪里 ✓

同一构建、同一客户端，只换夹具（**零探针** ✓）：

| 夹具 | 响应字节 | `goal_ms` | `decls` |
|---|---|---|---|
| 课程单元 `unit08-images-preimages`（27 条声明） | **91358** | **7.4** | 27 |
| `/tmp` 小文件（2 条声明） | **2316** | **0.74** | 2 |

⇒ **≈10× 的负载差 ⇒ ≈10× 的 `goal_ms` 差** ✓（同一构建、同一路径、同一客户端开销 ✓）。
**结论**：那 7ms 的主体就是\*\*"把整份报告序列化 + 传出去"\*\* ✗，**不是**编译、不是快照、
不是 arena ✓ —— 而 Lean 的 `plainGoal` 本来只回**该位置**的 goal ✓。

⇒ **按这条曲线，裁到"光标处那一份"（≈1–3 KB）落在 `goal_ms ≈ 0.7–1ms`** ✓
—— 比 Lean 4 的 **3.1ms 还快** ✓。**目标可达，路径唯一且已量化** ✓（§7.7 的三条就是它）。
⚠ 附带的诚实边界：小文件夹具的高亮 run 天然少 ⇒ 这条对照证明的是"**负载 ∝ 声明数、
`goal_ms` ∝ 负载**"✓；要把它变成交付，仍需 §7.7 第 3 条的**契约守卫**（裁完
`audit-wire-fields.py` 仍绿 + 扩展侧渲染不降级 ✓）。
