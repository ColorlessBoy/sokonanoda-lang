# 设计：I8 真增量 + I9 goal 视图深化 + 工程达标（2026-09-07）

> 依据：4 份并行调研（代码现状审计 / LSP 增量业界实践 / goal 视图 UX / VSCode+CI 标准）。
> 结论先行：**学 Lean4+coq-lsp 的"前缀精确复用 + 变化点后保守重算"，不引入 salsa；
> tactic 判定学 Lean PR #7192 的"建议生成时就过 kernel"；LSP 自定义请求学 coq-lsp
> `proof/goals` 模式（tower-lsp `custom_method` 直接支持）。**

## 1. I8 —— 真增量（Session 级快照复用，零内核改动）

### 业界结论
- Lean4 server：每命令快照；复用不变式 = "语法不变 ⇒ 起始态相同 ⇒ 结果可复用"；
  从第一条变化的命令重启 elaboration；诊断按版本丢弃。
- coq-lsp/Flèche：句子级 trimmed-hash 对齐，位置无关 memoization。
- salsa/rust-analyzer 的 query 图对单文件小编译器过重；L1（前缀复用）即可达成验收。

### 方案（P1）
`crates/front` 新增增量机制，`crates/kernel` **一行不动**：

1. `run_pass` 增加 `TrustPlan { before, after, exclude }`：
   - `[0, before)` 的命令**照常 elaborate + 入环境，但跳过内核重查**（与 prelude
     "受信任安装"同机制）——内核检查是贵的那一半；
   - `exclude`：信任区里缓存状态为 Failed 的命令，**不入环境**（镜像 pass2 的
     check-then-add 语义），状态从缓存原样复用；
   - `[after, …)` 的命令完全忽略（judge 模式用）。
2. `run_pass` 产出 `PassArtifacts`：每命令的 DeclState / hovers / events /
   `event_cmds`（事件→命令归属）与 `stats.kernel_checks`（try_check_declar 计数）。
3. `Session` 快照升级：每命令存 `DeclKey{text, start}` + `CmdSnapshot{state,
   hovers, events}`。update 流程：
   - keys 全等 → 零重编译：复用全部快照，**修复既有潜在 bug**——注释/空白编辑
     造成的 span 漂移按"新命令起点 − 旧命令起点"重映射 hovers/events；
   - `first_diff = i`（索引对齐，保守且可靠：它恒 ≤ 真实公共前缀）→ `[0,i)`
     信任复用（此路径前缀 offset 天然不变，无需重映射），`[i,…)` 全量重查；
   - prelude 模式变化 → 重建 Session 全量重编译（决策依赖整文件内容，语义与
     全量严格一致：explicit-Nat 探测、Eq all-or-nothing 都看整文件）。
4. 公开 API：`SessionUpdate` 增加 `stats: CompileStats{kernel_checks}`；
   `recompiled_from` 语义变为"真正首个被重查的命令索引"。
5. **正确性论证**：同文本命令 + 同 prelude 决策 ⇒ elaborate/内核结果确定性一致；
   prelude 决策基于整文件内容（两种路径都可见），故信任复用与全量重编译严格等价。
   失败声明不入环境，依赖者得到真实 unknown-identifier（与 I8a 一致）。

### 验收
- front 单测：改第 i 个声明 → `stats.kernel_checks == n-i`；中间插入/删除声明
  只信任公共前缀；注释编辑零重编译且 hover 位置正确。
- LSP 切到 Session（消灭每编辑 2×2 遍流水线），既有 LSP 测试全绿。

## 2. I9 —— goal 视图深化：tactic 判定 kernel 化

### 业界结论
- Lean `exact?` 的教训（PR #7192）：建议必须在生成时经 kernel/elaborator 验证，
  无效建议不进 code action。
- coq-lsp：证明状态用自定义请求 `proof/goals` 承载（结构化 JSON + 版本号）。
- ocaml-lsp：next-hole 的位置计算放 server 端（"position based tricks are
  frequent footguns"）。

### 方案（as-built：比设计稿更简，零 kernel 原语）
**judge = 合成完整声明走标准流水线**。不新增 kernel API——把"候选术语 +
已写 binders + 剩余目标"折叠成一条完整声明

```text
def _soko_judge_k {与声明相同的宇宙参数} : forall (b1 : T1) …, 剩余目标
                   := fun (b1 : T1) => … => 术语
```

