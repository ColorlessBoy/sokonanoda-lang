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
