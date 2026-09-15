# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-14（第六十四轮：发布加固——SHA256SUMS + SLSA provenance；0.35.1）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-14，第六十四轮：发布加固）

> 续 TODO（用户指定顺序：先 match 递归 IH，再发布加固）：`onboarding.md §5`
> 剩余的发布完整性三项。

1. **`SHA256SUMS`**：`release.yml` 的 `github-release` job 对 8 lsp + 8 cli +
   9 vsix 生成校验和清单并 `--clobber` 上传（资产 25 → **26**）。
2. **SLSA provenance**：`actions/attest-build-provenance@v2` 对上述资产签发
   构建来源证明；job 加 `id-token: write` + `attestations: write`。校验
   `gh attestation verify <file> -R ColorlessBoy/sokonanoda-lang`。
3. **文档**：`docs/RELEASE.md` §6（校验与证明）、`skills/sokonanoda-ci` §2.1
   资产数 26、`onboarding.md §5` 三项勾选（含 `rust-toolchain` 决策：**不钉**，
   跟随 stable；README 补 binstall/mise）。
4. **契约**：`crates/cli/tests/extension.rs` release 契约增 `SHA256SUMS` /
   `attest-build-provenance@v2` / `attestations: write` 断言。
5. **验收**：`sokonanoda gate` PASS；版本 0.35.0 → **0.35.1**；发布后核
   Release 26 资产 + `sha256sum -c` + `gh attestation verify` 通过。

## 本轮进度（2026-09-14，第六十三轮：`match` 递归归纳（IH））

> 续 TODO（用户指定顺序：先 match Phase 2 递归 IH，再发布加固）。

1. **前端**：递归归纳不再一律拒绝；构造子**递归字段后自动插入归纳假设** `ih`
   （避开既有名 → `ih2`…），类型 = match 结果类型 R（v1 非依赖 motive），push 进
   branch scope 供引用；minor 以「字段 + IH」序列折 lambda。递归函数/证明经 IH
   表达，**无需自引用**（`def add (a b : Nat) := match a with | zero => b |
   succ m => succ ih`）。
2. **测试**：front `match_recursive_inductive_uses_the_induction_hypothesis`
   （`add two two` 经内核归约到 `s (s (s (s z)))`）；CLI recursive match 3 项。
3. **课程**：unit5 `match` 小节加递归 IH 演示 + 练习 6（`recDouble`）+ `#reduce`
   自测；golden `(6,5,2)→(7,6,3)`、汇总 `checked 50→51 / open 38→39`。
4. **文档**：`match.md` §2/§5/§6/§10、`architecture.md` §4.1/§8、`TESTING.md` 同步。
5. **验收**：`sokonanoda gate` PASS；版本 0.34.0 → **0.35.0**（新能力 minor）。
6. **仍缺（Phase 2 余项）**：依赖 motive、参数化/带索引归纳、prelude `Nat`/`Eq`、
   `match` tactic、嵌套/字面量/守卫模式。

## 本轮进度（2026-09-14，第六十二轮：无注解 `let`）

> 续 TODO 清账：把 `let` 的类型标注变为可选（Phase 2 小切片）。

1. **实现**：`Expr::Let` 的 `binder.ty == None` 时，用当前 scope 的
   `judge_binders()` + `judge_infer`（复用有界缓存）推断值类型、回 AST 作 binder
   类型（`match` 轮已把 `ElabCtx{prefix,options}` 贯通进 elab，正好复用）。
   推断失败（值位 `sorry` 等）→ 新码 `elab-let-type-query-failed`（protocol +
   穷尽清单 + hint 同步）。
2. **测试**：front `let_without_annotation_infers_the_value_type` +
   `unannotated_let_that_cannot_be_inferred_reports_a_let_specific_error`；CLI
   `json_mode_unannotated_let_infers_or_reports_hint`；课程 unit3 注释更新。
3. **验收**：`sokonanoda gate` PASS；版本 0.33.1 → **0.34.0**（新能力 minor）；
   设计 as-built `docs/design/elaborator-let-match.md` §13。
