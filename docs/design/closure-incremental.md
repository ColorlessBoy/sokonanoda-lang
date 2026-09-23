# 闭包增量：一次编译，入口之间共享（K2）

> **状态**：设计 + spike（T-K20，2026-09-21）。**实现未做**（T-K21…）。
> **动机**：用户在同一个 VS Code 窗口里依次打开 `unit01` → `unit08` → `unit12`，
> 今天是**三趟完整闭包编译**（实测 **14.86s**）——而它们的闭包**大量重叠**
> （`lib/Logic` / `lib/Set` 每次都在里面重编一遍）。
> **spike 实测**：只编新增模块 ⇒ **8.89s（−40.2%）**，与计划里"15.06s → ≈8.5s"
> 的目标一致。

## 1. 现状：一个 Arena + 一个 EnvBuilder 跑整个闭包

```
project::plan_project          闭包加载（解析 + 环 + 阻断传播）   project/mod.rs
  └─ compile_all_units         闭包编译的唯一入口                  compile/units.rs:113
       └─ run_pass             一次跑完**整个**闭包                compile/check/mod.rs:635
            ├─ Arena::new()     一个新 arena（所有模块的项都住这里）
            ├─ EnvBuilder::new(arena, Config)
            └─ prelude 安装      **按闭包**决定装什么              check/mod.rs:644-690
```

要点：**闭包是一等公民，入口不是**。`run_pass` 每次从零建 arena + builder，
把闭包里每个模块的命令按拓扑序跑一遍，跑完把扁平事件流按模块切开
（`split_report`）。所以"再打开一个入口"= 把重叠的部分**从头再编一遍**。

## 2. 两个本刀独有的障碍（不能靠"再克隆一次"绕过）

### 2.1 O7：prelude 决策是"按闭包"的 —— 会**静默改变判卷**

`check/mod.rs` 的安装判据看的是**整个闭包**：

| 判据 | 规则 |
|---|---|
| `explicit_nat` | **任一**单元自带顶层 `inductive Nat` ⇒ 不装 prelude 的 `Nat` |
| `explicit_bool` | 同上（`Bool`） |
| `taken` | 整个闭包顶层名字的**并集** ⇒ `Eq` / L1 里撞车的那些名字让位 |

⇒ 一份共享环境对某个入口可能装着**它闭包里根本不存在**的 `Nat`/`Bool`/`Eq`
——那不只是"多装了东西"，而是**同一个源文件会判出不同结果**（名字解析变了）。

**守卫**：复用之前比对两件事的 **prelude 形状**：

```rust
pub struct PreludeShape {          // compile::prelude_shape(units)
    pub explicit_nat: bool,
    pub explicit_bool: bool,
    pub shadowed: Vec<String>,     // taken ∩ PRELUDE_NAMES（排序）
}
```

* **形状相同才复用**，否则整编。
* `shadowed` 用**撞车集**而不是整个 `taken`：后者会把"两个闭包的用户名字不同"
  误判成形状不同（而用户名字不同**不影响** prelude 装了什么）。
* **安装判据与守卫判据是同一个函数**（`run_pass` 现在就走 `prelude_shape`）
  ——两份手写的判据迟早会漂，而漂的后果是静默改变判卷。

**脚本化验证**（计划明确要求的）：`crates/front/tests/prelude_shape.rs`
——课程里**每一个** `.sokonanoda`（lib + units + solutions + 顶层）的闭包形状
都必须一模一样，且不撞 prelude 名字。调研说"是"，这个测试把它变成**代码保证**：
哪天有人给某个单元加一行 `inductive Nat`，测试立刻红。

### 2.2 O8：front 的每轮状态都是 `run_pass` 的局部量

跨调用复用环境，这些必须**一起**提升为会话状态（漏一个就是"上一轮的名字还活着"
或者"这一轮的声明看不见"）：

| 状态 | 作用 | 位置 |
|---|---|---|
| `known` | 名字 → 声明种类（补全/着色/`classify`） | `check/mod.rs` |
| `inductives` | 归纳表（`match`/`cases`/构造子） | 同上 |
| `defs` | 源级 delta 表（`by` 引擎看穿 def 头） | 同上 |
| `ns` | 命名空间栈 + `open` 集合（**单元边界要 reset**） | `NamespaceScope` |
| `exports` | 跨单元导出表（单元切换时重放） | 同上 |
| `GoalTemplates` | 建议材料（refine/intro） | `check/mod.rs:690-704` |
| `closure_prefixes` | 每个单元的闭包前缀（judge 合成文件用） | `check/mod.rs` |

**注意 `ns` 的语义**：`open` 是**文件**作用域、不跨 `import`（设计 N5），所以
单元边界必须 `reset`——复用时这条不能忘（"共享环境"不等于"共享命名空间栈"）。

