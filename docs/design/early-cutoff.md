# 设计 / as-built：Session early-cutoff（I8 余项，2026-09-14）

> 触发：ROADMAP I8 验收余项「受影响后缀的依赖精确化（当前为保守 suffix；
> early-cutoff 签名比较是可选优化）」。`docs/design/i8-i9.md` §4 原列
> 「明确不做」，本轮补上**保守、sound** 的版本。

## 1. 现状

`Session::update` 在命令 `i` 文本变化时，从 `i` 起重新内核检查整段后缀
`i..n`（`recompiled_from = i`，`stats.kernel_checks` 计后缀长度）。对「改一处、
后面都没引用它」的常见编辑，这是浪费。

## 2. 机制：环境贡献签名

- 每条命令在 `try_check_declar` **之前**，由已 elaborate 的 `Declar` +
  `ExportFile` 用内核的**结构化 `debug_print`** 渲染出「环境贡献签名」
  （`check.rs::declar_signature`/`inductive_signature`）：kind + name +
  声明宇宙 + type + **body**（def/theorem/opaque）+ reducibility hint +
  构造子/recursor/iota 元数据；归纳块把该块所有已装 declar 串联。
- 签名存进内部 `CmdSnapshot.signature`（**不改** `--json`/LSP 形状）。
  内核拒绝的命令签名为 `None`（check-then-add：名字保持自由）。
- 更新时经 `TrustPlan { prev_signatures, text_unchanged, allow_cutoff }` 传入。

## 3. Cutoff 算法（保守）

- 仅当**命令数不变**且 `first_diff` 之后的命令**源码逐字节不变**（单点编辑对
  齐）时启用；否则 `allow_cutoff=false`，退回旧后缀重查。
- 重查变化命令 `i` 起，累积 `[i, j)` 的贡献签名；遇到第一个 `j > i`（文本
  不变）且累积签名与上一 session 相同 → 停止；`[j, n)` 快照复用（重映射 span），
  后续内核检查**跳过**。
- **任何内核拒绝**都禁用本轮 cutoff（pass-1 环境含尚未被拒的声明，不可靠），
  pass 2 关闭 cutoff 重跑。
- `prelude_shape` 守卫（prelude 模式 / 显式 `inductive Nat` / `Eq` all-or-nothing）
  变化 → 整文件重建。

## 4. Soundness

`j` 之前的环境 = prelude + `[0, j)` 贡献；前缀文本未变且上轮已查、prelude
形状有守卫、fresh `[i, j)` 签名逐条比对。相同 ⇒ `j` 之前环境逐项一致 ⇒
确定性内核对文本未变的 `[j, n)` 结果相同，复用成立。**body 进签名**：delta
展开（`#reduce`）与 `#print` 可观察（测试：改 def body 翻转 `#reduce` 结果）。
结构化 `debug_print` 保证「签名相等 ⇒ elaborate 后的声明相同」；binder 名差异
只导致多查，不会假相等。

## 5. 测试（`session.rs`）

| 用例 | kernel_checks |
|---|---|
| `def one := 1` → `(1)`（4 defs） | 4 → **1** |
| `def one := 1` → `2`（改 body，含 `#reduce`） | 2（不 cut；reduce 1→2） |
| `axiom A : Prop` → `(Prop)`（3 axioms） | 3 → **1** |
| `def id {u}` → `def id {u, v}`（宇宙元数变化） | 2（不 cut） |
| Nat 归纳块 + 2 defs，等价改 one | 6 → **1** |
| 两处同时等价改动 | 3（不 cut） |
| 末条 axiom 改名 `Eq`（prelude 形状变化） | 重建，`recompiled_from == 0` |

既有 291 条 front 测试全绿（合计 298）；perf sentinel 不回退。

## 6. 边界（未做）

- 仅单点编辑；多处编辑/插入删除退回保守后缀重查；
- 无名字级依赖图：改任何 def body 会重查整个剩余后缀（除非签名恰好相等）；
  theorem/opaque body 保守纳入（不做 proof-irrelevance cutoff）；
- 归纳块原子：块内任一改动重查块及其后缀。