交给 `check_document_with`（含 prelude 决策与 check-then-add 语义），kernel
就是裁判：通过 = Match；kernel 拒绝且带 def_eq expected/actual = Mismatch；
elab 失败 = Error（稳定错误码 + 教学提示）。多个术语一次流水线批量判定。

- 规格来源：LSP 直接用 `DeclState.goal`（剩余目标）+ `DeclState.binders`；
  REPL 用 `ProofState.goal_text()` + binders——两调用方零适配。
- binder 折叠成**一个 Forall 望远镜**（依赖 binder 类型在同一 telescope 内
  elaborate），binder 名字/显隐风格不影响 kernel 检查。
- 接入点：
  - LSP `exact`：文本比对 → `judge_terms`（一次性判定全部假设）；不匹配不出
    action（Lean 教训）。`Not a` vs `a -> False` 这类 definitional-equal
    假设现在能被正确识别（新增端到端测试守护）；
  - REPL `#prove exact/apply/assumption`：`exact_kernel` 失败时反馈
    "类型不匹配：期望 X，实际是 Y"（I9 验收闭环）；
  - `proof.rs::assumption` 的文本比对已删除（REQUIREMENTS §2.8 清账）。
- goal 视图协议（tower-lsp `custom_method`，见 docs/protocol.md）：
  - `soko/goals`：全文件开放声明列表（name/kind/status/range/goal/binders/
    holeRange），多洞天然支持，hover 为降级渲染路径；
  - `soko/nextHole`：光标 + 方向 → server 计算下一个洞的 range。
- 已知限制：判定规格暂不携带声明的宇宙参数，带 `{u}` 的开放声明（如
  playground 的 Eq.symm 练习）暂无 exact 建议（intro 不受影响）。

### 附带发现：内核 conv 快路径 soundness 修复
实现 judge 的端到端测试暴露了一个**上游内核 bug**：conv 的 Pi/Pi body-expr
快路径（`unify_direct`）在闭包体表达式指针相等时直接判等，但 eval 闭包
（`ctx: None`，应用=求值体）与 infer 闭包（`mk_infer`，应用=惰性推断体的
类型）语义不同——同一 interned `Var 0` 体在两种闭包下分别表示 `$0` 与
`Sort 1`。后果：`(A : Sort 1) -> A`（不可居住类型）被
`fun (A : Sort 1) => A` 通过（官方 Lean 拒绝）。

修复：`unify_direct` 的 Pi 与 Lam 快路径增加 `closure_ctxs_compatible`
守卫（eval/eval 恒真；infer/infer 需 ctx 指针相等；混合 → 走完整比较）。
热路径仅增加一个 Option 判别分支；perf 冒烟测试无回归。回归测试：
kernel `dependent_codomain_is_not_inhabited_by_identity_lambda`（拒绝）+
`identity_over_sort_still_checks`（对照不过度拒绝）+ CLI 端到端两条。

### 验收
- kernel 单测：conv 回归 2 条；kernel 全量 43 测试全绿；
- front 单测：judge 8 条（匹配/不匹配含期望与实际/依赖 binder/defeq 不同文本/
  Bare 模式/前缀失败声明/宇宙参数）；session 增量 5 条；
- LSP 测试：goals 请求、nextHole 三向导航、defeq-exact 端到端；
- CLI 端到端：不可居住依赖 codomain 被拒绝 + 非依赖身份通过。

## 3. 工程达标（业内标准，本轮先行项）

- CI：`cargo fmt --check` → `cargo clippy -D warnings` → `cargo test`；
  `actions/cache` 换 `Swatinem/rust-cache@v2`；保留语料/协议/golden 步骤。
- VS Code 打包 P0（调研发现的真隐患）：`vscode-languageclient` 从
  devDependencies 移到 dependencies（否则 VSIX 装上即坏）；补 repository/LICENSE/
  CHANGELOG/.vscodeignore；`npx @vscode/vsce package` 冒烟。
- 扩展集成测试（@vscode/test-electron）、发布流水线：下轮。

## 4. 明确不做（本轮）
- 长驻 arena + append-only 环境复用（P2，kernel 需 resume API，收益边际）；
- salsa / 名字级依赖图 / early-cutoff 签名比较（L2/L3 优化）；
- INCREMENTAL sync（FULL + 服务端全文 diff 已够；切换与失效逻辑解耦）；
- webview goal 面板、fileProgress 通知流。