### 2.3 报告归因不能漂

`split_report`（`units.rs:44-56`）把扁平报告按单元切开并**重基命令下标**
（`decl.cmd` / `hover_cmds` / `error_cmds` / `check.cmd`）。它**不用 span**
（不同文件的 offset 不在同一个坐标空间里）。

⇒ 增量复用之后，"上一轮已经编过的模块"的 `PendingOp` / `DeclState` / 事件
**也必须一起留着**，否则入口报告会缺掉依赖里的声明（`soko/goals` 的声明栏、
跨文件导航、`soko/project` 的模块表都会变）。**判据是逐字节不变**。

## 3. 候选

### 3.1 K2-b（窄版，**先落地**）

> 只缓存**最近一次**编译的 `(闭包模块集合, Arena + Env + front 状态)`。
> 新入口的闭包若是旧集合的**超集**、且 **prelude 形状相同**、且旧集合是它的
> **合法前缀**（downward-closed：每个已编模块的依赖也都在里面），
> 则只编**新增模块**；否则整编。

* 好处：改动面小（一次编译的状态），风险可控，立刻拿到 spike 量出的 −40%。
* 代价：只对"同一个窗口里按顺序打开、闭包单调增长"这一条路径有效
  （**恰恰是 VS Code 里最常见的那条**）。
* **合法前缀**是硬条件，不是优化：拓扑序里 `A` 在 `B` 之前要求 `A` 的依赖都在
  更前面 ⇒ 只编新增模块等于把已编集合当成前缀，它必须 downward-closed。

### 3.2 K2-a（结构正解，**之后**）

> per-module-root 的 `ProjectSession`：一次编译、入口间共享 + 全局拓扑序 +
> **模块级**缓存（按内容哈希）。

* 好处：任意入口顺序都受益（不只单调链）、`build <dir>`（T-K30）也一起受益。
* 代价：要动 `project` 层的生命周期与缓存失效，风险高。

## 4. spike 实测（`scripts/spike-closure-incremental.py`）

只量不改。对单调链 `unit01 → unit08 → unit12`，每个入口**冷编译**一次
（全新缓存目录，release 构建）：

| 入口 | 闭包模块 | 冷编译 | 只编新增 | 合法前缀 | 新增模块 |
|---|---|---|---|---|---|
| `unit01-sets-membership` | 3 | 1848ms | 1848ms | ✓ | Logic Set unit01 |
| `unit08-images-preimages` | 5 | 4125ms | **2277ms** | ✓ | Exists Image unit08 |
| `unit12-synthesis` | 8 | 8886ms | **4761ms** | ✓ | Equiv Fun Rel unit12 |

**依次打开三个入口：今天 14859ms → K2-b 8886ms（−40.2%）**，三个前缀全部
downward-closed ✓。

**读法（别过度解读）**：
* "只编新增"是**上界估计**——它假设已编模块的成本在两个入口里一样
  （边际 = 本次 − 上次）。真实实现还要加上"把已编模块的报告/状态留住"的开销。
* 收益的**形状**是"每多打开一个入口，省的越多"：第一个入口没有可复用的，
  之后每个都只剩新增模块。
* 这与计划里记的目标（`lib/Set` 1.98s / `unit08` 4.59s / `unit12` 8.49s，
  总 15.06s → ≈8.5s）一致 ✓。

## 5. 实现切片（T-K21…）

1. **状态提升**：把 §2.2 的七样从 `run_pass` 的局部量提成 `ClosureSession`
   （不改行为，`run_pass` 仍然每次新建一个）。
2. **形状守卫**：`prelude_shape` 比对（已有 ✓）+ "合法前缀"判定
   （从 `LoadedModule.imports` 算，`project` 层已有数据）。
3. **复用路径**：命中守卫时只跑新增模块，把已编模块的报告/事件/`PendingOp`
   **按原顺序**拼回去。
4. **失效**：内容哈希变了（或 `ns`/`exports` 的单元边界没对齐）就整编。
5. **判据**（本刀总）：同进程依次打开 `lib/Set` / `unit08` / `unit12` 的总耗时
   从 15.06s 降到接近**一次最大闭包**；`soko/project` 的模块表与跨文件导航
   **逐字节不变**；`scripts/kernel-check.sh` 全绿。

## 6. 风险（高）

§2 的四样（prelude 形状、`decl_idx` 槽位、front 每轮状态、报告归因）任一漂移
都会**静默改变判卷**。所以每一步都要：

* `scripts/kernel-diff.sh`（判定逐字节对拍）；
* 课程门禁计数**逐项不变**（36 目标 · 328 checked · 99 open · 0 判负）；
* `crates/front/tests/prelude_shape.rs` 全绿；
* 性能账进 `docs/perf/ledger.jsonl`。
